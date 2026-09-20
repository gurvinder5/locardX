pub const PDF_MAGIC: [u8; 4] = [0x25, 0x50, 0x44, 0x46]; // %PDF

/// Validates and parses PDF structure.
/// Returns (file_length, is_valid) by finding the final %%EOF trailer.
pub fn parse_pdf_stream(data: &[u8]) -> Option<(u64, bool)> {
    if data.len() < 10 || &data[0..4] != &PDF_MAGIC {
        return None;
    }

    let eof_marker = b"%%EOF";
    let mut best_eof_end = None;

    // Scan forward for %%EOF occurrences within candidate window
    let mut pos = 0;
    while pos + 5 <= data.len() {
        if &data[pos..pos + 5] == eof_marker {
            let mut end = pos + 5;
            // Include trailing whitespace / newlines
            while end < data.len() && (data[end] == b'\r' || data[end] == b'\n' || data[end] == b' ') {
                end += 1;
            }

            // Check preceding window (up to 1024 bytes) for startxref or trailer
            let lookback_start = pos.saturating_sub(1024);
            let lookback_slice = &data[lookback_start..pos];
            let has_xref_trailer = lookback_slice.windows(9).any(|w| w == b"startxref")
                || lookback_slice.windows(7).any(|w| w == b"trailer")
                || lookback_slice.windows(4).any(|w| w == b"xref");

            best_eof_end = Some((end, has_xref_trailer));

            // If we find an EOF followed by zeros or non-PDF data, or another signature, we stop
            pos = end;
        } else {
            pos += 1;
        }
    }

    if let Some((end_offset, has_xref_trailer)) = best_eof_end {
        let slice = &data[..end_offset];
        let has_trailer = slice.windows(7).any(|w| w == b"trailer");
        let has_xref =
            slice.windows(4).any(|w| w == b"xref") || slice.windows(9).any(|w| w == b"startxref");
        let has_root = slice.windows(5).any(|w| w == b"/Root");

        let is_valid = has_xref_trailer || has_trailer || has_xref || has_root;
        return Some((end_offset as u64, is_valid));
    }

    None
}
