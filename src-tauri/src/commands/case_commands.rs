use crate::state::AppState;
use locardx_audit::{AuditChainVerification, AuditEvent};
use locardx_case_management::models::*;
use locardx_common::SafeErrorResponse;
use locardx_reporting::forensic_report::{CaseForensicReport, CaseReportSummary};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCasesResponse {
    pub cases: Vec<Case>,
    pub total: usize,
}

fn authenticate_with_permission(
    state: &AppState,
    token: Option<&str>,
    permission: locardx_auth::Permission,
) -> Result<locardx_auth::PublicUser, SafeErrorResponse> {
    let token = token.ok_or_else(|| SafeErrorResponse {
        code: "UNAUTHORIZED".to_string(),
        message: "Missing authentication session token".to_string(),
    })?;
    state
        .auth
        .authorize_permission(token, permission)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn create_case(
    state: State<'_, AppState>,
    session_token: Option<String>,
    request: CreateCaseRequest,
) -> Result<Case, SafeErrorResponse> {
    let user = authenticate_with_permission(
        &state,
        session_token.as_deref(),
        locardx_auth::Permission::CaseCreate,
    )?;
    state
        .case_service
        .create_case(request, &user)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn get_case(
    state: State<'_, AppState>,
    case_id: String,
) -> Result<Option<Case>, SafeErrorResponse> {
    state
        .case_service
        .get_case(&case_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn list_cases(
    state: State<'_, AppState>,
    status_filter: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<ListCasesResponse, SafeErrorResponse> {
    let filter = status_filter.and_then(|s| CaseStatus::from_str(&s).ok());
    let (cases, total) = state
        .case_service
        .list_cases(filter, limit.unwrap_or(50), offset.unwrap_or(0))
        .map_err(|e| SafeErrorResponse::from(&e))?;
    Ok(ListCasesResponse { cases, total })
}

#[tauri::command]
pub async fn update_case(
    state: State<'_, AppState>,
    session_token: Option<String>,
    case_id: String,
    request: UpdateCaseRequest,
) -> Result<Case, SafeErrorResponse> {
    let user = authenticate_with_permission(
        &state,
        session_token.as_deref(),
        locardx_auth::Permission::CaseModify,
    )?;
    state
        .case_service
        .update_case(&case_id, request, &user)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn update_case_status(
    state: State<'_, AppState>,
    session_token: Option<String>,
    case_id: String,
    status: String,
) -> Result<Case, SafeErrorResponse> {
    let user = authenticate_with_permission(
        &state,
        session_token.as_deref(),
        locardx_auth::Permission::CaseModify,
    )?;
    let new_status = CaseStatus::from_str(&status).map_err(|e| SafeErrorResponse {
        code: "INVALID_STATUS".to_string(),
        message: e,
    })?;
    state
        .case_service
        .update_case_status(&case_id, new_status, &user)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn associate_operation_to_case(
    state: State<'_, AppState>,
    session_token: Option<String>,
    case_id: String,
    operation_id: String,
    operation_type: String,
    notes: Option<String>,
) -> Result<CaseOperation, SafeErrorResponse> {
    let user = authenticate_with_permission(
        &state,
        session_token.as_deref(),
        locardx_auth::Permission::CaseModify,
    )?;
    state
        .case_service
        .associate_operation(
            &case_id,
            &operation_id,
            &operation_type,
            &user,
            notes.as_deref(),
        )
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn list_case_operations(
    state: State<'_, AppState>,
    case_id: String,
) -> Result<Vec<CaseOperation>, SafeErrorResponse> {
    state
        .case_service
        .list_case_operations(&case_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn add_case_evidence(
    state: State<'_, AppState>,
    session_token: Option<String>,
    case_id: String,
    request: AddEvidenceRequest,
) -> Result<CaseEvidence, SafeErrorResponse> {
    let user = authenticate_with_permission(
        &state,
        session_token.as_deref(),
        locardx_auth::Permission::CaseEvidenceAdd,
    )?;
    state
        .case_service
        .add_case_evidence(&case_id, request, &user)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn list_case_evidence(
    state: State<'_, AppState>,
    case_id: String,
) -> Result<Vec<CaseEvidence>, SafeErrorResponse> {
    state
        .case_service
        .list_case_evidence(&case_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn record_case_custody_event(
    state: State<'_, AppState>,
    session_token: Option<String>,
    case_id: String,
    request: RecordCustodyRequest,
) -> Result<CustodyEvent, SafeErrorResponse> {
    let user = authenticate_with_permission(
        &state,
        session_token.as_deref(),
        locardx_auth::Permission::CaseCustodyRecord,
    )?;
    state
        .case_service
        .record_custody_event(&case_id, request, &user)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn get_case_timeline(
    state: State<'_, AppState>,
    case_id: String,
) -> Result<Vec<CaseTimelineItem>, SafeErrorResponse> {
    state
        .case_service
        .get_case_timeline(&case_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn get_case_summary(
    state: State<'_, AppState>,
    case_id: String,
) -> Result<CaseSummary, SafeErrorResponse> {
    state
        .case_service
        .get_case_summary(&case_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn generate_case_report(
    state: State<'_, AppState>,
    session_token: Option<String>,
    case_id: String,
) -> Result<CaseForensicReport, SafeErrorResponse> {
    let user = authenticate_with_permission(
        &state,
        session_token.as_deref(),
        locardx_auth::Permission::CaseReportGenerate,
    )?;
    state
        .case_service
        .generate_case_report(&case_id, &user)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn get_case_report(
    state: State<'_, AppState>,
    report_id: String,
) -> Result<Option<CaseForensicReport>, SafeErrorResponse> {
    state
        .case_service
        .get_case_report(&report_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn list_case_reports(
    state: State<'_, AppState>,
    case_id: String,
) -> Result<Vec<CaseReportSummary>, SafeErrorResponse> {
    state
        .case_service
        .list_case_reports(&case_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn verify_case_report_integrity(
    state: State<'_, AppState>,
    report: CaseForensicReport,
) -> Result<bool, SafeErrorResponse> {
    Ok(state.case_service.verify_case_report(&report))
}

#[tauri::command]
pub async fn list_audit_events(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<AuditEvent>, SafeErrorResponse> {
    state
        .audit
        .list_events(limit.unwrap_or(100))
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub async fn verify_audit_chain(
    state: State<'_, AppState>,
) -> Result<AuditChainVerification, SafeErrorResponse> {
    state
        .audit
        .verify_chain()
        .map_err(|e| SafeErrorResponse::from(&e))
}
