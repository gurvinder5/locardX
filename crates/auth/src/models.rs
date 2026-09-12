use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Permitted user roles in LocardX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    Administrator,
    Investigator,
    Operator,
    Viewer,
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Administrator => write!(f, "Administrator"),
            Self::Investigator => write!(f, "Investigator"),
            Self::Operator => write!(f, "Operator"),
            Self::Viewer => write!(f, "Viewer"),
        }
    }
}

impl FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "administrator" | "admin" => Ok(Self::Administrator),
            "investigator" => Ok(Self::Investigator),
            "operator" => Ok(Self::Operator),
            "viewer" => Ok(Self::Viewer),
            _ => Err(format!("Invalid user role: '{}'", s)),
        }
    }
}

/// Internal user record stored in SQLite (includes password_hash).
#[derive(Debug, Clone)]
pub struct User {
    pub user_id: String,
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_login_at: Option<String>,
}

/// Sanitized user model returned to frontend and IPC calls.
/// STRICT SECURITY INVARIANT: NEVER exposes password_hash or internal secrets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicUser {
    pub user_id: String,
    pub username: String,
    pub role: UserRole,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_login_at: Option<String>,
}

impl From<&User> for PublicUser {
    fn from(user: &User) -> Self {
        Self {
            user_id: user.user_id.clone(),
            username: user.username.clone(),
            role: user.role,
            enabled: user.enabled,
            created_at: user.created_at.clone(),
            updated_at: user.updated_at.clone(),
            last_login_at: user.last_login_at.clone(),
        }
    }
}

/// Active desktop session record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub user_id: String,
    pub role: UserRole,
    pub created_at: i64,
    pub expires_at: i64,
}

/// Request to create the initial administrator during first-run setup.
#[derive(Debug, Clone, Deserialize)]
pub struct InitAdminRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub confirm_password: String,
}

/// Request to authenticate an existing user.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Successful login authentication payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSessionResponse {
    pub token: String,
    pub user: PublicUser,
    pub expires_at: i64,
}

/// Request by Administrator to create a new user account.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

/// Request by Administrator to enable or disable an existing user account.
#[derive(Debug, Clone, Deserialize)]
pub struct SetUserEnabledRequest {
    pub user_id: String,
    pub enabled: bool,
}

/// Request to update an authenticated user's password.
#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
    #[serde(alias = "current_password")]
    pub old_password: String,
    pub new_password: String,
}
