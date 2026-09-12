use crate::engine::RecoveryEngine;
use crate::models::{
    ConfidenceGrade, FileType, RecoveredFile, RecoveryFailureReason, RecoveryOptions, RecoveryPlan,
    RecoveryProgress, RecoveryReport, RecoveryResult, RecoveryStatus, ValidationStatus,
};
use crate::source::{validate_acquisition_artifact, RecoverySourceSnapshot};
use chrono::Utc;
use locardx_acquisition::AcquisitionArtifact;
use locardx_audit::AuditService;
use locardx_common::LocardError;
use locardx_database::Database;
use rusqlite::params;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Coordinates recovery workflows, database persistence, and hash-chained audit logging.
pub struct RecoveryService {
    db: Arc<Database>,
    audit: Arc<AuditService>,
    active_cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    latest_progress: Arc<Mutex<HashMap<String, RecoveryProgress>>>,
}

impl RecoveryService {
    pub fn new(db: Arc<Database>, audit: Arc<AuditService>) -> Self {
        Self {
            db,
            audit,
            active_cancellations: Arc::new(Mutex::new(HashMap::new())),
            latest_progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Validates an AcquisitionArtifact and registers it in the recovery sources registry.
    pub fn validate_source(
        &self,
        artifact: &AcquisitionArtifact,
        actor_id: Option<&str>,
    ) -> Result<RecoverySourceSnapshot, LocardError> {
        let snapshot = validate_acquisition_artifact(artifact).map_err(|e| {
            LocardError::Operation(format!("Failed to validate acquisition artifact: {}", e))
        })?;

        // Persist validated source into database
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO recovery_sources (
                    source_id, acquisition_id, image_path, image_size_bytes, image_sha256,
                    original_device_id, original_serial, verified_at, is_trusted
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    snapshot.source_id,
                    snapshot.acquisition_id,
                    snapshot.image_path,
                    snapshot.image_size_bytes as i64,
                    snapshot.image_sha256,
                    snapshot.original_device_id,
                    snapshot.original_serial,
                    snapshot.verified_at,
                    if snapshot.is_trusted { 1 } else { 0 },
                ],
            )?;
            Ok(())
        })?;

        let details = format!(
            "Recovery source validated for acquisition {}: image {} ({} bytes, sha256={})",
            snapshot.acquisition_id,
            snapshot.image_path,
            snapshot.image_size_bytes,
            snapshot.image_sha256
        );

        let _ = self.audit.log_structured_event(
            "RECOVERY_SOURCE_VALIDATED",
            actor_id,
            Some(&snapshot.acquisition_id),
            &details,
        );

        Ok(snapshot)
    }

    /// Evaluates pre-flight conditions and generates an immutable RecoveryPlan.
    pub fn create_plan(
        &self,
        artifact: &AcquisitionArtifact,
        options: RecoveryOptions,
        actor_id: Option<&str>,
    ) -> Result<RecoveryPlan, LocardError> {
        let snapshot = self.validate_source(artifact, actor_id)?;

        let plan_id = format!("plan-rec-{}", Uuid::new_v4());
        let plan = RecoveryPlan {
            plan_id: plan_id.clone(),
            acquisition_id: snapshot.acquisition_id.clone(),
            source_image_path: snapshot.image_path.clone(),
            source_image_sha256: snapshot.image_sha256.clone(),
            options,
            created_at: Utc::now().to_rfc3339(),
        };

        let details = format!(
            "Recovery plan created: mode={:?}, target_source={}, min_confidence={}",
            plan.options.recovery_mode, plan.source_image_path, plan.options.min_confidence_score
        );

        let _ = self.audit.log_structured_event(
            "RECOVERY_PLANNED",
            actor_id,
            Some(&plan.acquisition_id),
            &details,
        );

        Ok(plan)
    }

    /// Executes recovery synchronously, recording telemetry, persisting outputs, and generating reports.
    pub fn execute_recovery(
        &self,
        plan: &RecoveryPlan,
        actor_id: Option<&str>,
    ) -> Result<RecoveryResult, LocardError> {
        let job_id = format!("job-rec-{}", Uuid::new_v4());
        let operation_id = format!("op-rec-{}", Uuid::new_v4());

        let cancel_flag = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.active_cancellations.lock().unwrap();
            active.insert(operation_id.clone(), cancel_flag.clone());
        }

        let started_at = Utc::now().to_rfc3339();
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO recovery_jobs (
                    job_id, operation_id, actor_id, acquisition_id, source_image_path,
                    source_image_sha256, recovery_mode, status, bytes_scanned, files_recovered,
                    candidates_evaluated, elapsed_seconds, failure_reason, audit_reference,
                    started_at, completed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, 0, 0, 0.0, NULL, ?9, ?10, ?10)",
                params![
                    job_id,
                    operation_id,
                    actor_id,
                    plan.acquisition_id,
                    plan.source_image_path,
                    plan.source_image_sha256,
                    format!("{:?}", plan.options.recovery_mode),
                    "Scanning",
                    format!("audit-rec-{}", Uuid::new_v4()),
                    started_at,
                ],
            )?;
            Ok(())
        })?;

        let _ = self.audit.log_structured_event(
            "RECOVERY_STARTED",
            actor_id,
            Some(&plan.acquisition_id),
            &format!(
                "Recovery job {} started for image {}",
                job_id, plan.source_image_path
            ),
        );

        let progress_map = self.latest_progress.clone();
        let op_id_clone = operation_id.clone();

        let engine_result =
            RecoveryEngine::execute(plan, &operation_id, Some(&cancel_flag), move |prog| {
                let mut map = progress_map.lock().unwrap();
                map.insert(op_id_clone.clone(), prog);
            });

        {
            let mut active = self.active_cancellations.lock().unwrap();
            active.remove(&operation_id);
        }

        let result = match engine_result {
            Ok(mut res) => {
                res.job_id = job_id.clone();
                for f in &mut res.recovered_files {
                    f.job_id = job_id.clone();
                }
                res
            }
            Err(reason) => {
                let completed_at = Utc::now().to_rfc3339();
                let res = RecoveryResult {
                    job_id: job_id.clone(),
                    operation_id: operation_id.clone(),
                    acquisition_id: plan.acquisition_id.clone(),
                    source_image_path: plan.source_image_path.clone(),
                    source_image_sha256: plan.source_image_sha256.clone(),
                    status: RecoveryStatus::Failed,
                    bytes_scanned: 0,
                    files_recovered: 0,
                    candidates_evaluated: 0,
                    elapsed_seconds: 0.0,
                    failure_reason: Some(reason.clone()),
                    recovered_files: Vec::new(),
                    audit_reference: format!("audit-rec-{}", Uuid::new_v4()),
                    started_at: started_at.clone(),
                    completed_at,
                };

                let _ = self.db.with_conn(|conn| {
                    conn.execute(
                        "UPDATE recovery_jobs SET status = 'Failed', failure_reason = ?1 WHERE job_id = ?2",
                        params![format!("{}", reason), job_id],
                    )?;
                    Ok(())
                });

                let _ = self.audit.log_structured_event(
                    "RECOVERY_FAILED",
                    actor_id,
                    Some(&job_id),
                    &format!("Recovery job failed: {}", reason),
                );

                return Ok(res);
            }
        };

        // Persist files and update job status
        let files_to_insert = result.recovered_files.clone();
        let status_str = match result.status {
            RecoveryStatus::Completed => "Completed",
            RecoveryStatus::Cancelled => "Cancelled",
            RecoveryStatus::Failed => "Failed",
            _ => "Completed",
        };
        let reason_str = result.failure_reason.as_ref().map(|r| format!("{}", r));
        let completed_time = result.completed_at.clone();
        let b_scanned = result.bytes_scanned as i64;
        let f_rec = result.files_recovered as i64;
        let c_eval = result.candidates_evaluated as i64;
        let el_sec = result.elapsed_seconds;
        let j_id = job_id.clone();

        self.db.with_conn(|conn| {
            let tx = conn.transaction()?;

            for f in &files_to_insert {
                let factors_json = serde_json::to_string(&f.evidence_factors).unwrap_or_default();
                tx.execute(
                    "INSERT INTO recovered_files (
                        file_id, job_id, source_offset, size_bytes, file_type, mime_type,
                        suggested_filename, recovery_method, validation_status, confidence_score,
                        confidence_grade, is_fragmented, sha256_hash, output_relative_path,
                        evidence_factors_json, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                    params![
                        f.file_id,
                        f.job_id,
                        f.source_offset as i64,
                        f.size_bytes as i64,
                        format!("{}", f.file_type),
                        f.mime_type,
                        f.suggested_filename,
                        format!("{}", f.recovery_method),
                        format!("{}", f.validation_status),
                        f.confidence_score as i64,
                        format!("{}", f.confidence_grade),
                        if f.is_fragmented { 1 } else { 0 },
                        f.sha256_hash,
                        f.output_relative_path,
                        factors_json,
                        f.created_at,
                    ],
                )?;
            }

            tx.execute(
                "UPDATE recovery_jobs SET
                    status = ?1,
                    bytes_scanned = ?2,
                    files_recovered = ?3,
                    candidates_evaluated = ?4,
                    elapsed_seconds = ?5,
                    failure_reason = ?6,
                    completed_at = ?7
                WHERE job_id = ?8",
                params![
                    status_str,
                    b_scanned,
                    f_rec,
                    c_eval,
                    el_sec,
                    reason_str,
                    completed_time,
                    j_id,
                ],
            )?;

            tx.commit()?;
            Ok(())
        })?;

        let _ = self.generate_report(&job_id, actor_id);

        let event_type = match result.status {
            RecoveryStatus::Completed => "RECOVERY_COMPLETED",
            RecoveryStatus::Cancelled => "RECOVERY_CANCELLED",
            _ => "RECOVERY_FAILED",
        };

        let details = format!(
            "Recovery job {} {}: {} files recovered, {} bytes scanned, {:.2}s",
            job_id,
            result.status,
            result.files_recovered,
            result.bytes_scanned,
            result.elapsed_seconds
        );

        let _ = self
            .audit
            .log_structured_event(event_type, actor_id, Some(&job_id), &details);

        Ok(result)
    }

    /// Signals cancellation for an ongoing recovery operation.
    pub fn cancel_recovery(
        &self,
        operation_id: &str,
        actor_id: Option<&str>,
    ) -> Result<bool, LocardError> {
        let active = self.active_cancellations.lock().unwrap();
        if let Some(flag) = active.get(operation_id) {
            flag.store(true, Ordering::SeqCst);
            let _ = self.audit.log_structured_event(
                "RECOVERY_CANCEL_REQUESTED",
                actor_id,
                Some(operation_id),
                "Operator requested recovery cancellation",
            );
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Retrieves latest progress telemetry for an active operation.
    pub fn get_progress(&self, operation_id: &str) -> Option<RecoveryProgress> {
        let active = self.latest_progress.lock().unwrap();
        active.get(operation_id).cloned()
    }

    /// Retrieves a recovery job result and its recovered files.
    pub fn get_job(&self, job_id: &str) -> Result<Option<RecoveryResult>, LocardError> {
        let files = self.get_recovered_files(job_id)?;
        let j_id = job_id.to_string();

        let job_opt = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT job_id, operation_id, acquisition_id, source_image_path, source_image_sha256,
                        status, bytes_scanned, files_recovered, candidates_evaluated, elapsed_seconds,
                        failure_reason, audit_reference, started_at, completed_at
                 FROM recovery_jobs WHERE job_id = ?1",
            )?;

            let mut rows = stmt.query(params![j_id])?;

            if let Some(row) = rows.next()? {
                let status_str: String = row.get(5).unwrap_or_default();
                let status = match status_str.as_str() {
                    "Completed" => RecoveryStatus::Completed,
                    "Cancelled" => RecoveryStatus::Cancelled,
                    "Failed" => RecoveryStatus::Failed,
                    "Scanning" => RecoveryStatus::Scanning,
                    _ => RecoveryStatus::Pending,
                };

                let fail_reason: Option<String> = row.get(10).ok();

                Ok(Some(RecoveryResult {
                    job_id: row.get(0)?,
                    operation_id: row.get(1)?,
                    acquisition_id: row.get(2)?,
                    source_image_path: row.get(3)?,
                    source_image_sha256: row.get(4)?,
                    status,
                    bytes_scanned: row.get::<_, i64>(6)? as u64,
                    files_recovered: row.get::<_, i64>(7)? as usize,
                    candidates_evaluated: row.get::<_, i64>(8)? as usize,
                    elapsed_seconds: row.get(9)?,
                    failure_reason: fail_reason.map(RecoveryFailureReason::Unknown),
                    recovered_files: files,
                    audit_reference: row.get(11)?,
                    started_at: row.get(12)?,
                    completed_at: row.get(13)?,
                }))
            } else {
                Ok(None)
            }
        })?;

        Ok(job_opt)
    }

    /// Lists all recovery jobs stored in the database.
    pub fn list_jobs(&self) -> Result<Vec<RecoveryResult>, LocardError> {
        let job_ids: Vec<String> = self.db.with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT job_id FROM recovery_jobs ORDER BY started_at DESC")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            let mut ids = Vec::new();
            for id in rows {
                ids.push(id?);
            }
            Ok(ids)
        })?;

        let mut results = Vec::new();
        for id in job_ids {
            if let Some(job) = self.get_job(&id)? {
                results.push(job);
            }
        }

        Ok(results)
    }

    /// Retrieves all recovered files for a specific job.
    pub fn get_recovered_files(&self, job_id: &str) -> Result<Vec<RecoveredFile>, LocardError> {
        let j_id = job_id.to_string();
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT file_id, job_id, source_offset, size_bytes, file_type, mime_type,
                        suggested_filename, recovery_method, validation_status, confidence_score,
                        confidence_grade, is_fragmented, sha256_hash, output_relative_path,
                        evidence_factors_json, created_at
                 FROM recovered_files WHERE job_id = ?1 ORDER BY source_offset ASC",
            )?;

            let rows = stmt.query_map(params![j_id], |row| {
                let file_type_str: String = row.get(4)?;
                let file_type = match file_type_str.as_str() {
                    "JPEG" => FileType::Jpeg,
                    "PNG" => FileType::Png,
                    "PDF" => FileType::Pdf,
                    "ZIP" => FileType::Zip,
                    "DOCX (OpenXML)" => FileType::OfficeDocx,
                    "SQLite DB" => FileType::Sqlite,
                    "Text" => FileType::Text,
                    other => FileType::Unknown(other.to_string()),
                };

                let method_str: String = row.get(7)?;
                let method = match method_str.as_str() {
                    "Filesystem Metadata" => crate::models::RecoveryMethod::FilesystemMetadata,
                    "Signature Carving" => crate::models::RecoveryMethod::SignatureCarving,
                    "Structure Carving" => crate::models::RecoveryMethod::StructureCarving,
                    _ => crate::models::RecoveryMethod::FragmentReconstruction,
                };

                let val_str: String = row.get(8)?;
                let val_status = match val_str.as_str() {
                    "Valid" => ValidationStatus::Valid,
                    "Partially Valid" => ValidationStatus::PartiallyValid,
                    "Incomplete" => ValidationStatus::Incomplete,
                    "Invalid" => ValidationStatus::Invalid,
                    _ => ValidationStatus::Corrupted,
                };

                let grade_str: String = row.get(10)?;
                let grade = match grade_str.as_str() {
                    "High" => ConfidenceGrade::High,
                    "Medium" => ConfidenceGrade::Medium,
                    "Low" => ConfidenceGrade::Low,
                    _ => ConfidenceGrade::Uncertain,
                };

                let factors_json: String = row.get(14)?;
                let factors = serde_json::from_str(&factors_json).unwrap_or_default();

                Ok(RecoveredFile {
                    file_id: row.get(0)?,
                    job_id: row.get(1)?,
                    source_offset: row.get::<_, i64>(2)? as u64,
                    size_bytes: row.get::<_, i64>(3)? as u64,
                    category: file_type.category(),
                    file_type,
                    mime_type: row.get(5)?,
                    suggested_filename: row.get(6)?,
                    recovery_method: method,
                    validation_status: val_status,
                    confidence_score: row.get::<_, i64>(9)? as u32,
                    confidence_grade: grade,
                    is_fragmented: row.get::<_, i64>(11)? != 0,
                    sha256_hash: row.get(12)?,
                    output_relative_path: row.get(13)?,
                    evidence_factors: factors,
                    created_at: row.get(15)?,
                })
            })?;

            let mut files = Vec::new();
            for r in rows {
                files.push(r?);
            }

            Ok(files)
        })
    }

    /// Generates and persists an immutable forensic recovery summary report.
    pub fn generate_report(
        &self,
        job_id: &str,
        actor_id: Option<&str>,
    ) -> Result<RecoveryReport, LocardError> {
        let job = self
            .get_job(job_id)?
            .ok_or_else(|| LocardError::Operation(format!("Recovery job {} not found", job_id)))?;

        let mut category_counts = HashMap::new();
        let mut total_conf: f64 = 0.0;

        for file in &job.recovered_files {
            *category_counts
                .entry(file.category.to_string())
                .or_insert(0) += 1;
            total_conf += file.confidence_score as f64;
        }

        let avg_confidence = if !job.recovered_files.is_empty() {
            total_conf / (job.recovered_files.len() as f64)
        } else {
            0.0
        };

        let report_id = format!("rep-rec-{}", Uuid::new_v4());
        let audit_ref = format!("audit-rep-{}", Uuid::new_v4());
        let generated_at = Utc::now().to_rfc3339();

        let report_content = format!(
            "{}:{}:{}:{}:{:.2}",
            report_id, job.job_id, job.acquisition_id, job.files_recovered, avg_confidence
        );

        let mut hasher = Sha256::new();
        hasher.update(report_content.as_bytes());
        let report_digest = hex::encode(hasher.finalize());

        let report = RecoveryReport {
            report_id: report_id.clone(),
            job_id: job.job_id.clone(),
            acquisition_id: job.acquisition_id.clone(),
            source_image_sha256: job.source_image_sha256.clone(),
            total_files_recovered: job.files_recovered,
            category_counts,
            average_confidence: avg_confidence,
            report_digest: report_digest.clone(),
            audit_reference: audit_ref.clone(),
            generated_at: generated_at.clone(),
        };

        let report_json = serde_json::to_string(&report)
            .map_err(|e| LocardError::Operation(format!("Serialization error: {}", e)))?;

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO recovery_reports (
                    report_id, job_id, report_digest, audit_reference, report_json, generated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    report_id,
                    job.job_id,
                    report_digest,
                    audit_ref,
                    report_json,
                    generated_at,
                ],
            )?;
            Ok(())
        })?;

        let _ = self.audit.log_structured_event(
            "RECOVERY_REPORT_GENERATED",
            actor_id,
            Some(job_id),
            &format!(
                "Forensic recovery report generated for job {}: digest={}",
                job_id, report_digest
            ),
        );

        Ok(report)
    }

    /// Fetches a previously generated recovery report.
    pub fn get_report(&self, job_id: &str) -> Result<Option<RecoveryReport>, LocardError> {
        let j_id = job_id.to_string();
        let report_json: Option<String> = self.db.with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT report_json FROM recovery_reports WHERE job_id = ?1")?;
            let mut rows = stmt.query(params![j_id])?;

            if let Some(row) = rows.next()? {
                Ok(Some(row.get(0)?))
            } else {
                Ok(None)
            }
        })?;

        match report_json {
            Some(json_str) => {
                let report = serde_json::from_str(&json_str)
                    .map_err(|e| LocardError::Operation(format!("Deserialization error: {}", e)))?;
                Ok(Some(report))
            }
            None => Ok(None),
        }
    }

    /// Lists all validated recovery sources stored in the database.
    pub fn list_sources(&self) -> Result<Vec<RecoverySourceSnapshot>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT source_id, acquisition_id, image_path, image_size_bytes, image_sha256,
                        original_device_id, original_serial, verified_at, is_trusted
                 FROM recovery_sources ORDER BY verified_at DESC",
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(RecoverySourceSnapshot {
                    source_id: row.get(0)?,
                    acquisition_id: row.get(1)?,
                    image_path: row.get(2)?,
                    image_size_bytes: row.get::<_, i64>(3)? as u64,
                    image_sha256: row.get(4)?,
                    original_device_id: row.get(5)?,
                    original_serial: row.get(6)?,
                    verified_at: row.get(7)?,
                    is_trusted: row.get::<_, i64>(8)? != 0,
                })
            })?;

            let mut sources = Vec::new();
            for r in rows {
                sources.push(r?);
            }

            Ok(sources)
        })
    }

    /// Exports all recovered files from an existing job to a new destination folder.
    pub fn export_recovered_files(
        &self,
        job_id: &str,
        export_dir: &str,
    ) -> Result<usize, LocardError> {
        let job = self
            .get_job(job_id)?
            .ok_or_else(|| LocardError::Operation(format!("Recovery job {} not found", job_id)))?;

        let img_path = Path::new(&job.source_image_path);
        let mut img_file = std::fs::File::open(img_path)
            .map_err(|e| LocardError::Device(format!("Failed to open evidence image: {}", e)))?;

        let base_out = Path::new(export_dir);
        let mut count = 0;

        for file in &job.recovered_files {
            let cat_dir = base_out.join(file.category.to_string().to_lowercase());
            std::fs::create_dir_all(&cat_dir).map_err(|e| {
                LocardError::Operation(format!("Failed to create export folder: {}", e))
            })?;

            let target_path = cat_dir.join(&file.suggested_filename);
            use std::io::{Read, Seek, SeekFrom, Write};
            img_file
                .seek(SeekFrom::Start(file.source_offset))
                .map_err(|e| LocardError::Device(format!("Failed to seek image offset: {}", e)))?;

            let mut buf = vec![0u8; file.size_bytes as usize];
            img_file
                .read_exact(&mut buf)
                .map_err(|e| LocardError::Device(format!("Failed to read file bytes: {}", e)))?;

            let mut out = std::fs::File::create(&target_path).map_err(|e| {
                LocardError::Operation(format!("Failed to create export file: {}", e))
            })?;
            out.write_all(&buf).map_err(|e| {
                LocardError::Operation(format!("Failed to write export file: {}", e))
            })?;

            count += 1;
        }

        Ok(count)
    }
}
