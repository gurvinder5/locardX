use super::models::{
    MediaType, PreErasureEvidence, SanitizationMethod, SanitizationPlan, SanitizationScope,
    SnapshotComparisonResult, VerificationStrategy,
};
use super::planner::SanitizationPlanner;
use super::snapshot::compare_target_snapshots;
use chrono::Utc;
use locardx_audit::AuditService;
use locardx_common::{LocardError, TargetIdentity, TargetType};
use locardx_database::Database;
use locardx_device_manager::{DeviceClassification, DeviceManagerService};
use locardx_security::{SafetyEngine, TargetSnapshot};
use rusqlite::params;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

pub struct SanitizationService {
    db: Arc<Database>,
    audit: Arc<AuditService>,
    dev_mgr: Arc<DeviceManagerService>,
    safety: Option<Arc<SafetyEngine>>,
}

impl SanitizationService {
    pub fn new(
        db: Arc<Database>,
        audit: Arc<AuditService>,
        dev_mgr: Arc<DeviceManagerService>,
        safety: Option<Arc<SafetyEngine>>,
    ) -> Self {
        Self {
            db,
            audit,
            dev_mgr,
            safety,
        }
    }

    pub fn safety(&self) -> Option<&Arc<SafetyEngine>> {
        self.safety.as_ref()
    }

    /// Captures a live snapshot and infers MediaType for a target identity.
    pub fn probe_target_live(
        &self,
        target: &TargetIdentity,
    ) -> Result<(TargetSnapshot, MediaType), LocardError> {
        match target.target_type {
            TargetType::PhysicalDevice => {
                let devices = self.dev_mgr.list_devices()?;
                let dev = devices
                    .into_iter()
                    .find(|d| d.device_id.eq_ignore_ascii_case(&target.identifier))
                    .ok_or_else(|| {
                        LocardError::Device(format!(
                            "Physical drive '{}' not found during hardware probe",
                            target.identifier
                        ))
                    })?;

                let media_type = MediaType::from_device_type(
                    dev.device_type,
                    dev.model.as_deref().or(Some(&dev.display_name)),
                );

                let is_boot = dev.classification == DeviceClassification::BootDevice
                    || dev.volumes.iter().any(|v| v.is_boot_volume);

                let snapshot = TargetSnapshot {
                    target_identifier: dev.device_id.clone(),
                    target_type: TargetType::PhysicalDevice,
                    capacity_bytes: Some(dev.capacity_bytes),
                    filesystem: None,
                    device_id: Some(dev.device_id.clone()),
                    classification: Some(dev.classification),
                    is_system: dev.is_system_device
                        || dev.classification == DeviceClassification::SystemDevice,
                    is_boot,
                    snapshot_timestamp: Utc::now().to_rfc3339(),
                };

                Ok((snapshot, media_type))
            }
            TargetType::LogicalVolume => {
                let devices = self.dev_mgr.list_devices()?;
                let trimmed = target.identifier.trim_end_matches('\\');

                for dev in devices {
                    for vol in dev.volumes {
                        let mount_match = vol
                            .mount_point
                            .as_deref()
                            .map(|m| m.trim_end_matches('\\').eq_ignore_ascii_case(trimmed))
                            .unwrap_or(false);

                        if vol.volume_id.eq_ignore_ascii_case(trimmed) || mount_match {
                            let media_type = MediaType::from_device_type(
                                dev.device_type,
                                dev.model.as_deref().or(Some(&dev.display_name)),
                            );

                            let snapshot = TargetSnapshot {
                                target_identifier: vol
                                    .mount_point
                                    .clone()
                                    .unwrap_or_else(|| vol.volume_id.clone()),
                                target_type: TargetType::LogicalVolume,
                                capacity_bytes: Some(vol.capacity_bytes),
                                filesystem: Some(vol.filesystem.fs_type.as_str().to_string()),
                                device_id: Some(dev.device_id.clone()),
                                classification: Some(dev.classification),
                                is_system: vol.is_system_volume,
                                is_boot: vol.is_boot_volume
                                    || dev.classification == DeviceClassification::BootDevice,
                                snapshot_timestamp: Utc::now().to_rfc3339(),
                            };

                            return Ok((snapshot, media_type));
                        }
                    }
                }

                Err(LocardError::Device(format!(
                    "Logical volume '{}' not found in device enumeration",
                    target.identifier
                )))
            }
            TargetType::File | TargetType::Directory => {
                let path = std::path::Path::new(&target.identifier);
                if !path.exists() {
                    return Err(LocardError::Device(format!(
                        "File/folder target '{}' does not exist",
                        target.identifier
                    )));
                }

                let size = std::fs::metadata(path).ok().map(|m| m.len());
                let norm = target.identifier.to_uppercase();
                let is_sys = norm == "C:"
                    || norm == "C:\\"
                    || norm.starts_with("C:\\WINDOWS")
                    || norm == "/";

                let snapshot = TargetSnapshot {
                    target_identifier: target.identifier.clone(),
                    target_type: target.target_type,
                    capacity_bytes: size,
                    filesystem: None,
                    device_id: None,
                    classification: if is_sys {
                        Some(DeviceClassification::SystemDevice)
                    } else {
                        Some(DeviceClassification::FixedDataDevice)
                    },
                    is_system: is_sys,
                    is_boot: false,
                    snapshot_timestamp: Utc::now().to_rfc3339(),
                };

                Ok((snapshot, MediaType::Ssd))
            }
            _ => Err(LocardError::Operation(format!(
                "Unsupported target type '{:?}' for sanitization evaluation",
                target.target_type
            ))),
        }
    }

    /// Evaluates a sanitization plan, records it to the database and audit trail.
    pub fn evaluate_plan(
        &self,
        target: &TargetIdentity,
        scope: SanitizationScope,
        requested_method: Option<SanitizationMethod>,
        requested_strategy: Option<VerificationStrategy>,
        actor_id: Option<String>,
    ) -> Result<SanitizationPlan, LocardError> {
        // 1. Audit: Plan requested
        self.audit.log_structured_event(
            "SANITIZATION_PLAN_REQUESTED",
            actor_id.as_deref(),
            Some(&target.identifier),
            &format!(
                "Sanitization plan requested for target '{}' ({}) with scope {}",
                target.display_name, target.target_type, scope
            ),
        )?;

        // 2. Hardware probe
        let (snapshot, media_type) = self.probe_target_live(target)?;

        // 3. Plan evaluation
        let plan = SanitizationPlanner::evaluate_plan(
            target,
            &snapshot,
            media_type,
            scope,
            requested_method,
            requested_strategy,
            actor_id.clone(),
        );

        // 4. Audit & logging based on applicability
        if !plan.is_applicable {
            let reason_str = plan
                .reason_codes
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            self.audit.log_structured_event(
                "SANITIZATION_PLAN_REJECTED",
                actor_id.as_deref(),
                Some(&target.identifier),
                &format!(
                    "Sanitization plan rejected for target '{}': {}",
                    target.display_name, reason_str
                ),
            )?;
            warn!(
                target = %target.identifier,
                reasons = %reason_str,
                "Sanitization plan rejected"
            );
        } else {
            self.audit.log_structured_event(
                "SANITIZATION_PLAN_CREATED",
                actor_id.as_deref(),
                Some(&target.identifier),
                &format!(
                    "Sanitization plan created (plan_id: {}): method={}, standard={:?}",
                    plan.plan_id, plan.recommended_method, plan.applicable_standard
                ),
            )?;

            self.audit.log_structured_event(
                "SANITIZATION_VERIFICATION_PLANNED",
                actor_id.as_deref(),
                Some(&target.identifier),
                &format!(
                    "Verification planned (strategy: {}) for target '{}'",
                    plan.verification_strategy, target.display_name
                ),
            )?;

            info!(
                plan_id = %plan.plan_id,
                target = %target.display_name,
                method = %plan.recommended_method,
                "Sanitization plan evaluated and approved for dry-run"
            );
        }

        // 5. Persist to SQLite
        self.persist_plan(&plan)?;

        Ok(plan)
    }

    /// Persists a SanitizationPlan into the `sanitization_plans` table.
    fn persist_plan(&self, plan: &SanitizationPlan) -> Result<(), LocardError> {
        let limitations_json = serde_json::to_string(&plan.limitations)
            .map_err(|e| LocardError::Internal(e.to_string()))?;
        let reason_codes_json = serde_json::to_string(&plan.reason_codes)
            .map_err(|e| LocardError::Internal(e.to_string()))?;
        let snapshot_json = serde_json::to_string(&plan.target_snapshot)
            .map_err(|e| LocardError::Internal(e.to_string()))?;

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO sanitization_plans (
                    plan_id, target_type, target_identifier, media_type, sanitization_scope,
                    recommended_method, is_applicable, risk_level, verification_strategy,
                    applicable_standard, standard_method_id, limitations, reason_codes,
                    target_snapshot, actor_id, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    plan.plan_id,
                    plan.target.target_type.to_string(),
                    plan.target.identifier,
                    serde_json::to_string(&plan.media_type)
                        .unwrap_or_default()
                        .trim_matches('"'),
                    plan.scope.to_string(),
                    plan.recommended_method.to_string(),
                    if plan.is_applicable { 1 } else { 0 },
                    plan.risk_level.to_string(),
                    plan.verification_strategy.to_string(),
                    plan.applicable_standard,
                    plan.standard_method_id,
                    limitations_json,
                    reason_codes_json,
                    snapshot_json,
                    plan.actor_id,
                    plan.created_at,
                ],
            )?;
            Ok(())
        })?;

        Ok(())
    }

    /// Creates and persists pre-erasure evidence for an operation.
    pub fn record_pre_erasure_evidence(
        &self,
        operation_id: &str,
        plan: &SanitizationPlan,
        safety_eval_ref: Option<String>,
        actor_id: Option<String>,
    ) -> Result<PreErasureEvidence, LocardError> {
        let evidence_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        let evidence = PreErasureEvidence {
            evidence_id: evidence_id.clone(),
            operation_id: operation_id.to_string(),
            actor_id: actor_id.clone(),
            target: plan.target.clone(),
            target_snapshot: plan.target_snapshot.clone(),
            sanitization_method: plan.recommended_method,
            sanitization_scope: plan.scope,
            verification_strategy: plan.verification_strategy,
            timestamp: timestamp.clone(),
            applicable_limitations: plan.limitations.clone(),
            safety_evaluation_ref: safety_eval_ref,
        };

        let snapshot_json = serde_json::to_string(&evidence.target_snapshot)
            .map_err(|e| LocardError::Internal(e.to_string()))?;
        let limitations_json = serde_json::to_string(&evidence.applicable_limitations)
            .map_err(|e| LocardError::Internal(e.to_string()))?;

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO pre_erasure_evidence (
                    evidence_id, operation_id, actor_id, target_type, target_identifier,
                    target_display_name, target_snapshot, sanitization_method, sanitization_scope,
                    verification_strategy, applicable_limitations, safety_evaluation_ref, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    evidence.evidence_id,
                    evidence.operation_id,
                    evidence.actor_id,
                    evidence.target.target_type.to_string(),
                    evidence.target.identifier,
                    evidence.target.display_name,
                    snapshot_json,
                    evidence.sanitization_method.to_string(),
                    evidence.sanitization_scope.to_string(),
                    evidence.verification_strategy.to_string(),
                    limitations_json,
                    evidence.safety_evaluation_ref,
                    evidence.timestamp,
                ],
            )?;
            Ok(())
        })?;

        self.audit.log_structured_event(
            "PRE_ERASURE_EVIDENCE_RECORDED",
            actor_id.as_deref(),
            Some(&evidence.target.identifier),
            &format!(
                "Pre-erasure evidence recorded (evidence_id: {}) for operation {}",
                evidence_id, operation_id
            ),
        )?;

        Ok(evidence)
    }

    /// Re-probes live device and compares with snapshot stored in plan (TOCTOU guard).
    pub fn compare_plan_snapshot_live(
        &self,
        plan_id: &str,
    ) -> Result<SnapshotComparisonResult, LocardError> {
        let plan = self.get_plan(plan_id)?;

        let live_snapshot = match self.probe_target_live(&plan.target) {
            Ok((s, _)) => Some(s),
            Err(_) => None,
        };

        let result = compare_target_snapshots(&plan.target_snapshot, live_snapshot.as_ref());

        if !result.matches {
            self.audit.log_structured_event(
                "SANITIZATION_TARGET_CHANGED",
                plan.actor_id.as_deref(),
                Some(&plan.target.identifier),
                &format!(
                    "Target mutated between plan and execution: {:?}",
                    result.differences
                ),
            )?;
            warn!(
                target = %plan.target.identifier,
                diffs = ?result.differences,
                "Sanitization target changed (TOCTOU alert)"
            );
        }

        Ok(result)
    }

    /// Fetches a SanitizationPlan from the database.
    pub fn get_plan(&self, plan_id: &str) -> Result<SanitizationPlan, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT
                    plan_id, target_type, target_identifier, media_type, sanitization_scope,
                    recommended_method, is_applicable, risk_level, verification_strategy,
                    applicable_standard, standard_method_id, limitations, reason_codes,
                    target_snapshot, actor_id, created_at
                FROM sanitization_plans WHERE plan_id = ?1",
            )?;

            let plan = stmt.query_row(params![plan_id], |row| {
                let plan_id: String = row.get(0)?;
                let target_type_str: String = row.get(1)?;
                let target_identifier: String = row.get(2)?;
                let media_type_str: String = row.get(3)?;
                let scope_str: String = row.get(4)?;
                let method_str: String = row.get(5)?;
                let is_applicable_int: i64 = row.get(6)?;
                let risk_level_str: String = row.get(7)?;
                let strategy_str: String = row.get(8)?;
                let applicable_standard: Option<String> = row.get(9)?;
                let standard_method_id: Option<String> = row.get(10)?;
                let limitations_json: String = row.get(11)?;
                let reason_codes_json: String = row.get(12)?;
                let snapshot_json: String = row.get(13)?;
                let actor_id: Option<String> = row.get(14)?;
                let created_at: String = row.get(15)?;

                let target_type = match target_type_str.as_str() {
                    "File" => TargetType::File,
                    "Directory" => TargetType::Directory,
                    "LogicalVolume" => TargetType::LogicalVolume,
                    _ => TargetType::PhysicalDevice,
                };

                let media_type: MediaType =
                    serde_json::from_str(&format!("\"{}\"", media_type_str))
                        .unwrap_or(MediaType::Unknown);

                let scope = match scope_str.as_str() {
                    "File" => SanitizationScope::File,
                    "Folder" => SanitizationScope::Folder,
                    "LogicalVolume" => SanitizationScope::LogicalVolume,
                    _ => SanitizationScope::PhysicalDevice,
                };

                let recommended_method = match method_str.as_str() {
                    s if s.contains("NIST SP 800-88 Clear") => {
                        SanitizationMethod::Nist80088ClearZero
                    }
                    s if s.contains("NIST SP 800-88 Purge") => {
                        SanitizationMethod::Nist80088PurgeCrypto
                    }
                    s if s.contains("DoD 5220.22-M") => SanitizationMethod::Dod522022M,
                    s if s.contains("NVMe") => SanitizationMethod::NvmeCryptoErase,
                    s if s.contains("ATA") => SanitizationMethod::AtaSecureErase,
                    s if s.contains("Shred") => SanitizationMethod::LogicalFileShred,
                    _ => SanitizationMethod::Unsupported,
                };

                let risk_level = match risk_level_str.as_str() {
                    "Low" => locardx_security::RiskLevel::Low,
                    "Medium" => locardx_security::RiskLevel::Medium,
                    "High" => locardx_security::RiskLevel::High,
                    _ => locardx_security::RiskLevel::Critical,
                };

                let verification_strategy = match strategy_str.as_str() {
                    s if s.contains("Full") => VerificationStrategy::FullReadBack,
                    s if s.contains("Crypto") => VerificationStrategy::CryptoKeyDestructionCheck,
                    s if s.contains("Metadata") => VerificationStrategy::MetadataUnlinkCheck,
                    s if s.contains("Sampled") => VerificationStrategy::SampledRandomSectors,
                    _ => VerificationStrategy::NoVerification,
                };

                let limitations: Vec<String> =
                    serde_json::from_str(&limitations_json).unwrap_or_default();
                let reason_codes = serde_json::from_str(&reason_codes_json).unwrap_or_default();
                let target_snapshot: TargetSnapshot = serde_json::from_str(&snapshot_json)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                Ok(SanitizationPlan {
                    plan_id,
                    target: TargetIdentity {
                        target_type,
                        identifier: target_identifier.clone(),
                        display_name: target_identifier,
                        size_bytes: target_snapshot.capacity_bytes,
                    },
                    media_type,
                    scope,
                    recommended_method,
                    is_applicable: is_applicable_int == 1,
                    risk_level,
                    verification_strategy,
                    applicable_standard,
                    standard_method_id,
                    limitations,
                    reason_codes,
                    target_snapshot,
                    created_at,
                    actor_id,
                })
            })?;

            Ok(plan)
        })
    }
}
