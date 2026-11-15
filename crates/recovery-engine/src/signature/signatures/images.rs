/// Parses BMP (Windows Bitmap) structure.
/// Returns (file_length, is_valid)
pub fn parse_bmp_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 26 || &data[0..2] != b"BM" {
        return None;
    }

    // Reserved bytes (offsets 6..10) MUST be zero in all standard BMPs
    if data[6] != 0 || data[7] != 0 || data[8] != 0 || data[9] != 0 {
        return None;
    }

    let file_size = u32::from_le_bytes(data[2..6].try_into().unwrap()) as u64;
    let pixel_offset = u32::from_le_bytes(data[10..14].try_into().unwrap()) as u64;
    let dib_header_size = u32::from_le_bytes(data[14..18].try_into().unwrap()) as u64;

    // DIB header size must match one of the standard specifications:
    // 12 (BITMAPCOREHEADER / OS/2 1.x)
    // 40 (BITMAPINFOHEADER - 99% of all BMPs)
    // 52 (BITMAPV2HEADER)
    // 56 (BITMAPV3HEADER)
    // 64 (OS/2 2.x BITMAPCOREHEADER2)
    // 108 (BITMAPV4HEADER)
    // 124 (BITMAPV5HEADER)
    let valid_dib = matches!(dib_header_size, 12 | 40 | 52 | 56 | 64 | 108 | 124);
    if !valid_dib {
        return None;
    }

    // Pixel offset must be at least header (14) + DIB size
    if pixel_offset < 14 + dib_header_size || (file_size > 0 && pixel_offset > file_size) {
        return None;
    }

    // If planes field is present (offset 26..28), must be <= 1
    if data.len() >= 28 {
        let planes = u16::from_le_bytes([data[26], data[27]]);
        if planes > 1 {
            return None;
        }
    }

    // File size must be reasonable (at least 26 bytes and at most 100 MB)
    if file_size < 26 || file_size > 100 * 1024 * 1024 {
        return None;
    }

    Some((file_size, true))
}

/// Parses GIF structure (GIF87a / GIF89a).
/// Returns (file_length, is_valid)
pub fn parse_gif_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 14 {
        return None;
    }

    let is_87a = &data[0..6] == b"GIF87a";
    let is_89a = &data[0..6] == b"GIF89a";
    if !is_87a && !is_89a {
        return None;
    }

    let width = u16::from_le_bytes([data[6], data[7]]);
    let height = u16::from_le_bytes([data[8], data[9]]);
    if width == 0 || height == 0 {
        return None;
    }

    // Search forward for GIF trailer 0x3B
    let mut pos = 13;
    // Check Global Color Table
    let packed = data[10];
    if (packed & 0x80) != 0 {
        let gct_size = 3 * (1 << ((packed & 0x07) + 1));
        pos += gct_size;
    }

    while pos < data.len() {
        if data[pos] == 0x3B {
            // Trailer found!
            return Some(((pos + 1) as u64, true));
        }

        if data[pos] == 0x21 {
            // Extension block: 0x21, label, block size, sub-blocks, 0x00
            if pos + 2 >= data.len() {
                break;
            }
            pos += 2;
            while pos < data.len() && data[pos] != 0 {
                let block_len = data[pos] as usize;
                pos += 1 + block_len;
            }
            pos += 1; // skip terminating 0x00
        } else if data[pos] == 0x2C {
            // Image descriptor: 0x2C, 9 bytes descriptor
            if pos + 10 >= data.len() {
                break;
            }
            let img_packed = data[pos + 9];
            pos += 10;
            if (img_packed & 0x80) != 0 {
                let lct_size = 3 * (1 << ((img_packed & 0x07) + 1));
                pos += lct_size;
            }
            if pos < data.len() {
                pos += 1; // LZW minimum code size
                while pos < data.len() && data[pos] != 0 {
                    let sub_len = data[pos] as usize;
                    pos += 1 + sub_len;
                }
                pos += 1; // skip 0x00
            }
        } else {
            // Scan forward for next 0x3B trailer
            if let Some(trailer_pos) = data[pos..].iter().position(|&b| b == 0x3B) {
                return Some(((pos + trailer_pos + 1) as u64, true));
            }
            break;
        }
    }

    // Fallback: if trailer was found within slice
    if let Some(pos) = data.iter().rposition(|&b| b == 0x3B) {
        if pos >= 13 {
            return Some(((pos + 1) as u64, true));
        }
    }

    None
}

/// Parses TIFF structure (Little Endian: II*\0 or Big Endian: MM\0*).
/// Returns (file_length, is_valid)
pub fn parse_tiff_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 8 {
        return None;
    }

    let is_le = &data[0..4] == [0x49, 0x49, 0x2A, 0x00];
    let is_be = &data[0..4] == [0x4D, 0x4D, 0x00, 0x2A];
    if !is_le && !is_be {
        return None;
    }

    let ifd_offset = if is_le {
        u32::from_le_bytes(data[4..8].try_into().unwrap()) as u64
    } else {
        u32::from_be_bytes(data[4..8].try_into().unwrap()) as u64
    };

    if ifd_offset < 8 {
        return None;
    }

    // Bound TIFF size reasonably
    let final_len = (ifd_offset + 1024 * 1024).min(data.len() as u64).max(ifd_offset + 128);
    let is_valid = ifd_offset + 2 <= data.len() as u64;

    Some((final_len, is_valid))
}
