use super::rules::{generate_suggested_filename, infer_file_type_from_extension, resolve_category};
use crate::models::{FileCategory, FileType};
use crate::signature::registry::SignatureRegistry;

/// Classification details for an analyzed candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationResult {
    pub file_type: FileType,
    pub category: FileCategory,
    pub mime_type: String,
    pub suggested_filename: String,
}

/// Classifies a candidate using available metadata or byte content.
pub fn classify_candidate(
    data: &[u8],
    offset: u64,
    hint_filename: Option<&str>,
    registry: &SignatureRegistry,
) -> ClassificationResult {
    // 1. If hint_filename is present, try extension inference
    let inferred_from_hint = hint_filename.and_then(|name| {
        std::path::Path::new(name)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(infer_file_type_from_extension)
    });

    // 2. Try signature detection from raw bytes
    let detected_from_bytes = registry
        .all()
        .iter()
        .find(|sig| {
            data.len() >= sig.header_magic.len()
                && &data[0..sig.header_magic.len()] == sig.header_magic.as_slice()
        })
        .map(|s| s.file_type.clone());

    let file_type = detected_from_bytes
        .or(inferred_from_hint)
        .unwrap_or(FileType::Unknown("unknown".to_string()));

    let category = resolve_category(&file_type);
    let mime_type = file_type.default_mime_type().to_string();
    let suggested_filename = hint_filename
        .map(|s| s.to_string())
        .unwrap_or_else(|| generate_suggested_filename(offset, &file_type));

    ClassificationResult {
        file_type,
        category,
        mime_type,
        suggested_filename,
    }
}
