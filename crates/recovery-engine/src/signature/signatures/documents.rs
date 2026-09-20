use crate::models::FileType;

/// Magic signature for Compound File Binary Format (CFBF) used by legacy MS Office (.doc, .xls, .ppt).
pub const CFBF_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// Parses Compound File Binary Format (CFBF) streams and identifies document type.
/// Returns (file_length, is_valid, resolved_file_type)
pub fn parse_cfbf_stream(data: &[u8]) -> Option<(u64, bool, FileType)> {
    if data.len() < 512 || &data[0..8] != &CFBF_MAGIC {
        return None;
    }

    let sector_shift = u16::from_le_bytes([data[30], data[31]]) as usize;
    if sector_shift < 9 || sector_shift > 12 {
        // Must be 512 (shift 9) or 4096 (shift 12)
        return None;
    }
    let sector_size = 1 << sector_shift;

    let dir_sector_count = u32::from_le_bytes(data[40..44].try_into().unwrap()) as u64;
    let fat_sector_count = u32::from_le_bytes(data[44..48].try_into().unwrap()) as u64;

    // Scan the candidate slice to determine specific Office type
    let search_window = data.len().min(64 * 1024);
    let slice = &data[..search_window];

    let file_type = if slice.windows(12).any(|w| w == b"WordDocument") {
        FileType::OfficeDoc
    } else if slice.windows(8).any(|w| w == b"Workbook") || slice.windows(4).any(|w| w == b"Book") {
        FileType::OfficeXls
    } else if slice.windows(16).any(|w| w == b"PowerPoint Document") {
        FileType::OfficePpt
    } else {
        FileType::OfficeDoc
    };

    let min_expected_size = (fat_sector_count + dir_sector_count + 1) * sector_size as u64;
    let detected_size = if min_expected_size > 512 && min_expected_size <= data.len() as u64 {
        min_expected_size
    } else {
        data.len() as u64
    };

    Some((detected_size.min(100 * 1024 * 1024), true, file_type))
}

/// Parses RTF (Rich Text Format) document stream by balancing '{' and '}' braces.
/// Returns (file_length, is_valid)
pub fn parse_rtf_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 10 || &data[0..5] != b"{\\rtf" {
        return None;
    }

    let mut depth: i32 = 0;
    let mut end_pos = None;

    for (i, &b) in data.iter().enumerate() {
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                end_pos = Some(i + 1);
                break;
            }
        }
    }

    if let Some(pos) = end_pos {
        Some((pos as u64, true))
    } else {
        // Fallback: look for last '}'
        for i in (5..data.len()).rev() {
            if data[i] == b'}' {
                return Some(((i + 1) as u64, false));
            }
        }
        None
    }
}
