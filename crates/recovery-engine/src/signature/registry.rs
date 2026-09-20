use crate::models::FileType;

/// Definition of a recognized file format signature.
#[derive(Debug, Clone)]
pub struct FileSignature {
    pub file_type: FileType,
    pub header_magic: Vec<u8>,
    pub header_offset: usize,
    pub footer_magic: Option<Vec<u8>>,
    pub min_size: u64,
    pub max_size: u64,
}

/// Extensible registry of file format signatures used by carving detectors.
#[derive(Debug, Clone)]
pub struct SignatureRegistry {
    signatures: Vec<FileSignature>,
}

impl Default for SignatureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SignatureRegistry {
    pub fn new() -> Self {
        let signatures = vec![
            // JPEG / JFIF / EXIF
            FileSignature {
                file_type: FileType::Jpeg,
                header_magic: vec![0xFF, 0xD8, 0xFF],
                header_offset: 0,
                footer_magic: Some(vec![0xFF, 0xD9]),
                min_size: 30,
                max_size: 50 * 1024 * 1024, // 50 MiB
            },
            // PNG Image
            FileSignature {
                file_type: FileType::Png,
                header_magic: vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
                header_offset: 0,
                footer_magic: Some(vec![0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82]),
                min_size: 30,
                max_size: 50 * 1024 * 1024,
            },
            // GIF 87a
            FileSignature {
                file_type: FileType::Gif,
                header_magic: b"GIF87a".to_vec(),
                header_offset: 0,
                footer_magic: Some(vec![0x3B]),
                min_size: 14,
                max_size: 50 * 1024 * 1024,
            },
            // GIF 89a
            FileSignature {
                file_type: FileType::Gif,
                header_magic: b"GIF89a".to_vec(),
                header_offset: 0,
                footer_magic: Some(vec![0x3B]),
                min_size: 14,
                max_size: 50 * 1024 * 1024,
            },
            // BMP Image (BM)
            FileSignature {
                file_type: FileType::Bmp,
                header_magic: vec![0x42, 0x4D],
                header_offset: 0,
                footer_magic: None,
                min_size: 26,
                max_size: 50 * 1024 * 1024,
            },
            // TIFF (Little-Endian: II*\0)
            FileSignature {
                file_type: FileType::Tiff,
                header_magic: vec![0x49, 0x49, 0x2A, 0x00],
                header_offset: 0,
                footer_magic: None,
                min_size: 8,
                max_size: 100 * 1024 * 1024,
            },
            // TIFF (Big-Endian: MM\0*)
            FileSignature {
                file_type: FileType::Tiff,
                header_magic: vec![0x4D, 0x4D, 0x00, 0x2A],
                header_offset: 0,
                footer_magic: None,
                min_size: 8,
                max_size: 100 * 1024 * 1024,
            },
            // WEBP Image (RIFF....WEBP)
            FileSignature {
                file_type: FileType::Webp,
                header_magic: vec![0x52, 0x49, 0x46, 0x46],
                header_offset: 0,
                footer_magic: None,
                min_size: 16,
                max_size: 50 * 1024 * 1024,
            },
            // PDF Document
            FileSignature {
                file_type: FileType::Pdf,
                header_magic: vec![0x25, 0x50, 0x44, 0x46], // %PDF
                header_offset: 0,
                footer_magic: Some(vec![0x25, 0x25, 0x45, 0x4F, 0x46]), // %%EOF
                min_size: 30,
                max_size: 100 * 1024 * 1024,
            },
            // OLE / CFBF Compound Documents (DOC, XLS, PPT)
            FileSignature {
                file_type: FileType::OfficeDoc,
                header_magic: vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
                header_offset: 0,
                footer_magic: None,
                min_size: 512,
                max_size: 100 * 1024 * 1024,
            },
            // ZIP Archive / Office OpenXML (DOCX, XLSX, PPTX)
            FileSignature {
                file_type: FileType::Zip,
                header_magic: vec![0x50, 0x4B, 0x03, 0x04], // PK\x03\x04
                header_offset: 0,
                footer_magic: Some(vec![0x50, 0x4B, 0x05, 0x06]), // PK\x05\x06 (EOCD)
                min_size: 30,
                max_size: 200 * 1024 * 1024,
            },
            // RTF Document ({\rtf)
            FileSignature {
                file_type: FileType::Rtf,
                header_magic: vec![0x7B, 0x5C, 0x72, 0x74, 0x66],
                header_offset: 0,
                footer_magic: Some(vec![0x7D]), // '}'
                min_size: 10,
                max_size: 50 * 1024 * 1024,
            },
            // 7-Zip Archive (7z\xBC\xAF\x27\x1C)
            FileSignature {
                file_type: FileType::SevenZip,
                header_magic: vec![0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C],
                header_offset: 0,
                footer_magic: None,
                min_size: 32,
                max_size: 200 * 1024 * 1024,
            },
            // RAR Archive (Rar!\x1A\x07)
            FileSignature {
                file_type: FileType::Rar,
                header_magic: vec![0x52, 0x61, 0x72, 0x21, 0x1A, 0x07],
                header_offset: 0,
                footer_magic: None,
                min_size: 20,
                max_size: 200 * 1024 * 1024,
            },
            // MP3 Audio (ID3v2 tag)
            FileSignature {
                file_type: FileType::Mp3,
                header_magic: vec![0x49, 0x44, 0x33], // ID3
                header_offset: 0,
                footer_magic: None,
                min_size: 128,
                max_size: 100 * 1024 * 1024,
            },
            // WAV Audio (RIFF....WAVE)
            FileSignature {
                file_type: FileType::Wav,
                header_magic: vec![0x52, 0x49, 0x46, 0x46], // RIFF
                header_offset: 0,
                footer_magic: None,
                min_size: 44,
                max_size: 100 * 1024 * 1024,
            },
            // MP4 Video (ftyp at offset 4)
            FileSignature {
                file_type: FileType::Mp4,
                header_magic: vec![0x66, 0x74, 0x79, 0x70], // ftyp
                header_offset: 4,
                footer_magic: None,
                min_size: 32,
                max_size: 500 * 1024 * 1024,
            },
            // AVI Video (RIFF....AVI )
            FileSignature {
                file_type: FileType::Avi,
                header_magic: vec![0x52, 0x49, 0x46, 0x46], // RIFF
                header_offset: 0,
                footer_magic: None,
                min_size: 64,
                max_size: 500 * 1024 * 1024,
            },
            // MKV Video (EBML \x1A\x45\xDF\xA3)
            FileSignature {
                file_type: FileType::Mkv,
                header_magic: vec![0x1A, 0x45, 0xDF, 0xA3],
                header_offset: 0,
                footer_magic: None,
                min_size: 32,
                max_size: 500 * 1024 * 1024,
            },
            // SQLite 3 Database
            FileSignature {
                file_type: FileType::Sqlite,
                header_magic: b"SQLite format 3\0".to_vec(),
                header_offset: 0,
                footer_magic: None,
                min_size: 100,
                max_size: 500 * 1024 * 1024,
            },
        ];

        Self { signatures }
    }

    pub fn from_signatures(signatures: Vec<FileSignature>) -> Self {
        Self { signatures }
    }

    pub fn all(&self) -> &[FileSignature] {
        &self.signatures
    }

    pub fn filter_by_types(&self, types: &[FileType]) -> Vec<FileSignature> {
        self.signatures
            .iter()
            .filter(|s| {
                types.iter().any(|t| {
                    t == &s.file_type
                        || (matches!(t, FileType::OfficeDocx | FileType::OfficeXlsx | FileType::OfficePptx)
                            && s.file_type == FileType::Zip)
                        || (matches!(t, FileType::OfficeDoc | FileType::OfficeXls | FileType::OfficePpt)
                            && s.file_type == FileType::OfficeDoc)
                        || (matches!(t, FileType::Wav | FileType::Avi | FileType::Webp)
                            && (s.file_type == FileType::Wav || s.file_type == FileType::Avi || s.file_type == FileType::Webp))
                })
            })
            .cloned()
            .collect()
    }
}
