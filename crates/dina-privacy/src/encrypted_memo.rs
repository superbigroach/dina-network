use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

/// Errors from encrypted memo operations.
#[derive(Debug, Error)]
pub enum MemoError {
    #[error("decryption failed: ciphertext is invalid or key is wrong")]
    DecryptionFailed,
}

/// An encrypted memo attached to a transaction.
///
/// Uses X25519 ECDH for key agreement and XChaCha20-Poly1305 for
/// authenticated encryption. The sender generates an ephemeral keypair so
/// the recipient can derive the shared secret without revealing the sender.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedMemo {
    /// The ephemeral public key the sender generated for this memo.
    pub ephemeral_pubkey: [u8; 32],
    /// The XChaCha20-Poly1305 ciphertext (includes 16-byte auth tag).
    pub ciphertext: Vec<u8>,
    /// The 24-byte nonce used for encryption.
    pub nonce: [u8; 24],
}

/// Encrypt a plaintext memo for a recipient identified by their X25519 public key.
///
/// 1. Generate an ephemeral X25519 keypair.
/// 2. Perform ECDH with the recipient's public key to get a shared secret.
/// 3. Derive a 32-byte symmetric key via SHA-256(shared_secret).
/// 4. Generate a random 24-byte nonce.
/// 5. Encrypt with XChaCha20-Poly1305.
pub fn encrypt_memo(recipient_pubkey: &[u8; 32], plaintext: &[u8]) -> EncryptedMemo {
    let recipient_pk = PublicKey::from(*recipient_pubkey);

    // Step 1: ephemeral keypair
    let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_pubkey = PublicKey::from(&ephemeral_secret);

    // Step 2: ECDH shared secret
    let shared_secret = ephemeral_secret.diffie_hellman(&recipient_pk);

    // Step 3: derive symmetric key via SHA-256
    let sym_key = derive_symmetric_key(shared_secret.as_bytes());

    // Step 4: random nonce
    let mut nonce_bytes = [0u8; 24];
    rand::Rng::fill(&mut OsRng, &mut nonce_bytes);
    let nonce = XNonce::from(nonce_bytes);

    // Step 5: encrypt
    let cipher = XChaCha20Poly1305::new((&sym_key).into());
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .expect("XChaCha20Poly1305 encryption should never fail for valid inputs");

    EncryptedMemo {
        ephemeral_pubkey: ephemeral_pubkey.to_bytes(),
        ciphertext,
        nonce: nonce_bytes,
    }
}

/// Decrypt an encrypted memo using the recipient's X25519 secret key.
///
/// 1. Perform ECDH with the ephemeral public key from the memo.
/// 2. Derive the same symmetric key via SHA-256(shared_secret).
/// 3. Decrypt with XChaCha20-Poly1305.
pub fn decrypt_memo(
    recipient_secret: &[u8; 32],
    memo: &EncryptedMemo,
) -> Result<Vec<u8>, MemoError> {
    let secret = StaticSecret::from(*recipient_secret);
    let ephemeral_pk = PublicKey::from(memo.ephemeral_pubkey);

    // ECDH
    let shared_secret = secret.diffie_hellman(&ephemeral_pk);
    let sym_key = derive_symmetric_key(shared_secret.as_bytes());

    // Decrypt
    let nonce = XNonce::from(memo.nonce);
    let cipher = XChaCha20Poly1305::new((&sym_key).into());
    cipher
        .decrypt(&nonce, memo.ciphertext.as_slice())
        .map_err(|_| MemoError::DecryptionFailed)
}

/// Derive a 32-byte symmetric key from raw ECDH output via SHA-256.
fn derive_symmetric_key(shared_secret: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(shared_secret);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        // Generate recipient keypair
        let recipient_secret_bytes: [u8; 32] = rand::random();
        let recipient_secret = StaticSecret::from(recipient_secret_bytes);
        let recipient_pubkey = PublicKey::from(&recipient_secret);

        let plaintext = b"Hello from Dina Network! Payment of 42 DINA confirmed.";
        let memo = encrypt_memo(recipient_pubkey.as_bytes(), plaintext);

        let decrypted = decrypt_memo(&recipient_secret_bytes, &memo).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let recipient_secret_bytes: [u8; 32] = rand::random();
        let recipient_secret = StaticSecret::from(recipient_secret_bytes);
        let recipient_pubkey = PublicKey::from(&recipient_secret);

        let memo = encrypt_memo(recipient_pubkey.as_bytes(), b"secret data");

        // Try decrypting with a different key
        let wrong_key: [u8; 32] = rand::random();
        let result = decrypt_memo(&wrong_key, &memo);
        assert!(result.is_err());
    }

    #[test]
    fn empty_plaintext_roundtrip() {
        let recipient_secret_bytes: [u8; 32] = rand::random();
        let recipient_secret = StaticSecret::from(recipient_secret_bytes);
        let recipient_pubkey = PublicKey::from(&recipient_secret);

        let memo = encrypt_memo(recipient_pubkey.as_bytes(), b"");
        let decrypted = decrypt_memo(&recipient_secret_bytes, &memo).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn large_plaintext_roundtrip() {
        let recipient_secret_bytes: [u8; 32] = rand::random();
        let recipient_secret = StaticSecret::from(recipient_secret_bytes);
        let recipient_pubkey = PublicKey::from(&recipient_secret);

        let plaintext = vec![0xABu8; 10_000];
        let memo = encrypt_memo(recipient_pubkey.as_bytes(), &plaintext);
        let decrypted = decrypt_memo(&recipient_secret_bytes, &memo).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn memo_serialization_roundtrip() {
        let recipient_secret_bytes: [u8; 32] = rand::random();
        let recipient_secret = StaticSecret::from(recipient_secret_bytes);
        let recipient_pubkey = PublicKey::from(&recipient_secret);

        let memo = encrypt_memo(recipient_pubkey.as_bytes(), b"serialize me");
        let json = serde_json::to_string(&memo).unwrap();
        let deserialized: EncryptedMemo = serde_json::from_str(&json).unwrap();

        let decrypted = decrypt_memo(&recipient_secret_bytes, &deserialized).unwrap();
        assert_eq!(decrypted, b"serialize me");
    }

    #[test]
    fn different_messages_produce_different_ciphertexts() {
        // The ephemeral key is randomized, so even the same plaintext should
        // produce different ciphertexts each time.
        let recipient_secret_bytes: [u8; 32] = rand::random();
        let recipient_secret = StaticSecret::from(recipient_secret_bytes);
        let recipient_pubkey = PublicKey::from(&recipient_secret);

        let memo1 = encrypt_memo(recipient_pubkey.as_bytes(), b"hello");
        let memo2 = encrypt_memo(recipient_pubkey.as_bytes(), b"hello");

        // Ephemeral pubkeys must differ (different random keys each call)
        assert_ne!(memo1.ephemeral_pubkey, memo2.ephemeral_pubkey);
        // Ciphertexts must differ as a consequence
        assert_ne!(memo1.ciphertext, memo2.ciphertext);

        // Both should still decrypt to the same plaintext
        let d1 = decrypt_memo(&recipient_secret_bytes, &memo1).unwrap();
        let d2 = decrypt_memo(&recipient_secret_bytes, &memo2).unwrap();
        assert_eq!(d1, b"hello");
        assert_eq!(d2, b"hello");
    }

    #[test]
    fn different_plaintext_different_ciphertext() {
        let recipient_secret_bytes: [u8; 32] = rand::random();
        let recipient_secret = StaticSecret::from(recipient_secret_bytes);
        let recipient_pubkey = PublicKey::from(&recipient_secret);

        let memo_a = encrypt_memo(recipient_pubkey.as_bytes(), b"message A");
        let memo_b = encrypt_memo(recipient_pubkey.as_bytes(), b"message B");

        assert_ne!(memo_a.ciphertext, memo_b.ciphertext);
    }

    #[test]
    fn large_plaintext_1kb_roundtrip() {
        let recipient_secret_bytes: [u8; 32] = rand::random();
        let recipient_secret = StaticSecret::from(recipient_secret_bytes);
        let recipient_pubkey = PublicKey::from(&recipient_secret);

        // Exactly 1KB of patterned data
        let plaintext: Vec<u8> = (0u8..=255).cycle().take(1024).collect();
        let memo = encrypt_memo(recipient_pubkey.as_bytes(), &plaintext);
        let decrypted = decrypt_memo(&recipient_secret_bytes, &memo).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn tampered_ciphertext_fails_decryption() {
        let recipient_secret_bytes: [u8; 32] = rand::random();
        let recipient_secret = StaticSecret::from(recipient_secret_bytes);
        let recipient_pubkey = PublicKey::from(&recipient_secret);

        let mut memo = encrypt_memo(recipient_pubkey.as_bytes(), b"tamper test");
        // Flip a byte in the ciphertext
        if let Some(byte) = memo.ciphertext.first_mut() {
            *byte ^= 0xFF;
        }
        assert!(decrypt_memo(&recipient_secret_bytes, &memo).is_err());
    }

    #[test]
    fn tampered_nonce_fails_decryption() {
        let recipient_secret_bytes: [u8; 32] = rand::random();
        let recipient_secret = StaticSecret::from(recipient_secret_bytes);
        let recipient_pubkey = PublicKey::from(&recipient_secret);

        let mut memo = encrypt_memo(recipient_pubkey.as_bytes(), b"nonce tamper");
        memo.nonce[0] ^= 0xFF;
        assert!(decrypt_memo(&recipient_secret_bytes, &memo).is_err());
    }
}
