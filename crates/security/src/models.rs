use chrono::{DateTime, Utc};
use locardx_common::{OperationType, TargetIdentity, TargetType};
use locardx_device_manager::DeviceClassification;
use serde::{Deserialize, Serialize};
use std::fmt;

/// High-level outcome of a safety and authorization evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyDecisionOutcome {
    Allowed,
    Denied,
    RequiresConfirmation,
    Blocked,
}

impl fmt::Display for SafetyDecisionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allowed => write!(f, "Allowed"),
            Self::Denied => write!(f, "Denied"),
            Self::RequiresConfirmation => write!(f, "RequiresConfirmation"),
            Self::Blocked => write!(f, "Blocked"),
        }
    }
}

/// Machine-readable reason codes explaining safety evaluations.
/// INVARIANT: Never rely solely on human-readable error strings; use stable codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasonCode {
    ValidTarget,
    SystemDevice,
    BootDevice,
    UnknownTarget,
    InvalidTarget,
    UnauthorizedRole,
    MissingConfirmation,
    ConfirmationExpired,
    ConfirmationInvalid,
    TargetChanged,
    UnsupportedOperation,
    UnsupportedDeviceType,
    UnsupportedFilesystem,
    SafetyPolicyViolation,
    OperationDisabled,
}

impl fmt::Display for ReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidTarget => write!(f, "VALID_TARGET"),
            Self::SystemDevice => write!(f, "SYSTEM_DEVICE"),
            Self::BootDevice => write!(f, "BOOT_DEVICE"),
            Self::UnknownTarget => write!(f, "UNKNOWN_TARGET"),
            Self::InvalidTarget => write!(f, "INVALID_TARGET"),
            Self::UnauthorizedRole => write!(f, "UNAUTHORIZED_ROLE"),
            Self::MissingConfirmation => write!(f, "MISSING_CONFIRMATION"),
            Self::ConfirmationExpired => write!(f, "CONFIRMATION_EXPIRED"),
            Self::ConfirmationInvalid => write!(f, "CONFIRMATION_INVALID"),
            Self::TargetChanged => write!(f, "TARGET_CHANGED"),
            Self::UnsupportedOperation => write!(f, "UNSUPPORTED_OPERATION"),
            Self::UnsupportedDeviceType => write!(f, "UNSUPPORTED_DEVICE_TYPE"),
            Self::UnsupportedFilesystem => write!(f, "UNSUPPORTED_FILESYSTEM"),
            Self::SafetyPolicyViolation => write!(f, "SAFETY_POLICY_VIOLATION"),
            Self::OperationDisabled => write!(f, "OPERATION_DISABLED"),
        }
    }
}

/// Risk classification for targets and operations.
/// Invariant: Risk describes danger; Authorization describes permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

/// Immutable snapshot of a target taken during preparation/review.
/// Prevents Time-Of-Check to Time-Of-Use (TOCTOU) target substitution attacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetSnapshot {
    pub target_identifier: String,
    pub target_type: TargetType,
    pub capacity_bytes: Option<u64>,
    pub filesystem: Option<String>,
    pub device_id: Option<String>,
    pub classification: Option<DeviceClassification>,
    pub is_system: bool,
    pub is_boot: bool,
    pub snapshot_timestamp: String,
}

impl TargetSnapshot {
    /// Compares this snapshot with a current live target snapshot.
    /// If capacity, filesystem, device ID, or system/boot status differs,
    /// returns Err(ReasonCode::TargetChanged).
    pub fn detect_changes(&self, current: &TargetSnapshot) -> Result<(), ReasonCode> {
        if self.target_identifier != current.target_identifier {
            return Err(ReasonCode::TargetChanged);
        }
        if self.target_type != current.target_type {
            return Err(ReasonCode::TargetChanged);
        }
        if self.device_id.is_some()
            && current.device_id.is_some()
            && self.device_id != current.device_id
        {
            return Err(ReasonCode::TargetChanged);
        }
        if self.capacity_bytes.is_some()
            && current.capacity_bytes.is_some()
            && self.capacity_bytes != current.capacity_bytes
        {
            return Err(ReasonCode::TargetChanged);
        }
        if self.is_system != current.is_system || self.is_boot != current.is_boot {
            return Err(ReasonCode::TargetChanged);
        }
        if self.classification.is_some()
            && current.classification.is_some()
            && self.classification != current.classification
        {
            return Err(ReasonCode::TargetChanged);
        }
        Ok(())
    }
}

/// Comprehensive safety and authorization evaluation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyDecision {
    pub evaluation_id: String,
    pub target: TargetIdentity,
    pub operation_type: OperationType,
    pub actor_id: Option<String>,
    pub decision: SafetyDecisionOutcome,
    pub reason_code: ReasonCode,
    pub risk_level: RiskLevel,
    pub message: String,
    pub evaluated_at: String,
    pub target_snapshot: Option<TargetSnapshot>,
    pub requires_confirmation: bool,
}

/// Lifecycle status of an operation confirmation challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfirmationStatus {
    Pending,
    Confirmed,
    Rejected,
    Expired,
    Revoked,
}

impl fmt::Display for ConfirmationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Confirmed => write!(f, "Confirmed"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Expired => write!(f, "Expired"),
            Self::Revoked => write!(f, "Revoked"),
        }
    }
}

/// Server-generated confirmation challenge returned during Stage 1 Review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationChallenge {
    pub confirmation_id: String,
    pub operation_id: String,
    pub actor_id: String,
    pub target: TargetIdentity,
    pub operation_type: OperationType,
    pub risk_level: RiskLevel,
    pub target_snapshot: TargetSnapshot,
    pub created_at: String,
    pub expires_at: String,
}

/// Persistent record of an explicit confirmation bound to operation, target, and actor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationRecord {
    pub confirmation_id: String,
    pub operation_id: String,
    pub actor_id: String,
    pub target_identifier: String,
    pub target_type: TargetType,
    pub target_snapshot: TargetSnapshot,
    pub operation_type: OperationType,
    pub risk_level: RiskLevel,
    pub warning_acknowledged: bool,
    pub status: ConfirmationStatus,
    pub created_at: String,
    pub expires_at: String,
    pub confirmed_at: Option<String>,
}

impl ConfirmationRecord {
    pub fn is_expired(&self) -> bool {
        if let Ok(exp) = DateTime::parse_from_rfc3339(&self.expires_at) {
            Utc::now() > exp.with_timezone(&Utc)
        } else {
            true
        }
    }
}
