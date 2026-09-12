use super::registry::{FileSignature, SignatureRegistry};
use crate::models::FileType;

/// Discovered potential file header during byte scanning.
#[derive(Debug, Clone)]
pub struct CandidateHeader {
    pub global_offset: u64,
    pub file_type: FileType,
    pub signature: FileSignature,
}

/// Scans raw data buffers for registered file magic headers.
#[derive(Debug, Clone)]
pub struct SignatureDetector {
    registry: SignatureRegistry,
}

impl SignatureDetector {
    pub fn new(registry: SignatureRegistry) -> Self {
        Self { registry }
    }

    /// Scans a buffer at a given base global offset, yielding all detected headers.
    pub fn scan_buffer(&self, buffer: &[u8], base_offset: u64) -> Vec<CandidateHeader> {
        let mut candidates = Vec::new();

        for sig in self.registry.all() {
            let magic = &sig.header_magic;
            if buffer.len() < magic.len() {
                continue;
            }

            // Find occurrences of magic bytes in the buffer
            let mut pos = 0;
            while pos + magic.len() <= buffer.len() {
                if &buffer[pos..pos + magic.len()] == magic.as_slice() {
                    candidates.push(CandidateHeader {
                        global_offset: base_offset + pos as u64,
                        file_type: sig.file_type.clone(),
                        signature: sig.clone(),
                    });
                    pos += magic.len();
                } else {
                    pos += 1;
                }
            }
        }

        candidates.sort_by_key(|c| c.global_offset);
        candidates
    }
}
