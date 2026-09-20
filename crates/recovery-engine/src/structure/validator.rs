use super::parser::parse_file_structure;
use crate::models::{FileType, ValidationStatus};

/// Evaluates internal format rules and returns validation status and notes.
pub fn validate_file_candidate(
    file_type: &FileType,
    data: &[u8],
) -> (ValidationStatus, Vec<String>) {
    if data.is_empty() {
        return (
            ValidationStatus::Invalid,
            vec!["Zero-byte candidate payload".to_string()],
        );
    }

    match parse_file_structure(file_type, data) {
        Some(parsed) => {
            if parsed.is_valid_structure {
                if data.len() as u64 >= parsed.detected_size {
                    (ValidationStatus::Valid, parsed.validation_notes)
                } else {
                    let mut notes = parsed.validation_notes;
                    notes.push(format!(
                        "Incomplete stream: available {} bytes, expected {} bytes",
                        data.len(),
                        parsed.detected_size
                    ));
                    (ValidationStatus::Incomplete, notes)
                }
            } else {
                let mut notes = parsed.validation_notes;
                notes.push(
                    "Structural anomalies detected in headers or internal markers".to_string(),
                );
                (ValidationStatus::PartiallyValid, notes)
            }
        }
        None => (
            ValidationStatus::Corrupted,
            vec!["Failed internal structure parser check".to_string()],
        ),
    }
}
