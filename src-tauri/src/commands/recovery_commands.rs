use crate::state::AppState;
use locardx_acquisition::AcquisitionArtifact;
use locardx_common::SafeErrorResponse;
use locardx_recovery_engine::{
    RecoveredFile, RecoveryOptions, RecoveryPlan, RecoveryProgress, RecoveryReport, RecoveryResult,
    RecoverySourceSnapshot,
};
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRecoveryPlanRequest {
    pub artifact: AcquisitionArtifact,
    pub options: RecoveryOptions,
    pub session_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartRecoveryRequest {
    pub plan: RecoveryPlan,
    pub session_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportRecoveredFilesRequest {
    pub job_id: String,
    pub export_dir: String,
}

#[tauri::command]
pub async fn list_recovery_sources(
    state: State<'_, AppState>,
) -> Result<Vec<RecoverySourceSnapshot>, SafeErrorResponse> {
    state
        .recovery
        .list_sources()
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn validate_recovery_source(
    state: State<'_, AppState>,
    artifact: AcquisitionArtifact,
) -> Result<RecoverySourceSnapshot, SafeErrorResponse> {
    state
        .recovery
        .validate_source(&artifact, None)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn create_recovery_plan(
    state: State<'_, AppState>,
    request: CreateRecoveryPlanRequest,
) -> Result<RecoveryPlan, SafeErrorResponse> {
    let actor_id = request
        .session_token
        .as_deref()
        .and_then(|t| state.auth.get_current_user(t).ok())
        .map(|u| u.username);

    state
        .recovery
        .create_plan(&request.artifact, request.options, actor_id.as_deref())
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn start_recovery(
    state: State<'_, AppState>,
    request: StartRecoveryRequest,
) -> Result<RecoveryResult, SafeErrorResponse> {
    let actor_id = request
        .session_token
        .as_deref()
        .and_then(|t| state.auth.get_current_user(t).ok())
        .map(|u| u.username);

    let recovery_svc = state.recovery.clone();
    tokio::task::spawn_blocking(move || {
        recovery_svc.execute_recovery(&request.plan, actor_id.as_deref())
    })
    .await
    .map_err(|e| SafeErrorResponse {
        code: "THREAD_JOIN_ERROR".to_string(),
        message: format!("Recovery worker thread failure: {}", e),
    })?
    .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn cancel_recovery(
    state: State<'_, AppState>,
    operation_id: String,
    session_token: Option<String>,
) -> Result<bool, SafeErrorResponse> {
    let actor_id = session_token
        .as_deref()
        .and_then(|t| state.auth.get_current_user(t).ok())
        .map(|u| u.username);

    state
        .recovery
        .cancel_recovery(&operation_id, actor_id.as_deref())
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn get_recovery_progress(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Option<RecoveryProgress>, SafeErrorResponse> {
    Ok(state.recovery.get_progress(&operation_id))
}

#[tauri::command]
pub async fn get_recovery_job(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<RecoveryResult>, SafeErrorResponse> {
    state
        .recovery
        .get_job(&job_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn list_recovery_jobs(
    state: State<'_, AppState>,
) -> Result<Vec<RecoveryResult>, SafeErrorResponse> {
    state
        .recovery
        .list_jobs()
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn get_recovered_files(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Vec<RecoveredFile>, SafeErrorResponse> {
    state
        .recovery
        .get_recovered_files(&job_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn export_recovered_files(
    state: State<'_, AppState>,
    request: ExportRecoveredFilesRequest,
) -> Result<usize, SafeErrorResponse> {
    state
        .recovery
        .export_recovered_files(&request.job_id, &request.export_dir)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn get_recovery_report(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<RecoveryReport>, SafeErrorResponse> {
    state
        .recovery
        .get_report(&job_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}
