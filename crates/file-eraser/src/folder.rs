use crate::models::{FileEraseFailureReason, FileMetadataSnapshot, FolderEraseStats};
use crate::sanitizer::{sanitize_file_content_and_remove, CancellationCheck};
use crate::validation::{is_symlink_or_reparse_point, normalize_canonical_path};
use locardx_verification::sanitization::SanitizationMethod;
use std::path::{Path, PathBuf};

/// Progress callback reporting for folder erasure: (files_processed, total_files, current_path).
pub type FolderProgress = Box<dyn Fn(usize, usize, &str) + Send + Sync>;

/// Discovers all files and subdirectories under a directory tree safely without escaping.
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

/// Recursively sanitizes and removes all contents of a directory, then removes the directory itself.
pub fn sanitize_folder_recursive(
    root_path: &Path,
    method: SanitizationMethod,
    is_cancelled: Option<&CancellationCheck>,
    on_progress: Option<&FolderProgress>,
) -> Result<FolderEraseStats, (FolderEraseStats, FileEraseFailureReason)> {
    let mut stats = FolderEraseStats::default();

    let (files, mut dirs) = match scan_folder_tree(root_path) {
        Ok(res) => res,
        Err(err) => return Err((stats, err)),
    };

    stats.total_files = files.len();
    stats.total_directories = dirs.len() + 1; // including root

    let total_items = stats.total_files + stats.total_directories;
    let mut items_processed = 0;

    // 1. Sanitize and remove all files
    for file_path in &files {
        // Check cancellation
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

        // If it's a symlink or reparse point, remove it directly
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

        // Regular file sanitization
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

    // 2. Remove subdirectories bottom-up (sort by path length descending)
    dirs.sort_by(|a, b| b.as_os_str().len().cmp(&a.as_os_str().len()));

    for dir_path in &dirs {
        let dir_str = dir_path.to_string_lossy().to_string();
        if let Some(cb) = on_progress {
            cb(items_processed, total_items, &dir_str);
        }

        match std::fs::remove_dir(dir_path) {
            Ok(_) => {
                stats.directories_removed += 1;
            }
            Err(e) => {
                stats.failures.push((dir_str, e.to_string()));
            }
        }
        items_processed += 1;
    }

    // 3. Remove root directory if all files succeeded
    if stats.files_failed == 0 && stats.files_cancelled == 0 {
        let root_str = root_path.to_string_lossy().to_string();
        if let Some(cb) = on_progress {
            cb(items_processed, total_items, &root_str);
        }

        match std::fs::remove_dir(root_path) {
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
        let failed_count = stats.files_failed;
        Err((
            stats,
            FileEraseFailureReason::PartialFolderFailure(format!(
                "Folder erasure incomplete: {} files failed sanitization",
                failed_count
            )),
        ))
    }
}
