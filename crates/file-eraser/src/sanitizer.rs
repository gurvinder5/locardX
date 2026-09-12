use crate::models::{FileEraseFailureReason, FileMetadataSnapshot};
use locardx_verification::sanitization::SanitizationMethod;
use rand::RngCore;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

const CHUNK_SIZE: usize = 65536; // 64 KB bounded streaming memory

/// Cooperative cancellation check callback.
pub type CancellationCheck = Box<dyn Fn() -> bool + Send + Sync>;

/// Progress callback reporting (bytes_in_pass, total_bytes, current_pass, total_passes).
pub type SanitizerProgress = Box<dyn Fn(u64, u64, usize, usize) + Send + Sync>;

/// Executes the multi-pass content overwrite and metadata scrambling for a single file.
pub fn sanitize_file_content_and_remove(
    path: &Path,
    metadata: &FileMetadataSnapshot,
    method: SanitizationMethod,
    is_cancelled: Option<&CancellationCheck>,
    on_progress: Option<&SanitizerProgress>,
) -> Result<u64, FileEraseFailureReason> {
    if !path.exists() {
        return Err(FileEraseFailureReason::TargetNotFound(
            path.to_string_lossy().to_string(),
        ));
    }

    // 1. If file is marked read-only, remove read-only attribute
    if metadata.is_readonly {
        if let Ok(meta) = std::fs::metadata(path) {
            let mut perms = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(path, perms);
        }
    }

    let file_len = metadata.size_bytes;

    // Determine passes and byte patterns
    let passes: Vec<Option<u8>> = match method {
        SanitizationMethod::LogicalFileShred | SanitizationMethod::DirectoryRecursiveShred => {
            vec![Some(0x00), Some(0xFF), None] // Pass 1: 0x00, Pass 2: 0xFF, Pass 3: Random
        }
        SanitizationMethod::Nist80088ClearZero => {
            vec![Some(0x00)] // Pass 1: 0x00
        }
        _ => {
            return Err(FileEraseFailureReason::Unexpected(format!(
                "Method {:?} is not supported for logical file erasure",
                method
            )));
        }
    };

    let total_passes = passes.len();
    let mut total_bytes_written: u64 = 0;

    // If file has non-zero size, execute content overwrite passes
    if file_len > 0 {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    FileEraseFailureReason::PermissionDenied(path.to_string_lossy().to_string())
                }
                _ => FileEraseFailureReason::IoError(format!("Failed to open for write: {}", e)),
            })?;

        let mut buffer = [0u8; CHUNK_SIZE];
        let mut rng = rand::thread_rng();

        for (pass_idx, pattern) in passes.iter().enumerate() {
            let current_pass = pass_idx + 1;

            // Reset file pointer to beginning
            file.seek(SeekFrom::Start(0)).map_err(|e| {
                FileEraseFailureReason::IoError(format!(
                    "Seek error on pass {}: {}",
                    current_pass, e
                ))
            })?;

            let mut pass_bytes_written: u64 = 0;

            while pass_bytes_written < file_len {
                // Check cancellation
                if let Some(check) = is_cancelled {
                    if check() {
                        return Err(FileEraseFailureReason::Cancelled);
                    }
                }

                let remaining = file_len - pass_bytes_written;
                let chunk_len = remaining.min(CHUNK_SIZE as u64) as usize;

                match pattern {
                    Some(b) => {
                        buffer[..chunk_len].fill(*b);
                    }
                    None => {
                        rng.fill_bytes(&mut buffer[..chunk_len]);
                    }
                }

                file.write_all(&buffer[..chunk_len]).map_err(|e| {
                    FileEraseFailureReason::IoError(format!(
                        "Write error on pass {} at offset {}: {}",
                        current_pass, pass_bytes_written, e
                    ))
                })?;

                pass_bytes_written += chunk_len as u64;
                total_bytes_written += chunk_len as u64;

                if let Some(cb) = on_progress {
                    cb(pass_bytes_written, file_len, current_pass, total_passes);
                }
            }

            // Flush and sync to disk after each pass
            file.flush().map_err(|e| {
                FileEraseFailureReason::IoError(format!(
                    "Flush failed on pass {}: {}",
                    current_pass, e
                ))
            })?;
            file.sync_data().map_err(|e| {
                FileEraseFailureReason::IoError(format!(
                    "Sync data failed on pass {}: {}",
                    current_pass, e
                ))
            })?;
        }

        // Truncate file to 0 bytes
        file.set_len(0).map_err(|e| {
            FileEraseFailureReason::IoError(format!("Failed to truncate file: {}", e))
        })?;
        file.flush().map_err(|e| {
            FileEraseFailureReason::IoError(format!("Flush failed after truncate: {}", e))
        })?;
        file.sync_all().map_err(|e| {
            FileEraseFailureReason::IoError(format!("Sync all failed after truncate: {}", e))
        })?;

        drop(file);
    }

    // 2. Metadata Scrambling: Rename to random temporary UUID in same parent directory
    let target_for_removal = if let Some(parent) = path.parent() {
        let temp_name = format!(".locardx_{}.tmp", uuid::Uuid::new_v4());
        let temp_path = parent.join(temp_name);
        if std::fs::rename(path, &temp_path).is_ok() {
            temp_path
        } else {
            path.to_path_buf()
        }
    } else {
        path.to_path_buf()
    };

    // 3. Filesystem Entry Removal
    std::fs::remove_file(&target_for_removal).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => {
            FileEraseFailureReason::PermissionDenied(path.to_string_lossy().to_string())
        }
        _ => FileEraseFailureReason::IoError(format!("Failed to remove file entry: {}", e)),
    })?;

    Ok(total_bytes_written)
}
