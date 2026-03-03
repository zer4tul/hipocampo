//! Hashing utilities

use sha2::{Digest, Sha256};

/// Compute SHA-256 hash of text (truncated to 16 chars)
pub fn content_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

/// Compute composite chunk ID (compatible with OpenClaw format)
pub fn chunk_id(source: &str, start_line: usize, end_line: usize, content_hash: &str, model: &str) -> String {
    let composite = format!("markdown:{}:{}:{}:{}:{}", source, start_line, end_line, content_hash, model);
    let mut hasher = Sha256::new();
    hasher.update(composite.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_consistent() {
        let hash1 = content_hash("test content");
        let hash2 = content_hash("test content");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn chunk_id_format() {
        let id = chunk_id("/path/to/file.md", 1, 10, "abc123", "text-embedding-3-small");
        assert_eq!(id.len(), 32);
    }
}
