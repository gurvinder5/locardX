use super::registry::{FileSignature, SignatureRegistry};
use crate::models::FileType;

/// Discovered potential file header during byte scanning.
#[derive(Debug, Clone)]
pub struct CandidateHeader {
    pub global_offset: u64,
    pub file_type: FileType,
    pub signature: FileSignature,
}

/// Scans raw data buffers for registered file magic headers using a 256-way first-byte dispatch table.
#[derive(Debug, Clone)]
pub struct SignatureDetector {
    #[allow(dead_code)]
    registry: SignatureRegistry,
    first_byte_table: Box<[Vec<FileSignature>; 256]>,
}

impl SignatureDetector {
    pub fn new(registry: SignatureRegistry) -> Self {
        // Construct fixed 256-bucket dispatch array for O(1) L1-cache friendly first-byte lookup
        let mut table: Vec<Vec<FileSignature>> = (0..256).map(|_| Vec::new()).collect();
        for sig in registry.all() {
            if !sig.header_magic.is_empty() {
                let first = sig.header_magic[0] as usize;
                table[first].push(sig.clone());
            }
        }
        let boxed_slice = table.into_boxed_slice();
        let first_byte_table: Box<[Vec<FileSignature>; 256]> = match boxed_slice.try_into() {
            Ok(arr) => arr,
            Err(_) => unreachable!("Table slice length is strictly 256"),
        };
        Self {
            registry,
            first_byte_table,
        }
    }

    /// Scans a buffer at a given base global offset in a single linear pass, yielding all detected headers.
    pub fn scan_buffer(&self, buffer: &[u8], base_offset: u64) -> Vec<CandidateHeader> {
        let mut candidates = Vec::with_capacity(32);
        let buf_len = buffer.len();
        let mut pos = 0;

        while pos < buf_len {
            let b = buffer[pos] as usize;
            let sigs = &self.first_byte_table[b];
            if !sigs.is_empty() {
                for sig in sigs {
                    let magic = &sig.header_magic;
                    if pos + magic.len() <= buf_len
                        && &buffer[pos..pos + magic.len()] == magic.as_slice()
                    {
                        let file_start_offset = if pos >= sig.header_offset {
                            base_offset + (pos - sig.header_offset) as u64
                        } else if base_offset >= sig.header_offset as u64 {
                            (base_offset + pos as u64) - sig.header_offset as u64
                        } else {
                            base_offset + pos as u64
                        };

                        candidates.push(CandidateHeader {
                            global_offset: file_start_offset,
                            file_type: sig.file_type.clone(),
                            signature: sig.clone(),
                        });
                    }
                }
            }
            pos += 1;
        }

        candidates
    }
}
