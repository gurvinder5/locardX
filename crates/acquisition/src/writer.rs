use crate::models::AcquisitionFailureReason;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Sequential raw / DD bitstream forensic image writer.
pub struct RawImageWriter {
    path: PathBuf,
    writer: Option<BufWriter<File>>,
    bytes_written: u64,
}

impl RawImageWriter {
    /// Creates a new image writer targeting `destination_path`.
    pub fn create<P: AsRef<Path>>(destination_path: P) -> Result<Self, AcquisitionFailureReason> {
        let path = destination_path.as_ref().to_path_buf();

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|e| {
                AcquisitionFailureReason::WriteError(format!(
                    "Failed to create destination image file '{}': {}",
                    path.display(),
                    e
                ))
            })?;

        // 1 MiB write buffer for high-throughput sequential writing
        let writer = BufWriter::with_capacity(1024 * 1024, file);

        Ok(Self {
            path,
            writer: Some(writer),
            bytes_written: 0,
        })
    }

    /// Writes a sequential block of bytes to the evidence image.
    pub fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), AcquisitionFailureReason> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            AcquisitionFailureReason::WriteError("Image writer is closed".to_string())
        })?;

        writer.write_all(chunk).map_err(|e| {
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

    /// Flushes all user-space buffers and syncs file contents and metadata to disk.
    pub fn flush_and_sync(&mut self) -> Result<u64, AcquisitionFailureReason> {
        let mut writer = self.writer.take().ok_or_else(|| {
            AcquisitionFailureReason::WriteError("Image writer is already closed".to_string())
        })?;

        writer.flush().map_err(|e| {
            AcquisitionFailureReason::WriteError(format!(
                "Failed to flush destination image: {}",
                e
            ))
        })?;

        let file = writer.into_inner().map_err(|e| {
            AcquisitionFailureReason::WriteError(format!("Buffer flush failure: {}", e))
        })?;

        file.sync_all().map_err(|e| {
            AcquisitionFailureReason::WriteError(format!(
                "Failed to sync_all destination image to disk: {}",
                e
            ))
        })?;

        Ok(self.bytes_written)
    }

    /// Aborts writing and cleans up/removes the incomplete image file so it cannot be treated as valid evidence.
    pub fn abort_and_cleanup(mut self) {
        // Drop open file handle first
        self.writer = None;
        if self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    pub fn destination_path(&self) -> &Path {
        &self.path
    }
}
