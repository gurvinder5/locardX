use crate::state::AppState;
use locardx_common::SafeErrorResponse;
use locardx_file_eraser::{
    FileErasePlan, FileEraseResult, FileMetadataSnapshot, FileVerificationResult, FolderEraseStats,
};
use locardx_verification::sanitization::SanitizationMethod;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Request payload for planning a file erasure operation.
#[derive(Debug, Clone, Deserialize)]
pub struct PlanFileEraseRequest {
    pub target_path: String,
    pub method: Option<String>,
    pub session_token: Option<String>,
}

/// Request payload for planning a recursive folder erasure operation.
#[derive(Debug, Clone, Deserialize)]
pub struct PlanFolderEraseRequest {
    pub target_path: String,
    pub method: Option<String>,
    pub session_token: Option<String>,
}

/// Request payload for executing a confirmed file erasure operation.
#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteFileEraseRequest {
    pub plan_id: String,
    pub confirmation_id: String,
    pub operation_id: String,
    pub typed_confirmation: String,
    pub warning_acknowledged: bool,
    pub session_token: String,
}

/// Request payload for executing a confirmed recursive folder erasure operation.
#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteFolderEraseRequest {
    pub plan_id: String,
    pub confirmation_id: String,
    pub operation_id: String,
    pub typed_confirmation: String,
    pub warning_acknowledged: bool,
    pub session_token: String,
}

/// DTO for file metadata snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadataSnapshotDto {
    pub path: String,
    pub canonical_path: String,
    pub is_directory: bool,
    pub size_bytes: u64,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub is_readonly: bool,
    pub pre_erasure_sha256: Option<String>,
    pub snapshot_timestamp: String,
}

impl From<&FileMetadataSnapshot> for FileMetadataSnapshotDto {
    fn from(m: &FileMetadataSnapshot) -> Self {
        Self {
            path: m.path.clone(),
            canonical_path: m.canonical_path.clone(),
            is_directory: m.is_directory,
            size_bytes: m.size_bytes,
            created_at: m.created_at.clone(),
            modified_at: m.modified_at.clone(),
            is_readonly: m.is_readonly,
            pre_erasure_sha256: m.pre_erasure_sha256.clone(),
            snapshot_timestamp: m.snapshot_timestamp.clone(),
        }
    }
}

/// DTO for post-erasure verification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVerificationResultDto {
    pub outcome: String,
    pub strategy: String,
    pub path_exists: bool,
    pub inaccessible: bool,
    pub details: String,
    pub verified_at: String,
}

impl From<&FileVerificationResult> for FileVerificationResultDto {
    fn from(v: &FileVerificationResult) -> Self {
        Self {
            outcome: v.outcome.to_string(),
            strategy: v.strategy.to_string(),
            path_exists: v.path_exists,
            inaccessible: v.inaccessible,
            details: v.details.clone(),
            verified_at: v.verified_at.clone(),
        }
    }
}

/// DTO representing a generated file/folder erasure plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileErasePlanDto {
    pub plan_id: String,
    pub target_path: String,
    pub canonical_path: String,
    pub scope: String,
    pub method: String,
    pub passes: usize,
    pub verification_strategy: String,
    pub limitations: Vec<String>,
    pub pre_metadata: FileMetadataSnapshotDto,
    pub risk_level: String,
    pub created_at: String,
    pub scope_description: String,
}

impl From<&FileErasePlan> for FileErasePlanDto {
    fn from(plan: &FileErasePlan) -> Self {
        Self {
            plan_id: plan.plan_id.clone(),
            target_path: plan.target_path.clone(),
            canonical_path: plan.canonical_path.clone(),
            scope: plan.scope.to_string(),
            method: plan.method.to_string(),
            passes: plan.passes,
            verification_strategy: plan.verification_strategy.to_string(),
            limitations: plan.limitations.clone(),
            pre_metadata: FileMetadataSnapshotDto::from(&plan.pre_metadata),
            risk_level: plan.risk_level.to_string(),
            created_at: plan.created_at.clone(),
            scope_description: plan.scope_description.clone(),
        }
    }
}

/// DTO representing recursive folder stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderEraseStatsDto {
    pub total_files: usize,
    pub files_sanitized: usize,
    pub files_failed: usize,
    pub files_cancelled: usize,
    pub total_directories: usize,
    pub directories_removed: usize,
    pub bytes_sanitized: u64,
}

impl From<&FolderEraseStats> for FolderEraseStatsDto {
    fn from(s: &FolderEraseStats) -> Self {
        Self {
            total_files: s.total_files,
            files_sanitized: s.files_sanitized,
            files_failed: s.files_failed,
            files_cancelled: s.files_cancelled,
            total_directories: s.total_directories,
            directories_removed: s.directories_removed,
            bytes_sanitized: s.bytes_sanitized,
        }
    }
}

/// DTO representing the result of an executed file/folder erasure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEraseResultDto {
    pub operation_id: String,
    pub plan_id: String,
    pub target_path: String,
    pub canonical_path: String,
    pub scope: String,
    pub method: String,
    pub status: String,
    pub bytes_processed: u64,
    pub verification: FileVerificationResultDto,
    pub folder_stats: Option<FolderEraseStatsDto>,
    pub failure_reason: Option<String>,
    pub started_at: String,
    pub completed_at: String,
    pub limitations: Vec<String>,
    pub scope_description: String,
}

impl From<&FileEraseResult> for FileEraseResultDto {
    fn from(r: &FileEraseResult) -> Self {
        Self {
            operation_id: r.operation_id.clone(),
            plan_id: r.plan_id.clone(),
            target_path: r.target_path.clone(),
            canonical_path: r.canonical_path.clone(),
            scope: r.scope.to_string(),
            method: r.method.to_string(),
            status: r.status.to_string(),
            bytes_processed: r.bytes_processed,
            verification: FileVerificationResultDto::from(&r.verification),
            folder_stats: r.folder_stats.as_ref().map(FolderEraseStatsDto::from),
            failure_reason: r.failure_reason.as_ref().map(|f| f.to_string()),
            started_at: r.started_at.clone(),
            completed_at: r.completed_at.clone(),
            limitations: r.limitations.clone(),
            scope_description: r.scope_description.clone(),
        }
    }
}

fn parse_method(method_str: Option<&str>) -> Option<SanitizationMethod> {
    match method_str {
        Some("nist_800_88_clear_zero") | Some("Nist80088ClearZero") => {
            Some(SanitizationMethod::Nist80088ClearZero)
        }
        Some("directory_recursive_shred") | Some("DirectoryRecursiveShred") => {
            Some(SanitizationMethod::DirectoryRecursiveShred)
        }
        Some(_) => Some(SanitizationMethod::LogicalFileShred),
        None => None,
    }
}

// Handlers for commands (allow testability without tauri State)

pub async fn plan_file_erasure_handler(
    state: &AppState,
    request: PlanFileEraseRequest,
) -> Result<FileErasePlanDto, SafeErrorResponse> {
    let actor_id = if let Some(tok) = &request.session_token {
        state.auth.get_current_user(tok).ok().map(|u| u.username)
    } else {
        None
    };

    let method = parse_method(request.method.as_deref());
    let plan = state
        .file_eraser
        .plan_file_erasure(&request.target_path, method, actor_id)
        .await
        .map_err(|e| SafeErrorResponse::from(&e))?;

    Ok(FileErasePlanDto::from(&plan))
}

pub async fn plan_folder_erasure_handler(
    state: &AppState,
    request: PlanFolderEraseRequest,
) -> Result<FileErasePlanDto, SafeErrorResponse> {
    let actor_id = if let Some(tok) = &request.session_token {
        state.auth.get_current_user(tok).ok().map(|u| u.username)
    } else {
        None
    };

    let method = parse_method(request.method.as_deref());
    let plan = state
        .file_eraser
        .plan_folder_erasure(&request.target_path, method, actor_id)
        .await
        .map_err(|e| SafeErrorResponse::from(&e))?;

    Ok(FileErasePlanDto::from(&plan))
}

pub async fn execute_file_erasure_handler(
    state: &AppState,
    request: ExecuteFileEraseRequest,
) -> Result<FileEraseResultDto, SafeErrorResponse> {
    let result = state
        .file_eraser
        .execute_file_erasure(
            &request.plan_id,
            &request.confirmation_id,
            &request.operation_id,
            &request.typed_confirmation,
            request.warning_acknowledged,
            &request.session_token,
            None,
            None,
        )
        .await
        .map_err(|e| SafeErrorResponse::from(&e))?;

    Ok(FileEraseResultDto::from(&result))
}

pub async fn execute_folder_erasure_handler(
    state: &AppState,
    request: ExecuteFolderEraseRequest,
) -> Result<FileEraseResultDto, SafeErrorResponse> {
    let result = state
        .file_eraser
        .execute_folder_erasure(
            &request.plan_id,
            &request.confirmation_id,
            &request.operation_id,
            &request.typed_confirmation,
            request.warning_acknowledged,
            &request.session_token,
            None,
            None,
        )
        .await
        .map_err(|e| SafeErrorResponse::from(&e))?;

    Ok(FileEraseResultDto::from(&result))
}

pub fn get_file_erasure_result_handler(
    state: &AppState,
    operation_id: &str,
) -> Result<Option<FileEraseResultDto>, SafeErrorResponse> {
    let result = state
        .file_eraser
        .get_erasure_result(operation_id)
        .map_err(|e| SafeErrorResponse::from(&e))?;

    Ok(result.as_ref().map(FileEraseResultDto::from))
}

// Tauri IPC Command declarations

#[tauri::command]
pub async fn plan_file_erasure(
    state: State<'_, AppState>,
    request: PlanFileEraseRequest,
) -> Result<FileErasePlanDto, SafeErrorResponse> {
    plan_file_erasure_handler(&state, request).await
}

#[tauri::command]
pub async fn plan_folder_erasure(
    state: State<'_, AppState>,
    request: PlanFolderEraseRequest,
) -> Result<FileErasePlanDto, SafeErrorResponse> {
    plan_folder_erasure_handler(&state, request).await
}

#[tauri::command]
pub async fn execute_file_erasure(
    state: State<'_, AppState>,
    request: ExecuteFileEraseRequest,
) -> Result<FileEraseResultDto, SafeErrorResponse> {
    execute_file_erasure_handler(&state, request).await
}

#[tauri::command]
pub async fn execute_folder_erasure(
    state: State<'_, AppState>,
    request: ExecuteFolderEraseRequest,
) -> Result<FileEraseResultDto, SafeErrorResponse> {
    execute_folder_erasure_handler(&state, request).await
}

#[tauri::command]
pub fn get_file_erasure_result(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Option<FileEraseResultDto>, SafeErrorResponse> {
    get_file_erasure_result_handler(&state, &operation_id)
}
