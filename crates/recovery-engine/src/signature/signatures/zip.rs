pub const ZIP_MAGIC: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
pub const EOCD_MAGIC: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];

/// Parses a ZIP archive stream, locating the End of Central Directory (EOCD).
/// Returns (file_length, is_valid, is_docx).
pub fn parse_zip_stream(data: &[u8]) -> Option<(u64, bool, bool)> {
    if data.len() < 22 || &data[0..4] != &ZIP_MAGIC {
        return None;
    }

    // Search backwards for EOCD record (max comment length is 65535 bytes)
    let search_start = data.len().saturating_sub(65535 + 22);
    let mut eocd_pos = None;

    for i in (search_start..data.len().saturating_sub(3)).rev() {
        if &data[i..i + 4] == &EOCD_MAGIC {
            eocd_pos = Some(i);
            break;
        }
    }

    if let Some(pos) = eocd_pos {
        if pos + 22 <= data.len() {
            let comment_len = u16::from_le_bytes([data[pos + 20], data[pos + 21]]) as usize;
            let total_len = pos + 22 + comment_len;
            let final_len = total_len.min(data.len());

            let slice = &data[..final_len];
            let is_docx = slice.windows(19).any(|w| w == b"[Content_Types].xml")
                || slice.windows(5).any(|w| w == b"word/")
                || slice.windows(3).any(|w| w == b"xl/");

            return Some((final_len as u64, true, is_docx));
        }
    }

    None
}
