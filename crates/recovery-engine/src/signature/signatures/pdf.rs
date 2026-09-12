pub const PDF_MAGIC: [u8; 4] = [0x25, 0x50, 0x44, 0x46]; // %PDF

/// Validates and parses PDF structure.
/// Returns (file_length, is_valid) by finding the final %%EOF trailer.
pub fn parse_pdf_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 10 || &data[0..4] != &PDF_MAGIC {
        return None;
    }

    // PDF files may have multiple %%EOF due to incremental revisions.
    // We search backwards from the end of available data for %%EOF.
    let eof_marker = b"%%EOF";
    let mut last_eof_end = None;

    for i in (0..=data.len().saturating_sub(5)).rev() {
        if &data[i..i + 5] == eof_marker {
            let mut end = i + 5;
            // Include trailing whitespace / newlines
            while end < data.len()
                && (data[end] == b'\r' || data[end] == b'\n' || data[end] == b' ')
            {
                end += 1;
            }
            last_eof_end = Some(end);
            break;
        }
    }

    if let Some(end_offset) = last_eof_end {
        // Check for PDF internal structures: trailer, xref, /Root
        let slice = &data[..end_offset];
        let has_trailer = slice.windows(7).any(|w| w == b"trailer");
        let has_xref =
            slice.windows(4).any(|w| w == b"xref") || slice.windows(9).any(|w| w == b"startxref");
        let has_root = slice.windows(5).any(|w| w == b"/Root");

        let is_valid = has_trailer || has_xref || has_root;
        return Some((end_offset as u64, is_valid));
    }

    None
}
