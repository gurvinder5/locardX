use crate::folder::{sanitize_folder_recursive, FolderProgress};
use crate::models::{
    FileEraseFailureReason, FileErasePlan, FileEraseResult, FileEraseScope, FileEraseStatus,
    FileVerificationResult, FolderEraseStats,
};
use crate::sanitizer::{sanitize_file_content_and_remove, CancellationCheck, SanitizerProgress};
use crate::validation::{
    validate_file_target, validate_folder_target, verify_target_snapshot_integrity,
};
use crate::verifier::{verify_file_erasure, verify_folder_erasure};
use chrono::Utc;
use locardx_audit::AuditService;
use locardx_common::LocardError;
use locardx_database::Database;
use locardx_security::{RiskLevel, SafetyDecisionOutcome, SafetyEngine};
use locardx_verification::sanitization::{
    SanitizationMethod, VerificationOutcome, VerificationStrategy,
};
use rusqlite::params;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Core service for planning, authorizing, executing, and verifying surgical file and folder erasure.
pub struct FileEraserService {
    db: Arc<Database>,
    audit: Arc<AuditService>,
    safety: Arc<SafetyEngine>,
    plans: Arc<RwLock<HashMap<String, FileErasePlan>>>,
}

impl FileEraserService {
    pub fn new(db: Arc<Database>, audit: Arc<AuditService>, safety: Arc<SafetyEngine>) -> Self {
        Self {
            db,
            audit,
            safety,
            plans: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Evaluates and generates a safe plan for single-file sanitization.
    pub async fn plan_file_erasure(
        &self,
        target_path: &str,
        requested_method: Option<SanitizationMethod>,
        actor_id: Option<String>,
    ) -> Result<FileErasePlan, LocardError> {
        // 1. Validate target file safely
        let metadata = validate_file_target(target_path).map_err(|e| {
            LocardError::SecurityViolation(format!("Target validation failed: {}", e))
        })?;

        let method = match requested_method {
            Some(m @ SanitizationMethod::LogicalFileShred)
            | Some(m @ SanitizationMethod::Nist80088ClearZero) => m,
            Some(other) => {
                return Err(LocardError::Operation(format!(
                    "Method '{:?}' is not supported for logical file erasure. Use LogicalFileShred or Nist80088ClearZero.",
                    other
                )));
            }
            None => SanitizationMethod::LogicalFileShred,
        };

        let passes = match method {
            SanitizationMethod::LogicalFileShred => 3,
            SanitizationMethod::Nist80088ClearZero => 1,
            _ => 1,
        };

        let limitations = vec![
            "Logical filesystem erasure does not guarantee physical block erasure on SSDs/NVMe due to Flash Translation Layer (FTL) wear leveling.".to_string(),
            "Filesystem journaling (NTFS USN / ext4 journal) may retain file name or metadata copies.".to_string(),
            "Volume Shadow Copies (VSS) or automated backups may retain historical versions.".to_string(),
            "Copy-on-write (CoW) filesystems may retain unreferenced allocations.".to_string(),
        ];

        let plan_id = format!("fplan-{}", Uuid::new_v4());
        let plan = FileErasePlan {
            plan_id: plan_id.clone(),
            target_path: target_path.to_string(),
            canonical_path: metadata.canonical_path.clone(),
            scope: FileEraseScope::File,
            method,
            passes,
            verification_strategy: VerificationStrategy::MetadataUnlinkCheck,
            limitations,
            pre_metadata: metadata,
            risk_level: RiskLevel::High,
            created_at: Utc::now().to_rfc3339(),
            scope_description: "LOGICAL FILESYSTEM SCOPE".to_string(),
        };

        // Cache plan
        {
            let mut p = self.plans.write().await;
            p.insert(plan_id.clone(), plan.clone());
        }

        // Audit planning event
        self.audit.log_structured_event(
            "FILE_ERASURE_REQUESTED",
            actor_id.as_deref(),
            Some(target_path),
            &format!(
                "File erasure plan '{}' created for target '{}' using method {:?}",
                plan_id, target_path, method
            ),
        )?;

        Ok(plan)
    }

    /// Evaluates and generates a safe plan for recursive folder sanitization.
    pub async fn plan_folder_erasure(
        &self,
        target_path: &str,
        requested_method: Option<SanitizationMethod>,
        actor_id: Option<String>,
    ) -> Result<FileErasePlan, LocardError> {
        let metadata = validate_folder_target(target_path).map_err(|e| {
            LocardError::SecurityViolation(format!("Target folder validation failed: {}", e))
        })?;

        let method = match requested_method {
            Some(m @ SanitizationMethod::DirectoryRecursiveShred)
            | Some(m @ SanitizationMethod::LogicalFileShred)
            | Some(m @ SanitizationMethod::Nist80088ClearZero) => m,
            Some(other) => {
                return Err(LocardError::Operation(format!(
                    "Method '{:?}' is not supported for folder erasure. Use DirectoryRecursiveShred, LogicalFileShred or Nist80088ClearZero.",
                    other
                )));
            }
            None => SanitizationMethod::DirectoryRecursiveShred,
        };

        let passes = match method {
            SanitizationMethod::DirectoryRecursiveShred | SanitizationMethod::LogicalFileShred => 3,
            SanitizationMethod::Nist80088ClearZero => 1,
            _ => 1,
        };

        let limitations = vec![
            "Recursive folder sanitization processes child files individually; FTL wear leveling applies to all underlying blocks.".to_string(),
            "Directory index entries and access timestamps in filesystem metadata may remain in unallocated space.".to_string(),
            "Operating system shadow copies and snapshot replicas are not purged by folder erasure.".to_string(),
        ];

        let plan_id = format!("dplan-{}", Uuid::new_v4());
        let plan = FileErasePlan {
            plan_id: plan_id.clone(),
            target_path: target_path.to_string(),
            canonical_path: metadata.canonical_path.clone(),
            scope: FileEraseScope::Folder,
            method,
            passes,
            verification_strategy: VerificationStrategy::MetadataUnlinkCheck,
            limitations,
            pre_metadata: metadata,
            risk_level: RiskLevel::High,
            created_at: Utc::now().to_rfc3339(),
            scope_description: "LOGICAL FILESYSTEM SCOPE".to_string(),
        };

        {
            let mut p = self.plans.write().await;
            p.insert(plan_id.clone(), plan.clone());
        }

        self.audit.log_structured_event(
            "FOLDER_ERASURE_REQUESTED",
            actor_id.as_deref(),
            Some(target_path),
            &format!(
                "Folder erasure plan '{}' created for target '{}' using method {:?}",
                plan_id, target_path, method
            ),
        )?;

        Ok(plan)
    }

    /// Fetches a previously generated plan by ID.
    pub async fn get_plan(&self, plan_id: &str) -> Result<FileErasePlan, LocardError> {
        let p = self.plans.read().await;
        p.get(plan_id)
            .cloned()
            .ok_or_else(|| LocardError::Operation(format!("Plan '{}' not found", plan_id)))
    }

    /// Executes file erasure after validating two-stage confirmation challenge and checking TOCTOU snapshot.
    pub async fn execute_file_erasure(
        &self,
        plan_id: &str,
        confirmation_id: &str,
        operation_id: &str,
        typed_target_confirmation: &str,
        warning_acknowledged: bool,
        session_token: &str,
        is_cancelled: Option<&CancellationCheck>,
        on_progress: Option<&SanitizerProgress>,
    ) -> Result<FileEraseResult, LocardError> {
        let plan = self.get_plan(plan_id).await?;
        let started_at = Utc::now().to_rfc3339();

        // 1. Validate confirmation through SafetyEngine
        let safety_decision = self.safety.confirm_destructive_operation(
            confirmation_id,
            operation_id,
            warning_acknowledged,
            typed_target_confirmation,
            session_token,
        )?;

        if safety_decision.decision != SafetyDecisionOutcome::Allowed {
            return Err(LocardError::SecurityViolation(format!(
                "Safety authorization denied: {} ({})",
                safety_decision.message, safety_decision.reason_code
            )));
        }

        self.audit.log_structured_event(
            "FILE_ERASURE_AUTHORIZED",
            safety_decision.actor_id.as_deref(),
            Some(&plan.canonical_path),
            &format!(
                "File erasure authorized for plan {} / operation {}",
                plan_id, operation_id
            ),
        )?;

        // 2. LIVE TARGET REVALIDATION (TOCTOU Defense)
        verify_target_snapshot_integrity(&plan.pre_metadata).map_err(|e| {
            let _ = self.audit.log_structured_event(
                "FILE_ERASURE_FAILED",
                safety_decision.actor_id.as_deref(),
                Some(&plan.canonical_path),
                &format!(
                    "Target snapshot verification failed before execution: {}",
                    e
                ),
            );
            LocardError::SecurityViolation(format!("Target snapshot verification failed: {}", e))
        })?;

        self.audit.log_structured_event(
            "FILE_ERASURE_STARTED",
            safety_decision.actor_id.as_deref(),
            Some(&plan.canonical_path),
            &format!(
                "Beginning multi-pass file content sanitization ({} passes)",
                plan.passes
            ),
        )?;

        // 3. Content Sanitization & Filesystem Entry Removal
        let target_path = Path::new(&plan.canonical_path);
        let sanitize_res = sanitize_file_content_and_remove(
            target_path,
            &plan.pre_metadata,
            plan.method,
            is_cancelled,
            on_progress,
        );

        let bytes_processed = match sanitize_res {
            Ok(bytes) => {
                self.audit.log_structured_event(
                    "FILE_ERASURE_CONTENT_SANITIZED",
                    safety_decision.actor_id.as_deref(),
                    Some(&plan.canonical_path),
                    &format!(
                        "File content sanitized ({} bytes written across {} passes)",
                        bytes, plan.passes
                    ),
                )?;
                self.audit.log_structured_event(
                    "FILE_ERASURE_ENTRY_REMOVED",
                    safety_decision.actor_id.as_deref(),
                    Some(&plan.canonical_path),
                    "Filesystem directory entry unlinked successfully",
                )?;
                bytes
            }
            Err(FileEraseFailureReason::Cancelled) => {
                let _ = self.audit.log_structured_event(
                    "FILE_ERASURE_CANCELLED",
                    safety_decision.actor_id.as_deref(),
                    Some(&plan.canonical_path),
                    "File erasure cancelled cooperatively by operator",
                );
                return self.record_result(FileEraseResult {
                    operation_id: operation_id.to_string(),
                    plan_id: plan_id.to_string(),
                    target_path: plan.target_path.clone(),
                    canonical_path: plan.canonical_path.clone(),
                    scope: FileEraseScope::File,
                    method: plan.method,
                    status: FileEraseStatus::Cancelled,
                    bytes_processed: 0,
                    verification: FileVerificationResult {
                        outcome: VerificationOutcome::NotApplicable,
                        strategy: VerificationStrategy::MetadataUnlinkCheck,
                        path_exists: true,
                        inaccessible: false,
                        details: "Operation cancelled before completion".to_string(),
                        verified_at: Utc::now().to_rfc3339(),
                    },
                    folder_stats: None,
                    failure_reason: Some(FileEraseFailureReason::Cancelled),
                    started_at,
                    completed_at: Utc::now().to_rfc3339(),
                    limitations: plan.limitations,
                    scope_description: plan.scope_description,
                });
            }
            Err(err) => {
                let _ = self.audit.log_structured_event(
                    "FILE_ERASURE_FAILED",
                    safety_decision.actor_id.as_deref(),
                    Some(&plan.canonical_path),
                    &format!("File sanitization failed: {}", err),
                );
                return self.record_result(FileEraseResult {
                    operation_id: operation_id.to_string(),
                    plan_id: plan_id.to_string(),
                    target_path: plan.target_path.clone(),
                    canonical_path: plan.canonical_path.clone(),
                    scope: FileEraseScope::File,
                    method: plan.method,
                    status: FileEraseStatus::Failed,
                    bytes_processed: 0,
                    verification: FileVerificationResult {
                        outcome: VerificationOutcome::VerificationFailed,
                        strategy: VerificationStrategy::MetadataUnlinkCheck,
                        path_exists: true,
                        inaccessible: false,
                        details: format!("Execution failed: {}", err),
                        verified_at: Utc::now().to_rfc3339(),
                    },
                    folder_stats: None,
                    failure_reason: Some(err),
                    started_at,
                    completed_at: Utc::now().to_rfc3339(),
                    limitations: plan.limitations,
                    scope_description: plan.scope_description,
                });
            }
        };

        // 4. Verification
        self.audit.log_structured_event(
            "FILE_ERASURE_VERIFICATION_STARTED",
            safety_decision.actor_id.as_deref(),
            Some(&plan.canonical_path),
            "Executing post-erasure target inaccessible verification",
        )?;

        let verification = verify_file_erasure(target_path, &plan.canonical_path);

        let final_status = if verification.outcome == VerificationOutcome::Verified {
            FileEraseStatus::Completed
        } else {
            FileEraseStatus::VerificationFailed
        };

        let completed_at = Utc::now().to_rfc3339();

        self.audit.log_structured_event(
            "FILE_ERASURE_COMPLETED",
            safety_decision.actor_id.as_deref(),
            Some(&plan.canonical_path),
            &format!(
                "File erasure finished with status: {} (Verification: {})",
                final_status, verification.outcome
            ),
        )?;

        let result = FileEraseResult {
            operation_id: operation_id.to_string(),
            plan_id: plan_id.to_string(),
            target_path: plan.target_path,
            canonical_path: plan.canonical_path,
            scope: FileEraseScope::File,
            method: plan.method,
            status: final_status,
            bytes_processed,
            verification,
            folder_stats: None,
            failure_reason: None,
            started_at,
            completed_at,
            limitations: plan.limitations,
            scope_description: plan.scope_description,
        };

        self.record_result(result)
    }

    /// Executes recursive folder erasure after validating two-stage confirmation challenge and checking TOCTOU snapshot.
    pub async fn execute_folder_erasure(
        &self,
        plan_id: &str,
        confirmation_id: &str,
        operation_id: &str,
        typed_target_confirmation: &str,
        warning_acknowledged: bool,
        session_token: &str,
        is_cancelled: Option<&CancellationCheck>,
        on_progress: Option<&FolderProgress>,
    ) -> Result<FileEraseResult, LocardError> {
        let plan = self.get_plan(plan_id).await?;
        let started_at = Utc::now().to_rfc3339();

        let safety_decision = self.safety.confirm_destructive_operation(
            confirmation_id,
            operation_id,
            warning_acknowledged,
            typed_target_confirmation,
            session_token,
        )?;

        if safety_decision.decision != SafetyDecisionOutcome::Allowed {
            return Err(LocardError::SecurityViolation(format!(
                "Safety authorization denied: {} ({})",
                safety_decision.message, safety_decision.reason_code
            )));
        }

        verify_target_snapshot_integrity(&plan.pre_metadata).map_err(|e| {
            let _ = self.audit.log_structured_event(
                "FOLDER_ERASURE_PARTIAL_FAILURE",
                safety_decision.actor_id.as_deref(),
                Some(&plan.canonical_path),
                &format!(
                    "Target snapshot verification failed before execution: {}",
                    e
                ),
            );
            LocardError::SecurityViolation(format!("Target snapshot verification failed: {}", e))
        })?;

        self.audit.log_structured_event(
            "FOLDER_ERASURE_STARTED",
            safety_decision.actor_id.as_deref(),
            Some(&plan.canonical_path),
            "Beginning recursive folder sanitization and tree traversal",
        )?;

        let target_path = Path::new(&plan.canonical_path);
        let folder_res =
            sanitize_folder_recursive(target_path, plan.method, is_cancelled, on_progress);

        let (stats, failure_reason) = match folder_res {
            Ok(s) => (s, None),
            Err((s, err)) => (s, Some(err)),
        };

        let verification = verify_folder_erasure(target_path, &plan.canonical_path, &stats);

        let status = match &failure_reason {
            None => {
                if verification.outcome == VerificationOutcome::Verified {
                    FileEraseStatus::Completed
                } else {
                    FileEraseStatus::VerificationFailed
                }
            }
            Some(FileEraseFailureReason::Cancelled) => FileEraseStatus::Cancelled,
            Some(_) => FileEraseStatus::Failed,
        };

        let completed_at = Utc::now().to_rfc3339();

        if status == FileEraseStatus::Completed {
            self.audit.log_structured_event(
                "FOLDER_ERASURE_COMPLETED",
                safety_decision.actor_id.as_deref(),
                Some(&plan.canonical_path),
                &format!(
                    "Folder erasure completed successfully. {} files sanitized, {} directories unlinked.",
                    stats.files_sanitized, stats.directories_removed
                ),
            )?;
        } else {
            self.audit.log_structured_event(
                "FOLDER_ERASURE_PARTIAL_FAILURE",
                safety_decision.actor_id.as_deref(),
                Some(&plan.canonical_path),
                &format!(
                    "Folder erasure ended with status {}: sanitized={}, failed={}, cancelled={}",
                    status, stats.files_sanitized, stats.files_failed, stats.files_cancelled
                ),
            )?;
        }

        let result = FileEraseResult {
            operation_id: operation_id.to_string(),
            plan_id: plan_id.to_string(),
            target_path: plan.target_path,
            canonical_path: plan.canonical_path,
            scope: FileEraseScope::Folder,
            method: plan.method,
            status,
            bytes_processed: stats.bytes_sanitized,
            verification,
            folder_stats: Some(stats),
            failure_reason,
            started_at,
            completed_at,
            limitations: plan.limitations,
            scope_description: plan.scope_description,
        };

        self.record_result(result)
    }

    /// Persists the execution record into SQLite table `file_erasure_records`.
    fn record_result(&self, result: FileEraseResult) -> Result<FileEraseResult, LocardError> {
        let record_id = Uuid::new_v4().to_string();
        let total_files = result
            .folder_stats
            .as_ref()
            .map(|s| s.total_files)
            .unwrap_or(1) as i64;
        let files_sanitized = result
            .folder_stats
            .as_ref()
            .map(|s| s.files_sanitized)
            .unwrap_or(if result.status == FileEraseStatus::Completed {
                1
            } else {
                0
            }) as i64;
        let files_failed = result
            .folder_stats
            .as_ref()
            .map(|s| s.files_failed)
            .unwrap_or(if result.status == FileEraseStatus::Failed {
                1
            } else {
                0
            }) as i64;
        let directories_removed = result
            .folder_stats
            .as_ref()
            .map(|s| s.directories_removed)
            .unwrap_or(0) as i64;

        let pre_metadata_json = serde_json::to_string(&result.verification).unwrap_or_default();
        let limitations_json = serde_json::to_string(&result.limitations).unwrap_or_default();
        let failure_reason_str = result.failure_reason.as_ref().map(|f| f.to_string());

        let _ = self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO file_erasure_records (
                    record_id, operation_id, actor_id, target_path, canonical_path,
                    scope, sanitization_method, status, verification_outcome, bytes_processed,
                    total_files, files_sanitized, files_failed, directories_removed,
                    pre_metadata_json, failure_reason, limitations, started_at, completed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    record_id,
                    result.operation_id,
                    Option::<String>::None,
                    result.target_path,
                    result.canonical_path,
                    result.scope.to_string(),
                    result.method.to_string(),
                    result.status.to_string(),
                    result.verification.outcome.to_string(),
                    result.bytes_processed as i64,
                    total_files,
                    files_sanitized,
                    files_failed,
                    directories_removed,
                    pre_metadata_json,
                    failure_reason_str,
                    limitations_json,
                    result.started_at,
                    result.completed_at,
                ],
            )?;
            Ok(())
        });

        Ok(result)
    }

    /// Fetches an erasure result by operation ID.
    pub fn get_erasure_result(
        &self,
        operation_id: &str,
    ) -> Result<Option<FileEraseResult>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT record_id, operation_id, target_path, canonical_path, scope,
                        sanitization_method, status, verification_outcome, bytes_processed,
                        total_files, files_sanitized, files_failed, directories_removed,
                        failure_reason, limitations, started_at, completed_at
                 FROM file_erasure_records WHERE operation_id = ?1",
            )?;

            let mut rows = stmt.query(params![operation_id])?;
            if let Some(row) = rows.next()? {
                let op_id: String = row.get(1)?;
                let target_path: String = row.get(2)?;
                let canonical_path: String = row.get(3)?;
                let scope_str: String = row.get(4)?;
                let method_str: String = row.get(5)?;
                let status_str: String = row.get(6)?;
                let outcome_str: String = row.get(7)?;
                let bytes_processed: i64 = row.get(8)?;
                let total_files: i64 = row.get(9)?;
                let files_sanitized: i64 = row.get(10)?;
                let files_failed: i64 = row.get(11)?;
                let directories_removed: i64 = row.get(12)?;
                let failure_reason_str: Option<String> = row.get(13)?;
                let limitations_json: String = row.get(14)?;
                let started_at: String = row.get(15)?;
                let completed_at: String = row.get(16)?;

                let scope = if scope_str == "Folder" {
                    FileEraseScope::Folder
                } else {
                    FileEraseScope::File
                };
                let method = match method_str.as_str() {
                    s if s.contains("Directory") => SanitizationMethod::DirectoryRecursiveShred,
                    s if s.contains("Shred") => SanitizationMethod::LogicalFileShred,
                    _ => SanitizationMethod::Nist80088ClearZero,
                };
                let status = match status_str.as_str() {
                    "Completed" => FileEraseStatus::Completed,
                    "Cancelled" => FileEraseStatus::Cancelled,
                    "VerificationFailed" => FileEraseStatus::VerificationFailed,
                    _ => FileEraseStatus::Failed,
                };
                let outcome = match outcome_str.as_str() {
                    "Verified" => VerificationOutcome::Verified,
                    "PartiallyVerified" => VerificationOutcome::PartiallyVerified,
                    _ => VerificationOutcome::VerificationFailed,
                };

                let limitations: Vec<String> =
                    serde_json::from_str(&limitations_json).unwrap_or_default();

                let folder_stats = if scope == FileEraseScope::Folder {
                    Some(FolderEraseStats {
                        total_files: total_files as usize,
                        files_sanitized: files_sanitized as usize,
                        files_failed: files_failed as usize,
                        files_cancelled: 0,
                        total_directories: directories_removed as usize,
                        directories_removed: directories_removed as usize,
                        bytes_sanitized: bytes_processed as u64,
                        failures: Vec::new(),
                    })
                } else {
                    None
                };

                Ok(Some(FileEraseResult {
                    operation_id: op_id,
                    plan_id: String::new(),
                    target_path,
                    canonical_path,
                    scope,
                    method,
                    status,
                    bytes_processed: bytes_processed as u64,
                    verification: FileVerificationResult {
                        outcome,
                        strategy: VerificationStrategy::MetadataUnlinkCheck,
                        path_exists: status != FileEraseStatus::Completed,
                        inaccessible: status == FileEraseStatus::Completed,
                        details: format!("Outcome recorded: {}", outcome_str),
                        verified_at: completed_at.clone(),
                    },
                    folder_stats,
                    failure_reason: failure_reason_str.map(FileEraseFailureReason::Unexpected),
                    started_at,
                    completed_at,
                    limitations,
                    scope_description: "LOGICAL FILESYSTEM SCOPE".to_string(),
                }))
            } else {
                Ok(None)
            }
        })
    }
}
