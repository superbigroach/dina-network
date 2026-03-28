use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};

use dina_core::crypto::hash_bytes;
use dina_core::transaction::{DeviceAttestation, Sig64, Transaction};
use dina_core::types::{Address, Hash};

/// A Cognitum device with its identity, firmware state, and witness chain root.
///
/// This struct provides device-specific utilities for creating registration
/// transactions, signing messages, and verifying witness chains.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CognitumDevice {
    /// The device's Ed25519 public key (32 bytes).
    pub pubkey: [u8; 32],
    /// Unique device identifier derived from the public key (SHA-256 hash).
    pub device_id: [u8; 32],
    /// Human-readable firmware version string (e.g., "1.0.0").
    pub firmware_version: String,
    /// Merkle root of the device's witness chain history.
    pub witness_root: [u8; 32],
}

/// An entry in a device's witness chain, forming a hash-linked sequence of
/// observed data events signed by the device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WitnessEntry {
    /// SHA-256 hash of the witnessed data.
    pub data_hash: [u8; 32],
    /// Hash of the previous witness entry (zero for the genesis entry).
    pub prev_hash: [u8; 32],
    /// Ed25519 signature over (data_hash || prev_hash || timestamp).
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    /// Unix timestamp (seconds) when this entry was created.
    pub timestamp: u64,
}

impl WitnessEntry {
    /// Compute the hash of this witness entry: SHA-256(data_hash || prev_hash || timestamp).
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.data_hash);
        hasher.update(self.prev_hash);
        hasher.update(self.timestamp.to_le_bytes());
        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }

    /// Compute the signing payload for this entry: data_hash || prev_hash || timestamp.
    pub fn signing_payload(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(72);
        payload.extend_from_slice(&self.data_hash);
        payload.extend_from_slice(&self.prev_hash);
        payload.extend_from_slice(&self.timestamp.to_le_bytes());
        payload
    }

    /// Verify the signature of this entry against a device public key.
    pub fn verify_signature(&self, pubkey: &[u8; 32]) -> bool {
        let vk = match VerifyingKey::from_bytes(pubkey) {
            Ok(vk) => vk,
            Err(_) => return false,
        };
        let sig = Signature::from_bytes(&self.signature);
        let payload = self.signing_payload();
        vk.verify(&payload, &sig).is_ok()
    }
}

impl CognitumDevice {
    /// Create a new CognitumDevice from a DeviceAttestation.
    ///
    /// The device_id is derived as SHA-256(pubkey), and the witness_root
    /// and firmware info are extracted from the attestation.
    pub fn from_attestation(attestation: &DeviceAttestation) -> Self {
        let id_hash = hash_bytes(&attestation.pubkey);

        Self {
            pubkey: attestation.pubkey,
            device_id: id_hash.0,
            firmware_version: format!("fw-{}", hex::encode(&attestation.firmware_hash.0[..4])),
            witness_root: attestation.witness_root.0,
        }
    }

    /// Create a RegisterDevice transaction for this device.
    ///
    /// # Arguments
    /// * `owner` - The owner address that will control this device.
    ///
    /// # Returns
    /// A `Transaction::RegisterDevice` with a zeroed signature. The caller
    /// must sign it with the owner's signing key before submitting.
    pub fn create_registration_tx(&self, owner: [u8; 32]) -> Transaction {
        let firmware_hash = Hash(self.firmware_hash_from_version());

        Transaction::RegisterDevice {
            device_pubkey: self.pubkey,
            owner: Address(owner),
            attestation: DeviceAttestation {
                pubkey: self.pubkey,
                firmware_hash,
                witness_root: Hash(self.witness_root),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                signature: Sig64([0u8; 64]),
            },
            nonce: 0,
            fee: 100,
            pub_key: [0u8; 32], // Caller must set this to the owner's Ed25519 public key before signing.
            signature: Sig64([0u8; 64]),
        }
    }

    /// Sign a message using the device's signing key.
    ///
    /// # Arguments
    /// * `message` - The message bytes to sign.
    /// * `signing_key` - The device's Ed25519 private key (32 bytes).
    ///
    /// # Returns
    /// A 64-byte Ed25519 signature.
    pub fn sign_with_device_key(&self, message: &[u8], signing_key: &[u8; 32]) -> [u8; 64] {
        let sk = SigningKey::from_bytes(signing_key);
        let sig = sk.sign(message);
        sig.to_bytes()
    }

    /// Verify a witness chain: each entry must link to the previous via prev_hash,
    /// have a valid signature from this device, and timestamps must be non-decreasing.
    ///
    /// # Arguments
    /// * `entries` - The witness chain entries, ordered from oldest to newest.
    ///
    /// # Returns
    /// `true` if the chain is valid, `false` otherwise.
    pub fn verify_witness_chain(&self, entries: &[WitnessEntry]) -> bool {
        if entries.is_empty() {
            return true;
        }

        // The first entry's prev_hash must be all zeros (genesis).
        if entries[0].prev_hash != [0u8; 32] {
            return false;
        }

        let mut prev_timestamp = 0u64;

        for (i, entry) in entries.iter().enumerate() {
            // Verify the signature of each entry against the device pubkey.
            if !entry.verify_signature(&self.pubkey) {
                return false;
            }

            // Verify timestamps are non-decreasing.
            if entry.timestamp < prev_timestamp {
                return false;
            }
            prev_timestamp = entry.timestamp;

            // Verify hash chain linkage (entries after the first must reference
            // the hash of the previous entry).
            if i > 0 {
                let expected_prev = entries[i - 1].hash();
                if entry.prev_hash != expected_prev {
                    return false;
                }
            }
        }

        true
    }

    /// Derive the device's address from its public key.
    pub fn address(&self) -> Address {
        let hash = hash_bytes(&self.pubkey);
        Address(hash.0)
    }

    /// Compute a firmware hash from the firmware_version string.
    /// Used internally when constructing attestation data.
    fn firmware_hash_from_version(&self) -> [u8; 32] {
        let hash = hash_bytes(self.firmware_version.as_bytes());
        hash.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::crypto;

    fn make_device_and_key() -> (CognitumDevice, SigningKey) {
        let (sk, vk) = crypto::generate_keypair();
        let pubkey = *vk.as_bytes();
        let device = CognitumDevice {
            pubkey,
            device_id: hash_bytes(&pubkey).0,
            firmware_version: "1.0.0".to_string(),
            witness_root: [0u8; 32],
        };
        (device, sk)
    }

    fn make_witness_entry(
        data: &[u8],
        prev_hash: [u8; 32],
        timestamp: u64,
        sk: &SigningKey,
    ) -> WitnessEntry {
        let data_hash = hash_bytes(data).0;
        let mut entry = WitnessEntry {
            data_hash,
            prev_hash,
            signature: [0u8; 64],
            timestamp,
        };
        let payload = entry.signing_payload();
        entry.signature = sk.sign(&payload).to_bytes();
        entry
    }

    #[test]
    fn device_from_attestation() {
        let (sk, vk) = crypto::generate_keypair();
        let attestation = DeviceAttestation {
            pubkey: *vk.as_bytes(),
            firmware_hash: Hash([0xaa; 32]),
            witness_root: Hash([0xbb; 32]),
            timestamp: 1_700_000_000,
            signature: Sig64([0u8; 64]),
        };
        let device = CognitumDevice::from_attestation(&attestation);
        assert_eq!(device.pubkey, *vk.as_bytes());
        assert_eq!(device.witness_root, [0xbb; 32]);
        assert!(device.firmware_version.starts_with("fw-"));
        let _ = sk; // suppress unused warning
    }

    #[test]
    fn device_address() {
        let (device, _sk) = make_device_and_key();
        let addr = device.address();
        let expected = hash_bytes(&device.pubkey);
        assert_eq!(addr.0, expected.0);
    }

    #[test]
    fn sign_with_device_key() {
        let (device, sk) = make_device_and_key();
        let message = b"hello dina";
        let sig = device.sign_with_device_key(message, &sk.to_bytes());

        let vk = sk.verifying_key();
        let ed_sig = Signature::from_bytes(&sig);
        assert!(vk.verify(message, &ed_sig).is_ok());
    }

    #[test]
    fn create_registration_tx() {
        let (device, _sk) = make_device_and_key();
        let owner = [0x01; 32];
        let tx = device.create_registration_tx(owner);

        match &tx {
            Transaction::RegisterDevice {
                device_pubkey,
                owner: tx_owner,
                ..
            } => {
                assert_eq!(*device_pubkey, device.pubkey);
                assert_eq!(tx_owner.0, owner);
            }
            _ => panic!("expected RegisterDevice transaction"),
        }
    }

    #[test]
    fn verify_empty_witness_chain() {
        let (device, _sk) = make_device_and_key();
        assert!(device.verify_witness_chain(&[]));
    }

    #[test]
    fn verify_single_entry_witness_chain() {
        let (device, sk) = make_device_and_key();
        let entry = make_witness_entry(b"data1", [0u8; 32], 100, &sk);
        assert!(device.verify_witness_chain(&[entry]));
    }

    #[test]
    fn verify_multi_entry_witness_chain() {
        let (device, sk) = make_device_and_key();
        let e1 = make_witness_entry(b"data1", [0u8; 32], 100, &sk);
        let e2 = make_witness_entry(b"data2", e1.hash(), 200, &sk);
        let e3 = make_witness_entry(b"data3", e2.hash(), 300, &sk);
        assert!(device.verify_witness_chain(&[e1, e2, e3]));
    }

    #[test]
    fn witness_chain_rejects_bad_prev_hash() {
        let (device, sk) = make_device_and_key();
        let e1 = make_witness_entry(b"data1", [0u8; 32], 100, &sk);
        let e2 = make_witness_entry(b"data2", [0xff; 32], 200, &sk); // wrong prev
        assert!(!device.verify_witness_chain(&[e1, e2]));
    }

    #[test]
    fn witness_chain_rejects_non_zero_genesis_prev() {
        let (device, sk) = make_device_and_key();
        let e1 = make_witness_entry(b"data1", [0x01; 32], 100, &sk); // non-zero genesis
        assert!(!device.verify_witness_chain(&[e1]));
    }

    #[test]
    fn witness_chain_rejects_decreasing_timestamps() {
        let (device, sk) = make_device_and_key();
        let e1 = make_witness_entry(b"data1", [0u8; 32], 200, &sk);
        let e2 = make_witness_entry(b"data2", e1.hash(), 100, &sk); // earlier timestamp
        assert!(!device.verify_witness_chain(&[e1, e2]));
    }

    #[test]
    fn witness_chain_rejects_bad_signature() {
        let (device, sk) = make_device_and_key();
        let mut e1 = make_witness_entry(b"data1", [0u8; 32], 100, &sk);
        e1.signature[0] ^= 0xff; // corrupt signature
        assert!(!device.verify_witness_chain(&[e1]));
    }

    #[test]
    fn witness_chain_rejects_wrong_key_signature() {
        let (device, _sk) = make_device_and_key();
        let (wrong_sk, _) = crypto::generate_keypair();
        let e1 = make_witness_entry(b"data1", [0u8; 32], 100, &wrong_sk);
        assert!(!device.verify_witness_chain(&[e1]));
    }

    #[test]
    fn witness_entry_hash_deterministic() {
        let entry = WitnessEntry {
            data_hash: [0xaa; 32],
            prev_hash: [0u8; 32],
            signature: [0u8; 64],
            timestamp: 12345,
        };
        assert_eq!(entry.hash(), entry.hash());
    }

    #[test]
    fn witness_entry_different_data_different_hash() {
        let e1 = WitnessEntry {
            data_hash: [0xaa; 32],
            prev_hash: [0u8; 32],
            signature: [0u8; 64],
            timestamp: 100,
        };
        let e2 = WitnessEntry {
            data_hash: [0xbb; 32],
            prev_hash: [0u8; 32],
            signature: [0u8; 64],
            timestamp: 100,
        };
        assert_ne!(e1.hash(), e2.hash());
    }

    #[test]
    fn device_serialization_roundtrip() {
        let (device, _sk) = make_device_and_key();
        let json = serde_json::to_string(&device).unwrap();
        let restored: CognitumDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.pubkey, device.pubkey);
        assert_eq!(restored.device_id, device.device_id);
        assert_eq!(restored.firmware_version, device.firmware_version);
        assert_eq!(restored.witness_root, device.witness_root);
    }

    #[test]
    fn witness_entry_serialization_roundtrip() {
        let (_, sk) = make_device_and_key();
        let entry = make_witness_entry(b"test", [0u8; 32], 999, &sk);
        let json = serde_json::to_string(&entry).unwrap();
        let restored: WitnessEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.data_hash, entry.data_hash);
        assert_eq!(restored.prev_hash, entry.prev_hash);
        assert_eq!(restored.signature, entry.signature);
        assert_eq!(restored.timestamp, entry.timestamp);
    }
}
