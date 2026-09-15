use crate::models::FileType;
use crate::signature::signatures::{
    parse_bmp_stream, parse_cfbf_stream, parse_gif_stream, parse_jpeg_stream, parse_mkv_stream,
    parse_mp3_stream, parse_mp4_stream, parse_pdf_stream, parse_png_stream, parse_rar_stream,
    parse_riff_stream, parse_rtf_stream, parse_seven_zip_stream, parse_tiff_stream,
    parse_zip_stream,
};

/// Output of structure-aware parser evaluating a file candidate.
#[derive(Debug, Clone)]
pub struct ParsedStructure {
    pub file_type: FileType,
    pub detected_size: u64,
    pub is_valid_structure: bool,
    pub validation_notes: Vec<String>,
}

/// Parses internal structure according to the suspected file type.
pub fn parse_file_structure(file_type: &FileType, data: &[u8]) -> Option<ParsedStructure> {
    match file_type {
        FileType::Jpeg => parse_jpeg(data),
        FileType::Png => parse_png(data),
        FileType::Gif => parse_gif(data),
        FileType::Bmp => parse_bmp(data),
        FileType::Tiff => parse_tiff(data),
        FileType::Webp | FileType::Wav | FileType::Avi => parse_riff(data),
        FileType::Pdf => parse_pdf(data),
        FileType::OfficeDoc | FileType::OfficeXls | FileType::OfficePpt => parse_cfbf(data),
        FileType::Zip
        | FileType::OfficeDocx
        | FileType::OfficeXlsx
        | FileType::OfficePptx => parse_zip(data),
        FileType::Rtf => parse_rtf(data),
        FileType::SevenZip => parse_seven_zip(data),
        FileType::Rar => parse_rar(data),
        FileType::Mp3 => parse_mp3(data),
        FileType::Mp4 => parse_mp4(data),
        FileType::Mkv => parse_mkv(data),
        FileType::Sqlite => parse_sqlite(data),
        FileType::Text => parse_text(data),
        FileType::Unknown(_) => None,
    }
}

fn parse_jpeg(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid_markers) = parse_jpeg_stream(data)?;
    let mut notes = vec![
        "SOI marker verified".to_string(),
        "EOI marker located".to_string(),
    ];
    if valid_markers {
        notes.push("SOF/DQT/DHT internal markers validated".to_string());
    }

    Some(ParsedStructure {
        file_type: FileType::Jpeg,
        detected_size: len,
        is_valid_structure: valid_markers,
        validation_notes: notes,
    })
}

fn parse_png(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid_chunks) = parse_png_stream(data)?;
    let mut notes = vec![
        "PNG magic signature verified".to_string(),
        "IEND chunk located".to_string(),
    ];
    if valid_chunks {
        notes.push("IHDR and chunk CRC chain validated".to_string());
    }

    Some(ParsedStructure {
        file_type: FileType::Png,
        detected_size: len,
        is_valid_structure: valid_chunks,
        validation_notes: notes,
    })
}

fn parse_gif(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid) = parse_gif_stream(data)?;
    Some(ParsedStructure {
        file_type: FileType::Gif,
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec![
            "GIF header and screen descriptor verified".to_string(),
            "GIF trailer 0x3B located".to_string(),
        ],
    })
}

fn parse_bmp(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid) = parse_bmp_stream(data)?;
    Some(ParsedStructure {
        file_type: FileType::Bmp,
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec![
            "BMP header verified".to_string(),
            "DIB header and pixel offset verified".to_string(),
        ],
    })
}

fn parse_tiff(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid) = parse_tiff_stream(data)?;
    Some(ParsedStructure {
        file_type: FileType::Tiff,
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec!["TIFF header and IFD pointer validated".to_string()],
    })
}

fn parse_riff(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid, resolved_type) = parse_riff_stream(data)?;
    Some(ParsedStructure {
        file_type: resolved_type.clone(),
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec![format!("RIFF container ({}) verified", resolved_type)],
    })
}

fn parse_pdf(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid_structure) = parse_pdf_stream(data)?;
    let mut notes = vec![
        "%PDF header verified".to_string(),
        "%%EOF trailer located".to_string(),
    ];
    if valid_structure {
        notes.push("XREF/trailer/root object validated".to_string());
    }

    Some(ParsedStructure {
        file_type: FileType::Pdf,
        detected_size: len,
        is_valid_structure: valid_structure,
        validation_notes: notes,
    })
}

fn parse_cfbf(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid, resolved_type) = parse_cfbf_stream(data)?;
    Some(ParsedStructure {
        file_type: resolved_type.clone(),
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec![format!(
            "Compound File Binary Format ({}) sectors verified",
            resolved_type
        )],
    })
}

fn parse_zip(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid_eocd, resolved_type) = parse_zip_stream(data)?;
    let mut notes = vec![
        "Local file header (PK) verified".to_string(),
        "EOCD record located".to_string(),
    ];
    if resolved_type != FileType::Zip {
        notes.push(format!("Office OpenXML structures identified ({})", resolved_type));
    }

    Some(ParsedStructure {
        file_type: resolved_type,
        detected_size: len,
        is_valid_structure: valid_eocd,
        validation_notes: notes,
    })
}

fn parse_rtf(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid) = parse_rtf_stream(data)?;
    Some(ParsedStructure {
        file_type: FileType::Rtf,
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec!["RTF control words and brace balance verified".to_string()],
    })
}

fn parse_seven_zip(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid) = parse_seven_zip_stream(data)?;
    Some(ParsedStructure {
        file_type: FileType::SevenZip,
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec!["7-Zip header and next-header offset verified".to_string()],
    })
}

fn parse_rar(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid) = parse_rar_stream(data)?;
    Some(ParsedStructure {
        file_type: FileType::Rar,
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec!["RAR archive magic and block header verified".to_string()],
    })
}

fn parse_mp3(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid) = parse_mp3_stream(data)?;
    Some(ParsedStructure {
        file_type: FileType::Mp3,
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec!["MP3 ID3v2/frame stream verified".to_string()],
    })
}

fn parse_mp4(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid) = parse_mp4_stream(data)?;
    Some(ParsedStructure {
        file_type: FileType::Mp4,
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec!["MP4 ISO base media boxes (ftyp/moov/mdat) verified".to_string()],
    })
}

fn parse_mkv(data: &[u8]) -> Option<ParsedStructure> {
    let (len, valid) = parse_mkv_stream(data)?;
    Some(ParsedStructure {
        file_type: FileType::Mkv,
        detected_size: len,
        is_valid_structure: valid,
        validation_notes: vec!["Matroska/WebM EBML header verified".to_string()],
    })
}

fn parse_sqlite(data: &[u8]) -> Option<ParsedStructure> {
    if data.len() < 100 || &data[0..16] != b"SQLite format 3\0" {
        return None;
    }

    let mut page_size = u16::from_be_bytes([data[16], data[17]]) as u64;
    if page_size == 1 {
        page_size = 65536;
    }

    let page_count = u32::from_be_bytes(data[28..32].try_into().unwrap()) as u64;

    if page_size >= 512 && page_size.is_power_of_two() {
        let total_size = if page_count > 0 {
            page_size * page_count
        } else {
            // In wal mode or nascent db, minimum 1 page
            page_size
        };

        Some(ParsedStructure {
            file_type: FileType::Sqlite,
            detected_size: total_size,
            is_valid_structure: page_count > 0,
            validation_notes: vec![
                "SQLite format 3 magic verified".to_string(),
                format!("Page size: {} bytes, Page count: {}", page_size, page_count),
            ],
        })
    } else {
        None
    }
}

fn parse_text(data: &[u8]) -> Option<ParsedStructure> {
    if data.is_empty() {
        return None;
    }

    // Check if the initial chunk is valid UTF-8 and mostly printable
    let check_len = data.len().min(4096);
    let slice = &data[..check_len];

    let printable_count = slice
        .iter()
        .filter(|&&b| b == b'\t' || b == b'\r' || b == b'\n' || (0x20..=0x7E).contains(&b))
        .count();

    let ratio = (printable_count as f64) / (slice.len() as f64);
    if ratio > 0.95 {
        // Find text length up to null terminator or end
        let end_idx = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        Some(ParsedStructure {
            file_type: FileType::Text,
            detected_size: (end_idx as u64).max(1),
            is_valid_structure: true,
            validation_notes: vec![format!(
                "Text stream: {:.1}% printable ASCII/UTF-8",
                ratio * 100.0
            )],
        })
    } else {
        None
    }
}
