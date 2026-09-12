pub const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Validates and parses PNG chunk structure.
/// Returns (file_length, is_valid) if PNG chunks lead to an IEND chunk.
pub fn parse_png_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 8 || &data[0..8] != &PNG_MAGIC {
        return None;
    }

    let mut offset = 8;
    let mut has_ihdr = false;
    let mut has_idat = false;
    let mut chunk_count = 0;

    while offset + 8 <= data.len() {
        let chunk_len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let chunk_type = &data[offset + 4..offset + 8];

        if chunk_count == 0 && chunk_type == b"IHDR" {
            has_ihdr = true;
        }

        if chunk_type == b"IDAT" {
            has_idat = true;
        }

        let total_chunk_len = 12 + chunk_len; // 4 len + 4 type + data + 4 crc
        if offset + total_chunk_len > data.len() {
            // Incomplete chunk
            break;
        }

        if chunk_type == b"IEND" {
            let file_size = offset + total_chunk_len;
            let is_valid = has_ihdr && (has_idat || chunk_count > 1);
            return Some((file_size as u64, is_valid));
        }

        offset += total_chunk_len;
        chunk_count += 1;
    }

    // Fallback: look for IEND signature within slice
    let iend_pattern = [0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82];
    for i in (8..data.len().saturating_sub(8)).rev() {
        if &data[i..i + 8] == &iend_pattern {
            return Some(((i + 8) as u64, has_ihdr));
        }
    }

    None
}
