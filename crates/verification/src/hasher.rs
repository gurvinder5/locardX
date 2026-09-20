use crate::models::{HashAlgorithm, HashResult, HashStatus, TargetIdentity, TargetType};
use chrono::Utc;
use locardx_common::LocardError;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Bounded buffer size for streaming file hashing (64 KiB).
pub const STREAM_CHUNK_SIZE: usize = 64 * 1024;

pub struct StreamHasher;

impl StreamHasher {
    /// Hashes an open reader using the requested algorithm.
    /// Memory consumption is bounded by `STREAM_CHUNK_SIZE`.
    pub fn hash_reader<R: Read>(
        mut reader: R,
        algorithm: HashAlgorithm,
    ) -> Result<(String, u64), LocardError> {
        match algorithm {
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                let mut buffer = [0u8; STREAM_CHUNK_SIZE];
                let mut total_bytes: u64 = 0;

                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            hasher.update(&buffer[..n]);
                            total_bytes = total_bytes.saturating_add(n as u64);
                        }
                        Err(e) => {
                            return Err(LocardError::Operation(format!(
                                "I/O error during streaming hash calculation: {}",
                                e
                            )));
                        }
                    }
                }

                let digest = hex::encode(hasher.finalize());
                Ok((digest, total_bytes))
            }
        }
    }

    /// Computes the cryptographic hash of a file on disk strictly in read-only mode.
    pub fn hash_file<P: AsRef<Path>>(
        path: P,
        algorithm: HashAlgorithm,
    ) -> Result<HashResult, LocardError> {
        let path_ref = path.as_ref();
        let started_at = Utc::now();
        let start_instant = std::time::Instant::now();

        // 1. Validate target existence and type
        if !path_ref.exists() {
            return Err(LocardError::Operation(format!(
                "Target file does not exist: {}",
                path_ref.display()
            )));
        }

        let metadata = std::fs::metadata(path_ref).map_err(|e| {
            LocardError::Operation(format!(
                "Failed to inspect target file metadata ({}): {}",
                path_ref.display(),
                e
            ))
        })?;

        if metadata.is_dir() {
            return Err(LocardError::Operation(format!(
                "Target is a directory, not a regular file: {}. Directory hashing requires a canonical Merkle manifest.",
                path_ref.display()
            )));
        }

        // 2. Normalize target identity
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
            size_bytes: Some(metadata.len()),
        };

        // 3. Open target strictly read-only
        let file = File::open(path_ref).map_err(|e| {
            LocardError::Operation(format!(
                "Failed to open target file for reading ({}): {}",
                path_ref.display(),
                e
            ))
        })?;

        // 4. Stream hash chunks
        let (digest, bytes_processed) = Self::hash_reader(file, algorithm)?;
        let completed_at = Utc::now();
        let duration_ms = start_instant.elapsed().as_millis() as u64;

        Ok(HashResult {
            target: target_identity,
            algorithm,
            digest,
            bytes_processed,
            started_at: started_at.to_rfc3339(),
            completed_at: completed_at.to_rfc3339(),
            duration_ms,
            status: HashStatus::Success,
        })
    }
}
