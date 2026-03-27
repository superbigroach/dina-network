use sha2::{Digest, Sha256};
use tracing::debug;

use crate::cctp::CctpMessage;

/// Verify a CCTP attestation from Circle against the encoded message.
///
/// In production, this would verify an ECDSA signature from Circle's attester
/// key set against the hash of the encoded CCTP message. For now, we
/// implement a SHA-256-based HMAC-style check that validates the attestation
/// is structurally correct and matches the message content.
///
/// The attestation format expected is:
///   attestation = SHA-256(encoded_message || "CCTP_ATTESTATION_V2")
///
/// This will be replaced with real ECDSA verification once Dina is registered
/// as a CCTP domain with Circle.
pub fn verify_cctp_attestation(message: &CctpMessage, attestation: &[u8]) -> bool {
    if attestation.len() < 32 {
        debug!(
            attestation_len = attestation.len(),
            "attestation too short, need at least 32 bytes"
        );
        return false;
    }

    let encoded = encode_cctp_message(message);
    let expected = compute_attestation_hash(&encoded);

    // Compare the first 32 bytes of the attestation against our expected hash.
    // In production this would be an ECDSA signature verification.
    if attestation.len() >= 32 && attestation[..32] == expected {
        debug!(
            nonce = message.nonce,
            source_domain = message.source_domain,
            "attestation verified successfully"
        );
        true
    } else {
        debug!(
            nonce = message.nonce,
            source_domain = message.source_domain,
            "attestation hash mismatch"
        );
        false
    }
}

/// Encode a CCTP message into its canonical byte representation.
///
/// The encoding follows Circle's CCTP message format:
///   [version:4][source_domain:4][dest_domain:4][nonce:8]
///   [sender:32][recipient:32][dest_caller:32][body_len:4][body:...]
pub fn encode_cctp_message(msg: &CctpMessage) -> Vec<u8> {
    let body_len = msg.message_body.len() as u32;
    let total_len = 4 + 4 + 4 + 8 + 32 + 32 + 32 + 4 + msg.message_body.len();
    let mut buf = Vec::with_capacity(total_len);

    buf.extend_from_slice(&msg.version.to_be_bytes());
    buf.extend_from_slice(&msg.source_domain.to_be_bytes());
    buf.extend_from_slice(&msg.destination_domain.to_be_bytes());
    buf.extend_from_slice(&msg.nonce.to_be_bytes());
    buf.extend_from_slice(&msg.sender);
    buf.extend_from_slice(&msg.recipient);
    buf.extend_from_slice(&msg.destination_caller);
    buf.extend_from_slice(&body_len.to_be_bytes());
    buf.extend_from_slice(&msg.message_body);

    buf
}

/// Decode a CCTP message from its canonical byte representation.
///
/// Returns an error if the data is too short or structurally invalid.
pub fn decode_cctp_message(data: &[u8]) -> Result<CctpMessage, DecodeError> {
    // Minimum size: 4 + 4 + 4 + 8 + 32 + 32 + 32 + 4 = 120 bytes (with empty body).
    const MIN_HEADER_SIZE: usize = 4 + 4 + 4 + 8 + 32 + 32 + 32 + 4;

    if data.len() < MIN_HEADER_SIZE {
        return Err(DecodeError::TooShort {
            expected: MIN_HEADER_SIZE,
            got: data.len(),
        });
    }

    let mut offset = 0;

    let version = read_u32(data, &mut offset);
    let source_domain = read_u32(data, &mut offset);
    let destination_domain = read_u32(data, &mut offset);
    let nonce = read_u64(data, &mut offset);
    let sender = read_bytes32(data, &mut offset);
    let recipient = read_bytes32(data, &mut offset);
    let destination_caller = read_bytes32(data, &mut offset);
    let body_len = read_u32(data, &mut offset) as usize;

    if data.len() < offset + body_len {
        return Err(DecodeError::TooShort {
            expected: offset + body_len,
            got: data.len(),
        });
    }

    let message_body = data[offset..offset + body_len].to_vec();

    Ok(CctpMessage {
        version,
        source_domain,
        destination_domain,
        nonce,
        sender,
        recipient,
        destination_caller,
        message_body,
    })
}

/// Compute the attestation hash for a given encoded message.
/// This is the value that Circle's attester would sign in production.
fn compute_attestation_hash(encoded_message: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(encoded_message);
    hasher.update(b"CCTP_ATTESTATION_V2");
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Create a valid attestation for a CCTP message (for testing and testnet use).
///
/// In production, only Circle's attester service can create valid attestations.
/// This function is provided for testnet environments and integration testing.
pub fn create_test_attestation(message: &CctpMessage) -> Vec<u8> {
    let encoded = encode_cctp_message(message);
    let hash = compute_attestation_hash(&encoded);
    hash.to_vec()
}

// ---------------------------------------------------------------------------
// Decode helpers
// ---------------------------------------------------------------------------

/// Errors that can occur when decoding a CCTP message.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("message too short: expected at least {expected} bytes, got {got}")]
    TooShort { expected: usize, got: usize },
}

fn read_u32(data: &[u8], offset: &mut usize) -> u32 {
    let val = u32::from_be_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
    ]);
    *offset += 4;
    val
}

fn read_u64(data: &[u8], offset: &mut usize) -> u64 {
    let val = u64::from_be_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
        data[*offset + 4],
        data[*offset + 5],
        data[*offset + 6],
        data[*offset + 7],
    ]);
    *offset += 8;
    val
}

fn read_bytes32(data: &[u8], offset: &mut usize) -> [u8; 32] {
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&data[*offset..*offset + 32]);
    *offset += 32;
    arr
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_message() -> CctpMessage {
        // Build a message body with a 1 USDC amount encoded in 32 bytes.
        let mut body = vec![0u8; 32];
        let amount = 1_000_000u64.to_be_bytes();
        body[24..32].copy_from_slice(&amount);

        CctpMessage {
            version: 2,
            source_domain: 6, // Base
            destination_domain: 99, // Dina
            nonce: 42,
            sender: [0x11; 32],
            recipient: [0x22; 32],
            destination_caller: [0u8; 32],
            message_body: body,
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let msg = sample_message();
        let encoded = encode_cctp_message(&msg);
        let decoded = decode_cctp_message(&encoded).unwrap();

        assert_eq!(msg.version, decoded.version);
        assert_eq!(msg.source_domain, decoded.source_domain);
        assert_eq!(msg.destination_domain, decoded.destination_domain);
        assert_eq!(msg.nonce, decoded.nonce);
        assert_eq!(msg.sender, decoded.sender);
        assert_eq!(msg.recipient, decoded.recipient);
        assert_eq!(msg.destination_caller, decoded.destination_caller);
        assert_eq!(msg.message_body, decoded.message_body);
    }

    #[test]
    fn decode_too_short_fails() {
        let data = vec![0u8; 10];
        let result = decode_cctp_message(&data);
        assert!(matches!(result, Err(DecodeError::TooShort { .. })));
    }

    #[test]
    fn decode_body_truncated_fails() {
        let msg = sample_message();
        let mut encoded = encode_cctp_message(&msg);
        // Truncate the body portion.
        encoded.truncate(encoded.len() - 10);
        let result = decode_cctp_message(&encoded);
        assert!(matches!(result, Err(DecodeError::TooShort { .. })));
    }

    #[test]
    fn verify_valid_attestation() {
        let msg = sample_message();
        let attestation = create_test_attestation(&msg);
        assert!(verify_cctp_attestation(&msg, &attestation));
    }

    #[test]
    fn verify_invalid_attestation() {
        let msg = sample_message();
        let bad_attestation = vec![0xffu8; 32];
        assert!(!verify_cctp_attestation(&msg, &bad_attestation));
    }

    #[test]
    fn verify_short_attestation() {
        let msg = sample_message();
        let short = vec![0u8; 16];
        assert!(!verify_cctp_attestation(&msg, &short));
    }

    #[test]
    fn modified_message_fails_attestation() {
        let msg = sample_message();
        let attestation = create_test_attestation(&msg);

        // Modify the nonce.
        let mut tampered = msg.clone();
        tampered.nonce = 999;
        assert!(!verify_cctp_attestation(&tampered, &attestation));
    }

    #[test]
    fn encode_empty_body() {
        let msg = CctpMessage {
            version: 1,
            source_domain: 0,
            destination_domain: 99,
            nonce: 0,
            sender: [0; 32],
            recipient: [0; 32],
            destination_caller: [0; 32],
            message_body: vec![],
        };

        let encoded = encode_cctp_message(&msg);
        let decoded = decode_cctp_message(&encoded).unwrap();
        assert!(decoded.message_body.is_empty());
    }

    #[test]
    fn encode_large_body() {
        let msg = CctpMessage {
            version: 2,
            source_domain: 6,
            destination_domain: 99,
            nonce: 1,
            sender: [0xaa; 32],
            recipient: [0xbb; 32],
            destination_caller: [0; 32],
            message_body: vec![0xcc; 1024],
        };

        let encoded = encode_cctp_message(&msg);
        let decoded = decode_cctp_message(&encoded).unwrap();
        assert_eq!(decoded.message_body.len(), 1024);
        assert_eq!(decoded.message_body, vec![0xcc; 1024]);
    }
}
