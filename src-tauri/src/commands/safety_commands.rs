use crate::state::AppState;
use locardx_common::{OperationType, SafeErrorResponse, TargetIdentity, TargetType};
use locardx_security::{ConfirmationChallenge, SafetyDecision};
use serde::Deserialize;
use std::str::FromStr;
use tauri::State;

#[derive(Debug, Clone, Deserialize)]
pub struct EvaluateSafetyRequest {
    pub target_type: String,
    pub target_identifier: String,
    pub target_display_name: Option<String>,
    pub target_size_bytes: Option<u64>,
    pub operation_type: String,
    pub session_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestConfirmationRequest {
    pub operation_id: String,
    pub target_type: String,
    pub target_identifier: String,
    pub target_display_name: Option<String>,
    pub target_size_bytes: Option<u64>,
    pub operation_type: String,
    pub session_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfirmOperationRequest {
    pub confirmation_id: String,
    pub operation_id: String,
    pub warning_acknowledged: bool,
    pub typed_target_confirmation: String,
    pub session_token: String,
}

fn parse_target_type(s: &str) -> TargetType {
    match s {
        "File" => TargetType::File,
        "Directory" => TargetType::Directory,
        "LogicalVolume" => TargetType::LogicalVolume,
        "PhysicalDevice" => TargetType::PhysicalDevice,
        "EvidenceObject" => TargetType::EvidenceObject,
        _ => TargetType::Unknown,
    }
}

// Handlers separated from Tauri command macros for testability

pub fn evaluate_operation_safety_handler(
    state: &AppState,
    req: EvaluateSafetyRequest,
) -> Result<SafetyDecision, SafeErrorResponse> {
    let op_type =
        OperationType::from_str(&req.operation_type).map_err(|e| SafeErrorResponse::from(&e))?;

    let target = TargetIdentity {
        target_type: parse_target_type(&req.target_type),
        identifier: req.target_identifier.clone(),
        display_name: req.target_display_name.unwrap_or(req.target_identifier),
        size_bytes: req.target_size_bytes,
    };

    state
        .safety
        .evaluate_safety(&target, op_type, req.session_token.as_deref())
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub fn request_destructive_confirmation_handler(
    state: &AppState,
    req: RequestConfirmationRequest,
) -> Result<ConfirmationChallenge, SafeErrorResponse> {
    let op_type =
        OperationType::from_str(&req.operation_type).map_err(|e| SafeErrorResponse::from(&e))?;

    let target = TargetIdentity {
        target_type: parse_target_type(&req.target_type),
        identifier: req.target_identifier.clone(),
        display_name: req.target_display_name.unwrap_or(req.target_identifier),
        size_bytes: req.target_size_bytes,
    };

    state
        .safety
        .request_destructive_confirmation(&req.operation_id, &target, op_type, &req.session_token)
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub fn confirm_destructive_operation_handler(
    state: &AppState,
    req: ConfirmOperationRequest,
) -> Result<SafetyDecision, SafeErrorResponse> {
    state
        .safety
        .confirm_destructive_operation(
            &req.confirmation_id,
            &req.operation_id,
            req.warning_acknowledged,
            &req.typed_target_confirmation,
            &req.session_token,
        )
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub fn list_safety_evaluations_handler(
    state: &AppState,
    limit: Option<u32>,
) -> Result<Vec<SafetyDecision>, SafeErrorResponse> {
    state
        .safety
        .list_safety_evaluations(limit)
        .map_err(|e| SafeErrorResponse::from(&e))
}

// Tauri IPC Command Wrappers

#[tauri::command]
pub fn evaluate_operation_safety(
    state: State<'_, AppState>,
    request: EvaluateSafetyRequest,
) -> Result<SafetyDecision, SafeErrorResponse> {
    evaluate_operation_safety_handler(&state, request)
}

#[tauri::command]
pub fn request_destructive_confirmation(
    state: State<'_, AppState>,
    request: RequestConfirmationRequest,
) -> Result<ConfirmationChallenge, SafeErrorResponse> {
    request_destructive_confirmation_handler(&state, request)
}

#[tauri::command]
pub fn confirm_destructive_operation(
    state: State<'_, AppState>,
    request: ConfirmOperationRequest,
) -> Result<SafetyDecision, SafeErrorResponse> {
    confirm_destructive_operation_handler(&state, request)
}

#[tauri::command]
pub fn list_safety_evaluations(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<SafetyDecision>, SafeErrorResponse> {
    list_safety_evaluations_handler(&state, limit)
}
