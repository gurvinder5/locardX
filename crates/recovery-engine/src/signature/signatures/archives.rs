/// Parses 7-Zip (7z) archive header.
/// Returns (file_length, is_valid)
pub fn parse_seven_zip_stream(data: &[u8]) -> Option<(u64, bool)> {
    // 7z signature: 37 7A BC AF 27 1C (6 bytes) + version (2 bytes) + StartHeader CRC (4 bytes) + NextHeaderOffset (8 bytes) + NextHeaderSize (8 bytes) + NextHeaderCRC (4 bytes) = 32 bytes
    if data.len() < 32 || &data[0..6] != &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C] {
        return None;
    }

    let next_header_offset = u64::from_le_bytes(data[12..20].try_into().unwrap());
    let next_header_size = u64::from_le_bytes(data[20..28].try_into().unwrap());

    let total_size = 32u64.saturating_add(next_header_offset).saturating_add(next_header_size);
    let is_valid = total_size >= 32;
    let final_len = total_size.min(data.len() as u64);

    Some((final_len, is_valid))
}

/// Parses RAR archive structure (v4.x and v5.0).
/// Returns (file_length, is_valid)
pub fn parse_rar_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 7 {
        return None;
    }

    let is_v4 = &data[0..7] == &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x00];
    let is_v5 = data.len() >= 8 && &data[0..8] == &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00];

    if !is_v4 && !is_v5 {
        return None;
    }

    // Estimate file size from slice
    let final_len = (data.len() as u64).min(200 * 1024 * 1024);
    Some((final_len, true))
}
