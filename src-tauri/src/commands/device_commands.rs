use crate::state::AppState;
use locardx_common::SafeErrorResponse;
use locardx_device_manager::StorageDeviceDto;

/// Discovers and returns all attached storage devices and logical volumes.
/// Safe, read-only endpoint returning sanitized DTOs without raw OS handles.
pub fn list_storage_devices_handler(
    state: &AppState,
) -> Result<Vec<StorageDeviceDto>, SafeErrorResponse> {
    state
        .device_manager
        .list_devices_dto()
        .map_err(SafeErrorResponse::from)
}

#[tauri::command]
pub fn list_storage_devices(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<StorageDeviceDto>, SafeErrorResponse> {
    list_storage_devices_handler(&state)
}
