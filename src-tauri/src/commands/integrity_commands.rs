use crate::state::AppState;
use locardx_common::SafeErrorResponse;
use locardx_verification::{HashResult, IntegrityRecord, VerificationResult};

/// Handler for calculating cryptographic hash of a target file.
pub fn calculate_file_hash_handler(
    state: &AppState,
    path: &str,
    session_token: Option<&str>,
) -> Result<HashResult, SafeErrorResponse> {
    let actor_id =
        session_token.and_then(|token| state.auth.get_current_user(token).ok().map(|u| u.username));

    state
        .integrity
        .calculate_file_hash(path, actor_id.as_deref())
        .map_err(SafeErrorResponse::from)
}

/// Handler for verifying cryptographic hash of a target file against an expected digest.
pub fn verify_file_hash_handler(
    state: &AppState,
    path: &str,
    expected_digest: &str,
    session_token: Option<&str>,
) -> Result<VerificationResult, SafeErrorResponse> {
    let actor_id =
        session_token.and_then(|token| state.auth.get_current_user(token).ok().map(|u| u.username));

    state
        .integrity
        .verify_file_hash(path, expected_digest, actor_id.as_deref())
        .map_err(SafeErrorResponse::from)
}

/// Handler for listing recorded integrity operations.
pub fn list_integrity_records_handler(
    state: &AppState,
    limit: u32,
) -> Result<Vec<IntegrityRecord>, SafeErrorResponse> {
    state
        .integrity
        .list_integrity_records(limit as usize)
        .map_err(SafeErrorResponse::from)
}

#[tauri::command]
pub fn calculate_file_hash(
    state: tauri::State<'_, AppState>,
    path: String,
    session_token: Option<String>,
) -> Result<HashResult, SafeErrorResponse> {
    calculate_file_hash_handler(&state, &path, session_token.as_deref())
}

#[tauri::command]
pub fn verify_file_hash(
    state: tauri::State<'_, AppState>,
    path: String,
    expected_digest: String,
    session_token: Option<String>,
) -> Result<VerificationResult, SafeErrorResponse> {
    verify_file_hash_handler(&state, &path, &expected_digest, session_token.as_deref())
}

#[tauri::command]
pub fn list_integrity_records(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<IntegrityRecord>, SafeErrorResponse> {
    let limit = limit.unwrap_or(50);
    list_integrity_records_handler(&state, limit)
}
