use crate::destination::validate_destination;
use crate::engine::AcquisitionEngine;
use crate::models::{
    AcquisitionArtifact, AcquisitionDeviceSnapshot, AcquisitionFailureReason, AcquisitionPlan,
    AcquisitionProgress, AcquisitionResult, AcquisitionStatus,
};
use crate::platform::{AcquisitionSourceReader, RealAcquisitionReader};
use crate::source::{create_source_snapshot, revalidate_source_snapshot};
use locardx_audit::AuditService;
use locardx_common::LocardError;
use locardx_database::Database;
use rusqlite::params;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tracing::warn;

/// Coordinates forensic disk acquisition workflows, safety validations, persistence, and audit logging.
pub struct AcquisitionService {
    db: Arc<Database>,
    audit: Arc<AuditService>,
    reader: Arc<dyn AcquisitionSourceReader>,
    active_cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    latest_progress: Arc<Mutex<HashMap<String, AcquisitionProgress>>>,
}

impl AcquisitionService {
    pub fn new(db: Arc<Database>, audit: Arc<AuditService>) -> Self {
        Self {
            db,
            audit,
            reader: Arc::new(RealAcquisitionReader::new()),
            active_cancellations: Arc::new(Mutex::new(HashMap::new())),
            latest_progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_reader(mut self, reader: Arc<dyn AcquisitionSourceReader>) -> Self {
        self.reader = reader;
        self
    }

    pub fn reader(&self) -> &Arc<dyn AcquisitionSourceReader> {
        &self.reader
    }

    /// Evaluates pre-flight conditions and generates an immutable AcquisitionPlan.
    pub fn create_plan(
        &self,
        source_device_id: &str,
        destination_path: &str,
        allow_overwrite: bool,
        actor_id: Option<&str>,
    ) -> Result<AcquisitionPlan, LocardError> {
        let snapshot = create_source_snapshot(self.reader.as_ref(), source_device_id)
            .map_err(|e| LocardError::Operation(e.to_string()))?;

        let validated_dest = validate_destination(
            self.reader.as_ref(),
            &snapshot.device_id,
            destination_path,
            snapshot.capacity_bytes,
            allow_overwrite,
        )
        .map_err(|e| LocardError::Operation(e.to_string()))?;

        let plan_id = format!("plan-acq-{}", uuid::Uuid::new_v4());
        let plan = AcquisitionPlan {
            plan_id: plan_id.clone(),
            source: snapshot.clone(),
            destination_path: validated_dest
                .destination_path
                .to_string_lossy()
                .to_string(),
            image_format: "raw".to_string(),
            chunk_size_bytes: 1024 * 1024, // 1 MiB chunk
            required_space_bytes: snapshot.capacity_bytes,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let details = format!(
            "Forensic acquisition planned for source {} ({} bytes) -> {}",
            plan.source.device_id, plan.source.capacity_bytes, plan.destination_path
        );

        let _ = self.audit.log_structured_event(
            "ACQUISITION_PLANNED",
            actor_id,
            Some(&plan.source.device_id),
            &details,
        );

        Ok(plan)
    }

    /// Executes a planned acquisition strictly read-only, streaming SHA-256 and writing to raw/dd image.
    pub fn execute_acquisition(
        &self,
        operation_id: &str,
        plan: &AcquisitionPlan,
        actor_id: Option<&str>,
    ) -> Result<AcquisitionResult, LocardError> {
        let acquisition_id = format!("acq-{}", uuid::Uuid::new_v4());
        let started_at = chrono::Utc::now().to_rfc3339();

        // 1. Setup cancellation flag
        let cancel_flag = Arc::new(AtomicBool::new(false));
        {
            let mut map = self.active_cancellations.lock().unwrap();
            map.insert(operation_id.to_string(), Arc::clone(&cancel_flag));
        }

        // 2. TOCTOU Defense: re-probe live source hardware state
        if let Err(e) = revalidate_source_snapshot(self.reader.as_ref(), &plan.source) {
            let details = format!("Acquisition aborted due to TOCTOU source mutation: {}", e);
            let _ = self.audit.log_structured_event(
                "ACQUISITION_SOURCE_MUTATED",
                actor_id,
                Some(&plan.source.device_id),
                &details,
            );

            let res = AcquisitionResult {
                acquisition_id,
                operation_id: operation_id.to_string(),
                source: plan.source.clone(),
                destination_path: plan.destination_path.clone(),
                image_format: plan.image_format.clone(),
                image_size_bytes: 0,
                image_sha256: String::new(),
                status: AcquisitionStatus::Failed,
                bytes_acquired: 0,
                elapsed_seconds: 0.0,
                average_throughput_mbps: 0.0,
                failure_reason: Some(e),
                audit_reference: "ACQUISITION_SOURCE_MUTATED".to_string(),
                started_at,
                completed_at: chrono::Utc::now().to_rfc3339(),
            };
            self.persist_result(&res, actor_id)?;
            return Ok(res);
        }

        let _ = self.audit.log_structured_event(
            "ACQUISITION_STARTED",
            actor_id,
            Some(&plan.source.device_id),
            &format!(
                "Beginning read-only physical acquisition of {}",
                plan.source.device_id
            ),
        );

        // 3. Open source device strictly read-only
        let stream = match self.reader.open_read_only(&plan.source.device_id) {
            Ok(s) => s,
            Err(e) => {
                let _ = self.audit.log_structured_event(
                    "ACQUISITION_FAILED",
                    actor_id,
                    Some(&plan.source.device_id),
                    &format!("Failed opening source {}: {}", plan.source.device_id, e),
                );

                let res = AcquisitionResult {
                    acquisition_id,
                    operation_id: operation_id.to_string(),
                    source: plan.source.clone(),
                    destination_path: plan.destination_path.clone(),
                    image_format: plan.image_format.clone(),
                    image_size_bytes: 0,
                    image_sha256: String::new(),
                    status: AcquisitionStatus::Failed,
                    bytes_acquired: 0,
                    elapsed_seconds: 0.0,
                    average_throughput_mbps: 0.0,
                    failure_reason: Some(e),
                    audit_reference: "ACQUISITION_FAILED".to_string(),
                    started_at,
                    completed_at: chrono::Utc::now().to_rfc3339(),
                };
                self.persist_result(&res, actor_id)?;
                return Ok(res);
            }
        };

        // 4. Run acquisition engine with progress callback
        let progress_store = Arc::clone(&self.latest_progress);
        let op_id_owned = operation_id.to_string();

        let is_cancelled = {
            let flag = Arc::clone(&cancel_flag);
            move || flag.load(Ordering::Relaxed)
        };

        let on_progress = move |prog: AcquisitionProgress| {
            let mut store = progress_store.lock().unwrap();
            store.insert(op_id_owned.clone(), prog);
        };

        let engine_res = AcquisitionEngine::execute(
            operation_id,
            stream,
            Path::new(&plan.destination_path),
            plan.chunk_size_bytes,
            plan.source.capacity_bytes,
            Some(&is_cancelled),
            Some(&on_progress),
        );

        // Clean up cancellation flag
        {
            let mut map = self.active_cancellations.lock().unwrap();
            map.remove(operation_id);
        }

        let completed_at = chrono::Utc::now().to_rfc3339();

        let result = match engine_res {
            Ok(out) => {
                let avg_throughput = if out.elapsed_seconds > 0.0 {
                    (out.bytes_acquired as f64 / (1024.0 * 1024.0)) / out.elapsed_seconds
                } else {
                    0.0
                };

                let audit_ref = self
                    .audit
                    .log_structured_event(
                        "ACQUISITION_COMPLETED",
                        actor_id,
                        Some(&plan.source.device_id),
                        &format!(
                            "Acquisition completed: {} bytes acquired, SHA-256: {}",
                            out.bytes_acquired, out.image_sha256
                        ),
                    )
                    .map(|ev| ev.current_hash)
                    .unwrap_or_else(|_| "ACQUISITION_COMPLETED".to_string());

                let _ = self.audit.log_structured_event(
                    "ACQUISITION_INTEGRITY_VERIFIED",
                    actor_id,
                    Some(&plan.destination_path),
                    &format!("SHA-256 integrity verified for image: {}", out.image_sha256),
                );

                AcquisitionResult {
                    acquisition_id,
                    operation_id: operation_id.to_string(),
                    source: plan.source.clone(),
                    destination_path: plan.destination_path.clone(),
                    image_format: plan.image_format.clone(),
                    image_size_bytes: out.image_size_bytes,
                    image_sha256: out.image_sha256,
                    status: AcquisitionStatus::Completed,
                    bytes_acquired: out.bytes_acquired,
                    elapsed_seconds: out.elapsed_seconds,
                    average_throughput_mbps: (avg_throughput * 10.0).round() / 10.0,
                    failure_reason: None,
                    audit_reference: audit_ref,
                    started_at,
                    completed_at,
                }
            }
            Err(AcquisitionFailureReason::Cancelled) => {
                let audit_ref = self
                    .audit
                    .log_structured_event(
                        "ACQUISITION_CANCELLED",
                        actor_id,
                        Some(&plan.source.device_id),
                        &format!(
                            "Acquisition cancelled by operator for {}",
                            plan.source.device_id
                        ),
                    )
                    .map(|ev| ev.current_hash)
                    .unwrap_or_else(|_| "ACQUISITION_CANCELLED".to_string());

                AcquisitionResult {
                    acquisition_id,
                    operation_id: operation_id.to_string(),
                    source: plan.source.clone(),
                    destination_path: plan.destination_path.clone(),
                    image_format: plan.image_format.clone(),
                    image_size_bytes: 0,
                    image_sha256: String::new(),
                    status: AcquisitionStatus::Cancelled,
                    bytes_acquired: 0,
                    elapsed_seconds: 0.0,
                    average_throughput_mbps: 0.0,
                    failure_reason: Some(AcquisitionFailureReason::Cancelled),
                    audit_reference: audit_ref,
                    started_at,
                    completed_at,
                }
            }
            Err(e) => {
                let audit_ref = self
                    .audit
                    .log_structured_event(
                        "ACQUISITION_FAILED",
                        actor_id,
                        Some(&plan.source.device_id),
                        &format!("Acquisition failed for {}: {}", plan.source.device_id, e),
                    )
                    .map(|ev| ev.current_hash)
                    .unwrap_or_else(|_| "ACQUISITION_FAILED".to_string());

                AcquisitionResult {
                    acquisition_id,
                    operation_id: operation_id.to_string(),
                    source: plan.source.clone(),
                    destination_path: plan.destination_path.clone(),
                    image_format: plan.image_format.clone(),
                    image_size_bytes: 0,
                    image_sha256: String::new(),
                    status: AcquisitionStatus::Failed,
                    bytes_acquired: 0,
                    elapsed_seconds: 0.0,
                    average_throughput_mbps: 0.0,
                    failure_reason: Some(e),
                    audit_reference: audit_ref,
                    started_at,
                    completed_at,
                }
            }
        };

        self.persist_result(&result, actor_id)?;
        Ok(result)
    }

    /// Signals cancellation for an active acquisition operation.
    pub fn cancel_acquisition(&self, operation_id: &str) -> bool {
        let map = self.active_cancellations.lock().unwrap();
        if let Some(flag) = map.get(operation_id) {
            flag.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Retrieves the most recent progress report for an active operation.
    pub fn get_progress(&self, operation_id: &str) -> Option<AcquisitionProgress> {
        let store = self.latest_progress.lock().unwrap();
        store.get(operation_id).cloned()
    }

    /// Persists an acquisition result into SQLite `acquisition_records`.
    pub fn persist_result(
        &self,
        result: &AcquisitionResult,
        actor_id: Option<&str>,
    ) -> Result<(), LocardError> {
        let snapshot_json = serde_json::to_string(&result.source).map_err(|e| {
            LocardError::Database(format!("Failed to serialize source snapshot: {}", e))
        })?;

        let failure_json = result
            .failure_reason
            .as_ref()
            .and_then(|r| serde_json::to_string(r).ok());

        let status_str = result.status.to_string();

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO acquisition_records (
                    acquisition_id, operation_id, actor_id,
                    source_device_id, source_display_name, source_vendor, source_model,
                    source_serial, source_media_type, source_capacity_bytes, source_sector_size,
                    source_bus_type, source_snapshot_json, destination_path, image_format,
                    image_size_bytes, image_sha256, status, bytes_acquired, elapsed_seconds,
                    failure_reason, audit_reference, started_at, completed_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                    ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
                )",
                params![
                    result.acquisition_id,
                    result.operation_id,
                    actor_id,
                    result.source.device_id,
                    result.source.display_name,
                    result.source.vendor,
                    result.source.model,
                    result.source.serial_number,
                    result.source.media_type,
                    result.source.capacity_bytes as i64,
                    result.source.sector_size as i64,
                    result.source.bus_type,
                    snapshot_json,
                    result.destination_path,
                    result.image_format,
                    result.image_size_bytes as i64,
                    result.image_sha256,
                    status_str,
                    result.bytes_acquired as i64,
                    result.elapsed_seconds,
                    failure_json,
                    result.audit_reference,
                    result.started_at,
                    result.completed_at,
                ],
            )?;
            Ok(())
        })
    }

    /// Queries an acquisition record by operation ID.
    pub fn get_acquisition_result(
        &self,
        operation_id: &str,
    ) -> Result<Option<AcquisitionResult>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT acquisition_id, operation_id, source_snapshot_json, destination_path,
                        image_format, image_size_bytes, image_sha256, status, bytes_acquired,
                        elapsed_seconds, failure_reason, audit_reference, started_at, completed_at
                 FROM acquisition_records WHERE operation_id = ?1",
            )?;

            let mut rows = stmt.query(params![operation_id])?;
            if let Some(row) = rows.next()? {
                let acquisition_id: String = row.get(0)?;
                let operation_id: String = row.get(1)?;
                let snapshot_json: String = row.get(2)?;
                let destination_path: String = row.get(3)?;
                let image_format: String = row.get(4)?;
                let image_size_bytes: i64 = row.get(5)?;
                let image_sha256: String = row.get(6)?;
                let status_str: String = row.get(7)?;
                let bytes_acquired: i64 = row.get(8)?;
                let elapsed_seconds: f64 = row.get(9)?;
                let failure_json: Option<String> = row.get(10)?;
                let audit_reference: String = row.get(11)?;
                let started_at: String = row.get(12)?;
                let completed_at: String = row.get(13)?;

                let source: AcquisitionDeviceSnapshot = serde_json::from_str(&snapshot_json)
                    .unwrap_or_else(|_| AcquisitionDeviceSnapshot {
                        device_id: "UNKNOWN".to_string(),
                        display_name: "Unknown / Legacy Device".to_string(),
                        vendor: None,
                        model: None,
                        serial_number: None,
                        media_type: "Unknown".to_string(),
                        capacity_bytes: 0,
                        sector_size: 512,
                        bus_type: None,
                        is_removable: false,
                        is_system: false,
                        snapshot_timestamp: String::new(),
                    });

                let status = status_str.parse::<AcquisitionStatus>().map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        7,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                let failure_reason = failure_json
                    .and_then(|j| serde_json::from_str::<AcquisitionFailureReason>(&j).ok());

                let avg_throughput = if elapsed_seconds > 0.0 {
                    (bytes_acquired as f64 / (1024.0 * 1024.0)) / elapsed_seconds
                } else {
                    0.0
                };

                Ok(Some(AcquisitionResult {
                    acquisition_id,
                    operation_id,
                    source,
                    destination_path,
                    image_format,
                    image_size_bytes: image_size_bytes as u64,
                    image_sha256,
                    status,
                    bytes_acquired: bytes_acquired as u64,
                    elapsed_seconds,
                    average_throughput_mbps: (avg_throughput * 10.0).round() / 10.0,
                    failure_reason,
                    audit_reference,
                    started_at,
                    completed_at,
                }))
            } else {
                Ok(None)
            }
        })
    }

    /// Constructs the Recovery-ready AcquisitionArtifact from a completed acquisition.
    pub fn get_artifact(
        &self,
        operation_id: &str,
    ) -> Result<Option<AcquisitionArtifact>, LocardError> {
        let result_opt = self.get_acquisition_result(operation_id)?;
        if let Some(res) = result_opt {
            let artifact = AcquisitionArtifact::from_result(&res)
                .map_err(|e| LocardError::Operation(e.to_string()))?;
            Ok(Some(artifact))
        } else {
            Ok(None)
        }
    }

    /// Startup crash recovery sweep: detects any incomplete acquisitions left in-flight
    /// by unexpected process termination, marking them Failed to uphold the fail-closed invariant.
    pub fn recover_interrupted_acquisitions(&self) -> Result<usize, LocardError> {
        let now = chrono::Utc::now().to_rfc3339();
        let failure_reason_json = serde_json::to_string(&AcquisitionFailureReason::Interrupted)
            .unwrap_or_else(|_| r#"{"reason":"Interrupted"}"#.to_string());

        let interrupted_ops: Vec<String> = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT operation_id FROM acquisition_records
                 WHERE status IN ('Planned', 'Validating', 'Acquiring')",
            )?;
            let rows = stmt.query_map([], |row| row.get(0))?;
            let mut ids = Vec::new();
            for id in rows {
                ids.push(id?);
            }
            Ok(ids)
        })?;

        if interrupted_ops.is_empty() {
            return Ok(0);
        }

        let count = interrupted_ops.len();
        self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE acquisition_records
                 SET status = 'Failed',
                     failure_reason = ?1,
                     completed_at = ?2
                 WHERE status IN ('Planned', 'Validating', 'Acquiring')",
                params![failure_reason_json, now],
            )?;
            Ok(())
        })?;

        for op_id in &interrupted_ops {
            let _ = self.audit.log_structured_event(
                "OPERATION_INTERRUPTED_RECOVERED",
                None,
                Some(op_id),
                &format!(
                    "Forensic acquisition operation '{}' was interrupted by abnormal process termination. Marked Failed (fail-closed).",
                    op_id
                ),
            );
        }

        warn!(
            recovered_count = count,
            "Recovered interrupted forensic acquisitions on startup"
        );

        Ok(count)
    }
}
