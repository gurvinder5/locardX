use crate::state::AppState;
use locardx_acquisition::{
    validate_destination, validate_source_device_id, AcquisitionArtifact,
    AcquisitionDeviceSnapshot, AcquisitionPlan, AcquisitionProgress, AcquisitionResult,
    AcquisitionStatus,
};
use locardx_common::SafeErrorResponse;
use locardx_device_manager::DeviceDiscoveryProvider;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAcquisitionPlanRequest {
    pub source_device_id: String,
    pub destination_path: String,
    pub chunk_size_bytes: Option<usize>,
    pub allow_overwrite: Option<bool>,
    pub session_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartAcquisitionRequest {
    pub plan: AcquisitionPlan,
    pub session_token: Option<String>,
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
    let actor_id = request
        .session_token
        .as_deref()
        .and_then(|tok| state.auth.get_current_user(tok).ok().map(|u| u.username));

    let acquisition_service = Arc::clone(&state.acquisition);
    let plan = request.plan;
    let operation_id = format!("op-acq-{}", uuid::Uuid::new_v4());

    // Run blocking acquisition stream in worker thread
    let result = tokio::task::spawn_blocking(move || {
        acquisition_service.execute_acquisition(&operation_id, &plan, actor_id.as_deref())
    })
    .await
    .map_err(|e| {
        SafeErrorResponse::from(&locardx_common::LocardError::Operation(format!(
            "Acquisition task failed: {}",
            e
        )))
    })?
    .map_err(|e| SafeErrorResponse::from(&e))?;

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
