use dina_core::Address;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

#[derive(Debug, Error)]
pub enum StealthError {
    #[error("stealth address detection failed")]
    DetectionFailed,
    #[error("invalid key material")]
    InvalidKey,
}

/// A stealth meta-address published by the recipient. Senders use it to
/// derive one-time stealth addresses that only the recipient can detect
/// and spend from.
///
/// Modelled after EIP-5564 stealth addresses adapted for Dina's X25519 keys.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StealthMetaAddress {
    /// Public key used for scanning — the recipient watches the chain with the
    /// corresponding secret to detect payments.
    pub scan_pubkey: [u8; 32],
    /// Public key used for spending — combined with the shared secret to form
    /// the one-time stealth address.
    pub spend_pubkey: [u8; 32],
}

/// A one-time stealth address produced by a sender.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StealthAddress {
    /// The derived one-time address that the sender pays to.
    pub address: Address,
    /// The ephemeral public key the sender publishes alongside the transaction
    /// so the recipient can detect and derive the spending key.
    pub ephemeral_pubkey: [u8; 32],
}

/// Generate a new stealth meta-address together with the scan and spend secrets.
///
/// The recipient publishes `StealthMetaAddress` and keeps the two secrets private.
pub fn generate_meta_address() -> (StealthMetaAddress, [u8; 32], [u8; 32]) {
    let scan_secret_bytes: [u8; 32] = rand::random();
    let spend_secret_bytes: [u8; 32] = rand::random();

    let scan_secret = StaticSecret::from(scan_secret_bytes);
    let spend_secret = StaticSecret::from(spend_secret_bytes);

    let scan_pubkey = PublicKey::from(&scan_secret);
    let spend_pubkey = PublicKey::from(&spend_secret);

    let meta = StealthMetaAddress {
        scan_pubkey: *scan_pubkey.as_bytes(),
        spend_pubkey: *spend_pubkey.as_bytes(),
    };

    (meta, scan_secret_bytes, spend_secret_bytes)
}

/// Derive a one-time stealth address from a recipient's meta-address.
///
/// Protocol:
/// 1. Sender generates ephemeral X25519 keypair (r, R).
/// 2. Sender computes shared_secret = ECDH(r, scan_pubkey).
/// 3. Sender computes stealth_key = SHA-256(shared_secret || spend_pubkey).
/// 4. The stealth address is SHA-256(stealth_key) — matching `Address::from_pubkey`
///    but using raw bytes since we don't have an Ed25519 key.
pub fn derive_stealth_address(meta: &StealthMetaAddress) -> StealthAddress {
    let scan_pk = PublicKey::from(meta.scan_pubkey);

    let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_pubkey = PublicKey::from(&ephemeral_secret);

    // ECDH with scan key
    let shared_secret = ephemeral_secret.diffie_hellman(&scan_pk);

    // Derive the one-time address bytes
    let address_bytes = derive_stealth_address_bytes(shared_secret.as_bytes(), &meta.spend_pubkey);

    StealthAddress {
        address: Address(address_bytes),
        ephemeral_pubkey: *ephemeral_pubkey.as_bytes(),
    }
}

/// Check whether a stealth address belongs to the recipient.
///
/// The recipient performs the same derivation using their scan secret and the
/// published ephemeral public key, then compares the result to the address.
pub fn detect_stealth(
    scan_secret: &[u8; 32],
    spend_pubkey: &[u8; 32],
    ephemeral_pubkey: &[u8; 32],
    address: &Address,
) -> bool {
    let secret = StaticSecret::from(*scan_secret);
    let eph_pk = PublicKey::from(*ephemeral_pubkey);

    let shared_secret = secret.diffie_hellman(&eph_pk);
    let expected = derive_stealth_address_bytes(shared_secret.as_bytes(), spend_pubkey);

    address.0 == expected
}

/// Derive the spending key for a stealth address so the recipient can spend
/// the funds received at that one-time address.
///
/// The spending key is SHA-256(shared_secret || spend_secret), giving the
/// recipient a deterministic private key unique to this transaction.
pub fn derive_stealth_spending_key(
    scan_secret: &[u8; 32],
    spend_secret: &[u8; 32],
    ephemeral_pubkey: &[u8; 32],
) -> [u8; 32] {
    let secret = StaticSecret::from(*scan_secret);
    let eph_pk = PublicKey::from(*ephemeral_pubkey);

    let shared_secret = secret.diffie_hellman(&eph_pk);

    let mut hasher = Sha256::new();
    hasher.update(shared_secret.as_bytes());
    hasher.update(spend_secret);
    hasher.finalize().into()
}

/// Internal: derive the 32-byte stealth address from ECDH output and spend pubkey.
///
/// address = SHA-256(SHA-256(shared_secret || spend_pubkey))
///
/// The double hash mirrors `Address::from_pubkey(SHA-256(pubkey))` but operates
/// on the combined key material instead of a raw Ed25519 public key.
fn derive_stealth_address_bytes(shared_secret: &[u8], spend_pubkey: &[u8; 32]) -> [u8; 32] {
    // First hash: combine shared secret with spend pubkey to get "stealth key"
    let mut hasher = Sha256::new();
    hasher.update(shared_secret);
    hasher.update(spend_pubkey);
    let stealth_key: [u8; 32] = hasher.finalize().into();

    // Second hash: derive the address from the stealth key (like Address::from_pubkey)
    let mut hasher2 = Sha256::new();
    hasher2.update(stealth_key);
    hasher2.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stealth_address_roundtrip() {
        let (meta, scan_secret, _spend_secret) = generate_meta_address();

        let stealth = derive_stealth_address(&meta);

        // Recipient should detect the address as theirs
        assert!(detect_stealth(
            &scan_secret,
            &meta.spend_pubkey,
            &stealth.ephemeral_pubkey,
            &stealth.address,
        ));
    }

    #[test]
    fn wrong_scan_secret_does_not_detect() {
        let (meta, _scan_secret, _spend_secret) = generate_meta_address();
        let stealth = derive_stealth_address(&meta);

        let wrong_secret: [u8; 32] = rand::random();
        assert!(!detect_stealth(
            &wrong_secret,
            &meta.spend_pubkey,
            &stealth.ephemeral_pubkey,
            &stealth.address,
        ));
    }

    #[test]
    fn different_transactions_produce_different_addresses() {
        let (meta, _, _) = generate_meta_address();

        let addr1 = derive_stealth_address(&meta);
        let addr2 = derive_stealth_address(&meta);

        // Each call generates a new ephemeral key, so addresses differ
        assert_ne!(addr1.address, addr2.address);
        assert_ne!(addr1.ephemeral_pubkey, addr2.ephemeral_pubkey);
    }

    #[test]
    fn spending_key_is_deterministic() {
        let (meta, scan_secret, spend_secret) = generate_meta_address();
        let stealth = derive_stealth_address(&meta);

        let key1 =
            derive_stealth_spending_key(&scan_secret, &spend_secret, &stealth.ephemeral_pubkey);
        let key2 =
            derive_stealth_spending_key(&scan_secret, &spend_secret, &stealth.ephemeral_pubkey);

        assert_eq!(key1, key2);
    }

    #[test]
    fn meta_address_serialization_roundtrip() {
        let (meta, _, _) = generate_meta_address();
        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: StealthMetaAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, deserialized);
    }

    #[test]
    fn generate_meta_address_produces_valid_keys() {
        let (meta, scan_secret, spend_secret) = generate_meta_address();

        // The public keys should match what we derive from the secrets
        let scan_pk = PublicKey::from(&StaticSecret::from(scan_secret));
        let spend_pk = PublicKey::from(&StaticSecret::from(spend_secret));

        assert_eq!(meta.scan_pubkey, *scan_pk.as_bytes());
        assert_eq!(meta.spend_pubkey, *spend_pk.as_bytes());
    }

    #[test]
    fn detect_stealth_returns_false_for_non_recipient() {
        let (meta1, _scan1, _spend1) = generate_meta_address();
        let (_meta2, scan2, _spend2) = generate_meta_address();

        let stealth = derive_stealth_address(&meta1);

        // Try detecting with a completely different recipient's secrets
        assert!(!detect_stealth(
            &scan2,
            &meta1.spend_pubkey,
            &stealth.ephemeral_pubkey,
            &stealth.address,
        ));
    }

    #[test]
    fn detect_stealth_fails_with_wrong_spend_pubkey() {
        let (meta, scan_secret, _spend_secret) = generate_meta_address();
        let stealth = derive_stealth_address(&meta);

        let wrong_spend_pubkey: [u8; 32] = rand::random();
        assert!(!detect_stealth(
            &scan_secret,
            &wrong_spend_pubkey,
            &stealth.ephemeral_pubkey,
            &stealth.address,
        ));
    }

    #[test]
    fn full_stealth_flow_generate_derive_detect_spend() {
        // Step 1: Recipient generates meta-address and publishes it
        let (meta, scan_secret, spend_secret) = generate_meta_address();

        // Step 2: Sender derives a one-time stealth address from the meta-address
        let stealth = derive_stealth_address(&meta);

        // Step 3: Recipient scans the chain and detects the stealth address as theirs
        let detected = detect_stealth(
            &scan_secret,
            &meta.spend_pubkey,
            &stealth.ephemeral_pubkey,
            &stealth.address,
        );
        assert!(
            detected,
            "recipient should detect their own stealth address"
        );

        // Step 4: Recipient derives the spending key for this stealth address
        let spending_key =
            derive_stealth_spending_key(&scan_secret, &spend_secret, &stealth.ephemeral_pubkey);

        // The spending key should be non-zero (valid key material)
        assert_ne!(spending_key, [0u8; 32]);

        // The spending key should be deterministic
        let spending_key2 =
            derive_stealth_spending_key(&scan_secret, &spend_secret, &stealth.ephemeral_pubkey);
        assert_eq!(spending_key, spending_key2);

        // A different ephemeral pubkey should yield a different spending key
        let stealth2 = derive_stealth_address(&meta);
        let spending_key3 =
            derive_stealth_spending_key(&scan_secret, &spend_secret, &stealth2.ephemeral_pubkey);
        assert_ne!(spending_key, spending_key3);
    }

    #[test]
    fn stealth_address_serialization_roundtrip() {
        let (meta, _, _) = generate_meta_address();
        let stealth = derive_stealth_address(&meta);

        let json = serde_json::to_string(&stealth).unwrap();
        let deserialized: StealthAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(stealth, deserialized);
    }
}
