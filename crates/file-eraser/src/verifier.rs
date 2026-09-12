use crate::models::{FileVerificationResult, FolderEraseStats};
use chrono::Utc;
use locardx_verification::sanitization::{VerificationOutcome, VerificationStrategy};
use std::path::Path;

/// Verifies that a file path is completely inaccessible and unlinked from the filesystem.
pub fn verify_file_erasure(path: &Path, canonical_str: &str) -> FileVerificationResult {
    let verified_at = Utc::now().to_rfc3339();
    let original_exists = path.exists();
    let canon_exists = Path::new(canonical_str).exists();
    let meta_readable =
        std::fs::symlink_metadata(path).is_ok() || std::fs::symlink_metadata(canonical_str).is_ok();

    if original_exists || canon_exists || meta_readable {
        FileVerificationResult {
            outcome: VerificationOutcome::VerificationFailed,
            strategy: VerificationStrategy::MetadataUnlinkCheck,
            path_exists: true,
            inaccessible: false,
            details: format!(
                "File entry still persists on filesystem at '{}' (exists={}, meta_readable={})",
                canonical_str,
                original_exists || canon_exists,
                meta_readable
            ),
            verified_at,
        }
    } else {
        FileVerificationResult {
            outcome: VerificationOutcome::Verified,
            strategy: VerificationStrategy::MetadataUnlinkCheck,
            path_exists: false,
            inaccessible: true,
            details: "Logical file entry verified completely inaccessible and removed from directory index. Physical flash/magnetic cell sanitization is constrained by filesystem journaling and FTL.".to_string(),
            verified_at,
        }
    }
}

/// Verifies that a folder path and all its child items are completely removed.
pub fn verify_folder_erasure(
    path: &Path,
    canonical_str: &str,
    stats: &FolderEraseStats,
) -> FileVerificationResult {
    let verified_at = Utc::now().to_rfc3339();
    let dir_exists = path.exists() || Path::new(canonical_str).exists();

    if stats.files_failed > 0 || stats.files_cancelled > 0 || dir_exists {
        FileVerificationResult {
            outcome: if stats.files_sanitized > 0 {
                VerificationOutcome::PartiallyVerified
            } else {
                VerificationOutcome::VerificationFailed
            },
            strategy: VerificationStrategy::MetadataUnlinkCheck,
            path_exists: dir_exists,
            inaccessible: !dir_exists,
            details: format!(
                "Folder erasure incomplete: dir_exists={}, sanitized={}/{}, failed={}, cancelled={}",
                dir_exists, stats.files_sanitized, stats.total_files, stats.files_failed, stats.files_cancelled
            ),
            verified_at,
        }
    } else {
        FileVerificationResult {
            outcome: VerificationOutcome::Verified,
            strategy: VerificationStrategy::MetadataUnlinkCheck,
            path_exists: false,
            inaccessible: true,
            details: format!(
                "Directory tree completely sanitized and removed. Total {} files and {} directories unlinked.",
                stats.files_sanitized, stats.directories_removed
            ),
            verified_at,
        }
    }
}
