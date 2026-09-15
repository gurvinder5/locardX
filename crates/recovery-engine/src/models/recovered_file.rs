use serde::{Deserialize, Serialize};
use std::fmt;

/// High-level file categories for classification and organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileCategory {
    Images,
    Documents,
    Archives,
    Databases,
    TextFiles,
    Media,
    Unknown,
}

impl fmt::Display for FileCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Images => write!(f, "Images"),
            Self::Documents => write!(f, "Documents"),
            Self::Archives => write!(f, "Archives"),
            Self::Databases => write!(f, "Databases"),
            Self::TextFiles => write!(f, "Text Files"),
            Self::Media => write!(f, "Media"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Supported and recognized file types for carving and structural recovery.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    Jpeg,
    Png,
    Gif,
    Bmp,
    Tiff,
    Webp,
    Pdf,
    #[serde(alias = "doc")]
    OfficeDoc,
    #[serde(alias = "docx")]
    OfficeDocx,
    #[serde(alias = "xls")]
    OfficeXls,
    #[serde(alias = "xlsx")]
    OfficeXlsx,
    #[serde(alias = "ppt")]
    OfficePpt,
    #[serde(alias = "pptx")]
    OfficePptx,
    Rtf,
    Zip,
    #[serde(alias = "7z")]
    SevenZip,
    Rar,
    Mp3,
    Wav,
    Mp4,
    Avi,
    Mkv,
    Sqlite,
    Text,
    #[serde(untagged)]
    Unknown(String),
}

impl FileType {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Webp => "webp",
            Self::Pdf => "pdf",
            Self::OfficeDoc => "doc",
            Self::OfficeDocx => "docx",
            Self::OfficeXls => "xls",
            Self::OfficeXlsx => "xlsx",
            Self::OfficePpt => "ppt",
            Self::OfficePptx => "pptx",
            Self::Rtf => "rtf",
            Self::Zip => "zip",
            Self::SevenZip => "7z",
            Self::Rar => "rar",
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Mp4 => "mp4",
            Self::Avi => "avi",
            Self::Mkv => "mkv",
            Self::Sqlite => "sqlite",
            Self::Text => "txt",
            Self::Unknown(_) => "bin",
        }
    }

    pub fn default_mime_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Gif => "image/gif",
            Self::Bmp => "image/bmp",
            Self::Tiff => "image/tiff",
            Self::Webp => "image/webp",
            Self::Pdf => "application/pdf",
            Self::OfficeDoc => "application/msword",
            Self::OfficeDocx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            Self::OfficeXls => "application/vnd.ms-excel",
            Self::OfficeXlsx => {
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            }
            Self::OfficePpt => "application/vnd.ms-powerpoint",
            Self::OfficePptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Self::Rtf => "application/rtf",
            Self::Zip => "application/zip",
            Self::SevenZip => "application/x-7z-compressed",
            Self::Rar => "application/vnd.rar",
            Self::Mp3 => "audio/mpeg",
            Self::Wav => "audio/wav",
            Self::Mp4 => "video/mp4",
            Self::Avi => "video/x-msvideo",
            Self::Mkv => "video/x-matroska",
            Self::Sqlite => "application/x-sqlite3",
            Self::Text => "text/plain",
            Self::Unknown(_) => "application/octet-stream",
        }
    }

    pub fn category(&self) -> FileCategory {
        match self {
            Self::Jpeg | Self::Png | Self::Gif | Self::Bmp | Self::Tiff | Self::Webp => {
                FileCategory::Images
            }
            Self::Pdf
            | Self::OfficeDoc
            | Self::OfficeDocx
            | Self::OfficeXls
            | Self::OfficeXlsx
            | Self::OfficePpt
            | Self::OfficePptx
            | Self::Rtf => FileCategory::Documents,
            Self::Zip | Self::SevenZip | Self::Rar => FileCategory::Archives,
            Self::Mp3 | Self::Wav | Self::Mp4 | Self::Avi | Self::Mkv => FileCategory::Media,
            Self::Sqlite => FileCategory::Databases,
            Self::Text => FileCategory::TextFiles,
            Self::Unknown(_) => FileCategory::Unknown,
        }
    }
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jpeg => write!(f, "JPEG"),
            Self::Png => write!(f, "PNG"),
            Self::Gif => write!(f, "GIF"),
            Self::Bmp => write!(f, "BMP"),
            Self::Tiff => write!(f, "TIFF"),
            Self::Webp => write!(f, "WEBP"),
            Self::Pdf => write!(f, "PDF"),
            Self::OfficeDoc => write!(f, "DOC (CFBF)"),
            Self::OfficeDocx => write!(f, "DOCX (OpenXML)"),
            Self::OfficeXls => write!(f, "XLS (CFBF)"),
            Self::OfficeXlsx => write!(f, "XLSX (OpenXML)"),
            Self::OfficePpt => write!(f, "PPT (CFBF)"),
            Self::OfficePptx => write!(f, "PPTX (OpenXML)"),
            Self::Rtf => write!(f, "RTF"),
            Self::Zip => write!(f, "ZIP"),
            Self::SevenZip => write!(f, "7-Zip"),
            Self::Rar => write!(f, "RAR"),
            Self::Mp3 => write!(f, "MP3"),
            Self::Wav => write!(f, "WAV"),
            Self::Mp4 => write!(f, "MP4"),
            Self::Avi => write!(f, "AVI"),
            Self::Mkv => write!(f, "MKV"),
            Self::Sqlite => write!(f, "SQLite DB"),
            Self::Text => write!(f, "Text"),
            Self::Unknown(name) => write!(f, "Unknown ({})", name),
        }
    }
}

/// Recovery method by which a file candidate was discovered and reconstructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryMethod {
    FilesystemMetadata,
    SignatureCarving,
    StructureCarving,
    FragmentReconstruction,
}

impl fmt::Display for RecoveryMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FilesystemMetadata => write!(f, "Filesystem Metadata"),
            Self::SignatureCarving => write!(f, "Signature Carving"),
            Self::StructureCarving => write!(f, "Structure Carving"),
            Self::FragmentReconstruction => write!(f, "Fragment Reconstruction"),
        }
    }
}

/// Outcome of structural and cryptographic validation checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Valid,
    PartiallyValid,
    Corrupted,
    Incomplete,
    Invalid,
}

impl fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Valid => write!(f, "Valid"),
            Self::PartiallyValid => write!(f, "Partially Valid"),
            Self::Corrupted => write!(f, "Corrupted"),
            Self::Incomplete => write!(f, "Incomplete"),
            Self::Invalid => write!(f, "Invalid"),
        }
    }
}

/// Qualitative confidence grade derived deterministically from evidence factors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceGrade {
    High,
    Medium,
    Low,
    Uncertain,
}

impl ConfidenceGrade {
    pub fn from_score(score: u32) -> Self {
        match score {
            85..=100 => Self::High,
            65..=84 => Self::Medium,
            40..=64 => Self::Low,
            _ => Self::Uncertain,
        }
    }
}

impl fmt::Display for ConfidenceGrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::High => write!(f, "High"),
            Self::Medium => write!(f, "Medium"),
            Self::Low => write!(f, "Low"),
            Self::Uncertain => write!(f, "Uncertain"),
        }
    }
}

/// Individual observable evidence factor contributing to confidence scoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFactor {
    pub factor_type: String,
    pub weight: i32,
    pub description: String,
    pub passed: bool,
}

/// Metadata and evidential properties of a recovered or carved file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveredFile {
    pub file_id: String,
    pub job_id: String,
    pub source_offset: u64,
    pub size_bytes: u64,
    pub file_type: FileType,
    pub category: FileCategory,
    pub mime_type: String,
    pub suggested_filename: String,
    pub recovery_method: RecoveryMethod,
    pub validation_status: ValidationStatus,
    pub confidence_score: u32,
    pub confidence_grade: ConfidenceGrade,
    pub is_fragmented: bool,
    pub sha256_hash: String,
    pub output_relative_path: Option<String>,
    pub evidence_factors: Vec<EvidenceFactor>,
    pub created_at: String,
}
