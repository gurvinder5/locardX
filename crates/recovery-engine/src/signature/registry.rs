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
            // PDF Document
            FileSignature {
                file_type: FileType::Pdf,
                header_magic: vec![0x25, 0x50, 0x44, 0x46], // %PDF
                header_offset: 0,
                footer_magic: Some(vec![0x25, 0x25, 0x45, 0x4F, 0x46]), // %%EOF
                min_size: 30,
                max_size: 100 * 1024 * 1024,
            },
            // ZIP Archive / Office OpenXML
            FileSignature {
                file_type: FileType::Zip,
                header_magic: vec![0x50, 0x4B, 0x03, 0x04], // PK\x03\x04
                header_offset: 0,
                footer_magic: Some(vec![0x50, 0x4B, 0x05, 0x06]), // PK\x05\x06 (EOCD)
                min_size: 30,
                max_size: 200 * 1024 * 1024,
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
                        || (*t == FileType::OfficeDocx && s.file_type == FileType::Zip)
                })
            })
            .cloned()
            .collect()
    }
}
