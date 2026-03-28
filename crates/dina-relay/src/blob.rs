//! RelayBlob — the core data structure that gets broadcast over BLE and relayed
//! across the Dina network for payment settlement propagation.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use dina_core::transaction::Sig64;
use dina_core::types::{Address, Hash};

/// Maximum age of a relay blob before it is considered expired (5 minutes).
pub const DEFAULT_BLOB_TTL_SECS: u64 = 300;

/// Maximum payload size for a relay blob (200 bytes, fits in BLE + QR).
pub const MAX_BLOB_PAYLOAD_BYTES: usize = 200;

/// A compact settlement blob that gets relayed across the BLE mesh network.
///
/// Contains a compressed representation of a payment channel settlement,
/// signed by the sender, and carrying enough information for validators
/// to finalize the settlement on-chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayBlob {
    /// Protocol version (currently 1).
    pub version: u8,
    /// Sender address (payer in the settlement).
    pub sender: Address,
    /// Receiver address (payee in the settlement).
    pub receiver: Address,
    /// Settlement amount in micro-USDC (1 USDC = 1_000_000).
    pub amount: u64,
    /// Channel sequence number for ordering.
    pub sequence: u64,
    /// Unix timestamp when this blob was created.
    pub created_at: u64,
    /// Time-to-live in seconds before this blob expires.
    pub ttl_secs: u64,
    /// Fee offered to the relay node (in micro-USDC).
    pub relay_fee: u64,
    /// SHA-256 hash of the full channel state being settled.
    pub channel_state_hash: Hash,
    /// Ed25519 signature from the sender over the blob fields.
    pub sender_signature: Sig64,
    /// Ed25519 signature from the receiver (counter-signature).
    pub receiver_signature: Sig64,
    /// Number of times this blob has been relayed (incremented by each relay).
    pub hop_count: u8,
    /// Maximum allowed hops before the blob is dropped.
    pub max_hops: u8,
}

impl RelayBlob {
    /// Compute the SHA-256 hash of the blob (used as its unique identifier).
    pub fn hash(&self) -> Hash {
        let bytes = bincode::serialize(self).expect("blob serialization cannot fail");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Hash(hash)
    }

    /// Return the bytes that both parties sign (all fields except signatures and hop_count).
    pub fn signing_bytes(&self) -> Vec<u8> {
        let payload = SigningPayload {
            version: self.version,
            sender: &self.sender,
            receiver: &self.receiver,
            amount: self.amount,
            sequence: self.sequence,
            created_at: self.created_at,
            ttl_secs: self.ttl_secs,
            relay_fee: self.relay_fee,
            channel_state_hash: &self.channel_state_hash,
            max_hops: self.max_hops,
        };
        bincode::serialize(&payload).expect("signing payload serialization cannot fail")
    }

    /// Verify the sender's signature using their public key.
    pub fn verify_sender_signature(&self, sender_pubkey: &VerifyingKey) -> bool {
        let msg = self.signing_bytes();
        let sig = Signature::from_bytes(&self.sender_signature.0);
        sender_pubkey.verify(&msg, &sig).is_ok()
    }

    /// Verify the receiver's counter-signature using their public key.
    pub fn verify_receiver_signature(&self, receiver_pubkey: &VerifyingKey) -> bool {
        let msg = self.signing_bytes();
        let sig = Signature::from_bytes(&self.receiver_signature.0);
        receiver_pubkey.verify(&msg, &sig).is_ok()
    }

    /// Check whether this blob has expired based on the current timestamp.
    pub fn is_expired(&self, now_unix: u64) -> bool {
        now_unix > self.created_at + self.ttl_secs
    }

    /// Check whether this blob has exceeded its maximum hop count.
    pub fn is_max_hops_reached(&self) -> bool {
        self.hop_count >= self.max_hops
    }

    /// Increment the hop count (called when a relay node forwards the blob).
    pub fn increment_hop(&mut self) {
        self.hop_count = self.hop_count.saturating_add(1);
    }

    /// Total size of this blob when serialized with bincode.
    pub fn serialized_size(&self) -> usize {
        bincode::serialized_size(self).unwrap_or(0) as usize
    }
}

/// Internal struct for deterministic signing (excludes signatures and hop_count).
#[derive(Serialize)]
struct SigningPayload<'a> {
    version: u8,
    sender: &'a Address,
    receiver: &'a Address,
    amount: u64,
    sequence: u64,
    created_at: u64,
    ttl_secs: u64,
    relay_fee: u64,
    channel_state_hash: &'a Hash,
    max_hops: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::crypto;

    fn make_test_blob() -> (
        RelayBlob,
        ed25519_dalek::SigningKey,
        ed25519_dalek::SigningKey,
    ) {
        let (sender_sk, sender_vk) = crypto::generate_keypair();
        let (receiver_sk, receiver_vk) = crypto::generate_keypair();

        let sender_addr = Address::from_pubkey(&sender_vk);
        let receiver_addr = Address::from_pubkey(&receiver_vk);

        let mut blob = RelayBlob {
            version: 1,
            sender: sender_addr,
            receiver: receiver_addr,
            amount: 50_000, // 0.05 USDC
            sequence: 1,
            created_at: 1700000000,
            ttl_secs: DEFAULT_BLOB_TTL_SECS,
            relay_fee: 10,
            channel_state_hash: Hash([0xaa; 32]),
            sender_signature: Sig64([0u8; 64]),
            receiver_signature: Sig64([0u8; 64]),
            hop_count: 0,
            max_hops: 10,
        };

        let msg = blob.signing_bytes();
        blob.sender_signature = Sig64(crypto::sign(&sender_sk, &msg));
        blob.receiver_signature = Sig64(crypto::sign(&receiver_sk, &msg));

        (blob, sender_sk, receiver_sk)
    }

    #[test]
    fn verify_signatures() {
        let (blob, sender_sk, receiver_sk) = make_test_blob();
        assert!(blob.verify_sender_signature(&sender_sk.verifying_key()));
        assert!(blob.verify_receiver_signature(&receiver_sk.verifying_key()));
    }

    #[test]
    fn wrong_key_rejects() {
        let (blob, _, _) = make_test_blob();
        let (_, wrong_vk) = crypto::generate_keypair();
        assert!(!blob.verify_sender_signature(&wrong_vk));
        assert!(!blob.verify_receiver_signature(&wrong_vk));
    }

    #[test]
    fn expiry_check() {
        let (blob, _, _) = make_test_blob();
        assert!(!blob.is_expired(1700000000 + 100));
        assert!(blob.is_expired(1700000000 + 301));
    }

    #[test]
    fn hop_count() {
        let (mut blob, _, _) = make_test_blob();
        assert!(!blob.is_max_hops_reached());
        for _ in 0..10 {
            blob.increment_hop();
        }
        assert!(blob.is_max_hops_reached());
    }

    #[test]
    fn hash_is_deterministic() {
        let (blob, _, _) = make_test_blob();
        assert_eq!(blob.hash(), blob.hash());
    }
}
