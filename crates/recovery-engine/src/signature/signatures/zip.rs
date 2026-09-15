use crate::models::FileType;

pub const ZIP_MAGIC: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
pub const CD_MAGIC: [u8; 4] = [0x50, 0x4B, 0x01, 0x02];
pub const EOCD_MAGIC: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];

/// Parses a ZIP archive stream, locating the End of Central Directory (EOCD).
/// Returns (file_length, is_valid, resolved_file_type).
pub fn parse_zip_stream(data: &[u8]) -> Option<(u64, bool, FileType)> {
    if data.len() < 22 || &data[0..4] != &ZIP_MAGIC {
        return None;
    }

    // Scan forward for EOCD magic marker PK\x05\x06
    let limit = data.len().saturating_sub(21);
    let mut best_eocd = None;

    let mut pos = 0;
    while pos < limit {
        if &data[pos..pos + 4] == &EOCD_MAGIC {
            let cd_size = u32::from_le_bytes(data[pos + 12..pos + 16].try_into().unwrap()) as usize;
            let cd_offset = u32::from_le_bytes(data[pos + 16..pos + 20].try_into().unwrap()) as usize;
            let comment_len = u16::from_le_bytes([data[pos + 20], data[pos + 21]]) as usize;

            // Validate that central directory offset is consistent with EOCD location
            let cd_consistent = cd_offset <= pos
                && cd_offset + cd_size <= pos
                && (cd_size == 0
                    || (cd_offset + 4 <= data.len() && &data[cd_offset..cd_offset + 4] == &CD_MAGIC));

            if cd_consistent {
                let total_len = (pos + 22 + comment_len).min(data.len());
                best_eocd = Some((total_len, true));
                // First structurally valid EOCD that correlates with CD
                break;
            } else if best_eocd.is_none() {
                let total_len = (pos + 22 + comment_len).min(data.len());
                best_eocd = Some((total_len, false));
            }
            pos += 4;
        } else {
            pos += 1;
        }
    }

    if let Some((final_len, is_struct_valid)) = best_eocd {
        let slice = &data[..final_len];
        let resolved_type = if slice.windows(5).any(|w| w == b"word/") {
            FileType::OfficeDocx
        } else if slice.windows(3).any(|w| w == b"xl/") {
            FileType::OfficeXlsx
        } else if slice.windows(4).any(|w| w == b"ppt/") {
            FileType::OfficePptx
        } else if slice.windows(19).any(|w| w == b"[Content_Types].xml") {
            FileType::OfficeDocx
        } else {
            FileType::Zip
        };

        return Some((final_len as u64, is_struct_valid, resolved_type));
    }

    None
}
