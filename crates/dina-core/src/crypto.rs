use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::types::{Address, Hash};

/// Compute the SHA-256 hash of arbitrary bytes.
pub fn hash_bytes(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    Hash(bytes)
}

/// Generate a new Ed25519 keypair using the OS random number generator.
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Sign a message with an Ed25519 signing key and return the 64-byte signature.
pub fn sign(signing_key: &SigningKey, message: &[u8]) -> [u8; 64] {
    let signature = signing_key.sign(message);
    signature.to_bytes()
}

/// Verify an Ed25519 signature against a message and verifying key.
pub fn verify(verifying_key: &VerifyingKey, message: &[u8], signature: &[u8; 64]) -> bool {
    let sig = Signature::from_bytes(signature);
    verifying_key.verify(message, &sig).is_ok()
}

/// Derive a Dina address from an Ed25519 public (verifying) key.
pub fn address_from_pubkey(pubkey: &VerifyingKey) -> Address {
    Address::from_pubkey(pubkey)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_bytes_deterministic() {
        let h1 = hash_bytes(b"hello dina");
        let h2 = hash_bytes(b"hello dina");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_bytes_different_input() {
        let h1 = hash_bytes(b"hello");
        let h2 = hash_bytes(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn sign_and_verify() {
        let (sk, vk) = generate_keypair();
        let msg = b"test message";
        let sig = sign(&sk, msg);
        assert!(verify(&vk, msg, &sig));
    }

    #[test]
    fn verify_rejects_wrong_message() {
        let (sk, vk) = generate_keypair();
        let sig = sign(&sk, b"correct message");
        assert!(!verify(&vk, b"wrong message", &sig));
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let (sk, _) = generate_keypair();
        let (_, wrong_vk) = generate_keypair();
        let sig = sign(&sk, b"message");
        assert!(!verify(&wrong_vk, b"message", &sig));
    }

    #[test]
    fn address_from_pubkey_consistent() {
        let (_, vk) = generate_keypair();
        let a1 = address_from_pubkey(&vk);
        let a2 = address_from_pubkey(&vk);
        assert_eq!(a1, a2);
    }
}
