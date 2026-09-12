use sha2::{Digest, Sha256};

/// Computes the SHA-256 hexadecimal digest for a recovered byte slice.
pub fn compute_candidate_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Checks whether data size falls within plausible boundaries for forensic carving.
pub fn is_size_plausible(size: u64, min_size: u64, max_size: u64) -> bool {
    size >= min_size && size <= max_size
}
