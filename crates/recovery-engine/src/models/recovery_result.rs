use super::recovered_file::{FileType, RecoveredFile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Operational mode directing which recovery techniques are executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryMode {
    All,
    FilesystemOnly,
    CarvingOnly,
    Custom,
}

impl fmt::Display for RecoveryMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "All (Filesystem + Carving)"),
            Self::FilesystemOnly => write!(f, "Filesystem Metadata Only"),
            Self::CarvingOnly => write!(f, "Raw Carving Only"),
            Self::Custom => write!(f, "Custom Selective"),
        }
    }
}

/// Lifecycle status of a recovery job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStatus {
    Pending,
    Validating,
    Scanning,
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for RecoveryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Validating => write!(f, "Validating Image"),
            Self::Scanning => write!(f, "Scanning & Carving"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Comprehensive failure reasons adhering strictly to fail-closed error handling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "details", rename_all = "snake_case")]
pub enum RecoveryFailureReason {
    MissingArtifact(String),
    ImageNotFound(String),
    InvalidFormat(String),
    SizeMismatch { expected: u64, actual: u64 },
    HashMismatch { expected: String, computed: String },
    IncompleteAcquisition(String),
    ReadError(String),
    InsufficientSpace { required: u64, available: u64 },
    Cancelled,
    Interrupted,
    CorruptImage(String),
    Unknown(String),
}

impl fmt::Display for RecoveryFailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArtifact(msg) => write!(f, "Missing AcquisitionArtifact: {}", msg),
            Self::ImageNotFound(msg) => write!(f, "Evidence image not found: {}", msg),
            Self::InvalidFormat(msg) => write!(f, "Invalid image format: {}", msg),
            Self::SizeMismatch { expected, actual } => {
                write!(
                    f,
                    "Image size mismatch: expected {} bytes, found {}",
                    expected, actual
                )
            }
            Self::HashMismatch { expected, computed } => {
                write!(
                    f,
                    "Forensic image SHA-256 hash mismatch: expected {}, computed {}",
                    expected, computed
                )
            }
            Self::IncompleteAcquisition(msg) => {
                write!(f, "Incomplete acquisition rejected: {}", msg)
            }
            Self::ReadError(msg) => write!(f, "Read I/O failure on evidence image: {}", msg),
            Self::InsufficientSpace {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient output disk space: required {} bytes, available {}",
                    required, available
                )
            }
            Self::Cancelled => write!(f, "Recovery job was cancelled by operator"),
            Self::Interrupted => write!(f, "Recovery job was interrupted by abnormal termination"),
            Self::CorruptImage(msg) => write!(f, "Evidence image corruption detected: {}", msg),
            Self::Unknown(msg) => write!(f, "Recovery failure: {}", msg),
        }
    }
}

/// User options configuring recovery execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryOptions {
    pub recovery_mode: RecoveryMode,
    pub target_file_types: Option<Vec<FileType>>,
    pub output_directory: String,
    pub enable_fragment_reconstruction: bool,
    pub min_confidence_score: u32,
    pub chunk_size_bytes: usize,
}

impl Default for RecoveryOptions {
    fn default() -> Self {
        Self {
            recovery_mode: RecoveryMode::All,
            target_file_types: None,
            output_directory: String::new(),
            enable_fragment_reconstruction: true,
            min_confidence_score: 40,
            chunk_size_bytes: 1024 * 1024,
        }
    }
}

/// Pre-flight recovery plan specifying the target image and execution parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryPlan {
    pub plan_id: String,
    pub acquisition_id: String,
    pub source_image_path: String,
    pub source_image_sha256: String,
    pub options: RecoveryOptions,
    pub created_at: String,
}

/// Real-time progress telemetry emitted during recovery scanning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryProgress {
    pub operation_id: String,
    pub bytes_scanned: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub throughput_mbps: f64,
    pub elapsed_seconds: f64,
    pub files_found: usize,
    pub stage: String,
}

/// Comprehensive outcome of an executed recovery operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryResult {
    pub job_id: String,
    pub operation_id: String,
    pub acquisition_id: String,
    pub source_image_path: String,
    pub source_image_sha256: String,
    pub status: RecoveryStatus,
    pub bytes_scanned: u64,
    pub files_recovered: usize,
    pub candidates_evaluated: usize,
    pub elapsed_seconds: f64,
    pub failure_reason: Option<RecoveryFailureReason>,
    pub recovered_files: Vec<RecoveredFile>,
    pub audit_reference: String,
    pub started_at: String,
    pub completed_at: String,
}

/// Immutable forensic recovery summary report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryReport {
    pub report_id: String,
    pub job_id: String,
    pub acquisition_id: String,
    pub source_image_sha256: String,
    pub total_files_recovered: usize,
    pub category_counts: HashMap<String, usize>,
    pub average_confidence: f64,
    pub report_digest: String,
    pub audit_reference: String,
    pub generated_at: String,
}
