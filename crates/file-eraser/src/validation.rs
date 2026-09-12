use crate::models::{FileEraseFailureReason, FileMetadataSnapshot};
use chrono::{DateTime, Utc};
use locardx_security::SafetyEngine;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Normalizes and canonicalizes a path, stripping the Windows extended prefix (`\\?\`) for display consistency.
pub fn normalize_canonical_path(path: &Path) -> Result<PathBuf, FileEraseFailureReason> {
    let canonical = path.canonicalize().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            FileEraseFailureReason::TargetNotFound(path.to_string_lossy().to_string())
        }
        std::io::ErrorKind::PermissionDenied => {
            FileEraseFailureReason::PermissionDenied(path.to_string_lossy().to_string())
        }
        _ => FileEraseFailureReason::IoError(e.to_string()),
    })?;

    // On Windows, canonicalize() prefixes local paths with \\?\ which can cause issues with some tooling.
    // Clean it up while retaining full path structure.
    let path_str = canonical.to_string_lossy();
    if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
        Ok(PathBuf::from(stripped))
    } else {
        Ok(canonical)
    }
}

/// Detects whether a path is a symbolic link, directory junction, or Windows reparse point.
pub fn is_symlink_or_reparse_point(path: &Path) -> bool {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return true;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
            if (meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
                return true;
            }
        }
    }
    false
}

/// Computes the streaming SHA-256 digest of a file for pre-erasure evidential integrity.
pub fn compute_file_sha256(path: &Path) -> Result<String, FileEraseFailureReason> {
    let mut file = File::open(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => {
            FileEraseFailureReason::PermissionDenied(path.to_string_lossy().to_string())
        }
        _ => FileEraseFailureReason::IoError(format!("Failed to open file for hashing: {}", e)),
    })?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536]; // 64 KB streaming buffer

    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| FileEraseFailureReason::IoError(format!("Read error: {}", e)))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Converts standard SystemTime to RFC 3339 formatted string.
fn system_time_to_rfc3339(time: std::io::Result<std::time::SystemTime>) -> Option<String> {
    time.ok().map(|t| {
        let dt: DateTime<Utc> = t.into();
        dt.to_rfc3339()
    })
}

/// Validates that a file target is safe, exists, is a regular file, and captures pre-erasure metadata.
pub fn validate_file_target(
    raw_path: &str,
) -> Result<FileMetadataSnapshot, FileEraseFailureReason> {
    let raw_trimmed = raw_path.trim();
    if raw_trimmed.is_empty() {
        return Err(FileEraseFailureReason::TargetNotFound(
            "Empty path specified".to_string(),
        ));
    }

    let path = Path::new(raw_trimmed);
    if !path.exists() {
        return Err(FileEraseFailureReason::TargetNotFound(
            raw_trimmed.to_string(),
        ));
    }

    // Detect symlink or junction
    if is_symlink_or_reparse_point(path) {
        return Err(FileEraseFailureReason::SymbolicLinkDetected(
            "Target is a symbolic link or reparse point; direct file erasure is prohibited"
                .to_string(),
        ));
    }

    let canonical = normalize_canonical_path(path)?;
    let canonical_str = canonical.to_string_lossy().to_string();

    // Check system or boot path protection
    let (is_sys, is_boot) = SafetyEngine::is_system_or_boot_path(&canonical_str);
    if is_sys || is_boot {
        return Err(FileEraseFailureReason::SystemOrBootPath(format!(
            "Target '{}' is a system or boot critical location",
            canonical_str
        )));
    }

    let meta = std::fs::metadata(&canonical).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => {
            FileEraseFailureReason::PermissionDenied(canonical_str.clone())
        }
        _ => FileEraseFailureReason::IoError(e.to_string()),
    })?;

    if meta.is_dir() {
        return Err(FileEraseFailureReason::InvalidTargetType(
            "Target is a directory, but file erasure was requested. Use folder erasure instead."
                .to_string(),
        ));
    }

    let pre_sha256 = if meta.len() > 0 {
        Some(compute_file_sha256(&canonical)?)
    } else {
        Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string())
    };

    Ok(FileMetadataSnapshot {
        path: raw_trimmed.to_string(),
        canonical_path: canonical_str,
        is_directory: false,
        is_symlink: false,
        size_bytes: meta.len(),
        created_at: system_time_to_rfc3339(meta.created()),
        modified_at: system_time_to_rfc3339(meta.modified()),
        accessed_at: system_time_to_rfc3339(meta.accessed()),
        is_readonly: meta.permissions().readonly(),
        pre_erasure_sha256: pre_sha256,
        snapshot_timestamp: Utc::now().to_rfc3339(),
    })
}

/// Validates that a folder target is safe, exists, is a directory, and captures pre-erasure metadata.
pub fn validate_folder_target(
    raw_path: &str,
) -> Result<FileMetadataSnapshot, FileEraseFailureReason> {
    let raw_trimmed = raw_path.trim();
    if raw_trimmed.is_empty() {
        return Err(FileEraseFailureReason::TargetNotFound(
            "Empty path specified".to_string(),
        ));
    }

    let path = Path::new(raw_trimmed);
    if !path.exists() {
        return Err(FileEraseFailureReason::TargetNotFound(
            raw_trimmed.to_string(),
        ));
    }

    if is_symlink_or_reparse_point(path) {
        return Err(FileEraseFailureReason::SymbolicLinkDetected(
            "Target folder is a symbolic link or junction; recursive erasure is prohibited"
                .to_string(),
        ));
    }

    let canonical = normalize_canonical_path(path)?;
    let canonical_str = canonical.to_string_lossy().to_string();

    // Check system or boot path protection
    let (is_sys, is_boot) = SafetyEngine::is_system_or_boot_path(&canonical_str);
    if is_sys || is_boot {
        return Err(FileEraseFailureReason::SystemOrBootPath(format!(
            "Target directory '{}' is a system or boot critical location",
            canonical_str
        )));
    }

    let meta = std::fs::metadata(&canonical).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => {
            FileEraseFailureReason::PermissionDenied(canonical_str.clone())
        }
        _ => FileEraseFailureReason::IoError(e.to_string()),
    })?;

    if !meta.is_dir() {
        return Err(FileEraseFailureReason::InvalidTargetType(
            "Target is a file, but folder erasure was requested. Use file erasure instead."
                .to_string(),
        ));
    }

    Ok(FileMetadataSnapshot {
        path: raw_trimmed.to_string(),
        canonical_path: canonical_str,
        is_directory: true,
        is_symlink: false,
        size_bytes: 0,
        created_at: system_time_to_rfc3339(meta.created()),
        modified_at: system_time_to_rfc3339(meta.modified()),
        accessed_at: system_time_to_rfc3339(meta.accessed()),
        is_readonly: meta.permissions().readonly(),
        pre_erasure_sha256: None,
        snapshot_timestamp: Utc::now().to_rfc3339(),
    })
}

/// Re-probes live target on disk to verify it has not mutated or been swapped (TOCTOU guard).
pub fn verify_target_snapshot_integrity(
    snapshot: &FileMetadataSnapshot,
) -> Result<(), FileEraseFailureReason> {
    let path = Path::new(&snapshot.canonical_path);
    if !path.exists() {
        return Err(FileEraseFailureReason::TargetNotFound(
            snapshot.canonical_path.clone(),
        ));
    }

    if is_symlink_or_reparse_point(path) {
        return Err(FileEraseFailureReason::SymbolicLinkDetected(
            "Target was replaced with a symlink or junction".to_string(),
        ));
    }

    let meta =
        std::fs::metadata(path).map_err(|e| FileEraseFailureReason::IoError(e.to_string()))?;

    if meta.is_dir() != snapshot.is_directory {
        return Err(FileEraseFailureReason::TargetChanged(format!(
            "Target type changed from is_dir={} to is_dir={}",
            snapshot.is_directory,
            meta.is_dir()
        )));
    }

    if !snapshot.is_directory {
        if meta.len() != snapshot.size_bytes {
            return Err(FileEraseFailureReason::TargetChanged(format!(
                "Target file size mutated from {} bytes to {} bytes",
                snapshot.size_bytes,
                meta.len()
            )));
        }
    }

    Ok(())
}
