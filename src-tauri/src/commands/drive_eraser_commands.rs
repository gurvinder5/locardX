use crate::state::AppState;
use locardx_common::SafeErrorResponse;
use locardx_device_manager::PhysicalDevice;
use locardx_drive_eraser::{
    detect_device_capabilities, inspect_physical_device, is_elevated_admin, DriveCapabilities,
    DriveCapabilitiesAssessment, DriveErasePlan, DriveEraseRequest, DriveEraseResult,
    ExecutionMode,
};
use locardx_reporting::DriveSanitizationReport;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Privilege status for physical drive access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrivePrivilegeStatus {
    pub is_elevated: bool,
    pub platform: String,
    pub message: String,
}

/// Request payload to plan whole-disk sanitization.
#[derive(Debug, Clone, Deserialize)]
pub struct PlanDriveEraseRequest {
    pub target_device_id: String,
    pub requested_method: Option<String>,
    pub execution_mode: Option<ExecutionMode>,
    pub session_token: Option<String>,
}

/// Request payload to execute simulated drive sanitization.
#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteDriveEraseSimulationRequest {
    pub plan_id: String,
    pub confirmation_id: String,
    pub operation_id: String,
    pub typed_confirmation: String,
    pub warning_acknowledged: bool,
    pub session_token: String,
}

// ==========================================
// Handlers (testable directly without Tauri)
// ==========================================

pub async fn plan_drive_erasure_handler(
    state: &AppState,
    request: PlanDriveEraseRequest,
) -> Result<DriveErasePlan, SafeErrorResponse> {
    let actor_id = if let Some(tok) = &request.session_token {
        state.auth.get_current_user(tok).ok().map(|u| u.username)
    } else {
        None
    };

    let domain_request = DriveEraseRequest {
        target_device_id: request.target_device_id,
        requested_method: request.requested_method,
        execution_mode: request.execution_mode.or(Some(ExecutionMode::Simulation)),
        session_token: request.session_token,
    };

    state
        .drive_eraser
        .plan_drive_erasure(domain_request, actor_id)
        .await
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub async fn execute_drive_erasure_simulation_handler(
    state: &AppState,
    request: ExecuteDriveEraseSimulationRequest,
) -> Result<DriveEraseResult, SafeErrorResponse> {
    state
        .drive_eraser
        .execute_drive_erasure_simulation(
            &request.plan_id,
            &request.confirmation_id,
            &request.operation_id,
            &request.typed_confirmation,
            request.warning_acknowledged,
            &request.session_token,
            ExecutionMode::Simulation,
            None,
            None,
        )
        .await
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub fn get_drive_capabilities_handler(
    state: &AppState,
    target_device_id: &str,
) -> Result<DriveCapabilities, SafeErrorResponse> {
    let provider = state.drive_eraser.device_provider();
    let (dev, _) = inspect_physical_device(target_device_id, provider, 512).map_err(|e| {
        SafeErrorResponse::from(&locardx_common::LocardError::Operation(e.to_string()))
    })?;
    Ok(detect_device_capabilities(&dev))
}

pub fn get_mock_test_devices_handler(
    state: &AppState,
) -> Result<Vec<PhysicalDevice>, SafeErrorResponse> {
    use locardx_device_manager::DeviceDiscoveryProvider;
    state
        .mock_device_registry
        .discover_devices()
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub fn get_real_storage_devices_handler(
    state: &AppState,
) -> Result<Vec<PhysicalDevice>, SafeErrorResponse> {
    state
        .drive_eraser
        .discover_real_devices()
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub fn refresh_real_devices_handler(
    state: &AppState,
) -> Result<Vec<PhysicalDevice>, SafeErrorResponse> {
    state
        .drive_eraser
        .refresh_real_devices()
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub async fn assess_drive_hardware_capabilities_handler(
    state: &AppState,
    target_device_id: &str,
) -> Result<DriveCapabilitiesAssessment, SafeErrorResponse> {
    state
        .drive_eraser
        .assess_drive_capabilities(target_device_id)
        .await
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub async fn execute_drive_erasure_hardware_handler(
    state: &AppState,
    request: ExecuteDriveEraseSimulationRequest,
) -> Result<DriveEraseResult, SafeErrorResponse> {
    // Passes through RealHardwareExecutionGate and RealHardwareSanitizer
    // which in Step 10B.1 strictly rejects execution with RealHardwareExecutionNotEnabled
    state
        .drive_eraser
        .execute_drive_erasure_with_gate(
            &request.plan_id,
            &request.confirmation_id,
            &request.operation_id,
            &request.typed_confirmation,
            request.warning_acknowledged,
            &request.session_token,
            ExecutionMode::RealHardware,
            None,
            None,
        )
        .await
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub fn get_drive_erasure_result_handler(
    state: &AppState,
    operation_id: &str,
) -> Result<Option<DriveEraseResult>, SafeErrorResponse> {
    state
        .drive_eraser
        .get_drive_erasure_result(operation_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

// ==========================================
// Tauri Commands
// ==========================================

#[tauri::command]
pub async fn plan_drive_erasure(
    state: State<'_, AppState>,
    request: PlanDriveEraseRequest,
) -> Result<DriveErasePlan, SafeErrorResponse> {
    plan_drive_erasure_handler(&state, request).await
}

#[tauri::command]
pub async fn execute_drive_erasure_simulation(
    state: State<'_, AppState>,
    request: ExecuteDriveEraseSimulationRequest,
) -> Result<DriveEraseResult, SafeErrorResponse> {
    execute_drive_erasure_simulation_handler(&state, request).await
}

#[tauri::command]
pub async fn execute_drive_erasure_hardware(
    state: State<'_, AppState>,
    request: ExecuteDriveEraseSimulationRequest,
) -> Result<DriveEraseResult, SafeErrorResponse> {
    execute_drive_erasure_hardware_handler(&state, request).await
}

#[tauri::command]
pub fn get_drive_capabilities(
    state: State<'_, AppState>,
    target_device_id: String,
) -> Result<DriveCapabilities, SafeErrorResponse> {
    get_drive_capabilities_handler(&state, &target_device_id)
}

#[tauri::command]
pub async fn assess_drive_hardware_capabilities(
    state: State<'_, AppState>,
    target_device_id: String,
) -> Result<DriveCapabilitiesAssessment, SafeErrorResponse> {
    assess_drive_hardware_capabilities_handler(&state, &target_device_id).await
}

#[tauri::command]
pub fn get_mock_test_devices(
    state: State<'_, AppState>,
) -> Result<Vec<PhysicalDevice>, SafeErrorResponse> {
    get_mock_test_devices_handler(&state)
}

#[tauri::command]
pub fn get_real_storage_devices(
    state: State<'_, AppState>,
) -> Result<Vec<PhysicalDevice>, SafeErrorResponse> {
    get_real_storage_devices_handler(&state)
}

#[tauri::command]
pub fn refresh_real_devices(
    state: State<'_, AppState>,
) -> Result<Vec<PhysicalDevice>, SafeErrorResponse> {
    refresh_real_devices_handler(&state)
}

#[tauri::command]
pub fn get_drive_erasure_result(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Option<DriveEraseResult>, SafeErrorResponse> {
    get_drive_erasure_result_handler(&state, &operation_id)
}

pub fn check_drive_eraser_privileges_handler() -> Result<DrivePrivilegeStatus, SafeErrorResponse> {
    let is_elevated = is_elevated_admin();
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unsupported"
    };

    let message = if is_elevated {
        "Administrative privileges verified: Process has elevated privileges required for raw physical storage access."
            .to_string()
    } else {
        "Elevation required: Exclusive direct hardware erasure requires running LocardX with elevated administrator/root privileges. Simulation mode remains available without elevation."
            .to_string()
    };

    Ok(DrivePrivilegeStatus {
        is_elevated,
        platform: platform.to_string(),
        message,
    })
}

pub fn generate_drive_erasure_report_handler(
    state: &AppState,
    operation_id: &str,
    session_token: Option<&str>,
) -> Result<DriveSanitizationReport, SafeErrorResponse> {
    let actor_id =
        session_token.and_then(|tok| state.auth.get_current_user(tok).ok().map(|u| u.username));

    let result_opt = state
        .drive_eraser
        .get_drive_erasure_result(operation_id)
        .map_err(|e| SafeErrorResponse::from(&e))?;

    let result = result_opt.ok_or_else(|| {
        SafeErrorResponse::from(&locardx_common::LocardError::Operation(format!(
            "No drive erasure result found for operation '{}'",
            operation_id
        )))
    })?;

    state
        .reporting
        .generate_drive_erasure_report(&result, actor_id.as_deref())
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub fn get_drive_erasure_report_handler(
    state: &AppState,
    operation_id: &str,
) -> Result<Option<DriveSanitizationReport>, SafeErrorResponse> {
    state
        .reporting
        .get_report_by_operation(operation_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub fn check_drive_eraser_privileges() -> Result<DrivePrivilegeStatus, SafeErrorResponse> {
    check_drive_eraser_privileges_handler()
}

#[tauri::command]
pub fn generate_drive_erasure_report(
    state: State<'_, AppState>,
    operation_id: String,
    session_token: Option<String>,
) -> Result<DriveSanitizationReport, SafeErrorResponse> {
    generate_drive_erasure_report_handler(&state, &operation_id, session_token.as_deref())
}

#[tauri::command]
pub fn get_drive_erasure_report(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Option<DriveSanitizationReport>, SafeErrorResponse> {
    get_drive_erasure_report_handler(&state, &operation_id)
}
