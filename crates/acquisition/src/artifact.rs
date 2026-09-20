use crate::hasher::StreamingSha256Hasher;
use crate::models::{
    AcquisitionArtifact, AcquisitionFailureReason, AcquisitionResult, AcquisitionStatus,
};
use locardx_common::LocardError;
use std::fs::File;
use std::io::Read;
use std::path::Path;

impl AcquisitionArtifact {
    /// Constructs a verified evidential artifact from a completed acquisition result.
    /// INVARIANT: Incomplete, failed, or unverified acquisitions strictly cannot produce an artifact.
    pub fn from_result(result: &AcquisitionResult) -> Result<Self, AcquisitionFailureReason> {
        if result.status != AcquisitionStatus::Completed {
            return Err(AcquisitionFailureReason::Unknown(format!(
                "Cannot create AcquisitionArtifact from non-completed acquisition (status: {})",
                result.status
            )));
        }

        if result.image_size_bytes == 0 {
            return Err(AcquisitionFailureReason::Unknown(
                "Acquired image size is 0 bytes".to_string(),
            ));
        }

        if result.image_sha256.is_empty() || result.image_sha256.len() != 64 {
            return Err(AcquisitionFailureReason::Unknown(
                "Invalid or missing SHA-256 hash in acquisition result".to_string(),
            ));
        }

        let p = Path::new(&result.destination_path);
        if !p.exists() {
            return Err(AcquisitionFailureReason::Unknown(format!(
                "Destination image file '{}' does not exist on disk",
                result.destination_path
            )));
        }

        Ok(Self {
            acquisition_id: result.acquisition_id.clone(),
            image_path: result.destination_path.clone(),
            image_format: result.image_format.clone(),
            image_size_bytes: result.image_size_bytes,
            image_sha256: result.image_sha256.clone(),
            source_device_snapshot: result.source.clone(),
            acquisition_timestamp: result.completed_at.clone(),
            is_verified: true,
            audit_reference: result.audit_reference.clone(),
        })
    }

    /// Verifies that the physical image on disk still matches the recorded SHA-256 hash.
    pub fn verify_integrity(&self) -> Result<bool, LocardError> {
        let p = Path::new(&self.image_path);
        if !p.exists() {
            return Ok(false);
        }

        let mut file = File::open(p).map_err(|e| {
            LocardError::Operation(format!("Failed to open image for integrity check: {}", e))
        })?;

        let mut hasher = StreamingSha256Hasher::new();
        let mut buffer = [0u8; 1024 * 1024];

        loop {
            let n = file.read(&mut buffer).map_err(|e| {
                LocardError::Operation(format!(
                    "Read failure during integrity re-verification: {}",
                    e
                ))
            })?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let (computed_hash, bytes) = hasher.finalize();
        Ok(
            bytes == self.image_size_bytes
                && computed_hash.eq_ignore_ascii_case(&self.image_sha256),
        )
    }
}
