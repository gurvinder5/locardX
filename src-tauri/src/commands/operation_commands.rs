use crate::state::AppState;
use locardx_common::{SafeErrorResponse, TargetIdentity, TargetType};
use locardx_operation_manager::{OperationDto, OperationType};
use std::path::Path;

/// Handler for submitting and dispatching a managed SHA-256 evidence hashing operation.
pub async fn submit_integrity_hash_operation_handler(
    state: &AppState,
    path: &str,
    session_token: Option<&str>,
) -> Result<OperationDto, SafeErrorResponse> {
    let actor_id =
        session_token.and_then(|token| state.auth.get_current_user(token).ok().map(|u| u.username));

    let path_obj = Path::new(path);
    let display_name = path_obj
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "target_file".to_string());

    let size_bytes = std::fs::metadata(path_obj).ok().map(|m| m.len());

    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: path.to_string(),
        display_name,
        size_bytes,
    };

    state
        .operation_manager
        .submit_and_run(OperationType::IntegrityHash, target, actor_id, None)
        .await
        .map(OperationDto::from)
        .map_err(SafeErrorResponse::from)
}

/// Handler for submitting and dispatching a managed cryptographic integrity verification operation.
pub async fn submit_integrity_verify_operation_handler(
    state: &AppState,
    path: &str,
    expected_digest: &str,
    session_token: Option<&str>,
) -> Result<OperationDto, SafeErrorResponse> {
    let actor_id =
        session_token.and_then(|token| state.auth.get_current_user(token).ok().map(|u| u.username));

    let path_obj = Path::new(path);
    let display_name = path_obj
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "target_file".to_string());

    let size_bytes = std::fs::metadata(path_obj).ok().map(|m| m.len());

    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: path.to_string(),
        display_name,
        size_bytes,
    };

    let params = serde_json::json!({
        "expected_digest": expected_digest.trim().to_lowercase(),
    });

    state
        .operation_manager
        .submit_and_run(
            OperationType::IntegrityVerify,
            target,
            actor_id,
            Some(params),
        )
        .await
        .map(OperationDto::from)
        .map_err(SafeErrorResponse::from)
}

/// Handler for querying a single operation by ID.
pub fn get_operation_handler(
    state: &AppState,
    operation_id: &str,
) -> Result<OperationDto, SafeErrorResponse> {
    state
        .operation_manager
        .get_operation(operation_id)
        .map(OperationDto::from)
        .map_err(SafeErrorResponse::from)
}

/// Handler for listing managed operations with optional filtering.
pub fn list_operations_handler(
    state: &AppState,
    limit: u32,
    state_filter: Option<&str>,
    type_filter: Option<&str>,
) -> Result<Vec<OperationDto>, SafeErrorResponse> {
    state
        .operation_manager
        .list_operations(limit as usize, state_filter, type_filter)
        .map(|list| list.into_iter().map(OperationDto::from).collect())
        .map_err(SafeErrorResponse::from)
}

/// Handler for requesting cancellation of an active or queued operation.
pub async fn cancel_operation_handler(
    state: &AppState,
    operation_id: &str,
    session_token: Option<&str>,
) -> Result<(), SafeErrorResponse> {
    let actor_id =
        session_token.and_then(|token| state.auth.get_current_user(token).ok().map(|u| u.username));

    state
        .operation_manager
        .request_cancellation(operation_id, actor_id.as_deref())
        .await
        .map_err(SafeErrorResponse::from)
}

#[tauri::command]
pub async fn submit_integrity_hash_operation(
    state: tauri::State<'_, AppState>,
    path: String,
    session_token: Option<String>,
) -> Result<OperationDto, SafeErrorResponse> {
    submit_integrity_hash_operation_handler(&state, &path, session_token.as_deref()).await
}

#[tauri::command]
pub async fn submit_integrity_verify_operation(
    state: tauri::State<'_, AppState>,
    path: String,
    expected_digest: String,
    session_token: Option<String>,
) -> Result<OperationDto, SafeErrorResponse> {
    submit_integrity_verify_operation_handler(
        &state,
        &path,
        &expected_digest,
        session_token.as_deref(),
    )
    .await
}

#[tauri::command]
pub fn get_operation(
    state: tauri::State<'_, AppState>,
    operation_id: String,
) -> Result<OperationDto, SafeErrorResponse> {
    get_operation_handler(&state, &operation_id)
}

#[tauri::command]
pub fn list_operations(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
    state_filter: Option<String>,
    type_filter: Option<String>,
) -> Result<Vec<OperationDto>, SafeErrorResponse> {
    let limit = limit.unwrap_or(50);
    list_operations_handler(
        &state,
        limit,
        state_filter.as_deref(),
        type_filter.as_deref(),
    )
}

#[tauri::command]
pub async fn cancel_operation(
    state: tauri::State<'_, AppState>,
    operation_id: String,
    session_token: Option<String>,
) -> Result<(), SafeErrorResponse> {
    cancel_operation_handler(&state, &operation_id, session_token.as_deref()).await
}
