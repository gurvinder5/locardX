use crate::models::{FileEraseFailureReason, FileMetadataSnapshot, FolderEraseStats};
use crate::sanitizer::{sanitize_file_content_and_remove, CancellationCheck};
use crate::validation::{is_symlink_or_reparse_point, normalize_canonical_path};
use locardx_verification::sanitization::SanitizationMethod;
use std::path::{Path, PathBuf};

/// Progress callback reporting for folder erasure: (files_processed, total_files, current_path).
pub type FolderProgress = Box<dyn Fn(usize, usize, &str) + Send + Sync>;

/// Unlinks a directory safely: clears read-only attributes, scrambles metadata, and retries on Windows transient file lock delays.
pub fn unlink_directory_safely(dir_path: &Path) -> Result<(), std::io::Error> {
    if !dir_path.exists() {
        return Ok(());
    }

    // 1. Clear readonly/system attributes if present
    #[cfg(windows)]
    {
        if let Ok(meta) = std::fs::metadata(dir_path) {
            let mut perms = meta.permissions();
            if perms.readonly() {
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(dir_path, perms);
            }
        }
    }

    // 2. Metadata Scrambling: Rename directory to random temporary UUID in parent directory
    let target_dir = if let Some(parent) = dir_path.parent() {
        let temp_name = format!(".locardx_dir_{}.tmp", uuid::Uuid::new_v4());
        let temp_path = parent.join(temp_name);
        if std::fs::rename(dir_path, &temp_path).is_ok() {
            temp_path
        } else {
            dir_path.to_path_buf()
        }
    } else {
        dir_path.to_path_buf()
    };

    // 3. Retry loop with exponential backoff on Windows
    // Windows file deletion (DeleteFileW) can take a few milliseconds for background filter drivers,
    // Search Indexer, and antivirus scanners to release directory index locks.
    let delays_ms = [10, 25, 50, 100, 200];
    let mut last_err = None;

    for (attempt, delay) in delays_ms.iter().enumerate() {
        match std::fs::remove_dir(&target_dir) {
            Ok(()) => return Ok(()),
            Err(e) => {
                let raw_code = e.raw_os_error();
                let is_transient = e.kind() == std::io::ErrorKind::PermissionDenied
                    || e.kind() == std::io::ErrorKind::DirectoryNotEmpty
                    || raw_code == Some(145)  // ERROR_DIR_NOT_EMPTY
                    || raw_code == Some(5)    // ERROR_ACCESS_DENIED
                    || raw_code == Some(32);  // ERROR_SHARING_VIOLATION

                last_err = Some(e);

                if is_transient && attempt < delays_ms.len() - 1 {
                    std::thread::sleep(std::time::Duration::from_millis(*delay));
                } else if !is_transient {
                    break;
                }
            }
        }
    }

    // Fallback: If renamed directory couldn't be removed, try removing original path if still there
    if target_dir != dir_path && dir_path.exists() {
        if let Ok(()) = std::fs::remove_dir(dir_path) {
            return Ok(());
        }
    }

    Err(last_err.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to remove directory '{}'", dir_path.display()),
        )
    }))
}

/// Discovers all files and subdirectories under a directory tree safely without escaping symlinks.
pub fn scan_folder_tree(
    root: &Path,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), FileEraseFailureReason> {
    let canonical_root = normalize_canonical_path(root)?;
    let mut files = Vec::new();
    let mut dirs = Vec::new();

    let mut stack = vec![canonical_root.clone()];

    while let Some(current_dir) = stack.pop() {
        let read_dir = std::fs::read_dir(&current_dir).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                FileEraseFailureReason::PermissionDenied(current_dir.to_string_lossy().to_string())
            }
            _ => FileEraseFailureReason::IoError(e.to_string()),
        })?;

        for entry_res in read_dir {
            let entry = entry_res.map_err(|e| {
                FileEraseFailureReason::IoError(format!("Read directory entry error: {}", e))
            })?;
            let entry_path = entry.path();

            // Guard against symlink loop or escaping outside canonical root
            if let Ok(entry_canonical) = normalize_canonical_path(&entry_path) {
                if !entry_canonical.starts_with(&canonical_root) {
                    // Symlink / junction points outside intended directory tree; do not follow!
                    continue;
                }
            }

            // Detect symlink or reparse point
            if is_symlink_or_reparse_point(&entry_path) {
                // Treat as file for direct removal; do NOT traverse into it
                files.push(entry_path);
                continue;
            }

            let file_type = entry
                .file_type()
                .map_err(|e| FileEraseFailureReason::IoError(e.to_string()))?;

            if file_type.is_dir() {
                dirs.push(entry_path.clone());
                stack.push(entry_path);
            } else {
                files.push(entry_path);
            }
        }
    }

    Ok((files, dirs))
}

/// Recursively sanitizes and removes all contents of a directory, then removes the selected directory itself.
pub fn sanitize_folder_recursive(
    root_path: &Path,
    method: SanitizationMethod,
    is_cancelled: Option<&CancellationCheck>,
    on_progress: Option<&FolderProgress>,
) -> Result<FolderEraseStats, (FolderEraseStats, FileEraseFailureReason)> {
    let mut stats = FolderEraseStats::default();

    // Clean trailing slashes/backslashes from root path
    let canonical_root = match normalize_canonical_path(root_path) {
        Ok(c) => c,
        Err(err) => return Err((stats, err)),
    };

    let (files, mut dirs) = match scan_folder_tree(&canonical_root) {
        Ok(res) => res,
        Err(err) => return Err((stats, err)),
    };

    stats.total_files = files.len();
    stats.total_directories = dirs.len() + 1; // child subdirectories + root directory itself

    let total_items = stats.total_files + stats.total_directories;
    let mut items_processed = 0;

    // 1. Sanitize and remove all files
    for file_path in &files {
        // Check cooperative cancellation
        if let Some(check) = is_cancelled {
            if check() {
                stats.files_cancelled =
                    stats.total_files - stats.files_sanitized - stats.files_failed;
                return Err((stats, FileEraseFailureReason::Cancelled));
            }
        }

        let path_str = file_path.to_string_lossy().to_string();

        if let Some(cb) = on_progress {
            cb(items_processed, total_items, &path_str);
        }

        // If it's a symlink or reparse point, remove link entry directly without traversing
        if is_symlink_or_reparse_point(file_path) {
            let removed =
                std::fs::remove_file(file_path).or_else(|_| std::fs::remove_dir(file_path));
            match removed {
                Ok(_) => {
                    stats.files_sanitized += 1;
                }
                Err(e) => {
                    stats.files_failed += 1;
                    stats.failures.push((path_str, e.to_string()));
                }
            }
            items_processed += 1;
            continue;
        }

        // Regular file sanitization: multi-pass overwrite, truncation, metadata scrambling, and unlinking
        let meta_res = std::fs::metadata(file_path);
        match meta_res {
            Ok(m) => {
                let snapshot = FileMetadataSnapshot {
                    path: path_str.clone(),
                    canonical_path: path_str.clone(),
                    is_directory: false,
                    is_symlink: false,
                    size_bytes: m.len(),
                    created_at: None,
                    modified_at: None,
                    accessed_at: None,
                    is_readonly: m.permissions().readonly(),
                    pre_erasure_sha256: None,
                    snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
                };

                match sanitize_file_content_and_remove(
                    file_path,
                    &snapshot,
                    method,
                    is_cancelled,
                    None,
                ) {
                    Ok(bytes) => {
                        stats.files_sanitized += 1;
                        stats.bytes_sanitized += bytes;
                    }
                    Err(e) => {
                        stats.files_failed += 1;
                        stats.failures.push((path_str, e.to_string()));
                    }
                }
            }
            Err(e) => {
                stats.files_failed += 1;
                stats.failures.push((path_str, e.to_string()));
            }
        }

        items_processed += 1;
    }

    // 2. Remove subdirectories bottom-up: depth-first topological sorting
    // Directories with more path components (greater depth) MUST be removed before shallower ancestors!
    dirs.sort_by(|a, b| b.components().count().cmp(&a.components().count()));

    for dir_path in &dirs {
        let dir_str = dir_path.to_string_lossy().to_string();
        if let Some(cb) = on_progress {
            cb(items_processed, total_items, &dir_str);
        }

        match unlink_directory_safely(dir_path) {
            Ok(_) => {
                stats.directories_removed += 1;
            }
            Err(e) => {
                stats.failures.push((dir_str, e.to_string()));
            }
        }
        items_processed += 1;
    }

    // 3. Remove root directory itself after all child files and subdirectories are processed
    if stats.files_failed == 0 && stats.files_cancelled == 0 && stats.failures.is_empty() {
        let root_str = canonical_root.to_string_lossy().to_string();
        if let Some(cb) = on_progress {
            cb(items_processed, total_items, &root_str);
        }

        match unlink_directory_safely(&canonical_root) {
            Ok(_) => {
                stats.directories_removed += 1;
                Ok(stats)
            }
            Err(e) => {
                stats.failures.push((root_str, e.to_string()));
                Err((
                    stats,
                    FileEraseFailureReason::PartialFolderFailure(format!(
                        "Failed to remove root directory after cleaning contents: {}",
                        e
                    )),
                ))
            }
        }
    } else {
        let failed_count = stats.files_failed + stats.failures.len();
        Err((
            stats,
            FileEraseFailureReason::PartialFolderFailure(format!(
                "Folder erasure incomplete: {} item(s) failed sanitization/removal",
                failed_count
            )),
        ))
    }
}
