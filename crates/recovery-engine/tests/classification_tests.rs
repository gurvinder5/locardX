use locardx_recovery_engine::classification::classifier::classify_candidate;
use locardx_recovery_engine::classification::rules::{
    generate_suggested_filename, infer_file_type_from_extension, resolve_category,
};
use locardx_recovery_engine::models::{FileCategory, FileType};
use locardx_recovery_engine::signature::registry::SignatureRegistry;

#[test]
fn test_extension_inference_and_categories() {
    assert_eq!(infer_file_type_from_extension("jpg"), FileType::Jpeg);
    assert_eq!(infer_file_type_from_extension("jpeg"), FileType::Jpeg);
    assert_eq!(infer_file_type_from_extension("png"), FileType::Png);
    assert_eq!(infer_file_type_from_extension("pdf"), FileType::Pdf);
    assert_eq!(infer_file_type_from_extension("zip"), FileType::Zip);
    assert_eq!(infer_file_type_from_extension("docx"), FileType::OfficeDocx);
    assert_eq!(infer_file_type_from_extension("sqlite"), FileType::Sqlite);
    assert_eq!(infer_file_type_from_extension("txt"), FileType::Text);

    assert_eq!(resolve_category(&FileType::Jpeg), FileCategory::Images);
    assert_eq!(resolve_category(&FileType::Pdf), FileCategory::Documents);
    assert_eq!(resolve_category(&FileType::Zip), FileCategory::Archives);
    assert_eq!(resolve_category(&FileType::Sqlite), FileCategory::Databases);
    assert_eq!(resolve_category(&FileType::Text), FileCategory::TextFiles);
}

#[test]
fn test_suggested_filename_format() {
    let fn_jpeg = generate_suggested_filename(0x1000, &FileType::Jpeg);
    assert_eq!(fn_jpeg, "carved_0x00001000.jpg");

    let fn_pdf = generate_suggested_filename(0x2000, &FileType::Pdf);
    assert_eq!(fn_pdf, "carved_0x00002000.pdf");
}

#[test]
fn test_candidate_classification_with_raw_bytes() {
    let reg = SignatureRegistry::new();
    let pdf_bytes = b"%PDF-1.4 sample content";
    let res = classify_candidate(pdf_bytes, 0x500, None, &reg);
    assert_eq!(res.file_type, FileType::Pdf);
    assert_eq!(res.category, FileCategory::Documents);
    assert_eq!(res.mime_type, "application/pdf");
    assert_eq!(res.suggested_filename, "carved_0x00000500.pdf");
}
