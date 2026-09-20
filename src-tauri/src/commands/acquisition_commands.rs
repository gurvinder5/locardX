use crate::state::AppState;
use locardx_acquisition::{
    validate_destination, validate_source_device_id, AcquisitionArtifact,
    AcquisitionDeviceSnapshot, AcquisitionPlan, AcquisitionProgress, AcquisitionRecordDto,
    AcquisitionResult, AcquisitionStatus,
};
use locardx_common::SafeErrorResponse;
use locardx_device_manager::DeviceDiscoveryProvider;
use locardx_drive_eraser::is_elevated_admin;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionPrivilegeStatus {
    pub is_elevated: bool,
    pub platform: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAcquisitionPlanRequest {
    pub source_device_id: String,
    pub destination_path: String,
    pub chunk_size_bytes: Option<usize>,
    pub allow_overwrite: Option<bool>,
    pub session_token: Option<String>,
    pub case_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartAcquisitionRequest {
    pub plan: AcquisitionPlan,
    pub session_token: Option<String>,
    pub operation_id: Option<String>,
    pub case_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidateSourceResponse {
    pub valid: bool,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidateDestinationRequest {
    pub destination_path: String,
    pub source_device_id: String,
    pub required_capacity_bytes: u64,
    pub allow_overwrite: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidateDestinationResponse {
    pub valid: bool,
    pub available_free_bytes: u64,
    pub required_bytes: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactVerificationResponse {
    pub verified: bool,
    pub expected_hash: String,
    pub calculated_hash: String,
    pub expected_size: u64,
    pub actual_size: u64,
    pub error_message: Option<String>,
}

// ==========================================
// Handlers (testable without Tauri runtime)
// ==========================================

pub fn list_acquisition_sources_handler(
    state: &AppState,
) -> Result<Vec<AcquisitionDeviceSnapshot>, SafeErrorResponse> {
    // 1. First probe real physical devices
    let real_devs = state
        .drive_eraser
        .discover_real_devices()
        .unwrap_or_default();

    let mut snapshots: Vec<AcquisitionDeviceSnapshot> = real_devs
        .into_iter()
        .map(|d| AcquisitionDeviceSnapshot {
            device_id: d.device_id,
            display_name: d.display_name,
            vendor: d.vendor,
            model: d.model,
            serial_number: d.serial_number,
            media_type: d.device_type.to_string(),
            capacity_bytes: d.capacity_bytes,
            sector_size: 512,
            bus_type: None,
            is_removable: d.removable,
            is_system: d.is_system_device,
            snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
        })
        .collect();

    // 2. If no real devices detected (e.g. non-elevated or dev environment), include mock devices
    if snapshots.is_empty() {
        let mock_devs = state
            .mock_device_registry
            .discover_devices()
            .unwrap_or_default();
        for d in mock_devs {
            snapshots.push(AcquisitionDeviceSnapshot {
                device_id: d.device_id,
                display_name: format!("[SIMULATED] {}", d.display_name),
                vendor: d.vendor,
                model: d.model,
                serial_number: d.serial_number,
                media_type: d.device_type.to_string(),
                capacity_bytes: d.capacity_bytes,
                sector_size: 512,
                bus_type: None,
                is_removable: d.removable,
                is_system: d.is_system_device,
                snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    Ok(snapshots)
}

pub fn validate_acquisition_source_handler(
    device_id: &str,
) -> Result<ValidateSourceResponse, SafeErrorResponse> {
    match validate_source_device_id(device_id) {
        Ok(valid_id) => Ok(ValidateSourceResponse {
            valid: true,
            message: format!(
                "Physical device source '{}' is valid for read-only acquisition",
                valid_id
            ),
        }),
        Err(e) => Ok(ValidateSourceResponse {
            valid: false,
            message: e.to_string(),
        }),
    }
}

pub fn validate_acquisition_destination_handler(
    state: &AppState,
    request: &ValidateDestinationRequest,
) -> Result<ValidateDestinationResponse, SafeErrorResponse> {
    match validate_destination(
        state.acquisition.reader().as_ref(),
        &request.source_device_id,
        &request.destination_path,
        request.required_capacity_bytes,
        request.allow_overwrite,
    ) {
        Ok(validated) => Ok(ValidateDestinationResponse {
            valid: true,
            available_free_bytes: validated.available_space_bytes,
            required_bytes: request.required_capacity_bytes,
            message: format!(
                "Destination valid: {:.2} GB available, {:.2} GB required",
                validated.available_space_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
                request.required_capacity_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
            ),
        }),
        Err(e) => Ok(ValidateDestinationResponse {
            valid: false,
            available_free_bytes: 0,
            required_bytes: request.required_capacity_bytes,
            message: e.to_string(),
        }),
    }
}

pub fn create_acquisition_plan_handler(
    state: &AppState,
    request: CreateAcquisitionPlanRequest,
) -> Result<AcquisitionPlan, SafeErrorResponse> {
    // Case Policy: Fail-closed if no active case or if case is closed
    state
        .case_service
        .validate_case_for_operation(request.case_id.as_deref())
        .map_err(|e| SafeErrorResponse::from(&e))?;

    let actor_id = request
        .session_token
        .as_deref()
        .and_then(|tok| state.auth.get_current_user(tok).ok().map(|u| u.username));

    state
        .acquisition
        .create_plan(
            &request.source_device_id,
            &request.destination_path,
            request.allow_overwrite.unwrap_or(false),
            actor_id.as_deref(),
        )
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub async fn start_acquisition_handler(
    state: &AppState,
    request: StartAcquisitionRequest,
) -> Result<AcquisitionResult, SafeErrorResponse> {
    // Case Policy: Fail-closed if no active case or if case is closed
    let active_case = state
        .case_service
        .validate_case_for_operation(request.case_id.as_deref())
        .map_err(|e| SafeErrorResponse::from(&e))?;
    let case_id = active_case.case_id.clone();

    let actor_id = request
        .session_token
        .as_deref()
        .and_then(|tok| state.auth.get_current_user(tok).ok().map(|u| u.username));

    let acquisition_service = Arc::clone(&state.acquisition);
    let plan = request.plan;
    let operation_id = request
        .operation_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("op-acq-{}", uuid::Uuid::new_v4()));

    let op_id_cloned = operation_id.clone();
    let case_id_cloned = case_id.clone();
    let actor_id_cloned = actor_id.clone();

    // Run blocking acquisition stream in worker thread
    let result = tokio::task::spawn_blocking(move || {
        acquisition_service.execute_acquisition_with_case(
            &op_id_cloned,
            &plan,
            actor_id_cloned.as_deref(),
            Some(&case_id_cloned),
        )
    })
    .await
    .map_err(|e| {
        SafeErrorResponse::from(&locardx_common::LocardError::Operation(format!(
            "Acquisition task failed: {}",
            e
        )))
    })?
    .map_err(|e| SafeErrorResponse::from(&e))?;

    // Associate operation with the active case in CaseService (auto-links evidence)
    let user = request
        .session_token
        .as_deref()
        .and_then(|tok| state.auth.get_current_user(tok).ok())
        .unwrap_or_else(|| locardx_auth::PublicUser {
            user_id: "sys-operator".to_string(),
            username: actor_id.clone().unwrap_or_else(|| "operator".to_string()),
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
        &operation_id,
        "ForensicAcquisition",
        &user,
        Some(&format!(
            "Forensic bitstream acquisition of {} -> {}",
            result.source.display_name, result.destination_path
        )),
    );

    Ok(result)
}

pub fn cancel_acquisition_handler(
    state: &AppState,
    operation_id: &str,
) -> Result<bool, SafeErrorResponse> {
    Ok(state.acquisition.cancel_acquisition(operation_id))
}

pub fn get_acquisition_progress_handler(
    state: &AppState,
    operation_id: &str,
) -> Result<Option<AcquisitionProgress>, SafeErrorResponse> {
    Ok(state.acquisition.get_progress(operation_id))
}

pub fn get_acquisition_result_handler(
    state: &AppState,
    operation_id: &str,
) -> Result<Option<AcquisitionResult>, SafeErrorResponse> {
    state
        .acquisition
        .get_acquisition_result(operation_id)
        .map_err(|e| SafeErrorResponse::from(&e))
}

pub fn get_acquisition_artifact_handler(
    state: &AppState,
    operation_id: &str,
) -> Result<Option<AcquisitionArtifact>, SafeErrorResponse> {
    let result_opt = state
        .acquisition
        .get_acquisition_result(operation_id)
        .map_err(|e| SafeErrorResponse::from(&e))?;

    match result_opt {
        Some(res) if res.status == AcquisitionStatus::Completed => {
            let artifact = AcquisitionArtifact::from_result(&res).map_err(|e| {
                SafeErrorResponse::from(&locardx_common::LocardError::Operation(e.to_string()))
            })?;
            Ok(Some(artifact))
        }
        _ => Ok(None),
    }
}

pub fn verify_acquisition_artifact_handler(
    artifact: AcquisitionArtifact,
) -> Result<ArtifactVerificationResponse, SafeErrorResponse> {
    let actual_size = std::fs::metadata(&artifact.image_path)
        .map(|m| m.len())
        .unwrap_or(0);

    match artifact.verify_integrity() {
        Ok(true) => Ok(ArtifactVerificationResponse {
            verified: true,
            expected_hash: artifact.image_sha256.clone(),
            calculated_hash: artifact.image_sha256,
            expected_size: artifact.image_size_bytes,
            actual_size,
            error_message: None,
        }),
        Ok(false) => Ok(ArtifactVerificationResponse {
            verified: false,
            expected_hash: artifact.image_sha256,
            calculated_hash: String::from("MISMATCH"),
            expected_size: artifact.image_size_bytes,
            actual_size,
            error_message: Some(
                "Integrity verification failed: SHA-256 hash or size mismatch".to_string(),
            ),
        }),
        Err(e) => Ok(ArtifactVerificationResponse {
            verified: false,
            expected_hash: artifact.image_sha256,
            calculated_hash: String::new(),
            expected_size: artifact.image_size_bytes,
            actual_size,
            error_message: Some(e.to_string()),
        }),
    }
}

// ==========================================
// Tauri Command Wrappers
// ==========================================

#[tauri::command]
pub fn list_acquisition_sources(
    state: State<'_, AppState>,
) -> Result<Vec<AcquisitionDeviceSnapshot>, SafeErrorResponse> {
    list_acquisition_sources_handler(&state)
}

#[tauri::command]
pub fn validate_acquisition_source(
    device_id: String,
) -> Result<ValidateSourceResponse, SafeErrorResponse> {
    validate_acquisition_source_handler(&device_id)
}

#[tauri::command]
pub fn validate_acquisition_destination(
    state: State<'_, AppState>,
    request: ValidateDestinationRequest,
) -> Result<ValidateDestinationResponse, SafeErrorResponse> {
    validate_acquisition_destination_handler(&state, &request)
}

#[tauri::command]
pub fn create_acquisition_plan(
    state: State<'_, AppState>,
    request: CreateAcquisitionPlanRequest,
) -> Result<AcquisitionPlan, SafeErrorResponse> {
    create_acquisition_plan_handler(&state, request)
}

#[tauri::command]
pub async fn start_acquisition(
    state: State<'_, AppState>,
    request: StartAcquisitionRequest,
) -> Result<AcquisitionResult, SafeErrorResponse> {
    start_acquisition_handler(&state, request).await
}

#[tauri::command]
pub fn cancel_acquisition(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<bool, SafeErrorResponse> {
    cancel_acquisition_handler(&state, &operation_id)
}

#[tauri::command]
pub fn get_acquisition_progress(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Option<AcquisitionProgress>, SafeErrorResponse> {
    get_acquisition_progress_handler(&state, &operation_id)
}

#[tauri::command]
pub fn get_acquisition_result(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Option<AcquisitionResult>, SafeErrorResponse> {
    get_acquisition_result_handler(&state, &operation_id)
}

#[tauri::command]
pub fn get_acquisition_artifact(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Option<AcquisitionArtifact>, SafeErrorResponse> {
    get_acquisition_artifact_handler(&state, &operation_id)
}

#[tauri::command]
pub fn verify_acquisition_artifact(
    artifact: AcquisitionArtifact,
) -> Result<ArtifactVerificationResponse, SafeErrorResponse> {
    verify_acquisition_artifact_handler(artifact)
}

pub fn check_acquisition_privileges_handler() -> Result<AcquisitionPrivilegeStatus, SafeErrorResponse> {
    let is_elevated = is_elevated_admin();
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unsupported"
    };

    let message = if is_elevated {
        "Administrative privileges verified: Process has elevated privileges required for raw physical bitstream acquisition."
            .to_string()
    } else {
        "Administrative elevation required: Bitstream acquisition from physical storage devices requires running LocardX as Administrator."
            .to_string()
    };

    Ok(AcquisitionPrivilegeStatus {
        is_elevated,
        platform: platform.to_string(),
        message,
    })
}

#[tauri::command]
pub fn check_acquisition_privileges() -> Result<AcquisitionPrivilegeStatus, SafeErrorResponse> {
    check_acquisition_privileges_handler()
}

pub fn list_acquisition_records_handler(
    state: &AppState,
) -> Result<Vec<AcquisitionRecordDto>, SafeErrorResponse> {
    state
        .acquisition
        .list_acquisition_records()
        .map_err(|e| SafeErrorResponse::from(&e))
}

#[tauri::command]
pub fn list_acquisition_records(
    state: State<'_, AppState>,
) -> Result<Vec<AcquisitionRecordDto>, SafeErrorResponse> {
    list_acquisition_records_handler(&state)
}
