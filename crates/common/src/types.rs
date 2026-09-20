use serde::{Deserialize, Serialize};

/// High-level application metadata returned by Tauri command `get_app_info`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub build_status: String,
    pub environment: String,
}

impl Default for AppInfo {
    fn default() -> Self {
        Self {
            name: "LocardX".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_status: "Foundation Verified".to_string(),
            environment: "development".to_string(),
        }
    }
}

/// Category of storage target being inspected, operated on, or verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetType {
    File,
    Directory,
    LogicalVolume,
    PhysicalDevice,
    EvidenceObject,
    Unknown,
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "File"),
            Self::Directory => write!(f, "Directory"),
            Self::LogicalVolume => write!(f, "LogicalVolume"),
            Self::PhysicalDevice => write!(f, "PhysicalDevice"),
            Self::EvidenceObject => write!(f, "EvidenceObject"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Normalized representation of an operation target.
/// Explicitly separates target identification from authorization or destructive operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetIdentity {
    pub target_type: TargetType,
    pub identifier: String,
    pub display_name: String,
    pub size_bytes: Option<u64>,
}

/// Category of operation managed by the Operation Orchestrator.
/// Extensible to future forensic and sanitization routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    IntegrityHash,
    IntegrityVerify,
    SanitizationPlanEvaluation,
    FileErasure,
    FolderErasure,
    DriveErasure,
    ForensicAcquisition,
    Recovery,
    Unknown,
}

impl OperationType {
    /// Invariant: Only read-only operations, sanitization planning, forensic acquisition,
    /// and logical file/folder erasure are executable in the base system.
    pub fn is_executable(&self) -> bool {
        matches!(
            self,
            Self::IntegrityHash
                | Self::IntegrityVerify
                | Self::SanitizationPlanEvaluation
                | Self::FileErasure
                | Self::FolderErasure
                | Self::ForensicAcquisition
        )
    }

    /// Identifies whether the operation involves data destruction or sanitization.
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            Self::FileErasure | Self::FolderErasure | Self::DriveErasure
        )
    }

    /// Identifies whether the operation is strictly read-only.
    pub fn is_read_only(&self) -> bool {
        matches!(
            self,
            Self::IntegrityHash
                | Self::IntegrityVerify
                | Self::SanitizationPlanEvaluation
                | Self::ForensicAcquisition
                | Self::Recovery
        )
    }
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IntegrityHash => write!(f, "IntegrityHash"),
            Self::IntegrityVerify => write!(f, "IntegrityVerify"),
            Self::SanitizationPlanEvaluation => write!(f, "SanitizationPlanEvaluation"),
            Self::FileErasure => write!(f, "FileErasure"),
            Self::FolderErasure => write!(f, "FolderErasure"),
            Self::DriveErasure => write!(f, "DriveErasure"),
            Self::ForensicAcquisition => write!(f, "ForensicAcquisition"),
            Self::Recovery => write!(f, "Recovery"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::str::FromStr for OperationType {
    type Err = crate::error::LocardError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "IntegrityHash" => Ok(Self::IntegrityHash),
            "IntegrityVerify" => Ok(Self::IntegrityVerify),
            "SanitizationPlanEvaluation" => Ok(Self::SanitizationPlanEvaluation),
            "FileErasure" => Ok(Self::FileErasure),
            "FolderErasure" => Ok(Self::FolderErasure),
            "DriveErasure" => Ok(Self::DriveErasure),
            "ForensicAcquisition" => Ok(Self::ForensicAcquisition),
            "Recovery" => Ok(Self::Recovery),
            "Unknown" => Ok(Self::Unknown),
            other => Err(crate::error::LocardError::Operation(format!(
                "Unrecognized operation type: {}",
                other
            ))),
        }
    }
}
