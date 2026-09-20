use crate::state::AppState;
use locardx_common::{AppInfo, SafeErrorResponse};
use tracing::info;

/// Harmless application info command.
/// Returns basic metadata without touching disks, evidence, or shell commands.
pub fn get_app_info_handler(state: &AppState) -> Result<AppInfo, SafeErrorResponse> {
    info!("Handling get_app_info request");

    let info = AppInfo {
        name: "LocardX".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_status: "Foundation Verified".to_string(),
        environment: state.config.env.clone(),
    };

    Ok(info)
}

#[tauri::command]
pub fn get_app_info(state: tauri::State<'_, AppState>) -> Result<AppInfo, SafeErrorResponse> {
    get_app_info_handler(&state)
}
