use crate::models::{
    ConfirmationChallenge, ConfirmationRecord, ConfirmationStatus, ReasonCode, RiskLevel,
    SafetyDecision, SafetyDecisionOutcome, TargetSnapshot,
};
use crate::policy::SafetyPolicy;
use chrono::{Duration, Utc};
use locardx_audit::AuditService;
use locardx_auth::{models::UserRole, AuthService, AuthorizationEngine};
use locardx_common::{LocardError, OperationType, TargetIdentity, TargetType};
use locardx_database::Database;
use locardx_device_manager::{DeviceClassification, DeviceManagerService};
use rusqlite::params;
use std::sync::Arc;
use uuid::Uuid;

/// Safety & Authorization Interlock Service.
/// Mandatory fail-closed gate separating the Operation Manager from all execution engines.
pub struct SafetyEngine {
    db: Arc<Database>,
    audit: Arc<AuditService>,
    auth: Arc<AuthService>,
    device_manager: Arc<DeviceManagerService>,
}

impl SafetyEngine {
    pub fn new(
        db: Arc<Database>,
        audit: Arc<AuditService>,
        auth: Arc<AuthService>,
        device_manager: Arc<DeviceManagerService>,
    ) -> Self {
        Self {
            db,
            audit,
            auth,
            device_manager,
        }
    }

    /// Re-queries the device manager in real time to discover target hardware,
    /// partition layout, and system/boot status.
    /// NEVER trusts stale frontend state.
    pub fn discover_target_live(
        &self,
        target: &TargetIdentity,
    ) -> Result<TargetSnapshot, ReasonCode> {
        let devices = self
            .device_manager
            .list_devices()
            .map_err(|_| ReasonCode::UnknownTarget)?;

        let trimmed_id = target.identifier.trim();
        if trimmed_id.is_empty() {
            return Err(ReasonCode::InvalidTarget);
        }

        let (is_known_os_root, is_known_boot_root) = Self::is_system_or_boot_path(trimmed_id);

        match target.target_type {
            TargetType::PhysicalDevice => {
                for dev in &devices {
                    if dev.device_id.eq_ignore_ascii_case(trimmed_id)
                        || dev.display_name.eq_ignore_ascii_case(trimmed_id)
                    {
                        return Ok(TargetSnapshot {
                            target_identifier: dev.device_id.clone(),
                            target_type: TargetType::PhysicalDevice,
                            capacity_bytes: Some(dev.capacity_bytes),
                            filesystem: None,
                            device_id: Some(dev.device_id.clone()),
                            classification: Some(dev.classification),
                            is_system: dev.is_system_device || is_known_os_root,
                            is_boot: dev.classification == DeviceClassification::BootDevice,
                            snapshot_timestamp: Utc::now().to_rfc3339(),
                        });
                    }
                }
                Err(ReasonCode::InvalidTarget)
            }
            TargetType::LogicalVolume => {
                for dev in &devices {
                    for vol in &dev.volumes {
                        let mount_match = vol
                            .mount_point
                            .as_ref()
                            .map(|m| {
                                m.trim_end_matches('\\')
                                    .eq_ignore_ascii_case(trimmed_id.trim_end_matches('\\'))
                            })
                            .unwrap_or(false);

                        if vol.volume_id.eq_ignore_ascii_case(trimmed_id) || mount_match {
                            return Ok(TargetSnapshot {
                                target_identifier: vol
                                    .mount_point
                                    .clone()
                                    .unwrap_or_else(|| vol.volume_id.clone()),
                                target_type: TargetType::LogicalVolume,
                                capacity_bytes: Some(vol.capacity_bytes),
                                filesystem: Some(vol.filesystem.fs_type.as_str().to_string()),
                                device_id: Some(dev.device_id.clone()),
                                classification: Some(dev.classification),
                                is_system: vol.is_system_volume || is_known_os_root,
                                is_boot: vol.is_boot_volume
                                    || dev.classification == DeviceClassification::BootDevice,
                                snapshot_timestamp: Utc::now().to_rfc3339(),
                            });
                        }
                    }
                }
                Err(ReasonCode::InvalidTarget)
            }
            TargetType::File | TargetType::Directory => {
                let path = std::path::Path::new(trimmed_id);
                if !path.exists() {
                    return Err(ReasonCode::InvalidTarget);
                }

                let is_sys = is_known_os_root;
                let is_boot = is_known_boot_root;
                let size = std::fs::metadata(path).ok().map(|m| m.len());

                Ok(TargetSnapshot {
                    target_identifier: trimmed_id.to_string(),
                    target_type: target.target_type,
                    capacity_bytes: size,
                    filesystem: None,
                    device_id: None,
                    classification: if is_sys {
                        Some(DeviceClassification::SystemDevice)
                    } else if is_boot {
                        Some(DeviceClassification::BootDevice)
                    } else {
                        Some(DeviceClassification::FixedDataDevice)
                    },
                    is_system: is_sys,
                    is_boot,
                    snapshot_timestamp: Utc::now().to_rfc3339(),
                })
            }
            TargetType::EvidenceObject => Ok(TargetSnapshot {
                target_identifier: trimmed_id.to_string(),
                target_type: TargetType::EvidenceObject,
                capacity_bytes: target.size_bytes,
                filesystem: None,
                device_id: None,
                classification: Some(DeviceClassification::FixedDataDevice),
                is_system: false,
                is_boot: false,
                snapshot_timestamp: Utc::now().to_rfc3339(),
            }),
            TargetType::Unknown => Err(ReasonCode::UnknownTarget),
        }
    }

    /// Evaluates target safety, RBAC authorization, and method compatibility.
    pub fn evaluate_safety(
        &self,
        target: &TargetIdentity,
        op_type: OperationType,
        session_token: Option<&str>,
    ) -> Result<SafetyDecision, LocardError> {
        let evaluation_id = Uuid::new_v4().to_string();
        let evaluated_at = Utc::now().to_rfc3339();

        // 1. Resolve actor and role
        let current_user = match session_token {
            Some(tok) => self.auth.get_current_user(tok).ok(),
            None => None,
        };

        let actor_id = current_user.as_ref().map(|u| u.username.clone());
        let role = current_user
            .as_ref()
            .map(|u| u.role)
            .unwrap_or(UserRole::Viewer);

        // 2. Evaluate RBAC permission
        if let Err(auth_err) = AuthorizationEngine::can_request_operation(role, op_type) {
            let _ = self.audit.log_structured_event(
                "AUTHORIZATION_DENIED",
                actor_id.as_deref(),
                Some(&target.identifier),
                &format!(
                    "User '{}' ({}) denied request for {:?}",
                    actor_id.as_deref().unwrap_or("Anonymous"),
                    role,
                    op_type
                ),
            );

            let decision = SafetyDecision {
                evaluation_id: evaluation_id.clone(),
                target: target.clone(),
                operation_type: op_type,
                actor_id: actor_id.clone(),
                decision: SafetyDecisionOutcome::Denied,
                reason_code: ReasonCode::UnauthorizedRole,
                risk_level: RiskLevel::High,
                message: auth_err.to_string(),
                evaluated_at: evaluated_at.clone(),
                target_snapshot: None,
                requires_confirmation: false,
            };
            self.persist_evaluation(&decision)?;
            return Ok(decision);
        }

        // 3. Live Target Discovery
        let snapshot_res = self.discover_target_live(target);
        let snapshot = match snapshot_res {
            Ok(s) => s,
            Err(reason) => {
                let msg = match reason {
                    ReasonCode::InvalidTarget => {
                        "Target does not exist or cannot be identified.".to_string()
                    }
                    ReasonCode::UnknownTarget => {
                        "Target storage classification is indeterminate.".to_string()
                    }
                    other => format!("Target discovery failed: {}", other),
                };

                let _ = self.audit.log_structured_event(
                    "SAFETY_CHECK_BLOCKED",
                    actor_id.as_deref(),
                    Some(&target.identifier),
                    &format!("Target discovery blocked: {}", msg),
                );

                let decision = SafetyDecision {
                    evaluation_id: evaluation_id.clone(),
                    target: target.clone(),
                    operation_type: op_type,
                    actor_id: actor_id.clone(),
                    decision: SafetyDecisionOutcome::Blocked,
                    reason_code: reason,
                    risk_level: RiskLevel::Critical,
                    message: msg,
                    evaluated_at: evaluated_at.clone(),
                    target_snapshot: None,
                    requires_confirmation: false,
                };
                self.persist_evaluation(&decision)?;
                return Ok(decision);
            }
        };

        // 4. Hard Safety Checks: System & Boot Protection
        if snapshot.is_system || snapshot.is_boot {
            if op_type.is_destructive() {
                let reason = if snapshot.is_boot && !snapshot.is_system {
                    ReasonCode::BootDevice
                } else {
                    ReasonCode::SystemDevice
                };

                let msg = format!(
                    "HARD SAFETY BLOCK: Target '{}' is an active {} and cannot be targeted for destructive sanitization.",
                    target.identifier,
                    if reason == ReasonCode::BootDevice { "Boot Device" } else { "System Device" }
                );

                let _ = self.audit.log_structured_event(
                    "SAFETY_CHECK_BLOCKED",
                    actor_id.as_deref(),
                    Some(&target.identifier),
                    &msg,
                );

                let decision = SafetyDecision {
                    evaluation_id: evaluation_id.clone(),
                    target: target.clone(),
                    operation_type: op_type,
                    actor_id: actor_id.clone(),
                    decision: SafetyDecisionOutcome::Blocked,
                    reason_code: reason,
                    risk_level: RiskLevel::Critical,
                    message: msg,
                    evaluated_at: evaluated_at.clone(),
                    target_snapshot: Some(snapshot),
                    requires_confirmation: false,
                };
                self.persist_evaluation(&decision)?;
                return Ok(decision);
            }
        }

        // 5. Risk Assessment
        let risk_level = SafetyPolicy::assess_risk(
            op_type,
            snapshot.classification,
            snapshot.is_system || snapshot.is_boot,
        );

        // 6. Method Compatibility
        let compat_res = SafetyPolicy::evaluate_compatibility(op_type, None, None);

        // 7. Decision Synthesis
        let (decision_outcome, reason_code, message, requires_confirmation) =
            if op_type.is_read_only() {
                (
                    SafetyDecisionOutcome::Allowed,
                    ReasonCode::ValidTarget,
                    format!(
                        "Operation {:?} is verified read-only and safe to execute on target '{}'.",
                        op_type, target.identifier
                    ),
                    false,
                )
            } else {
                // Destructive operation:
                // Check method compatibility
                if let Err(compat_reason) = compat_res {
                    (
                        SafetyDecisionOutcome::RequiresConfirmation,
                        ReasonCode::MissingConfirmation,
                        format!(
                        "Target '{}' is eligible for review. NOTE: Execution engine status is {}.",
                        target.identifier, compat_reason
                    ),
                        true,
                    )
                } else {
                    (
                        SafetyDecisionOutcome::RequiresConfirmation,
                        ReasonCode::MissingConfirmation,
                        format!(
                        "Target '{}' requires explicit two-stage confirmation before execution.",
                        target.identifier
                    ),
                        true,
                    )
                }
            };

        let _ = self.audit.log_structured_event(
            "SAFETY_CHECK_STARTED",
            actor_id.as_deref(),
            Some(&target.identifier),
            &format!(
                "Safety check evaluated: {:?} -> {} ({})",
                op_type, decision_outcome, reason_code
            ),
        );

        let decision = SafetyDecision {
            evaluation_id: evaluation_id.clone(),
            target: target.clone(),
            operation_type: op_type,
            actor_id: actor_id.clone(),
            decision: decision_outcome,
            reason_code,
            risk_level,
            message,
            evaluated_at: evaluated_at.clone(),
            target_snapshot: Some(snapshot),
            requires_confirmation,
        };

        self.persist_evaluation(&decision)?;
        Ok(decision)
    }

    /// Stage 1: Prepares a structured confirmation challenge bound to a live target snapshot.
    pub fn request_destructive_confirmation(
        &self,
        operation_id: &str,
        target: &TargetIdentity,
        op_type: OperationType,
        session_token: &str,
    ) -> Result<ConfirmationChallenge, LocardError> {
        let current_user = self.auth.get_current_user(session_token)?;
        AuthorizationEngine::can_request_operation(current_user.role, op_type)?;

        // Live target discovery
        let snapshot = self.discover_target_live(target).map_err(|reason| {
            LocardError::SecurityViolation(format!("Target discovery failed: {}", reason))
        })?;

        // Hard System/Boot Block Invariant
        if snapshot.is_system || snapshot.is_boot {
            let _ = self.audit.log_structured_event(
                "SAFETY_CHECK_BLOCKED",
                Some(&current_user.username),
                Some(&target.identifier),
                "Attempted to request confirmation for system or boot device",
            );
            return Err(LocardError::SecurityViolation(
                "System and Boot devices cannot be targeted for destructive confirmation."
                    .to_string(),
            ));
        }

        let risk_level = SafetyPolicy::assess_risk(
            op_type,
            snapshot.classification,
            snapshot.is_system || snapshot.is_boot,
        );

        let confirmation_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let created_at = now.to_rfc3339();
        let expires_at = (now + Duration::minutes(5)).to_rfc3339();

        let snapshot_json = serde_json::to_string(&snapshot).map_err(|e| {
            LocardError::Internal(format!("Failed to serialize target snapshot: {}", e))
        })?;

        // Persist confirmation record
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO operation_confirmations (
                    confirmation_id, operation_id, actor_id, target_identifier, target_type,
                    target_snapshot, operation_type, risk_level, warning_acknowledged,
                    status, created_at, expires_at, confirmed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    confirmation_id,
                    operation_id,
                    current_user.username,
                    target.identifier,
                    target.target_type.to_string(),
                    snapshot_json,
                    op_type.to_string(),
                    risk_level.to_string(),
                    0i32,
                    ConfirmationStatus::Pending.to_string(),
                    created_at,
                    expires_at,
                    Option::<String>::None,
                ],
            )?;
            Ok(())
        })?;

        let _ = self.audit.log_structured_event(
            "CONFIRMATION_REQUESTED",
            Some(&current_user.username),
            Some(&target.identifier),
            &format!(
                "Confirmation challenge generated for op {} (expires in 5 minutes)",
                operation_id
            ),
        );

        Ok(ConfirmationChallenge {
            confirmation_id,
            operation_id: operation_id.to_string(),
            actor_id: current_user.username,
            target: target.clone(),
            operation_type: op_type,
            risk_level,
            target_snapshot: snapshot,
            created_at,
            expires_at,
        })
    }

    /// Stage 2: Verifies and commits an explicit destructive confirmation.
    /// Re-evaluates target live to prevent TOCTOU substitution attacks.
    pub fn confirm_destructive_operation(
        &self,
        confirmation_id: &str,
        operation_id: &str,
        warning_acknowledged: bool,
        typed_target_confirmation: &str,
        session_token: &str,
    ) -> Result<SafetyDecision, LocardError> {
        let current_user = self.auth.get_current_user(session_token)?;

        // 1. Fetch confirmation record
        let record = self.get_confirmation_record(confirmation_id)?;

        if record.operation_id != operation_id {
            return Err(LocardError::SecurityViolation(
                "Confirmation operation ID does not match target operation.".to_string(),
            ));
        }

        if record.actor_id != current_user.username {
            return Err(LocardError::SecurityViolation(
                "Confirmation actor mismatch. Only the requesting operator may confirm."
                    .to_string(),
            ));
        }

        if record.status != ConfirmationStatus::Pending {
            return Err(LocardError::SecurityViolation(format!(
                "Confirmation challenge is already {}.",
                record.status
            )));
        }

        if record.is_expired() {
            self.update_confirmation_status(confirmation_id, ConfirmationStatus::Expired, None)?;
            let _ = self.audit.log_structured_event(
                "CONFIRMATION_REJECTED",
                Some(&current_user.username),
                Some(&record.target_identifier),
                "Confirmation expired before operator submission",
            );
            return Err(LocardError::SecurityViolation(
                "Confirmation challenge has expired. Please review the target again.".to_string(),
            ));
        }

        if !warning_acknowledged {
            return Err(LocardError::SecurityViolation(
                "Destructive consequences warning must be explicitly acknowledged.".to_string(),
            ));
        }

        // Validate typed confirmation
        let typed_trimmed = typed_target_confirmation.trim();
        let target_id_trimmed = record.target_identifier.trim();
        let matches_id = typed_trimmed.eq_ignore_ascii_case(target_id_trimmed);
        let matches_snapshot =
            typed_trimmed.eq_ignore_ascii_case(&record.target_snapshot.target_identifier);

        if !matches_id && !matches_snapshot {
            return Err(LocardError::SecurityViolation(format!(
                "Typed confirmation string '{}' does not match target identifier '{}'.",
                typed_trimmed, target_id_trimmed
            )));
        }

        // 2. LIVE TARGET REVALIDATION (TOCTOU Defense)
        let target_identity = TargetIdentity {
            target_type: record.target_type,
            identifier: record.target_identifier.clone(),
            display_name: record.target_identifier.clone(),
            size_bytes: record.target_snapshot.capacity_bytes,
        };

        let live_snapshot_res = self.discover_target_live(&target_identity);
        let live_snapshot = match live_snapshot_res {
            Ok(s) => s,
            Err(reason) => {
                self.update_confirmation_status(
                    confirmation_id,
                    ConfirmationStatus::Revoked,
                    None,
                )?;
                let _ = self.audit.log_structured_event(
                    "TARGET_REVALIDATION_FAILED",
                    Some(&current_user.username),
                    Some(&record.target_identifier),
                    &format!("Target revalidation failed: {}", reason),
                );
                return Err(LocardError::SecurityViolation(format!(
                    "Target revalidation failed: {}. Confirmation revoked.",
                    reason
                )));
            }
        };

        if let Err(change_reason) = record.target_snapshot.detect_changes(&live_snapshot) {
            self.update_confirmation_status(confirmation_id, ConfirmationStatus::Revoked, None)?;
            let _ = self.audit.log_structured_event(
                "TARGET_REVALIDATION_FAILED",
                Some(&current_user.username),
                Some(&record.target_identifier),
                "Target properties changed since confirmation request (TOCTOU violation)",
            );
            return Err(LocardError::SecurityViolation(format!(
                "Target change detected: {}. The storage device or layout changed unexpectedly. Confirmation revoked.",
                change_reason
            )));
        }

        // 3. Re-enforce Hard System/Boot Block
        if live_snapshot.is_system || live_snapshot.is_boot {
            self.update_confirmation_status(confirmation_id, ConfirmationStatus::Revoked, None)?;
            return Err(LocardError::SecurityViolation(
                "System and Boot devices cannot be confirmed for destruction.".to_string(),
            ));
        }

        // 4. Mark confirmation as Confirmed
        let confirmed_at = Utc::now().to_rfc3339();
        self.update_confirmation_status(
            confirmation_id,
            ConfirmationStatus::Confirmed,
            Some(&confirmed_at),
        )?;

        let _ = self.audit.log_structured_event(
            "CONFIRMATION_ACCEPTED",
            Some(&current_user.username),
            Some(&record.target_identifier),
            &format!(
                "Explicit confirmation verified for operation {} on '{}'",
                operation_id, record.target_identifier
            ),
        );

        // 5. Execution Decision based on Operation Type
        let (decision_outcome, reason_code, message) = match record.operation_type {
            OperationType::FileErasure | OperationType::FolderErasure => (
                SafetyDecisionOutcome::Allowed,
                ReasonCode::ValidTarget,
                format!(
                    "Confirmation verified and committed for {:?}. Target '{}' authorized for execution.",
                    record.operation_type, record.target_identifier
                ),
            ),
            _ => (
                SafetyDecisionOutcome::Blocked,
                ReasonCode::OperationDisabled,
                "Confirmation verified and committed. INVARIANT: Destructive drive erasure executors are permanently disabled in this step.".to_string(),
            ),
        };

        let decision = SafetyDecision {
            evaluation_id: Uuid::new_v4().to_string(),
            target: target_identity,
            operation_type: record.operation_type,
            actor_id: Some(current_user.username),
            decision: decision_outcome,
            reason_code,
            risk_level: record.risk_level,
            message,
            evaluated_at: confirmed_at,
            target_snapshot: Some(live_snapshot),
            requires_confirmation: false,
        };

        self.persist_evaluation(&decision)?;
        Ok(decision)
    }

    /// Queries historic safety evaluations for audit and monitoring.
    pub fn list_safety_evaluations(
        &self,
        limit: Option<u32>,
    ) -> Result<Vec<SafetyDecision>, LocardError> {
        let max_rows = limit.unwrap_or(50).min(500);

        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT evaluation_id, operation_id, target_identifier, target_type, operation_type,
                        actor_id, decision, reason_code, risk_level, reason_message, evaluated_at
                 FROM safety_evaluations
                 ORDER BY evaluated_at DESC
                 LIMIT ?1",
            )?;

            let rows = stmt.query_map(params![max_rows], |row| {
                let eval_id: String = row.get(0)?;
                let _op_id: Option<String> = row.get(1)?;
                let target_ident: String = row.get(2)?;
                let target_type_str: String = row.get(3)?;
                let op_type_str: String = row.get(4)?;
                let actor_id: Option<String> = row.get(5)?;
                let decision_str: String = row.get(6)?;
                let reason_str: String = row.get(7)?;
                let risk_str: String = row.get(8)?;
                let msg: String = row.get(9)?;
                let evaluated_at: String = row.get(10)?;

                let target_type = match target_type_str.as_str() {
                    "File" => TargetType::File,
                    "Directory" => TargetType::Directory,
                    "LogicalVolume" => TargetType::LogicalVolume,
                    "PhysicalDevice" => TargetType::PhysicalDevice,
                    "EvidenceObject" => TargetType::EvidenceObject,
                    _ => TargetType::Unknown,
                };

                let op_type = match op_type_str.as_str() {
                    "IntegrityHash" => OperationType::IntegrityHash,
                    "IntegrityVerify" => OperationType::IntegrityVerify,
                    "FileErasure" => OperationType::FileErasure,
                    "FolderErasure" => OperationType::FolderErasure,
                    "DriveErasure" => OperationType::DriveErasure,
                    "Recovery" => OperationType::Recovery,
                    _ => OperationType::Unknown,
                };

                let decision = match decision_str.as_str() {
                    "Allowed" => SafetyDecisionOutcome::Allowed,
                    "Denied" => SafetyDecisionOutcome::Denied,
                    "RequiresConfirmation" => SafetyDecisionOutcome::RequiresConfirmation,
                    _ => SafetyDecisionOutcome::Blocked,
                };

                let reason_code = match reason_str.as_str() {
                    "VALID_TARGET" => ReasonCode::ValidTarget,
                    "SYSTEM_DEVICE" => ReasonCode::SystemDevice,
                    "BOOT_DEVICE" => ReasonCode::BootDevice,
                    "UNKNOWN_TARGET" => ReasonCode::UnknownTarget,
                    "INVALID_TARGET" => ReasonCode::InvalidTarget,
                    "UNAUTHORIZED_ROLE" => ReasonCode::UnauthorizedRole,
                    "MISSING_CONFIRMATION" => ReasonCode::MissingConfirmation,
                    "CONFIRMATION_EXPIRED" => ReasonCode::ConfirmationExpired,
                    "CONFIRMATION_INVALID" => ReasonCode::ConfirmationInvalid,
                    "TARGET_CHANGED" => ReasonCode::TargetChanged,
                    "OPERATION_DISABLED" => ReasonCode::OperationDisabled,
                    _ => ReasonCode::SafetyPolicyViolation,
                };

                let risk_level = match risk_str.as_str() {
                    "Low" => RiskLevel::Low,
                    "Medium" => RiskLevel::Medium,
                    "High" => RiskLevel::High,
                    _ => RiskLevel::Critical,
                };

                Ok(SafetyDecision {
                    evaluation_id: eval_id,
                    target: TargetIdentity {
                        target_type,
                        identifier: target_ident.clone(),
                        display_name: target_ident,
                        size_bytes: None,
                    },
                    operation_type: op_type,
                    actor_id,
                    decision,
                    reason_code,
                    risk_level,
                    message: msg,
                    evaluated_at,
                    target_snapshot: None,
                    requires_confirmation: decision == SafetyDecisionOutcome::RequiresConfirmation,
                })
            })?;

            let mut results = Vec::new();
            for r in rows {
                results.push(r?);
            }
            Ok(results)
        })
    }

    fn persist_evaluation(&self, decision: &SafetyDecision) -> Result<(), LocardError> {
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO safety_evaluations (
                    evaluation_id, operation_id, target_identifier, target_type, operation_type,
                    actor_id, decision, reason_code, risk_level, reason_message, evaluated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    decision.evaluation_id,
                    Option::<String>::None,
                    decision.target.identifier,
                    decision.target.target_type.to_string(),
                    decision.operation_type.to_string(),
                    decision.actor_id,
                    decision.decision.to_string(),
                    decision.reason_code.to_string(),
                    decision.risk_level.to_string(),
                    decision.message,
                    decision.evaluated_at,
                ],
            )?;
            Ok(())
        })
    }

    fn get_confirmation_record(
        &self,
        confirmation_id: &str,
    ) -> Result<ConfirmationRecord, LocardError> {
        self.db
            .with_conn(|conn| {
                let mut stmt = conn.prepare(
                "SELECT confirmation_id, operation_id, actor_id, target_identifier, target_type,
                        target_snapshot, operation_type, risk_level, warning_acknowledged,
                        status, created_at, expires_at, confirmed_at
                 FROM operation_confirmations WHERE confirmation_id = ?1",
            )?;

                let mut rows = stmt.query(params![confirmation_id])?;
                if let Some(row) = rows.next()? {
                    let conf_id: String = row.get(0)?;
                    let op_id: String = row.get(1)?;
                    let actor_id: String = row.get(2)?;
                    let target_ident: String = row.get(3)?;
                    let target_type_str: String = row.get(4)?;
                    let snapshot_json: String = row.get(5)?;
                    let op_type_str: String = row.get(6)?;
                    let risk_str: String = row.get(7)?;
                    let warning_ack: i32 = row.get(8)?;
                    let status_str: String = row.get(9)?;
                    let created_at: String = row.get(10)?;
                    let expires_at: String = row.get(11)?;
                    let confirmed_at: Option<String> = row.get(12)?;

                    let target_type = match target_type_str.as_str() {
                        "File" => TargetType::File,
                        "Directory" => TargetType::Directory,
                        "LogicalVolume" => TargetType::LogicalVolume,
                        "PhysicalDevice" => TargetType::PhysicalDevice,
                        "EvidenceObject" => TargetType::EvidenceObject,
                        _ => TargetType::Unknown,
                    };

                    let op_type = match op_type_str.as_str() {
                        "IntegrityHash" => OperationType::IntegrityHash,
                        "IntegrityVerify" => OperationType::IntegrityVerify,
                        "FileErasure" => OperationType::FileErasure,
                        "FolderErasure" => OperationType::FolderErasure,
                        "DriveErasure" => OperationType::DriveErasure,
                        "Recovery" => OperationType::Recovery,
                        _ => OperationType::Unknown,
                    };

                    let risk_level = match risk_str.as_str() {
                        "Low" => RiskLevel::Low,
                        "Medium" => RiskLevel::Medium,
                        "High" => RiskLevel::High,
                        _ => RiskLevel::Critical,
                    };

                    let status = match status_str.as_str() {
                        "Pending" => ConfirmationStatus::Pending,
                        "Confirmed" => ConfirmationStatus::Confirmed,
                        "Rejected" => ConfirmationStatus::Rejected,
                        "Expired" => ConfirmationStatus::Expired,
                        _ => ConfirmationStatus::Revoked,
                    };

                    let snapshot: TargetSnapshot =
                        serde_json::from_str(&snapshot_json).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;

                    Ok(ConfirmationRecord {
                        confirmation_id: conf_id,
                        operation_id: op_id,
                        actor_id,
                        target_identifier: target_ident,
                        target_type,
                        target_snapshot: snapshot,
                        operation_type: op_type,
                        risk_level,
                        warning_acknowledged: warning_ack != 0,
                        status,
                        created_at,
                        expires_at,
                        confirmed_at,
                    })
                } else {
                    Err(rusqlite::Error::QueryReturnedNoRows)
                }
            })
            .map_err(|e| {
                LocardError::SecurityViolation(format!("Confirmation record not found: {}", e))
            })
    }

    fn update_confirmation_status(
        &self,
        confirmation_id: &str,
        status: ConfirmationStatus,
        confirmed_at: Option<&str>,
    ) -> Result<(), LocardError> {
        self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE operation_confirmations SET status = ?1, confirmed_at = ?2 WHERE confirmation_id = ?3",
                params![status.to_string(), confirmed_at, confirmation_id],
            )?;
            Ok(())
        })
    }

    /// Determines if a file, folder, or drive path belongs to a known system or boot critical location.
    pub fn is_system_or_boot_path(raw_path: &str) -> (bool, bool) {
        let trimmed = raw_path.trim();
        let upper = trimmed.to_uppercase();
        let norm = upper.replace('/', "\\");

        let is_boot = norm == "C:\\BOOTMGR"
            || norm.starts_with("C:\\BOOT\\")
            || norm == "C:\\BOOT"
            || trimmed == "/boot"
            || trimmed.starts_with("/boot/");

        let is_sys = norm == "C:"
            || norm == "C:\\"
            || norm.starts_with("C:\\WINDOWS")
            || norm.starts_with("C:\\PROGRAM FILES")
            || norm.starts_with("C:\\PROGRAMDATA\\MICROSOFT")
            || norm.starts_with("C:\\USERS\\DEFAULT")
            || norm.starts_with("C:\\RECOVERY")
            || norm == "C:\\PAGEFILE.SYS"
            || norm == "C:\\HIBERFIL.SYS"
            || norm == "C:\\SWAPFILE.SYS"
            || norm == "C:\\DUMPSTACK.LOG.TMP"
            || trimmed == "/"
            || trimmed == "/etc"
            || trimmed.starts_with("/etc/")
            || trimmed == "/sys"
            || trimmed.starts_with("/sys/")
            || trimmed == "/proc"
            || trimmed.starts_with("/proc/")
            || trimmed == "/dev"
            || trimmed.starts_with("/dev/")
            || trimmed == "/usr"
            || trimmed.starts_with("/usr/")
            || trimmed == "/bin"
            || trimmed.starts_with("/bin/")
            || trimmed == "/sbin"
            || trimmed.starts_with("/sbin/");

        // Also inspect canonical path if path exists to prevent traversal bypasses
        if let Ok(canon) = std::fs::canonicalize(std::path::Path::new(trimmed)) {
            let canon_str = canon.to_string_lossy();
            let canon_upper = canon_str.trim_start_matches(r"\\?\").to_uppercase();
            let canon_norm = canon_upper.replace('/', "\\");

            let canon_sys = canon_norm == "C:"
                || canon_norm == "C:\\"
                || canon_norm.starts_with("C:\\WINDOWS")
                || canon_norm.starts_with("C:\\PROGRAM FILES")
                || canon_norm.starts_with("C:\\PROGRAMDATA\\MICROSOFT")
                || canon_norm.starts_with("C:\\USERS\\DEFAULT")
                || canon_norm.starts_with("C:\\RECOVERY")
                || canon_norm == "C:\\PAGEFILE.SYS"
                || canon_norm == "C:\\HIBERFIL.SYS"
                || canon_norm == "C:\\SWAPFILE.SYS";

            let canon_boot = canon_norm == "C:\\BOOTMGR"
                || canon_norm.starts_with("C:\\BOOT\\")
                || canon_norm == "C:\\BOOT";

            return (is_sys || canon_sys, is_boot || canon_boot);
        }

        (is_sys, is_boot)
    }
}
