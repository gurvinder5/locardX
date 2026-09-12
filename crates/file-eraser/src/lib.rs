pub mod folder;
pub mod models;
pub mod sanitizer;
pub mod service;
pub mod validation;
pub mod verifier;

pub use folder::sanitize_folder_recursive;
pub use models::*;
pub use sanitizer::sanitize_file_content_and_remove;
pub use service::FileEraserService;
pub use validation::{
    compute_file_sha256, is_symlink_or_reparse_point, normalize_canonical_path,
    validate_file_target, validate_folder_target, verify_target_snapshot_integrity,
};
pub use verifier::{verify_file_erasure, verify_folder_erasure};
