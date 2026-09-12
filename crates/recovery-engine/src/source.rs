use crate::models::RecoveryFailureReason;
use chrono::Utc;
use locardx_acquisition::AcquisitionArtifact;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use uuid::Uuid;

/// Immutable forensic snapshot of a verified recovery evidence source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverySourceSnapshot {
    pub source_id: String,
    pub acquisition_id: String,
    pub image_path: String,
    pub image_size_bytes: u64,
    pub image_sha256: String,
    pub original_device_id: String,
    pub original_serial: Option<String>,
    pub verified_at: String,
    pub is_trusted: bool,
}

/// Validates an AcquisitionArtifact strictly against the filesystem and computes streaming SHA-256.
/// Enforces fail-closed semantics: any mismatch immediately rejects the artifact.
pub fn validate_acquisition_artifact(
    artifact: &AcquisitionArtifact,
) -> Result<RecoverySourceSnapshot, RecoveryFailureReason> {
    if !artifact.is_verified {
        return Err(RecoveryFailureReason::IncompleteAcquisition(format!(
            "Artifact {} is not marked as verified",
            artifact.acquisition_id
        )));
    }

    let fmt = artifact.image_format.to_lowercase();
    if fmt != "raw" && fmt != "dd" {
        return Err(RecoveryFailureReason::InvalidFormat(format!(
            "Unsupported image format '{}'; only 'raw' or 'dd' are accepted",
            artifact.image_format
        )));
    }

    let path = Path::new(&artifact.image_path);
    if !path.exists() {
        return Err(RecoveryFailureReason::ImageNotFound(format!(
            "Image file not found at path: {}",
            artifact.image_path
        )));
    }

    let metadata = std::fs::metadata(path).map_err(|e| {
        RecoveryFailureReason::ReadError(format!(
            "Failed to read metadata for {}: {}",
            artifact.image_path, e
        ))
    })?;

    let actual_size = metadata.len();
    if actual_size != artifact.image_size_bytes {
        return Err(RecoveryFailureReason::SizeMismatch {
            expected: artifact.image_size_bytes,
            actual: actual_size,
        });
    }

    // Stream SHA-256 validation
    let computed_hash = compute_streaming_sha256(path)?;
    if !computed_hash.eq_ignore_ascii_case(&artifact.image_sha256) {
        return Err(RecoveryFailureReason::HashMismatch {
            expected: artifact.image_sha256.clone(),
            computed: computed_hash,
        });
    }

    Ok(RecoverySourceSnapshot {
        source_id: format!("src-rec-{}", Uuid::new_v4()),
        acquisition_id: artifact.acquisition_id.clone(),
        image_path: artifact.image_path.clone(),
        image_size_bytes: actual_size,
        image_sha256: computed_hash,
        original_device_id: artifact.source_device_snapshot.device_id.clone(),
        original_serial: artifact.source_device_snapshot.serial_number.clone(),
        verified_at: Utc::now().to_rfc3339(),
        is_trusted: true,
    })
}

/// Streams SHA-256 over an image file using 1 MiB chunks.
pub fn compute_streaming_sha256(path: &Path) -> Result<String, RecoveryFailureReason> {
    let file = File::open(path).map_err(|e| {
        RecoveryFailureReason::ReadError(format!("Failed to open image {}: {}", path.display(), e))
    })?;

    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                hasher.update(&buffer[..n]);
            }
            Err(e) => {
                return Err(RecoveryFailureReason::ReadError(format!(
                    "Read error while computing SHA-256 for {}: {}",
                    path.display(),
                    e
                )));
            }
        }
    }

    let hash_bytes = hasher.finalize();
    Ok(hex::encode(hash_bytes))
}
