use crate::state::AppState;
use locardx_auth::models::{
    AuthSessionResponse, ChangePasswordRequest, ChangeUserRoleRequest, CreateUserRequest,
    InitAdminRequest, LoginRequest, Permission, PublicUser, SessionValidationResponse,
    SetUserEnabledRequest, UnlockUserRequest, UpdateUserRequest,
};
use locardx_common::SafeErrorResponse;
use serde::{Deserialize, Serialize};

/// Status response for first-run bootstrap checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatusResponse {
    pub is_first_run: bool,
    pub initialized: bool,
}

// ==========================================
// Handlers (testable directly without Tauri)
// ==========================================

/// Checks whether the application is in first-run state or initialized.
pub fn get_authentication_status_handler(
    state: &AppState,
) -> Result<AuthStatusResponse, SafeErrorResponse> {
    let is_first = state.auth.is_first_run().map_err(SafeErrorResponse::from)?;
    Ok(AuthStatusResponse {
        is_first_run: is_first,
        initialized: !is_first,
    })
}

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

/// Validates a session token and returns user details + permissions.
pub fn validate_session_handler(
    state: &AppState,
    token: &str,
) -> Result<SessionValidationResponse, SafeErrorResponse> {
    state
        .auth
        .validate_session(token)
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

/// Retrieves a specific user by ID (requires UserView permission).
pub fn get_user_handler(
    state: &AppState,
    token: &str,
    user_id: &str,
) -> Result<PublicUser, SafeErrorResponse> {
    state
        .auth
        .authorize_permission(token, Permission::UserView)
        .map_err(SafeErrorResponse::from)?;
    state
        .auth
        .get_user(token, user_id)
        .map_err(SafeErrorResponse::from)
}

/// Updates user details (display name, metadata) (requires UserUpdate permission).
pub fn update_user_handler(
    state: &AppState,
    token: &str,
    payload: UpdateUserRequest,
) -> Result<PublicUser, SafeErrorResponse> {
    state
        .auth
        .update_user(token, payload)
        .map_err(SafeErrorResponse::from)
}

/// Enables a user account (requires UserUpdate permission).
pub fn enable_user_handler(
    state: &AppState,
    token: &str,
    user_id: &str,
) -> Result<(), SafeErrorResponse> {
    state
        .auth
        .set_user_enabled(token, user_id, true)
        .map_err(SafeErrorResponse::from)
}

/// Disables a user account (requires UserUpdate permission).
pub fn disable_user_handler(
    state: &AppState,
    token: &str,
    user_id: &str,
) -> Result<(), SafeErrorResponse> {
    state
        .auth
        .set_user_enabled(token, user_id, false)
        .map_err(SafeErrorResponse::from)
}

/// Changes a user's role (requires RoleAssign permission).
pub fn change_user_role_handler(
    state: &AppState,
    token: &str,
    payload: ChangeUserRoleRequest,
) -> Result<PublicUser, SafeErrorResponse> {
    state
        .auth
        .change_user_role(token, payload)
        .map_err(SafeErrorResponse::from)
}

/// Unlocks a locked user account (requires UserUpdate permission).
pub fn unlock_user_handler(
    state: &AppState,
    token: &str,
    payload: UnlockUserRequest,
) -> Result<(), SafeErrorResponse> {
    state
        .auth
        .unlock_user(token, payload)
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

/// Retrieves the permission list for the currently active session token.
pub fn get_current_permissions_handler(
    state: &AppState,
    token: &str,
) -> Result<Vec<String>, SafeErrorResponse> {
    let session = state
        .auth
        .validate_session(token)
        .map_err(SafeErrorResponse::from)?;
    Ok(session.permissions)
}

/// Checks whether the active session has a specific permission.
pub fn check_permission_handler(
    state: &AppState,
    token: &str,
    permission: Permission,
) -> Result<bool, SafeErrorResponse> {
    Ok(state.auth.authorize_permission(token, permission).is_ok())
}

// ==========================================
// Tauri Commands
// ==========================================

#[tauri::command]
pub fn get_authentication_status(
    state: tauri::State<'_, AppState>,
) -> Result<AuthStatusResponse, SafeErrorResponse> {
    get_authentication_status_handler(&state)
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
pub fn initialize_administrator(
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
pub fn validate_session(
    state: tauri::State<'_, AppState>,
    token: String,
) -> Result<SessionValidationResponse, SafeErrorResponse> {
    validate_session_handler(&state, &token)
}

#[tauri::command]
pub fn get_current_user(
    state: tauri::State<'_, AppState>,
    token: String,
) -> Result<PublicUser, SafeErrorResponse> {
    get_current_user_handler(&state, &token)
}

#[tauri::command]
pub fn get_user(
    state: tauri::State<'_, AppState>,
    token: String,
    user_id: String,
) -> Result<PublicUser, SafeErrorResponse> {
    get_user_handler(&state, &token, &user_id)
}

#[tauri::command]
pub fn update_user(
    state: tauri::State<'_, AppState>,
    token: String,
    payload: UpdateUserRequest,
) -> Result<PublicUser, SafeErrorResponse> {
    update_user_handler(&state, &token, payload)
}

#[tauri::command]
pub fn enable_user(
    state: tauri::State<'_, AppState>,
    token: String,
    user_id: String,
) -> Result<(), SafeErrorResponse> {
    enable_user_handler(&state, &token, &user_id)
}

#[tauri::command]
pub fn disable_user(
    state: tauri::State<'_, AppState>,
    token: String,
    user_id: String,
) -> Result<(), SafeErrorResponse> {
    disable_user_handler(&state, &token, &user_id)
}

#[tauri::command]
pub fn change_user_role(
    state: tauri::State<'_, AppState>,
    token: String,
    payload: ChangeUserRoleRequest,
) -> Result<PublicUser, SafeErrorResponse> {
    change_user_role_handler(&state, &token, payload)
}

#[tauri::command]
pub fn unlock_user(
    state: tauri::State<'_, AppState>,
    token: String,
    payload: UnlockUserRequest,
) -> Result<(), SafeErrorResponse> {
    unlock_user_handler(&state, &token, payload)
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

#[tauri::command]
pub fn get_current_permissions(
    state: tauri::State<'_, AppState>,
    token: String,
) -> Result<Vec<String>, SafeErrorResponse> {
    get_current_permissions_handler(&state, &token)
}

#[tauri::command]
pub fn check_permission(
    state: tauri::State<'_, AppState>,
    token: String,
    permission: Permission,
) -> Result<bool, SafeErrorResponse> {
    check_permission_handler(&state, &token, permission)
}
