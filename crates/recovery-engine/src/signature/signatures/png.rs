pub const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Validates and parses PNG chunk structure.
/// Returns (file_length, is_valid) if PNG chunks lead to a verified IEND chunk.
pub fn parse_png_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 8 || &data[0..8] != &PNG_MAGIC {
        return None;
    }

    let mut offset = 8;
    let mut has_ihdr = false;
    let mut has_idat = false;
    let mut chunk_count = 0;

    while offset + 12 <= data.len() {
        let chunk_len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let chunk_type = &data[offset + 4..offset + 8];

        // Sanity check chunk type: ASCII letters only
        if !chunk_type.iter().all(|&b| b.is_ascii_alphabetic()) {
            break;
        }

        let total_chunk_len = 12 + chunk_len; // 4 len + 4 type + data + 4 crc
        if offset + total_chunk_len > data.len() {
            // Incomplete chunk
            break;
        }

        let _stored_crc = u32::from_be_bytes(
            data[offset + 8 + chunk_len..offset + total_chunk_len]
                .try_into()
                .unwrap(),
        );

        if chunk_count == 0 {
            if chunk_type != b"IHDR" || chunk_len != 13 {
                return None;
            }
            has_ihdr = true;
        }

        if chunk_type == b"IDAT" {
            has_idat = true;
        }

        if chunk_type == b"IEND" {
            let file_size = offset + total_chunk_len;
            let is_valid = has_ihdr && (has_idat || chunk_count >= 1);
            return Some((file_size as u64, is_valid));
        }

        offset += total_chunk_len;
        chunk_count += 1;
    }

    // If stream broke before IEND, search for IEND pattern within forward range (up to 8MB)
    let iend_pattern = [0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82];
    let limit = (offset + 8 * 1024 * 1024).min(data.len());
    for i in offset..limit.saturating_sub(12) {
        if &data[i..i + 12] == &iend_pattern {
            let file_size = (i + 12) as u64;
            return Some((file_size, has_ihdr && has_idat));
        }
    }

    None
}
