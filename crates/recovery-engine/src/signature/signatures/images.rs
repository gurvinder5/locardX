/// Parses BMP (Windows Bitmap) structure.
/// Returns (file_length, is_valid)
pub fn parse_bmp_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 26 || &data[0..2] != b"BM" {
        return None;
    }

    let file_size = u32::from_le_bytes(data[2..6].try_into().unwrap()) as u64;
    let pixel_offset = u32::from_le_bytes(data[10..14].try_into().unwrap()) as u64;
    let dib_header_size = u32::from_le_bytes(data[14..18].try_into().unwrap()) as u64;

    // Minimum valid BMP is 26 bytes (14 header + 12 OS/2 DIB)
    if file_size < 26 || pixel_offset < 14 + dib_header_size {
        return None;
    }

    let is_valid = pixel_offset <= file_size && (dib_header_size == 40 || dib_header_size == 12 || dib_header_size == 108 || dib_header_size == 124);
    let final_len = file_size.min(data.len() as u64);

    Some((final_len, is_valid))
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
    for i in (13..data.len()).rev() {
        if data[i] == 0x3B {
            return Some(((i + 1) as u64, true));
        }
    }

    None
}

/// Parses TIFF structure (II*\0 or MM\0*).
/// Returns (file_length, is_valid)
pub fn parse_tiff_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 8 {
        return None;
    }

    let is_le = &data[0..4] == &[0x49, 0x49, 0x2A, 0x00];
    let is_be = &data[0..4] == &[0x4D, 0x4D, 0x00, 0x2A];
    if !is_le && !is_be {
        return None;
    }

    let first_ifd_offset = if is_le {
        u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize
    } else {
        u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize
    };

    if first_ifd_offset < 8 || first_ifd_offset >= data.len() {
        return None;
    }

    // Estimate length from IFD and strip/tile offsets if possible, or bounded size
    let is_valid = first_ifd_offset >= 8;
    let detected_size = (data.len() as u64).min(50 * 1024 * 1024);

    Some((detected_size, is_valid))
}
