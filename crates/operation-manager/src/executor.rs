use crate::cancellation::CancellationToken;
use crate::models::{Operation, OperationProgress, OperationType};
use locardx_common::LocardError;
use locardx_verification::IntegrityService;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type ProgressCallback = Arc<dyn Fn(OperationProgress) + Send + Sync>;

/// Contract for asynchronous execution of an operation.
/// Establishes an explicit boundary between the Operation Manager (orchestration)
/// and the domain engine (execution).
pub trait OperationExecutor: Send + Sync {
    fn execute(
        &self,
        operation: &Operation,
        parameters: Option<serde_json::Value>,
        cancellation: CancellationToken,
        on_progress: ProgressCallback,
    ) -> Pin<Box<dyn Future<Output = Result<String, LocardError>> + Send>>;
}

/// Executor for cryptographic SHA-256 evidence hashing.
pub struct IntegrityHashExecutor {
    integrity_service: Arc<IntegrityService>,
}

impl IntegrityHashExecutor {
    pub fn new(integrity_service: Arc<IntegrityService>) -> Self {
        Self { integrity_service }
    }
}

impl OperationExecutor for IntegrityHashExecutor {
    fn execute(
        &self,
        operation: &Operation,
        _parameters: Option<serde_json::Value>,
        cancellation: CancellationToken,
        on_progress: ProgressCallback,
    ) -> Pin<Box<dyn Future<Output = Result<String, LocardError>> + Send>> {
        let service = Arc::clone(&self.integrity_service);
        let path = operation.target.identifier.clone();
        let actor = operation.actor_id.clone();

        Box::pin(async move {
            cancellation.check_cancelled()?;

            on_progress(
                OperationProgress::new("Hashing", "Computing streaming SHA-256 digest")
                    .with_percentage(10.0)?,
            );

            // Execute read-only streaming calculation
            let result = service.calculate_file_hash(&path, actor.as_deref())?;

            cancellation.check_cancelled()?;

            let completed_progress = OperationProgress::new("Completed", "SHA-256 calculated")
                .with_bytes(result.bytes_processed, Some(result.bytes_processed))
                .with_percentage(100.0)?;
            on_progress(completed_progress);

            Ok(format!("Calculated SHA-256: {}", result.digest))
        })
    }
}

/// Executor for cryptographic integrity verification against an expected reference hash.
pub struct IntegrityVerifyExecutor {
    integrity_service: Arc<IntegrityService>,
}

impl IntegrityVerifyExecutor {
    pub fn new(integrity_service: Arc<IntegrityService>) -> Self {
        Self { integrity_service }
    }
}

impl OperationExecutor for IntegrityVerifyExecutor {
    fn execute(
        &self,
        operation: &Operation,
        parameters: Option<serde_json::Value>,
        cancellation: CancellationToken,
        on_progress: ProgressCallback,
    ) -> Pin<Box<dyn Future<Output = Result<String, LocardError>> + Send>> {
        let service = Arc::clone(&self.integrity_service);
        let path = operation.target.identifier.clone();
        let actor = operation.actor_id.clone();

        Box::pin(async move {
            cancellation.check_cancelled()?;

            let expected_digest = parameters
                .and_then(|p| {
                    p.get("expected_digest")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                })
                .ok_or_else(|| {
                    LocardError::Operation(
                        "IntegrityVerify operation requires 'expected_digest' parameter"
                            .to_string(),
                    )
                })?;

            on_progress(
                OperationProgress::new("Verifying", "Comparing target digest against expected")
                    .with_percentage(15.0)?,
            );

            // Execute read-only streaming verification
            let result = service.verify_file_hash(&path, &expected_digest, actor.as_deref())?;

            cancellation.check_cancelled()?;

            let completed_progress =
                OperationProgress::new("Completed", format!("Status: {}", result.status))
                    .with_bytes(result.bytes_processed, Some(result.bytes_processed))
                    .with_percentage(100.0)?;
            on_progress(completed_progress);

            match result.status {
                locardx_verification::VerificationStatus::Verified => {
                    Ok(format!("VERIFIED (Match: {})", result.expected_digest))
                }
                locardx_verification::VerificationStatus::Mismatch {
                    expected,
                    calculated,
                } => Err(LocardError::Operation(format!(
                    "MISMATCH: Expected '{}', calculated '{}'",
                    expected, calculated
                ))),
                locardx_verification::VerificationStatus::UnableToVerify { reason } => Err(
                    LocardError::Operation(format!("UNABLE TO VERIFY: {}", reason)),
                ),
            }
        })
    }
}

/// Fallback executor for disabled/unimplemented destructive operations.
/// Explicitly enforces security boundaries: prevents accidental or unauthorized execution.
pub struct DisabledOperationExecutor {
    operation_type: OperationType,
}

impl DisabledOperationExecutor {
    pub fn new(operation_type: OperationType) -> Self {
        Self { operation_type }
    }
}

impl OperationExecutor for DisabledOperationExecutor {
    fn execute(
        &self,
        _operation: &Operation,
        _parameters: Option<serde_json::Value>,
        _cancellation: CancellationToken,
        _on_progress: ProgressCallback,
    ) -> Pin<Box<dyn Future<Output = Result<String, LocardError>> + Send>> {
        let op_type = self.operation_type;
        Box::pin(async move {
            Err(LocardError::Operation(format!(
                "Operation type '{}' is disabled / not implemented in this build.",
                op_type
            )))
        })
    }
}

/// Non-destructive executor for evaluating and persisting sanitization plans.
pub struct SanitizationPlanExecutor {
    sanitization_service: Arc<locardx_verification::SanitizationService>,
}

impl SanitizationPlanExecutor {
    pub fn new(sanitization_service: Arc<locardx_verification::SanitizationService>) -> Self {
        Self {
            sanitization_service,
        }
    }
}

impl OperationExecutor for SanitizationPlanExecutor {
    fn execute(
        &self,
        operation: &Operation,
        parameters: Option<serde_json::Value>,
        cancellation: CancellationToken,
        on_progress: ProgressCallback,
    ) -> Pin<Box<dyn Future<Output = Result<String, LocardError>> + Send>> {
        let service = Arc::clone(&self.sanitization_service);
        let target = operation.target.clone();
        let actor = operation.actor_id.clone();
        let op_id = operation.operation_id.clone();

        Box::pin(async move {
            cancellation.check_cancelled()?;

            on_progress(
                OperationProgress::new(
                    "Probing",
                    "Inspecting target media and device capabilities",
                )
                .with_percentage(20.0)?,
            );

            // Determine scope from parameters or infer from target type
            let (scope, req_method, req_strategy) = if let Some(params) = &parameters {
                let scope = params
                    .get("scope")
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "File" => locardx_verification::SanitizationScope::File,
                        "Folder" => locardx_verification::SanitizationScope::Folder,
                        "LogicalVolume" => locardx_verification::SanitizationScope::LogicalVolume,
                        _ => locardx_verification::SanitizationScope::PhysicalDevice,
                    })
                    .unwrap_or_else(|| match target.target_type {
                        locardx_common::TargetType::File => {
                            locardx_verification::SanitizationScope::File
                        }
                        locardx_common::TargetType::Directory => {
                            locardx_verification::SanitizationScope::Folder
                        }
                        locardx_common::TargetType::LogicalVolume => {
                            locardx_verification::SanitizationScope::LogicalVolume
                        }
                        _ => locardx_verification::SanitizationScope::PhysicalDevice,
                    });
                (scope, None, None)
            } else {
                let scope = match target.target_type {
                    locardx_common::TargetType::File => {
                        locardx_verification::SanitizationScope::File
                    }
                    locardx_common::TargetType::Directory => {
                        locardx_verification::SanitizationScope::Folder
                    }
                    locardx_common::TargetType::LogicalVolume => {
                        locardx_verification::SanitizationScope::LogicalVolume
                    }
                    _ => locardx_verification::SanitizationScope::PhysicalDevice,
                };
                (scope, None, None)
            };

            cancellation.check_cancelled()?;

            on_progress(
                OperationProgress::new(
                    "Evaluating",
                    "Evaluating sanitization standards and method applicability",
                )
                .with_percentage(50.0)?,
            );

            let plan =
                service.evaluate_plan(&target, scope, req_method, req_strategy, actor.clone())?;

            cancellation.check_cancelled()?;

            on_progress(
                OperationProgress::new(
                    "Evidence",
                    "Recording pre-erasure evidence and verification plan",
                )
                .with_percentage(80.0)?,
            );

            let _ = service.record_pre_erasure_evidence(&op_id, &plan, None, actor)?;

            on_progress(
                OperationProgress::new(
                    "Completed",
                    "Sanitization plan evaluation complete (Dry Run)",
                )
                .with_percentage(100.0)?,
            );

            if plan.is_applicable {
                Ok(format!(
                    "PLAN CREATED: Method={}, Standard={:?}, Risk={}, Verification={}",
                    plan.recommended_method,
                    plan.applicable_standard,
                    plan.risk_level,
                    plan.verification_strategy
                ))
            } else {
                Err(LocardError::Operation(format!(
                    "PLAN REJECTED: Target '{}' is not eligible for sanitization ({:?})",
                    target.identifier, plan.reason_codes
                )))
            }
        })
    }
}

/// Executor for surgical single-file sanitization and unlinking.
pub struct FileEraserExecutor {
    file_eraser_service: Arc<locardx_file_eraser::FileEraserService>,
}

impl FileEraserExecutor {
    pub fn new(file_eraser_service: Arc<locardx_file_eraser::FileEraserService>) -> Self {
        Self {
            file_eraser_service,
        }
    }
}

impl OperationExecutor for FileEraserExecutor {
    fn execute(
        &self,
        operation: &Operation,
        parameters: Option<serde_json::Value>,
        cancellation: CancellationToken,
        on_progress: ProgressCallback,
    ) -> Pin<Box<dyn Future<Output = Result<String, LocardError>> + Send>> {
        let service = Arc::clone(&self.file_eraser_service);
        let path = operation.target.identifier.clone();
        let actor = operation.actor_id.clone();
        let op_id = operation.operation_id.clone();

        Box::pin(async move {
            cancellation.check_cancelled()?;

            let params = parameters.unwrap_or_default();
            let confirmation_id = params
                .get("confirmation_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let typed_confirmation = params
                .get("typed_confirmation")
                .and_then(|v| v.as_str())
                .unwrap_or(&path)
                .to_string();
            let warning_ack = params
                .get("warning_acknowledged")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let session_token = params
                .get("session_token")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let method_param = params.get("method").and_then(|v| v.as_str());
            let method = match method_param {
                Some("nist_800_88_clear_zero") | Some("Nist80088ClearZero") => {
                    Some(locardx_verification::sanitization::SanitizationMethod::Nist80088ClearZero)
                }
                _ => Some(locardx_verification::sanitization::SanitizationMethod::LogicalFileShred),
            };

            // 1. Plan file erasure
            let plan = service.plan_file_erasure(&path, method, actor).await?;

            on_progress(
                OperationProgress::new(
                    "Authorizing",
                    "Validating confirmation challenge and target snapshot",
                )
                .with_percentage(10.0)?,
            );

            let cancel_clone = cancellation.clone();
            let cancel_check: locardx_file_eraser::sanitizer::CancellationCheck =
                Box::new(move || cancel_clone.is_cancelled());

            let on_progress_clone = Arc::clone(&on_progress);
            let sanitizer_progress: locardx_file_eraser::sanitizer::SanitizerProgress =
                Box::new(move |bytes_done, total_bytes, pass, total_passes| {
                    let pct = 10.0
                        + (pass as f32 - 1.0) / (total_passes as f32) * 80.0
                        + if total_bytes > 0 {
                            (bytes_done as f32 / total_bytes as f32) * (80.0 / total_passes as f32)
                        } else {
                            0.0
                        };
                    let _ = on_progress_clone(
                        OperationProgress::new(
                            "Sanitizing",
                            format!(
                                "Pass {}/{}: {}/{} bytes written",
                                pass, total_passes, bytes_done, total_bytes
                            ),
                        )
                        .with_percentage(pct.min(90.0))
                        .unwrap_or_default(),
                    );
                });

            // 2. Execute
            let result = service
                .execute_file_erasure(
                    &plan.plan_id,
                    &confirmation_id,
                    &op_id,
                    &typed_confirmation,
                    warning_ack,
                    &session_token,
                    Some(&cancel_check),
                    Some(&sanitizer_progress),
                )
                .await?;

            on_progress(
                OperationProgress::new("Completed", format!("Finished: {}", result.status))
                    .with_percentage(100.0)?,
            );

            if result.status == locardx_file_eraser::FileEraseStatus::Completed {
                Ok(format!(
                    "FILE ERASED: Target '{}' unlinked and verified inaccessible ({})",
                    result.canonical_path, result.verification.outcome
                ))
            } else {
                Err(LocardError::Operation(format!(
                    "File erasure ended with status: {} (Verification: {})",
                    result.status, result.verification.outcome
                )))
            }
        })
    }
}

/// Executor for recursive folder sanitization and tree unlinking.
pub struct FolderEraserExecutor {
    file_eraser_service: Arc<locardx_file_eraser::FileEraserService>,
}

impl FolderEraserExecutor {
    pub fn new(file_eraser_service: Arc<locardx_file_eraser::FileEraserService>) -> Self {
        Self {
            file_eraser_service,
        }
    }
}

impl OperationExecutor for FolderEraserExecutor {
    fn execute(
        &self,
        operation: &Operation,
        parameters: Option<serde_json::Value>,
        cancellation: CancellationToken,
        on_progress: ProgressCallback,
    ) -> Pin<Box<dyn Future<Output = Result<String, LocardError>> + Send>> {
        let service = Arc::clone(&self.file_eraser_service);
        let path = operation.target.identifier.clone();
        let actor = operation.actor_id.clone();
        let op_id = operation.operation_id.clone();

        Box::pin(async move {
            cancellation.check_cancelled()?;

            let params = parameters.unwrap_or_default();
            let confirmation_id = params
                .get("confirmation_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let typed_confirmation = params
                .get("typed_confirmation")
                .and_then(|v| v.as_str())
                .unwrap_or(&path)
                .to_string();
            let warning_ack = params
                .get("warning_acknowledged")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let session_token = params
                .get("session_token")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let method_param = params.get("method").and_then(|v| v.as_str());
            let method = match method_param {
                Some("nist_800_88_clear_zero") | Some("Nist80088ClearZero") => {
                    Some(locardx_verification::sanitization::SanitizationMethod::Nist80088ClearZero)
                }
                _ => Some(
                    locardx_verification::sanitization::SanitizationMethod::DirectoryRecursiveShred,
                ),
            };

            let plan = service.plan_folder_erasure(&path, method, actor).await?;

            on_progress(
                OperationProgress::new(
                    "Scanning",
                    "Traversing directory tree and validating safety",
                )
                .with_percentage(10.0)?,
            );

            let cancel_clone = cancellation.clone();
            let cancel_check: locardx_file_eraser::sanitizer::CancellationCheck =
                Box::new(move || cancel_clone.is_cancelled());

            let on_progress_clone = Arc::clone(&on_progress);
            let folder_progress: locardx_file_eraser::folder::FolderProgress =
                Box::new(move |processed, total, current_item| {
                    let pct = if total > 0 {
                        10.0 + (processed as f32 / total as f32) * 85.0
                    } else {
                        10.0
                    };
                    let _ = on_progress_clone(
                        OperationProgress::new(
                            "Sanitizing Folder",
                            format!("Processing item {}/{}: {}", processed, total, current_item),
                        )
                        .with_percentage(pct.min(95.0))
                        .unwrap_or_default(),
                    );
                });

            let result = service
                .execute_folder_erasure(
                    &plan.plan_id,
                    &confirmation_id,
                    &op_id,
                    &typed_confirmation,
                    warning_ack,
                    &session_token,
                    Some(&cancel_check),
                    Some(&folder_progress),
                )
                .await?;

            on_progress(
                OperationProgress::new("Completed", format!("Status: {}", result.status))
                    .with_percentage(100.0)?,
            );

            if result.status == locardx_file_eraser::FileEraseStatus::Completed {
                let stats = result.folder_stats.unwrap_or_default();
                Ok(format!(
                    "FOLDER ERASED: Target '{}' completely removed ({} files, {} dirs unlinked)",
                    result.canonical_path, stats.files_sanitized, stats.directories_removed
                ))
            } else {
                Err(LocardError::Operation(format!(
                    "Folder erasure ended with status: {} (Verification: {})",
                    result.status, result.verification.outcome
                )))
            }
        })
    }
}

/// Executor for simulated physical storage device sanitization.
/// In Step 10A, execution is strictly confined to the simulation layer (`ExecutionMode::Simulation`).
pub struct DriveEraserSimulationExecutor {
    drive_eraser: Arc<locardx_drive_eraser::DriveEraserService>,
}

impl DriveEraserSimulationExecutor {
    pub fn new(drive_eraser: Arc<locardx_drive_eraser::DriveEraserService>) -> Self {
        Self { drive_eraser }
    }
}

impl OperationExecutor for DriveEraserSimulationExecutor {
    fn execute(
        &self,
        operation: &Operation,
        parameters: Option<serde_json::Value>,
        cancellation: CancellationToken,
        on_progress: ProgressCallback,
    ) -> Pin<Box<dyn Future<Output = Result<String, LocardError>> + Send>> {
        let service = Arc::clone(&self.drive_eraser);
        let target_id = operation.target.identifier.clone();
        let op_id = operation.operation_id.clone();
        let actor = operation.actor_id.clone();

        Box::pin(async move {
            cancellation.check_cancelled()?;

            let params = parameters.ok_or_else(|| {
                LocardError::Operation(
                    "Parameters required for drive erasure simulation".to_string(),
                )
            })?;

            let requested_method = params
                .get("requested_method")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let confirmation_id = params
                .get("confirmation_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let typed_confirmation = params
                .get("typed_confirmation")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let warning_ack = params
                .get("warning_acknowledged")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let session_token = params
                .get("session_token")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            on_progress(
                OperationProgress::new(
                    "Planning",
                    format!(
                        "Evaluating hardware capabilities for target '{}'",
                        target_id
                    ),
                )
                .with_percentage(5.0)?,
            );

            // 1. Generate plan
            let plan = service
                .plan_drive_erasure(
                    locardx_drive_eraser::DriveEraseRequest {
                        target_device_id: target_id.clone(),
                        requested_method,
                        execution_mode: Some(locardx_drive_eraser::ExecutionMode::Simulation),
                        session_token: Some(session_token.clone()),
                    },
                    actor,
                )
                .await?;

            cancellation.check_cancelled()?;

            on_progress(
                OperationProgress::new(
                    "Simulating Sanitization",
                    format!("Method: {:?}, Passes: {}", plan.method, plan.passes),
                )
                .with_percentage(15.0)?,
            );

            let cancel_clone = cancellation.clone();
            let cancel_fn = move || cancel_clone.is_cancelled();

            let on_prog_clone = Arc::clone(&on_progress);
            let progress_fn = move |p: locardx_drive_eraser::DriveEraseProgress| {
                let pct: f32 = 15.0 + (p.percentage / 100.0) * 80.0;
                let _ = on_prog_clone(
                    OperationProgress::new(
                        "Simulating Sanitization",
                        format!(
                            "Pass {}/{}: {:.1}% ({}/{} bytes)",
                            p.current_pass,
                            p.total_passes,
                            p.percentage,
                            p.bytes_processed,
                            p.total_bytes
                        ),
                    )
                    .with_percentage(pct.min(95.0))
                    .unwrap_or_default(),
                );
            };

            // 2. Execute simulation
            let result = service
                .execute_drive_erasure_simulation(
                    &plan.plan_id,
                    &confirmation_id,
                    &op_id,
                    &typed_confirmation,
                    warning_ack,
                    &session_token,
                    locardx_drive_eraser::ExecutionMode::Simulation,
                    Some(&cancel_fn),
                    Some(&progress_fn),
                )
                .await?;

            on_progress(
                OperationProgress::new("Completed", format!("Status: {}", result.status))
                    .with_percentage(100.0)?,
            );

            if result.status == locardx_drive_eraser::DriveEraseStatus::Completed {
                Ok(format!(
                    "SIMULATED DRIVE ERASURE COMPLETED: Physical device '{}' sanitized via {:?} (Verification: {})",
                    result.physical_device_id, result.method, result.verification.outcome
                ))
            } else if result.status == locardx_drive_eraser::DriveEraseStatus::Cancelled {
                Err(LocardError::Operation(
                    "Drive erasure simulation cancelled".to_string(),
                ))
            } else {
                Err(LocardError::Operation(format!(
                    "Drive erasure simulation ended with status: {:?}",
                    result.status
                )))
            }
        })
    }
}
