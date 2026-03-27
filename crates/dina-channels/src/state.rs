use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::error::{ChannelError, Result};

/// A single state update within a payment channel, representing the current
/// balance distribution at a given sequence number.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateUpdate {
    pub channel_id: [u8; 32],
    pub balance_a: u64,
    pub balance_b: u64,
    pub sequence: u64,
    pub timestamp: u64,
}

/// A state update signed by both channel parties, proving mutual agreement
/// on the balance distribution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedState {
    pub state: StateUpdate,
    #[serde(with = "BigArray")]
    pub signature_a: [u8; 64],
    #[serde(with = "BigArray")]
    pub signature_b: [u8; 64],
}

/// Produce a canonical byte representation of a StateUpdate for signing.
/// Uses bincode for deterministic, compact serialization.
pub fn to_bytes(state: &StateUpdate) -> Vec<u8> {
    // We use a manual layout for determinism across platforms:
    // channel_id (32) + balance_a (8) + balance_b (8) + sequence (8) + timestamp (8) = 64 bytes
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&state.channel_id);
    buf.extend_from_slice(&state.balance_a.to_le_bytes());
    buf.extend_from_slice(&state.balance_b.to_le_bytes());
    buf.extend_from_slice(&state.sequence.to_le_bytes());
    buf.extend_from_slice(&state.timestamp.to_le_bytes());
    buf
}

/// Deserialize a StateUpdate from its canonical byte representation.
pub fn from_bytes(bytes: &[u8]) -> Result<StateUpdate> {
    if bytes.len() != 64 {
        return Err(ChannelError::SerializationError(format!(
            "expected 64 bytes, got {}",
            bytes.len()
        )));
    }

    let mut channel_id = [0u8; 32];
    channel_id.copy_from_slice(&bytes[0..32]);

    let balance_a = u64::from_le_bytes(
        bytes[32..40]
            .try_into()
            .map_err(|e| ChannelError::SerializationError(format!("{e}")))?,
    );
    let balance_b = u64::from_le_bytes(
        bytes[40..48]
            .try_into()
            .map_err(|e| ChannelError::SerializationError(format!("{e}")))?,
    );
    let sequence = u64::from_le_bytes(
        bytes[48..56]
            .try_into()
            .map_err(|e| ChannelError::SerializationError(format!("{e}")))?,
    );
    let timestamp = u64::from_le_bytes(
        bytes[56..64]
            .try_into()
            .map_err(|e| ChannelError::SerializationError(format!("{e}")))?,
    );

    Ok(StateUpdate {
        channel_id,
        balance_a,
        balance_b,
        sequence,
        timestamp,
    })
}

/// Sign a state update with an Ed25519 signing key, returning a 64-byte signature.
pub fn sign(state: &StateUpdate, signing_key: &SigningKey) -> [u8; 64] {
    let message = to_bytes(state);
    let signature = signing_key.sign(&message);
    signature.to_bytes()
}

/// Verify a signature against a state update and an Ed25519 public key.
pub fn verify(state: &StateUpdate, pubkey: &[u8; 32], signature: &[u8; 64]) -> bool {
    let verifying_key = match VerifyingKey::from_bytes(pubkey) {
        Ok(vk) => vk,
        Err(_) => return false,
    };
    let sig = Signature::from_bytes(signature);
    let message = to_bytes(state);
    verifying_key.verify(&message, &sig).is_ok()
}

/// Validate that a SignedState carries valid signatures from both channel parties.
pub fn is_valid(signed: &SignedState, party_a: &[u8; 32], party_b: &[u8; 32]) -> bool {
    verify(&signed.state, party_a, &signed.signature_a)
        && verify(&signed.state, party_b, &signed.signature_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn test_keys() -> (SigningKey, SigningKey) {
        let key_a = SigningKey::from_bytes(&[1u8; 32]);
        let key_b = SigningKey::from_bytes(&[2u8; 32]);
        (key_a, key_b)
    }

    fn test_state() -> StateUpdate {
        StateUpdate {
            channel_id: [0xAA; 32],
            balance_a: 500_000,
            balance_b: 500_000,
            sequence: 1,
            timestamp: 1700000000,
        }
    }

    #[test]
    fn to_bytes_from_bytes_roundtrip() {
        let state = test_state();
        let bytes = to_bytes(&state);
        assert_eq!(bytes.len(), 64);
        let recovered = from_bytes(&bytes).unwrap();
        assert_eq!(state, recovered);
    }

    #[test]
    fn from_bytes_wrong_length() {
        assert!(from_bytes(&[0u8; 63]).is_err());
        assert!(from_bytes(&[0u8; 65]).is_err());
    }

    #[test]
    fn sign_and_verify_valid() {
        let (key_a, _) = test_keys();
        let state = test_state();
        let sig = sign(&state, &key_a);
        let pubkey = key_a.verifying_key().to_bytes();
        assert!(verify(&state, &pubkey, &sig));
    }

    #[test]
    fn verify_wrong_key_fails() {
        let (key_a, key_b) = test_keys();
        let state = test_state();
        let sig = sign(&state, &key_a);
        let wrong_pubkey = key_b.verifying_key().to_bytes();
        assert!(!verify(&state, &wrong_pubkey, &sig));
    }

    #[test]
    fn is_valid_both_signatures() {
        let (key_a, key_b) = test_keys();
        let state = test_state();
        let sig_a = sign(&state, &key_a);
        let sig_b = sign(&state, &key_b);

        let signed = SignedState {
            state,
            signature_a: sig_a,
            signature_b: sig_b,
        };

        let pub_a = key_a.verifying_key().to_bytes();
        let pub_b = key_b.verifying_key().to_bytes();
        assert!(is_valid(&signed, &pub_a, &pub_b));
    }

    #[test]
    fn is_valid_fails_with_swapped_signatures() {
        let (key_a, key_b) = test_keys();
        let state = test_state();
        let sig_a = sign(&state, &key_a);
        let sig_b = sign(&state, &key_b);

        // Swap the signatures
        let signed = SignedState {
            state,
            signature_a: sig_b,
            signature_b: sig_a,
        };

        let pub_a = key_a.verifying_key().to_bytes();
        let pub_b = key_b.verifying_key().to_bytes();
        assert!(!is_valid(&signed, &pub_a, &pub_b));
    }
}
