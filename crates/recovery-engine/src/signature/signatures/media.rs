use crate::models::FileType;

/// Parses RIFF container formats: WAV, AVI, WEBP.
/// Returns (file_length, is_valid, resolved_file_type)
pub fn parse_riff_stream(data: &[u8]) -> Option<(u64, bool, FileType)> {
    if data.len() < 12 || &data[0..4] != b"RIFF" {
        return None;
    }

    let payload_size = u32::from_le_bytes(data[4..8].try_into().unwrap()) as u64;
    let total_size = 8 + payload_size;

    let form_type = &data[8..12];
    let (file_type, is_known) = match form_type {
        b"WAVE" => (FileType::Wav, true),
        b"AVI " => (FileType::Avi, true),
        b"WEBP" => (FileType::Webp, true),
        _ => (FileType::Unknown("riff".to_string()), false),
    };

    if !is_known {
        return None;
    }

    let is_valid = total_size >= 12;
    let final_len = total_size.min(data.len() as u64);

    Some((final_len, is_valid, file_type))
}

/// Parses MP4 / ISO Base Media File Format (ftyp box).
/// Returns (file_length, is_valid)
pub fn parse_mp4_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 16 {
        return None;
    }

    // Check for 'ftyp' at offset 4
    if &data[4..8] != b"ftyp" {
        return None;
    }

    let mut offset = 0;
    let mut total_size = 0u64;
    let mut box_count = 0;
    let mut has_moov = false;
    let mut has_mdat = false;

    while offset + 8 <= data.len() {
        let box_len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let box_type = &data[offset + 4..offset + 8];

        if box_type == b"moov" {
            has_moov = true;
        } else if box_type == b"mdat" {
            has_mdat = true;
        }

        if box_len == 0 {
            // Extends to EOF
            total_size = data.len() as u64;
            break;
        } else if box_len == 1 {
            // Extended 64-bit size
            if offset + 16 > data.len() {
                break;
            }
            let ext_len = u64::from_be_bytes(data[offset + 8..offset + 16].try_into().unwrap()) as usize;
            if ext_len < 16 || offset + ext_len > data.len() {
                break;
            }
            offset += ext_len;
            total_size = offset as u64;
        } else {
            if box_len < 8 || offset + box_len > data.len() {
                break;
            }
            offset += box_len;
            total_size = offset as u64;
        }

        box_count += 1;
        if has_moov && has_mdat {
            break;
        }
    }

    if box_count > 0 && total_size >= 16 {
        Some((total_size.min(data.len() as u64), true))
    } else {
        None
    }
}

/// Parses MP3 audio stream (ID3v2 header or MPEG frame sync).
/// Returns (file_length, is_valid)
pub fn parse_mp3_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 10 {
        return None;
    }

    if &data[0..3] == b"ID3" {
        // ID3v2 tag: 10 bytes header
        let size_bytes = [data[6], data[7], data[8], data[9]];
        // Synchsafe integer (7 bits per byte)
        let tag_size = ((size_bytes[0] as u64 & 0x7F) << 21)
            | ((size_bytes[1] as u64 & 0x7F) << 14)
            | ((size_bytes[2] as u64 & 0x7F) << 7)
            | (size_bytes[3] as u64 & 0x7F);

        let total_tag_len = 10 + tag_size;
        let is_valid = total_tag_len <= data.len() as u64;

        // Estimate full song length bounded by data
        let final_len = (data.len() as u64).min(50 * 1024 * 1024);
        return Some((final_len, is_valid));
    }

    // Direct MPEG frame sync (0xFF 0xFB, 0xFF 0xF3, 0xFF 0xF2)
    if data[0] == 0xFF && (data[1] & 0xE0) == 0xE0 {
        let final_len = (data.len() as u64).min(50 * 1024 * 1024);
        return Some((final_len, true));
    }

    None
}

/// Parses Matroska / WebM (MKV) EBML container.
/// Returns (file_length, is_valid)
pub fn parse_mkv_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 12 || &data[0..4] != &[0x1A, 0x45, 0xDF, 0xA3] {
        return None;
    }

    // Check for DocType "matroska" or "webm" in the EBML header window
    let check_window = data.len().min(512);
    let has_doctype = data[..check_window].windows(8).any(|w| w == b"matroska")
        || data[..check_window].windows(4).any(|w| w == b"webm");

    let is_valid = has_doctype;
    let final_len = (data.len() as u64).min(500 * 1024 * 1024);

    Some((final_len, is_valid))
}
