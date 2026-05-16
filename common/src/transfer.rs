//! Binary chunk framing for transferring large payloads (e.g. Docker image
//! tarballs) over the agent WebSocket without base64 overhead.
//!
//! A chunk frame is a raw binary WebSocket message laid out as:
//!
//! ```text
//! [ magic(4) | transfer_id(16) | chunk_index(4) | total_chunks(4) | sha256(32) | payload(..) ]
//! ```
//!
//! All integers are big-endian. `magic` identifies an Icefall chunk frame so
//! it can be distinguished from any other binary traffic. `sha256` is the
//! digest of `payload` only, letting the receiver verify and re-request a
//! single corrupt chunk.

/// Magic prefix identifying an Icefall image-transfer chunk frame: "IFCK".
pub const CHUNK_MAGIC: [u8; 4] = *b"IFCK";

/// Fixed header size: magic(4) + transfer_id(16) + chunk_index(4)
/// + total_chunks(4) + sha256(32).
pub const CHUNK_HEADER_LEN: usize = 4 + 16 + 4 + 4 + 32;

#[derive(Debug, Clone)]
pub struct ChunkFrame {
    pub transfer_id: [u8; 16],
    pub chunk_index: u32,
    pub total_chunks: u32,
    /// SHA-256 of `payload`.
    pub sha256: [u8; 32],
    pub payload: Vec<u8>,
}

impl ChunkFrame {
    /// Serialize this frame to a binary WebSocket message body.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(CHUNK_HEADER_LEN + self.payload.len());
        buf.extend_from_slice(&CHUNK_MAGIC);
        buf.extend_from_slice(&self.transfer_id);
        buf.extend_from_slice(&self.chunk_index.to_be_bytes());
        buf.extend_from_slice(&self.total_chunks.to_be_bytes());
        buf.extend_from_slice(&self.sha256);
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Parse a binary WebSocket message body into a chunk frame. Returns `None`
    /// if the bytes are not a well-formed Icefall chunk frame.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < CHUNK_HEADER_LEN || bytes[0..4] != CHUNK_MAGIC {
            return None;
        }
        let mut transfer_id = [0u8; 16];
        transfer_id.copy_from_slice(&bytes[4..20]);
        let chunk_index = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
        let total_chunks = u32::from_be_bytes(bytes[24..28].try_into().ok()?);
        let mut sha256 = [0u8; 32];
        sha256.copy_from_slice(&bytes[28..60]);
        let payload = bytes[CHUNK_HEADER_LEN..].to_vec();
        Some(Self {
            transfer_id,
            chunk_index,
            total_chunks,
            sha256,
            payload,
        })
    }

    /// True if `payload`'s SHA-256 matches the frame's declared digest.
    pub fn verify(&self) -> bool {
        sha256_of(&self.payload) == self.sha256
    }
}

/// Compute the SHA-256 digest of `data`.
pub fn sha256_of(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Lowercase hex encoding of a 32-byte digest.
pub fn sha256_hex(digest: &[u8; 32]) -> String {
    hex_encode(digest)
}

/// Lowercase hex encoding of an arbitrary byte slice.
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ChunkFrame {
        ChunkFrame {
            transfer_id: [7u8; 16],
            chunk_index: 3,
            total_chunks: 10,
            sha256: sha256_of(b"hello world"),
            payload: b"hello world".to_vec(),
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let frame = sample();
        let decoded = ChunkFrame::decode(&frame.encode()).expect("decodes");
        assert_eq!(decoded.transfer_id, frame.transfer_id);
        assert_eq!(decoded.chunk_index, 3);
        assert_eq!(decoded.total_chunks, 10);
        assert_eq!(decoded.payload, b"hello world");
        assert_eq!(decoded.sha256, frame.sha256);
    }

    #[test]
    fn verify_passes_for_matching_payload() {
        assert!(sample().verify());
    }

    #[test]
    fn verify_fails_for_corrupted_payload() {
        let mut frame = sample();
        frame.payload[0] ^= 0xff;
        assert!(!frame.verify());
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let mut bytes = sample().encode();
        bytes[0] = b'X';
        assert!(ChunkFrame::decode(&bytes).is_none());
    }

    #[test]
    fn decode_rejects_short_input() {
        assert!(ChunkFrame::decode(&[1, 2, 3]).is_none());
    }

    #[test]
    fn sha256_hex_is_64_chars() {
        assert_eq!(sha256_hex(&sha256_of(b"x")).len(), 64);
    }
}
