use crate::models::{FileCategory, FileType};

/// Infers FileType from a file extension string.
pub fn infer_file_type_from_extension(ext: &str) -> FileType {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => FileType::Jpeg,
        "png" => FileType::Png,
        "gif" => FileType::Gif,
        "bmp" => FileType::Bmp,
        "tif" | "tiff" => FileType::Tiff,
        "webp" => FileType::Webp,
        "pdf" => FileType::Pdf,
        "doc" => FileType::OfficeDoc,
        "docx" => FileType::OfficeDocx,
        "xls" => FileType::OfficeXls,
        "xlsx" => FileType::OfficeXlsx,
        "ppt" => FileType::OfficePpt,
        "pptx" => FileType::OfficePptx,
        "rtf" => FileType::Rtf,
        "zip" => FileType::Zip,
        "7z" => FileType::SevenZip,
        "rar" => FileType::Rar,
        "mp3" => FileType::Mp3,
        "wav" => FileType::Wav,
        "mp4" | "m4v" => FileType::Mp4,
        "avi" => FileType::Avi,
        "mkv" | "webm" => FileType::Mkv,
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
