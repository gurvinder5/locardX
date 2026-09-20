/// Validates and parses JPEG structure from a byte slice.
/// Returns (file_length, is_valid) if a valid JPEG stream is detected.
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
    let mut has_dht = false;

    // Phase 1: Parse header markers until SOS (0xDA)
    while offset + 1 < data.len() {
        if data[offset] != 0xFF {
            match data[offset..].iter().position(|&b| b == 0xFF) {
                Some(pos) => offset += pos,
                None => break,
            }
        }

        while offset < data.len() && data[offset] == 0xFF {
            offset += 1;
        }

        if offset >= data.len() {
            break;
        }

        let marker = data[offset];
        offset += 1;

        if marker == 0x00 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }

        if marker == 0xD8 {
            continue;
        }

        if marker == 0xD9 {
            let is_valid = has_sof || has_dqt || has_dht || offset > 20;
            return Some((offset as u64, is_valid));
        }

        if offset + 2 > data.len() {
            break;
        }

        let payload_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        if payload_len < 2 || offset + payload_len > data.len() {
            break;
        }

        if marker == 0xC0 || marker == 0xC1 || marker == 0xC2 || marker == 0xC3 {
            has_sof = true;
        } else if marker == 0xDB {
            has_dqt = true;
        } else if marker == 0xC4 {
            has_dht = true;
        } else if marker == 0xDA {
            // SOS (Start of Scan) reached - advance past SOS header and enter Phase 2
            offset += payload_len;
            break;
        }

        let next_marker_pos = offset + payload_len;
        let is_misaligned = if next_marker_pos < data.len() {
            data[next_marker_pos] != 0xFF
        } else {
            next_marker_pos > data.len() || (data.len() >= 2 && data[data.len() - 2] == 0xFF && data[data.len() - 1] == 0xD9)
        };

        if is_misaligned {
            let search_end = next_marker_pos.min(data.len());
            if let Some(pos) = data[offset..search_end].iter().position(|&b| b == 0xFF) {
                offset += pos;
                continue;
            }
        }

        offset += payload_len;
    }

    if !has_sof {
        return None;
    }

    // Phase 2: Scan entropy-coded image data to find genuine EOI (FF D9)
    let mut scan_offset = offset;
    while scan_offset + 1 < data.len() {
        if data[scan_offset] != 0xFF {
            match data[scan_offset..].iter().position(|&b| b == 0xFF) {
                Some(pos) => scan_offset += pos,
                None => break,
            }
        }

        let mut marker_pos = scan_offset;
        while marker_pos < data.len() && data[marker_pos] == 0xFF {
            marker_pos += 1;
        }

        if marker_pos >= data.len() {
            break;
        }

        let marker = data[marker_pos];
        let next_pos = marker_pos + 1;

        if marker == 0x00 {
            scan_offset = next_pos;
            continue;
        }

        if (0xD0..=0xD7).contains(&marker) {
            scan_offset = next_pos;
            continue;
        }

        if marker == 0xD9 {
            let file_size = next_pos as u64;
            let is_valid = has_sof && (has_dqt || has_dht || file_size > 512);
            return Some((file_size, is_valid));
        }

        if (marker == 0xC4 || marker == 0xDB || marker == 0xDD || marker == 0xDA)
            && next_pos + 2 <= data.len()
        {
            let seg_len = u16::from_be_bytes([data[next_pos], data[next_pos + 1]]) as usize;
            if seg_len >= 2 && next_pos + seg_len <= data.len() {
                scan_offset = next_pos + seg_len;
                continue;
            }
        }

        scan_offset = next_pos;
    }

    // Forward bounded scan only
    let forward_limit = (offset + 8 * 1024 * 1024).min(data.len());
    for i in offset..forward_limit.saturating_sub(1) {
        if data[i] == 0xFF && data[i + 1] == 0xD9 {
            let file_size = (i + 2) as u64;
            let is_valid = has_sof && (has_dqt || has_dht);
            return Some((file_size, is_valid));
        }
    }

    None
}

