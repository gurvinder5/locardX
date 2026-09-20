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
    pub case_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartRecoveryRequest {
    pub plan: RecoveryPlan,
    pub session_token: Option<String>,
    pub operation_id: Option<String>,
    pub case_id: Option<String>,
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
    let recovery_svc = state.recovery.clone();
    tokio::task::spawn_blocking(move || {
        recovery_svc
            .validate_source(&artifact, None)
            .map_err(|e| SafeErrorResponse::from(&e))
    })
    .await
    .map_err(|e| SafeErrorResponse {
        code: "THREAD_JOIN_ERROR".to_string(),
        message: format!("Worker thread failure during source validation: {}", e),
    })?
}

#[tauri::command]
pub async fn create_recovery_plan(
    state: State<'_, AppState>,
    request: CreateRecoveryPlanRequest,
) -> Result<RecoveryPlan, SafeErrorResponse> {
    // Case Policy: Fail-closed if no active case or if specified/active case is closed
    state
        .case_service
        .validate_case_for_operation(request.case_id.as_deref())
        .map_err(|e| SafeErrorResponse::from(&e))?;

    let actor_id = request
        .session_token
        .as_deref()
        .and_then(|t| state.auth.get_current_user(t).ok())
        .map(|u| u.username);

    let recovery_svc = state.recovery.clone();
    tokio::task::spawn_blocking(move || {
        recovery_svc
            .create_plan(&request.artifact, request.options, actor_id.as_deref())
            .map_err(|e| SafeErrorResponse::from(&e))
    })
    .await
    .map_err(|e| SafeErrorResponse {
        code: "THREAD_JOIN_ERROR".to_string(),
        message: format!("Worker thread failure during plan creation: {}", e),
    })?
}

#[tauri::command]
pub async fn start_recovery(
    state: State<'_, AppState>,
    request: StartRecoveryRequest,
) -> Result<RecoveryResult, SafeErrorResponse> {
    // Case Policy: Fail-closed if no active case or if specified/active case is closed
    let case = state
        .case_service
        .validate_case_for_operation(request.case_id.as_deref())
        .map_err(|e| SafeErrorResponse::from(&e))?;
    let case_id = case.case_id.clone();

    let actor_id = request
        .session_token
        .as_deref()
        .and_then(|t| state.auth.get_current_user(t).ok())
        .map(|u| u.username);

    let recovery_svc = state.recovery.clone();
    let op_id = request.operation_id.clone();
    let actor_id_cloned = actor_id.clone();
    let case_id_cloned = case_id.clone();

    let result = tokio::task::spawn_blocking(move || {
        recovery_svc.execute_recovery_with_case(
            &request.plan,
            actor_id_cloned.as_deref(),
            op_id.as_deref(),
            Some(&case_id_cloned),
        )
    })
    .await
    .map_err(|e| SafeErrorResponse {
        code: "THREAD_JOIN_ERROR".to_string(),
        message: format!("Recovery worker thread failure: {}", e),
    })?
    .map_err(|e| SafeErrorResponse::from(&e))?;

    // Associate recovery job with the active case
    let user = request
        .session_token
        .as_deref()
        .and_then(|t| state.auth.get_current_user(t).ok())
        .unwrap_or_else(|| locardx_auth::PublicUser {
            user_id: "sys-operator".to_string(),
            username: actor_id.unwrap_or_else(|| "operator".to_string()),
            role: locardx_auth::UserRole::Operator,
            display_name: Some("Forensic Operator".to_string()),
            enabled: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            last_login_at: None,
            metadata_json: "{}".to_string(),
        });

    let _ = state.case_service.associate_operation(
        &case_id,
        &result.job_id,
        "Recovery",
        &user,
        Some(&format!(
            "Forensic file recovery from {} -> {} recovered files",
            result.source_image_path, result.files_recovered
        )),
    );

    Ok(result)
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

#[derive(Debug, Clone, Deserialize)]
pub struct OpenRecoveredFileRequest {
    pub job_id: String,
    pub file_id: String,
}

#[tauri::command]
pub async fn open_recovered_file(
    state: State<'_, AppState>,
    request: OpenRecoveredFileRequest,
) -> Result<String, SafeErrorResponse> {
    let recovery_svc = state.recovery.clone();
    tokio::task::spawn_blocking(move || {
        recovery_svc
            .open_recovered_file(&request.job_id, &request.file_id)
            .map_err(|e| SafeErrorResponse::from(&e))
    })
    .await
    .map_err(|e| SafeErrorResponse {
        code: "THREAD_JOIN_ERROR".to_string(),
        message: format!("Worker thread failure during file opening: {}", e),
    })?
}

#[tauri::command]
pub async fn reveal_recovered_file(
    state: State<'_, AppState>,
    request: OpenRecoveredFileRequest,
) -> Result<String, SafeErrorResponse> {
    let recovery_svc = state.recovery.clone();
    tokio::task::spawn_blocking(move || {
        recovery_svc
            .reveal_recovered_file(&request.job_id, &request.file_id)
            .map_err(|e| SafeErrorResponse::from(&e))
    })
    .await
    .map_err(|e| SafeErrorResponse {
        code: "THREAD_JOIN_ERROR".to_string(),
        message: format!("Worker thread failure during file reveal: {}", e),
    })?
}
