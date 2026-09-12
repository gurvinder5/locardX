use crate::state::AppState;
use locardx_auth::models::{
    AuthSessionResponse, ChangePasswordRequest, CreateUserRequest, InitAdminRequest, LoginRequest,
    PublicUser, SetUserEnabledRequest,
};
use locardx_common::SafeErrorResponse;

/// Checks whether the application is in first-run state (no administrator exists).
pub fn is_first_run_handler(state: &AppState) -> Result<bool, SafeErrorResponse> {
    state.auth.is_first_run().map_err(SafeErrorResponse::from)
}

/// Creates the initial administrator during first-run setup.
pub fn initialize_admin_handler(
    state: &AppState,
    payload: InitAdminRequest,
) -> Result<PublicUser, SafeErrorResponse> {
    state
        .auth
        .initialize_admin(payload)
        .map_err(SafeErrorResponse::from)
}

/// Authenticates a user and returns an active session token.
pub fn login_handler(
    state: &AppState,
    payload: LoginRequest,
) -> Result<AuthSessionResponse, SafeErrorResponse> {
    state.auth.login(payload).map_err(SafeErrorResponse::from)
}

/// Logs out the active user session.
pub fn logout_handler(state: &AppState, token: &str) -> Result<(), SafeErrorResponse> {
    state.auth.logout(token).map_err(SafeErrorResponse::from)
}

/// Retrieves the current authenticated user by session token.
pub fn get_current_user_handler(
    state: &AppState,
    token: &str,
) -> Result<PublicUser, SafeErrorResponse> {
    state
        .auth
        .get_current_user(token)
        .map_err(SafeErrorResponse::from)
}

/// Administrator-only: Creates a new user account.
pub fn create_user_handler(
    state: &AppState,
    token: &str,
    payload: CreateUserRequest,
) -> Result<PublicUser, SafeErrorResponse> {
    state
        .auth
        .create_user(token, payload)
        .map_err(SafeErrorResponse::from)
}

/// Administrator-only: Enables or disables an existing user account.
pub fn set_user_enabled_handler(
    state: &AppState,
    token: &str,
    payload: SetUserEnabledRequest,
) -> Result<(), SafeErrorResponse> {
    state
        .auth
        .set_user_enabled(token, &payload.user_id, payload.enabled)
        .map_err(SafeErrorResponse::from)
}

/// Administrator-only: Lists all registered user accounts.
pub fn list_users_handler(
    state: &AppState,
    token: &str,
) -> Result<Vec<PublicUser>, SafeErrorResponse> {
    state
        .auth
        .list_users(token)
        .map_err(SafeErrorResponse::from)
}

/// Authenticated user: Changes their own password.
pub fn change_password_handler(
    state: &AppState,
    token: &str,
    payload: ChangePasswordRequest,
) -> Result<(), SafeErrorResponse> {
    state
        .auth
        .change_password(token, payload)
        .map_err(SafeErrorResponse::from)
}

#[tauri::command]
pub fn is_first_run(state: tauri::State<'_, AppState>) -> Result<bool, SafeErrorResponse> {
    is_first_run_handler(&state)
}

#[tauri::command]
pub fn initialize_admin(
    state: tauri::State<'_, AppState>,
    payload: InitAdminRequest,
) -> Result<PublicUser, SafeErrorResponse> {
    initialize_admin_handler(&state, payload)
}

#[tauri::command]
pub fn login(
    state: tauri::State<'_, AppState>,
    payload: LoginRequest,
) -> Result<AuthSessionResponse, SafeErrorResponse> {
    login_handler(&state, payload)
}

#[tauri::command]
pub fn logout(state: tauri::State<'_, AppState>, token: String) -> Result<(), SafeErrorResponse> {
    logout_handler(&state, &token)
}

#[tauri::command]
pub fn get_current_user(
    state: tauri::State<'_, AppState>,
    token: String,
) -> Result<PublicUser, SafeErrorResponse> {
    get_current_user_handler(&state, &token)
}

#[tauri::command]
pub fn create_user(
    state: tauri::State<'_, AppState>,
    token: String,
    payload: CreateUserRequest,
) -> Result<PublicUser, SafeErrorResponse> {
    create_user_handler(&state, &token, payload)
}

#[tauri::command]
pub fn set_user_enabled(
    state: tauri::State<'_, AppState>,
    token: String,
    payload: SetUserEnabledRequest,
) -> Result<(), SafeErrorResponse> {
    set_user_enabled_handler(&state, &token, payload)
}

#[tauri::command]
pub fn list_users(
    state: tauri::State<'_, AppState>,
    token: String,
) -> Result<Vec<PublicUser>, SafeErrorResponse> {
    list_users_handler(&state, &token)
}

#[tauri::command]
pub fn change_password(
    state: tauri::State<'_, AppState>,
    token: String,
    payload: ChangePasswordRequest,
) -> Result<(), SafeErrorResponse> {
    change_password_handler(&state, &token, payload)
}
