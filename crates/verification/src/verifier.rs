use crate::hasher::StreamHasher;
use crate::models::{
    HashAlgorithm, TargetIdentity, TargetType, VerificationResult, VerificationStatus,
};
use chrono::Utc;
use std::path::Path;

pub struct HashVerifier;

impl HashVerifier {
    /// Validates whether a string represents a syntactically valid hex digest for the given algorithm.
    pub fn validate_digest_syntax(
        expected: &str,
        algorithm: HashAlgorithm,
    ) -> Result<String, String> {
        let cleaned = expected.trim().to_lowercase();
        let expected_len = match algorithm {
            HashAlgorithm::Sha256 => 64,
        };

        if cleaned.len() != expected_len {
            return Err(format!(
                "Invalid {} hash length: expected {} hex characters, found {}",
                algorithm,
                expected_len,
                cleaned.len()
            ));
        }

        if !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Hash digest contains invalid non-hexadecimal characters".to_string());
        }

        Ok(cleaned)
    }

    /// Compares two hex strings in constant time.
    pub fn constant_time_compare(a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut diff = 0u8;
        for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
            diff |= byte_a ^ byte_b;
        }

        diff == 0
    }

    /// Verifies the cryptographic content integrity of a file against an expected hash value.
    pub fn verify_file<P: AsRef<Path>>(
        path: P,
        expected_digest: &str,
        algorithm: HashAlgorithm,
    ) -> VerificationResult {
        let path_ref = path.as_ref();
        let start_instant = std::time::Instant::now();
        let timestamp = Utc::now().to_rfc3339();

        let normalized_path = path_ref
            .canonicalize()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path_ref.to_string_lossy().to_string());

        let display_name = path_ref
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown_file".to_string());

        let target_identity = TargetIdentity {
            target_type: TargetType::File,
            identifier: normalized_path,
            display_name,
            size_bytes: std::fs::metadata(path_ref).ok().map(|m| m.len()),
        };

        // 1. Validate syntax of expected digest
        let cleaned_expected = match Self::validate_digest_syntax(expected_digest, algorithm) {
            Ok(v) => v,
            Err(reason) => {
                return VerificationResult {
                    target: target_identity,
                    algorithm,
                    expected_digest: expected_digest.trim().to_string(),
                    calculated_digest: None,
                    bytes_processed: 0,
                    status: VerificationStatus::UnableToVerify { reason },
                    timestamp,
                    duration_ms: start_instant.elapsed().as_millis() as u64,
                };
            }
        };

        // 2. Stream and compute hash from target
        match StreamHasher::hash_file(path_ref, algorithm) {
            Ok(hash_result) => {
                let is_match = Self::constant_time_compare(&hash_result.digest, &cleaned_expected);
                let status = if is_match {
                    VerificationStatus::Verified
                } else {
                    VerificationStatus::Mismatch {
                        expected: cleaned_expected.clone(),
                        calculated: hash_result.digest.clone(),
                    }
                };

                VerificationResult {
                    target: hash_result.target,
                    algorithm,
                    expected_digest: cleaned_expected,
                    calculated_digest: Some(hash_result.digest),
                    bytes_processed: hash_result.bytes_processed,
                    status,
                    timestamp,
                    duration_ms: start_instant.elapsed().as_millis() as u64,
                }
            }
            Err(err) => VerificationResult {
                target: target_identity,
                algorithm,
                expected_digest: cleaned_expected,
                calculated_digest: None,
                bytes_processed: 0,
                status: VerificationStatus::UnableToVerify {
                    reason: err.to_string(),
                },
                timestamp,
                duration_ms: start_instant.elapsed().as_millis() as u64,
            },
        }
    }
}
