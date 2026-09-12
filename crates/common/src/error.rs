use std::fmt;

/// Core application error types for LocardX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocardError {
    Config(String),
    Database(String),
    SecurityViolation(String),
    Operation(String),
    Device(String),
    AuditIntegrity(String),
    Internal(String),
}

impl fmt::Display for LocardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Database(msg) => write!(f, "Database error: {}", msg),
            Self::SecurityViolation(msg) => write!(f, "Security policy violation: {}", msg),
            Self::Operation(msg) => write!(f, "Operation error: {}", msg),
            Self::Device(msg) => write!(f, "Device I/O error: {}", msg),
            Self::AuditIntegrity(msg) => write!(f, "Audit log integrity error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for LocardError {}

use serde::{Deserialize, Serialize};

/// Sanitized error representation safe to return over Tauri IPC to the frontend.
/// Omits raw system pointers, raw memory traces, or confidential file paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeErrorResponse {
    pub code: String,
    pub message: String,
}

impl From<&LocardError> for SafeErrorResponse {
    fn from(err: &LocardError) -> Self {
        match err {
            LocardError::Config(msg) => Self {
                code: "CONFIG_ERROR".to_string(),
                message: msg.clone(),
            },
            LocardError::Database(_) => Self {
                code: "DATABASE_ERROR".to_string(),
                message: "Database operation failed.".to_string(),
            },
            LocardError::SecurityViolation(msg) => Self {
                code: "SECURITY_VIOLATION".to_string(),
                message: msg.clone(),
            },
            LocardError::Operation(msg) => Self {
                code: "OPERATION_ERROR".to_string(),
                message: msg.clone(),
            },
            LocardError::Device(_) => Self {
                code: "DEVICE_ERROR".to_string(),
                message: "A storage device communication failure occurred.".to_string(),
            },
            LocardError::AuditIntegrity(_) => Self {
                code: "AUDIT_INTEGRITY_ERROR".to_string(),
                message: "Audit record integrity verification failed.".to_string(),
            },
            LocardError::Internal(_) => Self {
                code: "INTERNAL_ERROR".to_string(),
                message: "An internal application error occurred.".to_string(),
            },
        }
    }
}

impl From<LocardError> for SafeErrorResponse {
    fn from(err: LocardError) -> Self {
        SafeErrorResponse::from(&err)
    }
}
