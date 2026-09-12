use crate::state::AppState;
use locardx_common::{SafeErrorResponse, TargetIdentity, TargetType};
use locardx_verification::{
    SanitizationMethod, SanitizationPlan, SanitizationScope, SanitizationStandardDefinition,
    SanitizationStandardsRegistry, SnapshotComparisonResult, VerificationStrategy,
};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Request payload for evaluating a sanitization plan.
#[derive(Debug, Clone, Deserialize)]
pub struct EvaluatePlanRequest {
    pub target_identifier: String,
    pub target_type: String,
    pub scope: String,
    pub requested_method: Option<String>,
    pub requested_strategy: Option<String>,
    pub session_token: Option<String>,
}

/// DTO representing an evaluated sanitization plan safe for Tauri IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationPlanDto {
    pub plan_id: String,
    pub target_identifier: String,
    pub target_type: String,
    pub media_type: String,
    pub scope: String,
    pub recommended_method: String,
    pub is_applicable: bool,
    pub risk_level: String,
    pub verification_strategy: String,
    pub applicable_standard: Option<String>,
    pub standard_method_id: Option<String>,
    pub limitations: Vec<String>,
    pub reason_codes: Vec<String>,
    pub created_at: String,
    pub is_dry_run: bool,
}

impl From<&SanitizationPlan> for SanitizationPlanDto {
    fn from(plan: &SanitizationPlan) -> Self {
        Self {
            plan_id: plan.plan_id.clone(),
            target_identifier: plan.target.identifier.clone(),
            target_type: plan.target.target_type.to_string(),
            media_type: plan.media_type.as_str().to_string(),
            scope: plan.scope.to_string(),
            recommended_method: plan.recommended_method.to_string(),
            is_applicable: plan.is_applicable,
            risk_level: plan.risk_level.to_string(),
            verification_strategy: plan.verification_strategy.to_string(),
            applicable_standard: plan.applicable_standard.clone(),
            standard_method_id: plan.standard_method_id.clone(),
            limitations: plan.limitations.clone(),
            reason_codes: plan.reason_codes.iter().map(|r| r.to_string()).collect(),
            created_at: plan.created_at.clone(),
            is_dry_run: true,
        }
    }
}

/// DTO for formal sanitization standard definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationStandardDto {
    pub standard_id: String,
    pub standard_name: String,
    pub method_id: String,
    pub method_name: String,
    pub applicable_scopes: Vec<String>,
    pub applicable_media: Vec<String>,
    pub recommended_verification: String,
    pub verification_requirements: String,
    pub limitations: Vec<String>,
    pub description: String,
}

impl From<&SanitizationStandardDefinition> for SanitizationStandardDto {
    fn from(std: &SanitizationStandardDefinition) -> Self {
        Self {
            standard_id: std.standard_id.clone(),
            standard_name: std.standard_name.clone(),
            method_id: std.method_id.clone(),
            method_name: std.method_name.clone(),
            applicable_scopes: std
                .applicable_scopes
                .iter()
                .map(|s| s.to_string())
                .collect(),
            applicable_media: std
                .applicable_media
                .iter()
                .map(|m| m.as_str().to_string())
                .collect(),
            recommended_verification: std.recommended_verification.to_string(),
            verification_requirements: std.verification_requirements.clone(),
            limitations: std.limitations.clone(),
            description: std.description.clone(),
        }
    }
}

/// DTO for TOCTOU snapshot comparison results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotComparisonResultDto {
    pub matches: bool,
    pub reason: Option<String>,
    pub differences: Vec<String>,
    pub plan_target_identifier: String,
    pub current_target_identifier: Option<String>,
}

impl From<&SnapshotComparisonResult> for SnapshotComparisonResultDto {
    fn from(res: &SnapshotComparisonResult) -> Self {
        Self {
            matches: res.matches,
            reason: res.reason.clone(),
            differences: res.differences.clone(),
            plan_target_identifier: res.plan_snapshot.target_identifier.clone(),
            current_target_identifier: res
                .current_snapshot
                .as_ref()
                .map(|s| s.target_identifier.clone()),
        }
    }
}

/// DTO for sanitization verification plan details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPlanDto {
    pub plan_id: String,
    pub target_identifier: String,
    pub verification_strategy: String,
    pub verification_requirements: String,
    pub limitations: Vec<String>,
    pub applicable_standard: Option<String>,
}

/// Handler for evaluate_sanitization_plan.
pub fn evaluate_sanitization_plan_handler(
    state: &AppState,
    request: EvaluatePlanRequest,
) -> Result<SanitizationPlanDto, SafeErrorResponse> {
    let actor_id = if let Some(token) = &request.session_token {
        state.auth.get_current_user(token).ok().map(|u| u.username)
    } else {
        None
    };

    let target_type = match request.target_type.as_str() {
        "File" => TargetType::File,
        "Directory" => TargetType::Directory,
        "LogicalVolume" => TargetType::LogicalVolume,
        _ => TargetType::PhysicalDevice,
    };

    let target = TargetIdentity {
        target_type,
        identifier: request.target_identifier.clone(),
        display_name: request.target_identifier.clone(),
        size_bytes: None,
    };

    let scope = match request.scope.as_str() {
        "File" => SanitizationScope::File,
        "Folder" => SanitizationScope::Folder,
        "LogicalVolume" => SanitizationScope::LogicalVolume,
        _ => SanitizationScope::PhysicalDevice,
    };

    let requested_method = request.requested_method.as_deref().and_then(|m| match m {
        "Nist80088ClearZero" => Some(SanitizationMethod::Nist80088ClearZero),
        "Nist80088PurgeCrypto" => Some(SanitizationMethod::Nist80088PurgeCrypto),
        "Dod522022M" => Some(SanitizationMethod::Dod522022M),
        "NvmeCryptoErase" => Some(SanitizationMethod::NvmeCryptoErase),
        "AtaSecureErase" => Some(SanitizationMethod::AtaSecureErase),
        "BlockZeroOverwrite" => Some(SanitizationMethod::BlockZeroOverwrite),
        "LogicalFileShred" => Some(SanitizationMethod::LogicalFileShred),
        _ => None,
    });

    let requested_strategy = request.requested_strategy.as_deref().and_then(|s| match s {
        "FullReadBack" => Some(VerificationStrategy::FullReadBack),
        "SampledRandomSectors" => Some(VerificationStrategy::SampledRandomSectors),
        "CryptoKeyDestructionCheck" => Some(VerificationStrategy::CryptoKeyDestructionCheck),
        "MetadataUnlinkCheck" => Some(VerificationStrategy::MetadataUnlinkCheck),
        "NoVerification" => Some(VerificationStrategy::NoVerification),
        _ => None,
    });

    let plan = state
        .sanitization
        .evaluate_plan(
            &target,
            scope,
            requested_method,
            requested_strategy,
            actor_id,
        )
        .map_err(|e| SafeErrorResponse::from(&e))?;

    Ok(SanitizationPlanDto::from(&plan))
}

/// Evaluates a sanitization plan without modifying storage.
#[tauri::command]
pub async fn evaluate_sanitization_plan(
    state: State<'_, AppState>,
    request: EvaluatePlanRequest,
) -> Result<SanitizationPlanDto, SafeErrorResponse> {
    evaluate_sanitization_plan_handler(&state, request)
}

/// Handler for get_sanitization_methods.
pub fn get_sanitization_methods_handler() -> Result<Vec<SanitizationStandardDto>, SafeErrorResponse>
{
    let standards = SanitizationStandardsRegistry::get_all_standards();
    let dtos = standards
        .iter()
        .map(SanitizationStandardDto::from)
        .collect();
    Ok(dtos)
}

/// Returns all available sanitization methods and their formal standards metadata.
#[tauri::command]
pub async fn get_sanitization_methods() -> Result<Vec<SanitizationStandardDto>, SafeErrorResponse> {
    get_sanitization_methods_handler()
}

/// Handler for compare_target_snapshot.
pub fn compare_target_snapshot_handler(
    state: &AppState,
    plan_id: &str,
) -> Result<SnapshotComparisonResultDto, SafeErrorResponse> {
    let result = state
        .sanitization
        .compare_plan_snapshot_live(plan_id)
        .map_err(|e| SafeErrorResponse::from(&e))?;

    Ok(SnapshotComparisonResultDto::from(&result))
}

/// Compares a pre-evaluated plan snapshot against live hardware to detect TOCTOU changes.
#[tauri::command]
pub async fn compare_target_snapshot(
    state: State<'_, AppState>,
    plan_id: String,
) -> Result<SnapshotComparisonResultDto, SafeErrorResponse> {
    compare_target_snapshot_handler(&state, &plan_id)
}

/// Handler for get_sanitization_verification_plan.
pub fn get_sanitization_verification_plan_handler(
    state: &AppState,
    plan_id: &str,
) -> Result<VerificationPlanDto, SafeErrorResponse> {
    let plan = state
        .sanitization
        .get_plan(plan_id)
        .map_err(|e| SafeErrorResponse::from(&e))?;

    let standard = SanitizationStandardsRegistry::find_by_method(plan.recommended_method);
    let reqs = standard
        .as_ref()
        .map(|s| s.verification_requirements.clone())
        .unwrap_or_else(|| "No standard verification requirements specified.".to_string());

    Ok(VerificationPlanDto {
        plan_id: plan.plan_id,
        target_identifier: plan.target.identifier,
        verification_strategy: plan.verification_strategy.to_string(),
        verification_requirements: reqs,
        limitations: plan.limitations,
        applicable_standard: plan.applicable_standard,
    })
}

/// Returns detailed verification plan and limitations for a previously generated plan.
#[tauri::command]
pub async fn get_sanitization_verification_plan(
    state: State<'_, AppState>,
    plan_id: String,
) -> Result<VerificationPlanDto, SafeErrorResponse> {
    get_sanitization_verification_plan_handler(&state, &plan_id)
}
