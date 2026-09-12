use crate::models::{FileCategory, FileType};

/// Infers FileType from a file extension string.
pub fn infer_file_type_from_extension(ext: &str) -> FileType {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => FileType::Jpeg,
        "png" => FileType::Png,
        "pdf" => FileType::Pdf,
        "zip" => FileType::Zip,
        "docx" | "xlsx" | "pptx" => FileType::OfficeDocx,
        "sqlite" | "db" | "sqlite3" => FileType::Sqlite,
        "txt" | "log" | "csv" => FileType::Text,
        other => FileType::Unknown(other.to_string()),
    }
}

/// Generates a forensic default filename given offset and file type.
pub fn generate_suggested_filename(offset: u64, file_type: &FileType) -> String {
    format!("carved_0x{:08X}.{}", offset, file_type.extension())
}

/// Resolves category for a given file type.
pub fn resolve_category(file_type: &FileType) -> FileCategory {
    file_type.category()
}
