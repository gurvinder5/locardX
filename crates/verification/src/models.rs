use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported cryptographic hash algorithms.
/// Initial support: SHA-256 (FIPS 180-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashAlgorithm {
    Sha256,
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sha256 => write!(f, "SHA-256"),
        }
    }
}

pub use locardx_common::{TargetIdentity, TargetType};

/// Execution status of a hash calculation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashStatus {
    Success,
    Error(String),
}

/// Result of a cryptographic hash calculation.
/// Sanitized model safe for Tauri IPC transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashResult {
    pub target: TargetIdentity,
    pub algorithm: HashAlgorithm,
    pub digest: String,
    pub bytes_processed: u64,
    pub started_at: String,
    pub completed_at: String,
    pub duration_ms: u64,
    pub status: HashStatus,
}

/// Outcome of cryptographic hash comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    Verified,
    Mismatch {
        expected: String,
        calculated: String,
    },
    UnableToVerify {
        reason: String,
    },
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Verified => write!(f, "VERIFIED"),
            Self::Mismatch { .. } => write!(f, "MISMATCH"),
            Self::UnableToVerify { reason } => write!(f, "UNABLE TO VERIFY: {}", reason),
        }
    }
}

/// Full verification result comparing expected vs calculated digests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResult {
    pub target: TargetIdentity,
    pub algorithm: HashAlgorithm,
    pub expected_digest: String,
    pub calculated_digest: Option<String>,
    pub bytes_processed: u64,
    pub status: VerificationStatus,
    pub timestamp: String,
    pub duration_ms: u64,
}

/// Persistent record stored in SQLite `integrity_records` table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrityRecord {
    pub record_id: String,
    pub operation_id: String,
    pub target_type: TargetType,
    pub target_path: String,
    pub target_name: String,
    pub algorithm: HashAlgorithm,
    pub digest: String,
    pub bytes_processed: u64,
    pub verification_status: String,
    pub expected_digest: Option<String>,
    pub actor_id: Option<String>,
    pub created_at: String,
}
