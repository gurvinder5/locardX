use sha2::{Digest, Sha256};

/// Streaming SHA-256 hasher for calculating reproducible forensic image digests during acquisition.
pub struct StreamingSha256Hasher {
    hasher: Sha256,
    total_bytes: u64,
}

impl StreamingSha256Hasher {
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
            total_bytes: 0,
        }
    }

    pub fn update(&mut self, chunk: &[u8]) {
        self.hasher.update(chunk);
        self.total_bytes += chunk.len() as u64;
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub fn finalize(self) -> (String, u64) {
        let digest_bytes = self.hasher.finalize();
        let hex_str = hex::encode(digest_bytes);
        (hex_str, self.total_bytes)
    }
}

impl Default for StreamingSha256Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string_hash() {
        let hasher = StreamingSha256Hasher::new();
        let (digest, bytes) = hasher.finalize();
        assert_eq!(bytes, 0);
        assert_eq!(
            digest,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_streaming_chunks() {
        let mut hasher = StreamingSha256Hasher::new();
        hasher.update(b"hello ");
        hasher.update(b"world");
        let (digest, bytes) = hasher.finalize();
        assert_eq!(bytes, 11);
        // SHA-256 of "hello world"
        assert_eq!(
            digest,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
