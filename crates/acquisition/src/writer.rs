use crate::models::{AcquisitionFailureReason, ArtifactCleanupStatus};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Sequential raw / DD bitstream forensic image writer.
///
/// Bytes are written sequentially to a staging artifact (`<final>.partial`)
/// using direct unbuffered file I/O to avoid redundant double-buffering when writing
/// large chunk buffers (512 KiB - 32 MiB). The artifact is only promoted to the final image
/// path after a successful flush, sync_all, and cryptographic integrity check.
pub struct RawImageWriter {
    final_path: PathBuf,
    staging_path: PathBuf,
    file: Option<File>,
    bytes_written: u64,
    write_fail_after: Option<u64>,
}

/// Filesystem-verified result of attempting to remove a partial acquisition artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactCleanupResult {
    pub status: ArtifactCleanupStatus,
    pub path: PathBuf,
    pub error: Option<String>,
}

impl ArtifactCleanupResult {
    pub fn confirmed(path: PathBuf) -> Self {
        Self {
            status: ArtifactCleanupStatus::CleanupConfirmed,
            path,
            error: None,
        }
    }

    pub fn failed(path: PathBuf, error: String) -> Self {
        Self {
            status: ArtifactCleanupStatus::CleanupFailed,
            path,
            error: Some(error),
        }
    }

    pub fn is_confirmed(&self) -> bool {
        self.status == ArtifactCleanupStatus::CleanupConfirmed
    }
}

/// Staging path used while an image is still being written / unverified.
pub fn staging_path_for(final_destination: &Path) -> PathBuf {
    let mut os = final_destination.as_os_str().to_os_string();
    os.push(".partial");
    PathBuf::from(os)
}

impl RawImageWriter {
    /// Creates a new image writer targeting a `.partial` staging file for `destination_path`.
    pub fn create<P: AsRef<Path>>(destination_path: P) -> Result<Self, AcquisitionFailureReason> {
        Self::create_with_write_limit(destination_path, None)
    }

    /// Test/diagnostic helper: fail `write_chunk` after `write_fail_after` bytes have been accepted.
    pub fn create_with_write_limit<P: AsRef<Path>>(
        destination_path: P,
        write_fail_after: Option<u64>,
    ) -> Result<Self, AcquisitionFailureReason> {
        let final_path = destination_path.as_ref().to_path_buf();
        let staging_path = staging_path_for(&final_path);

        if staging_path.exists() {
            std::fs::remove_file(&staging_path).map_err(|e| {
                AcquisitionFailureReason::WriteError(format!(
                    "Failed to remove leftover staging file '{}': {}",
                    staging_path.display(),
                    e
                ))
            })?;
            if staging_path.exists() {
                return Err(AcquisitionFailureReason::WriteError(format!(
                    "Leftover staging file '{}' still exists after delete",
                    staging_path.display()
                )));
            }
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&staging_path)
            .map_err(|e| {
                AcquisitionFailureReason::WriteError(format!(
                    "Failed to create destination staging file '{}': {}",
                    staging_path.display(),
                    e
                ))
            })?;

        Ok(Self {
            final_path,
            staging_path,
            file: Some(file),
            bytes_written: 0,
            write_fail_after,
        })
    }

    /// Writes a sequential block of bytes to the evidence image.
    pub fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), AcquisitionFailureReason> {
        if let Some(limit) = self.write_fail_after {
            if self.bytes_written.saturating_add(chunk.len() as u64) > limit {
                return Err(AcquisitionFailureReason::WriteError(format!(
                    "Injected write failure after {} bytes (limit {})",
                    self.bytes_written, limit
                )));
            }
        }

        let file = self.file.as_mut().ok_or_else(|| {
            AcquisitionFailureReason::WriteError("Image writer is closed".to_string())
        })?;

        file.write_all(chunk).map_err(|e| {
            AcquisitionFailureReason::WriteError(format!(
                "Failed writing {} bytes at offset {} in destination image: {}",
                chunk.len(),
                self.bytes_written,
                e
            ))
        })?;

        self.bytes_written += chunk.len() as u64;
        Ok(())
    }

    /// Flushes all user-space buffers and syncs file contents and metadata to disk, then closes the handle.
    pub fn flush_and_sync(&mut self) -> Result<u64, AcquisitionFailureReason> {
        let file = self.file.take().ok_or_else(|| {
            AcquisitionFailureReason::WriteError("Image writer is already closed".to_string())
        })?;

        file.sync_all().map_err(|e| {
            AcquisitionFailureReason::WriteError(format!(
                "Failed to sync destination image to disk: {}",
                e
            ))
        })?;

        drop(file);
        Ok(self.bytes_written)
    }

    /// Promotes the staging artifact to the final image path. The writer handle must already be closed.
    pub fn finalize_to_destination(self) -> Result<(u64, PathBuf), AcquisitionFailureReason> {
        if self.file.is_some() {
            return Err(AcquisitionFailureReason::WriteError(
                "Cannot finalize destination while the image writer is still open".to_string(),
            ));
        }

        let staging = self.staging_path.clone();
        let final_path = self.final_path.clone();
        let written = self.bytes_written;

        if staging != final_path {
            if final_path.exists() {
                std::fs::remove_file(&final_path).map_err(|e| {
                    AcquisitionFailureReason::WriteError(format!(
                        "Failed replacing existing destination '{}': {}",
                        final_path.display(),
                        e
                    ))
                })?;
            }

            std::fs::rename(&staging, &final_path).map_err(|e| {
                AcquisitionFailureReason::WriteError(format!(
                    "Failed promoting staging image '{}' to '{}': {}",
                    staging.display(),
                    final_path.display(),
                    e
                ))
            })?;
        }

        if !final_path.exists() || !final_path.is_file() {
            return Err(AcquisitionFailureReason::WriteError(format!(
                "Final image '{}' is missing after promotion",
                final_path.display()
            )));
        }

        if staging != final_path && staging.exists() {
            return Err(AcquisitionFailureReason::WriteError(format!(
                "Staging file '{}' still exists after rename to '{}'",
                staging.display(),
                final_path.display()
            )));
        }

        Ok((written, final_path))
    }

    /// Closes the output handle, then attempts to delete the staging artifact and **verifies** the result.
    #[must_use]
    pub fn abort_and_cleanup(mut self) -> ArtifactCleanupResult {
        self.close_output();
        delete_artifact_verified(&self.staging_path)
    }

    fn close_output(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.sync_all();
            drop(file);
        }
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    pub fn destination_path(&self) -> &Path {
        &self.final_path
    }

    pub fn staging_path(&self) -> &Path {
        &self.staging_path
    }
}

/// Deletes `path` and reports CLEANUP_CONFIRMED only if the path no longer exists afterwards.
pub fn delete_artifact_verified(path: &Path) -> ArtifactCleanupResult {
    if !path.exists() {
        return ArtifactCleanupResult::confirmed(path.to_path_buf());
    }

    if !path.is_file() {
        return ArtifactCleanupResult::failed(
            path.to_path_buf(),
            format!(
                "CLEANUP_FAILED: path '{}' exists but is not a regular file",
                path.display()
            ),
        );
    }

    let delete_result = std::fs::remove_file(path);
    // Authoritative check: filesystem state, not the return value of remove_file alone.
    if !path.exists() {
        return ArtifactCleanupResult::confirmed(path.to_path_buf());
    }

    let os_error = match delete_result {
        Err(e) => e.to_string(),
        Ok(()) => "remove_file returned success but the path still exists".to_string(),
    };

    ArtifactCleanupResult::failed(
        path.to_path_buf(),
        format!(
            "CLEANUP_FAILED: partial artifact still exists at '{}': {}",
            path.display(),
            os_error
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_path_appends_partial_without_replacing_extension() {
        let p = PathBuf::from("evidence.raw");
        assert_eq!(staging_path_for(&p), PathBuf::from("evidence.raw.partial"));
    }

    #[test]
    fn abort_confirms_deletion_when_file_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("gone.raw");
        let mut w = RawImageWriter::create(&dest).unwrap();
        w.write_chunk(b"partial").unwrap();
        let staging = w.staging_path().to_path_buf();
        assert!(staging.exists());
        let cleanup = w.abort_and_cleanup();
        assert_eq!(cleanup.status, ArtifactCleanupStatus::CleanupConfirmed);
        assert!(!staging.exists());
        assert!(!dest.exists());
    }

    #[test]
    #[allow(clippy::permissions_set_readonly_false)]
    fn abort_reports_cleanup_failed_when_deletion_is_prevented() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("locked.raw");
        let mut w = RawImageWriter::create(&dest).unwrap();
        w.write_chunk(b"partial-image-bytes").unwrap();
        let staging = w.staging_path().to_path_buf();

        #[cfg(windows)]
        {
            let mut perms = std::fs::metadata(&staging).unwrap().permissions();
            perms.set_readonly(true);
            std::fs::set_permissions(&staging, perms).unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(dir.path()).unwrap().permissions();
            perms.set_mode(0o555);
            std::fs::set_permissions(dir.path(), perms).unwrap();
        }

        let cleanup = w.abort_and_cleanup();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(dir.path()).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(dir.path(), perms).unwrap();
        }

        if staging.exists() {
            assert_eq!(cleanup.status, ArtifactCleanupStatus::CleanupFailed);
            assert!(cleanup
                .error
                .as_deref()
                .unwrap_or("")
                .contains("CLEANUP_FAILED"));
            #[cfg(windows)]
            {
                let mut perms = std::fs::metadata(&staging).unwrap().permissions();
                perms.set_readonly(false);
                std::fs::set_permissions(&staging, perms).unwrap();
            }
            let _ = std::fs::remove_file(&staging);
        } else {
            // Some platforms allow deleting a read-only file; the invariant is:
            // confirmed <=> file is gone, failed <=> file remains.
            assert_eq!(cleanup.status, ArtifactCleanupStatus::CleanupConfirmed);
        }
    }
}
