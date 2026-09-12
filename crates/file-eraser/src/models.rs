use chrono::Utc;
use locardx_security::RiskLevel;
use locardx_verification::sanitization::{
    SanitizationMethod, VerificationOutcome, VerificationStrategy,
};
use serde::{Deserialize, Serialize};

/// Identifies the scope of a file-system erasure operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileEraseScope {
    File,
    Folder,
}

impl std::fmt::Display for FileEraseScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "File"),
            Self::Folder => write!(f, "Folder"),
        }
    }
}

/// Lifecycle execution status of a file or folder erasure operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileEraseStatus {
    Planned,
    Authorized,
    Running,
    Completed,
    Failed,
    Cancelled,
    VerificationFailed,
    UnableToVerify,
}

impl std::fmt::Display for FileEraseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Planned => write!(f, "Planned"),
            Self::Authorized => write!(f, "Authorized"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::VerificationFailed => write!(f, "VerificationFailed"),
            Self::UnableToVerify => write!(f, "UnableToVerify"),
        }
    }
}

/// Categorized failure reasons for file or folder sanitization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileEraseFailureReason {
    TargetNotFound(String),
    InvalidTargetType(String),
    SystemOrBootPath(String),
    SymbolicLinkDetected(String),
    TargetChanged(String),
    PermissionDenied(String),
    IoError(String),
    VerificationFailed(String),
    Cancelled,
    PartialFolderFailure(String),
    SecurityViolation(String),
    Unexpected(String),
}

impl std::fmt::Display for FileEraseFailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetNotFound(msg) => write!(f, "Target not found: {}", msg),
            Self::InvalidTargetType(msg) => write!(f, "Invalid target type: {}", msg),
            Self::SystemOrBootPath(msg) => write!(f, "System or boot path protected: {}", msg),
            Self::SymbolicLinkDetected(msg) => {
                write!(f, "Symbolic link or reparse point detected: {}", msg)
            }
            Self::TargetChanged(msg) => write!(f, "Target mutated since planning: {}", msg),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Self::IoError(msg) => write!(f, "I/O failure: {}", msg),
            Self::VerificationFailed(msg) => write!(f, "Post-erasure verification failed: {}", msg),
            Self::Cancelled => write!(f, "Operation cancelled cooperatively by operator"),
            Self::PartialFolderFailure(msg) => write!(f, "Partial folder erasure: {}", msg),
            Self::SecurityViolation(msg) => write!(f, "Security violation: {}", msg),
            Self::Unexpected(msg) => write!(f, "Unexpected error: {}", msg),
        }
    }
}

/// Pre-erasure evidential snapshot capturing the file/folder state on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMetadataSnapshot {
    pub path: String,
    pub canonical_path: String,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub size_bytes: u64,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub accessed_at: Option<String>,
    pub is_readonly: bool,
    pub pre_erasure_sha256: Option<String>,
    pub snapshot_timestamp: String,
}

impl FileMetadataSnapshot {
    pub fn new_empty(path: &str) -> Self {
        Self {
            path: path.to_string(),
            canonical_path: path.to_string(),
            is_directory: false,
            is_symlink: false,
            size_bytes: 0,
            created_at: None,
            modified_at: None,
            accessed_at: None,
            is_readonly: false,
            pre_erasure_sha256: None,
            snapshot_timestamp: Utc::now().to_rfc3339(),
        }
    }
}

/// Post-operation verification outcome for logical file/folder erasure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileVerificationResult {
    pub outcome: VerificationOutcome,
    pub strategy: VerificationStrategy,
    pub path_exists: bool,
    pub inaccessible: bool,
    pub details: String,
    pub verified_at: String,
}

/// Structured plan generated for a file or folder sanitization operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileErasePlan {
    pub plan_id: String,
    pub target_path: String,
    pub canonical_path: String,
    pub scope: FileEraseScope,
    pub method: SanitizationMethod,
    pub passes: usize,
    pub verification_strategy: VerificationStrategy,
    pub limitations: Vec<String>,
    pub pre_metadata: FileMetadataSnapshot,
    pub risk_level: RiskLevel,
    pub created_at: String,
    pub scope_description: String,
}

/// Statistics collected during recursive folder erasure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FolderEraseStats {
    pub total_files: usize,
    pub files_sanitized: usize,
    pub files_failed: usize,
    pub files_cancelled: usize,
    pub total_directories: usize,
    pub directories_removed: usize,
    pub bytes_sanitized: u64,
    pub failures: Vec<(String, String)>,
}

/// Detailed result of a completed or failed file/folder erasure operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEraseResult {
    pub operation_id: String,
    pub plan_id: String,
    pub target_path: String,
    pub canonical_path: String,
    pub scope: FileEraseScope,
    pub method: SanitizationMethod,
    pub status: FileEraseStatus,
    pub bytes_processed: u64,
    pub verification: FileVerificationResult,
    pub folder_stats: Option<FolderEraseStats>,
    pub failure_reason: Option<FileEraseFailureReason>,
    pub started_at: String,
    pub completed_at: String,
    pub limitations: Vec<String>,
    pub scope_description: String,
}
