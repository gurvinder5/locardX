use locardx_recovery_engine::models::{FileType, ValidationStatus};
use locardx_recovery_engine::structure::validator::validate_file_candidate;

#[test]
fn test_valid_png_structure() {
    let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&[0; 13]);
    data.extend_from_slice(&[0; 4]);
    // IDAT
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x04]);
    data.extend_from_slice(b"IDAT");
    data.extend_from_slice(&[1, 2, 3, 4]);
    data.extend_from_slice(&[0; 4]);
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    data.extend_from_slice(b"IEND");
    data.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);

    let (status, notes) = validate_file_candidate(&FileType::Png, &data);
    assert_eq!(status, ValidationStatus::Valid);
    assert!(!notes.is_empty());
}

#[test]
fn test_incomplete_png_truncation() {
    // Only PNG header and IHDR without IEND chunk
    let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&[0; 13]);

    let (status, _) = validate_file_candidate(&FileType::Png, &data);
    assert_eq!(status, ValidationStatus::Corrupted);
}

#[test]
fn test_corrupted_structure_rejection() {
    let random_junk = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04];
    let (status, _) = validate_file_candidate(&FileType::Pdf, &random_junk);
    assert_eq!(status, ValidationStatus::Corrupted);

    let (status_empty, _) = validate_file_candidate(&FileType::Jpeg, &[]);
    assert_eq!(status_empty, ValidationStatus::Invalid);
}
