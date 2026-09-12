/// Validates and parses JPEG structure from a byte slice.
/// Returns (file_length, has_valid_markers) if a valid JPEG stream is detected.
pub fn parse_jpeg_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 4 {
        return None;
    }
    // Must start with SOI: FF D8
    if data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }

    let mut offset = 2;
    let mut has_sof = false;
    let mut has_dqt = false;

    while offset + 1 < data.len() {
        if data[offset] != 0xFF {
            // In entropy-coded scan data, 0xFF may be preceded by non-FF bytes.
            // Scan forward for next 0xFF.
            match data[offset..].iter().position(|&b| b == 0xFF) {
                Some(pos) => {
                    offset += pos;
                }
                None => break,
            }
        }

        // Skip fill bytes (consecutive 0xFF)
        while offset < data.len() && data[offset] == 0xFF {
            offset += 1;
        }

        if offset >= data.len() {
            break;
        }

        let marker = data[offset];
        offset += 1;

        if marker == 0x00 {
            // Escaped 0xFF in scan data
            continue;
        }

        // EOI marker: FF D9
        if marker == 0xD9 {
            let is_valid = has_sof || has_dqt || offset > 100;
            return Some((offset as u64, is_valid));
        }

        // Restart markers (RST0 - RST7: 0xD0 - 0xD7) or SOI
        if (0xD0..=0xD8).contains(&marker) {
            continue;
        }

        // Markers with variable payload length
        if offset + 2 > data.len() {
            break;
        }

        let payload_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        if payload_len < 2 {
            break;
        }

        if marker == 0xC0 || marker == 0xC2 {
            has_sof = true;
        } else if marker == 0xDB {
            has_dqt = true;
        }

        if marker == 0xDA {
            // SOS (Start of Scan) - following bytes are entropy-coded image data
            offset += payload_len;
            continue;
        }

        offset += payload_len;
    }

    // If EOI wasn't reached cleanly within the slice, check if an EOI marker exists anywhere
    if let Some(pos) = find_jpeg_eoi(data) {
        return Some((pos as u64, has_sof || has_dqt));
    }

    None
}

fn find_jpeg_eoi(data: &[u8]) -> Option<usize> {
    for i in (2..data.len().saturating_sub(1)).rev() {
        if data[i] == 0xFF && data[i + 1] == 0xD9 {
            return Some(i + 2);
        }
    }
    None
}
