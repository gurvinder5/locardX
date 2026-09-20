use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Permitted user roles in LocardX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            "investigator" | "examiner" => Ok(Self::Investigator),
            "operator" | "analyst" => Ok(Self::Operator),
            "viewer" => Ok(Self::Viewer),
            _ => Err(format!("Invalid user role: '{}'", s)),
        }
    }
}

/// Granular system permissions across all LocardX modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // Case Management (Step 13)
    CaseCreate,
    CaseView,
    CaseModify,
    CaseClose,
    CaseEvidenceAdd,
    CaseCustodyRecord,
    CaseReportGenerate,

    // Acquisition (Step 11)
    AcquisitionViewSources,
    AcquisitionStart,
    AcquisitionCancel,
    AcquisitionViewArtifacts,

    // Recovery (Step 12)
    RecoveryStart,
    RecoveryViewResults,
    RecoveryExportFiles,
    RecoveryGenerateReport,

    // Erasure (Step 10)
    ErasureViewCapabilities,
    ErasurePlan,
    ErasureExecute,
    ErasureGenerateReport,

    // Audit & Verification
    AuditView,
    AuditVerify,

    // User Administration (Step 14)
    UserCreate,
    UserList,
    UserView,
    UserUpdate,
    UserDisable,
    UserChangeRole,
    UserUnlock,
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for Permission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CaseCreate" => Ok(Self::CaseCreate),
            "CaseView" => Ok(Self::CaseView),
            "CaseModify" => Ok(Self::CaseModify),
            "CaseClose" => Ok(Self::CaseClose),
            "CaseEvidenceAdd" => Ok(Self::CaseEvidenceAdd),
            "CaseCustodyRecord" => Ok(Self::CaseCustodyRecord),
            "CaseReportGenerate" => Ok(Self::CaseReportGenerate),
            "AcquisitionViewSources" => Ok(Self::AcquisitionViewSources),
            "AcquisitionStart" => Ok(Self::AcquisitionStart),
            "AcquisitionCancel" => Ok(Self::AcquisitionCancel),
            "AcquisitionViewArtifacts" => Ok(Self::AcquisitionViewArtifacts),
            "RecoveryStart" => Ok(Self::RecoveryStart),
            "RecoveryViewResults" => Ok(Self::RecoveryViewResults),
            "RecoveryExportFiles" => Ok(Self::RecoveryExportFiles),
            "RecoveryGenerateReport" => Ok(Self::RecoveryGenerateReport),
            "ErasureViewCapabilities" => Ok(Self::ErasureViewCapabilities),
            "ErasurePlan" => Ok(Self::ErasurePlan),
            "ErasureExecute" => Ok(Self::ErasureExecute),
            "ErasureGenerateReport" => Ok(Self::ErasureGenerateReport),
            "AuditView" => Ok(Self::AuditView),
            "AuditVerify" => Ok(Self::AuditVerify),
            "UserCreate" => Ok(Self::UserCreate),
            "UserList" => Ok(Self::UserList),
            "UserView" => Ok(Self::UserView),
            "UserUpdate" => Ok(Self::UserUpdate),
            "UserDisable" => Ok(Self::UserDisable),
            "UserChangeRole" => Ok(Self::UserChangeRole),
            "UserUnlock" => Ok(Self::UserUnlock),
            _ => Err(format!("Unknown permission identifier: '{}'", s)),
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
    pub display_name: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_login_at: Option<String>,
    pub metadata_json: String,
}

/// Sanitized user model returned to frontend and IPC calls.
/// STRICT SECURITY INVARIANT: NEVER exposes password_hash or internal secrets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicUser {
    pub user_id: String,
    pub username: String,
    pub role: UserRole,
    pub display_name: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_login_at: Option<String>,
    pub metadata_json: String,
}

impl From<&User> for PublicUser {
    fn from(user: &User) -> Self {
        Self {
            user_id: user.user_id.clone(),
            username: user.username.clone(),
            role: user.role,
            display_name: user.display_name.clone(),
            enabled: user.enabled,
            created_at: user.created_at.clone(),
            updated_at: user.updated_at.clone(),
            last_login_at: user.last_login_at.clone(),
            metadata_json: user.metadata_json.clone(),
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
    #[serde(default)]
    pub display_name: Option<String>,
}

impl InitAdminRequest {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        let p = password.into();
        Self {
            username: username.into(),
            confirm_password: p.clone(),
            password: p,
            display_name: None,
        }
    }
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
    pub permissions: Vec<String>,
}

/// Comprehensive session validation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionValidationResponse {
    pub is_valid: bool,
    pub user: Option<PublicUser>,
    pub expires_at: Option<i64>,
    pub time_remaining_seconds: Option<i64>,
    pub permissions: Vec<String>,
}

/// Request by Administrator to create a new user account.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: UserRole,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

impl CreateUserRequest {
    pub fn new(username: impl Into<String>, password: impl Into<String>, role: UserRole) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            role,
            display_name: None,
            metadata_json: None,
        }
    }
}

/// Request to update a user's details.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    pub user_id: String,
    pub display_name: Option<String>,
    pub metadata_json: Option<String>,
}

/// Request by Administrator to change a user's role.
#[derive(Debug, Clone, Deserialize)]
pub struct ChangeUserRoleRequest {
    pub user_id: String,
    pub new_role: UserRole,
}

/// Request by Administrator to unlock a temporarily locked user account.
#[derive(Debug, Clone, Deserialize)]
pub struct UnlockUserRequest {
    pub username: String,
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
