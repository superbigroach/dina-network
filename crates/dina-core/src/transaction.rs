use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::crypto::hash_bytes;
use crate::types::{Address, Hash};

/// A 64-byte Ed25519 signature, newtype wrapper for serde compatibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sig64(pub [u8; 64]);

impl Serialize for Sig64 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        BigArray::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for Sig64 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let arr: [u8; 64] = BigArray::deserialize(deserializer)?;
        Ok(Sig64(arr))
    }
}

impl From<[u8; 64]> for Sig64 {
    fn from(arr: [u8; 64]) -> Self {
        Sig64(arr)
    }
}

impl From<Sig64> for [u8; 64] {
    fn from(sig: Sig64) -> Self {
        sig.0
    }
}

/// Proof that a hardware device witnessed a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessProof {
    /// Hash of the witnessed event data.
    pub witness_hash: Hash,
    /// Ed25519 signature from the witnessing device.
    pub device_signature: Sig64,
}

/// Attestation proving a device's identity and firmware integrity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAttestation {
    /// Device's Ed25519 public key.
    pub pubkey: [u8; 32],
    /// SHA-256 hash of the device firmware.
    pub firmware_hash: Hash,
    /// Merkle root of the device's witness history.
    pub witness_root: Hash,
    /// Unix timestamp when the attestation was created.
    pub timestamp: u64,
    /// Ed25519 signature over the attestation fields.
    pub signature: Sig64,
}

/// A transaction on the Dina blockchain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transaction {
    /// Transfer USDC between accounts.
    Transfer {
        from: Address,
        to: Address,
        amount: u64,
        memo: Option<Vec<u8>>,
        device_witness: Option<WitnessProof>,
        nonce: u64,
        fee: u64,
        /// Ed25519 public key of the sender (used for signature verification).
        pub_key: [u8; 32],
        signature: Sig64,
    },

    /// Deploy a new smart contract.
    DeployContract {
        from: Address,
        wasm_bytecode: Vec<u8>,
        init_args: Vec<u8>,
        nonce: u64,
        fee: u64,
        /// Ed25519 public key of the sender (used for signature verification).
        pub_key: [u8; 32],
        signature: Sig64,
    },

    /// Call a method on a deployed smart contract.
    CallContract {
        from: Address,
        contract: Address,
        method: String,
        args: Vec<u8>,
        usdc_attached: u64,
        nonce: u64,
        fee: u64,
        /// Ed25519 public key of the sender (used for signature verification).
        pub_key: [u8; 32],
        signature: Sig64,
    },

    /// Register a new device on-chain.
    RegisterDevice {
        device_pubkey: [u8; 32],
        owner: Address,
        attestation: DeviceAttestation,
        nonce: u64,
        fee: u64,
        /// Ed25519 public key of the sender (used for signature verification).
        pub_key: [u8; 32],
        signature: Sig64,
    },
}

impl Transaction {
    /// Compute the SHA-256 hash of the full transaction (including the signature).
    pub fn hash(&self) -> Hash {
        let bytes = self.to_bytes();
        hash_bytes(&bytes)
    }

    /// Serialize the entire transaction to bytes (for hashing / storage).
    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("transaction serialization cannot fail")
    }

    /// Return the bytes that should be signed (all fields except the signature).
    pub fn signing_bytes(&self) -> Vec<u8> {
        match self {
            Transaction::Transfer {
                from,
                to,
                amount,
                memo,
                device_witness,
                nonce,
                fee,
                ..
            } => {
                let payload = TransferPayload {
                    tag: 0u8,
                    from,
                    to,
                    amount: *amount,
                    memo,
                    device_witness,
                    nonce: *nonce,
                    fee: *fee,
                };
                bincode::serialize(&payload).expect("serialization cannot fail")
            }
            Transaction::DeployContract {
                from,
                wasm_bytecode,
                init_args,
                nonce,
                fee,
                ..
            } => {
                let payload = DeployPayload {
                    tag: 1u8,
                    from,
                    wasm_bytecode,
                    init_args,
                    nonce: *nonce,
                    fee: *fee,
                };
                bincode::serialize(&payload).expect("serialization cannot fail")
            }
            Transaction::CallContract {
                from,
                contract,
                method,
                args,
                usdc_attached,
                nonce,
                fee,
                ..
            } => {
                let payload = CallPayload {
                    tag: 2u8,
                    from,
                    contract,
                    method,
                    args,
                    usdc_attached: *usdc_attached,
                    nonce: *nonce,
                    fee: *fee,
                };
                bincode::serialize(&payload).expect("serialization cannot fail")
            }
            Transaction::RegisterDevice {
                device_pubkey,
                owner,
                attestation,
                nonce,
                fee,
                ..
            } => {
                let payload = RegisterPayload {
                    tag: 3u8,
                    device_pubkey,
                    owner,
                    attestation,
                    nonce: *nonce,
                    fee: *fee,
                };
                bincode::serialize(&payload).expect("serialization cannot fail")
            }
        }
    }

    /// Verify the transaction signature using the embedded public key.
    ///
    /// This performs full Ed25519 signature verification:
    /// 1. Rejects transactions with an all-zero public key.
    /// 2. Parses the embedded `pub_key` bytes into a `VerifyingKey`.
    /// 3. Derives the address from the public key and checks it matches the sender.
    /// 4. Verifies the Ed25519 signature over the signing payload.
    ///
    /// Returns `true` only if all checks pass.
    pub fn verify_signature(&self) -> bool {
        let pk_bytes = self.pub_key_bytes();

        // Reject all-zero public key (missing key).
        if pk_bytes == [0u8; 32] {
            return false;
        }

        // Parse the public key bytes into a VerifyingKey.
        let verifying_key = match VerifyingKey::from_bytes(&pk_bytes) {
            Ok(vk) => vk,
            Err(_) => return false,
        };

        // Derive the address from the public key and verify it matches the sender.
        let derived_address = Address::from_pubkey(&verifying_key);
        if derived_address != self.sender() {
            return false;
        }

        // Verify the Ed25519 signature over the signing payload.
        let sig_bytes = self.signature_bytes();
        let sig = Signature::from_bytes(&sig_bytes);
        let msg = self.signing_bytes();
        verifying_key.verify(&msg, &sig).is_ok()
    }

    /// Verify the transaction signature against an externally-provided public key.
    ///
    /// This is useful in tests or contexts where the caller already has the key.
    pub fn verify_signature_with_key(&self, verifying_key: &VerifyingKey) -> bool {
        let sig_bytes = self.signature_bytes();
        let sig = Signature::from_bytes(&sig_bytes);
        let msg = self.signing_bytes();
        verifying_key.verify(&msg, &sig).is_ok()
    }

    /// Extract the sender address.
    pub fn sender(&self) -> Address {
        match self {
            Transaction::Transfer { from, .. }
            | Transaction::DeployContract { from, .. }
            | Transaction::CallContract { from, .. } => *from,
            Transaction::RegisterDevice { owner, .. } => *owner,
        }
    }

    /// Extract the nonce.
    pub fn nonce(&self) -> u64 {
        match self {
            Transaction::Transfer { nonce, .. }
            | Transaction::DeployContract { nonce, .. }
            | Transaction::CallContract { nonce, .. }
            | Transaction::RegisterDevice { nonce, .. } => *nonce,
        }
    }

    /// Extract the fee.
    pub fn fee(&self) -> u64 {
        match self {
            Transaction::Transfer { fee, .. }
            | Transaction::DeployContract { fee, .. }
            | Transaction::CallContract { fee, .. }
            | Transaction::RegisterDevice { fee, .. } => *fee,
        }
    }

    /// Extract the raw 64-byte signature.
    pub fn signature_bytes(&self) -> [u8; 64] {
        match self {
            Transaction::Transfer { signature, .. }
            | Transaction::DeployContract { signature, .. }
            | Transaction::CallContract { signature, .. }
            | Transaction::RegisterDevice { signature, .. } => signature.0,
        }
    }

    /// Extract the sender's 32-byte Ed25519 public key.
    pub fn pub_key_bytes(&self) -> [u8; 32] {
        match self {
            Transaction::Transfer { pub_key, .. }
            | Transaction::DeployContract { pub_key, .. }
            | Transaction::CallContract { pub_key, .. }
            | Transaction::RegisterDevice { pub_key, .. } => *pub_key,
        }
    }
}

// Internal signable payloads (bincode-serialized, excluding the signature field).

#[derive(Serialize)]
struct TransferPayload<'a> {
    tag: u8,
    from: &'a Address,
    to: &'a Address,
    amount: u64,
    memo: &'a Option<Vec<u8>>,
    device_witness: &'a Option<WitnessProof>,
    nonce: u64,
    fee: u64,
}

#[derive(Serialize)]
struct DeployPayload<'a> {
    tag: u8,
    from: &'a Address,
    wasm_bytecode: &'a [u8],
    init_args: &'a [u8],
    nonce: u64,
    fee: u64,
}

#[derive(Serialize)]
struct CallPayload<'a> {
    tag: u8,
    from: &'a Address,
    contract: &'a Address,
    method: &'a str,
    args: &'a [u8],
    usdc_attached: u64,
    nonce: u64,
    fee: u64,
}

#[derive(Serialize)]
struct RegisterPayload<'a> {
    tag: u8,
    device_pubkey: &'a [u8; 32],
    owner: &'a Address,
    attestation: &'a DeviceAttestation,
    nonce: u64,
    fee: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto;

    fn make_signed_transfer(sk: &ed25519_dalek::SigningKey) -> Transaction {
        let vk = sk.verifying_key();
        let from = Address::from_pubkey(&vk);
        let to = Address([0xbb; 32]);

        let mut tx = Transaction::Transfer {
            from,
            to,
            amount: 1000,
            memo: None,
            device_witness: None,
            nonce: 0,
            fee: 10,
            pub_key: *vk.as_bytes(),
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(sk, &msg);

        if let Transaction::Transfer {
            ref mut signature, ..
        } = tx
        {
            *signature = Sig64(sig);
        }

        tx
    }

    #[test]
    fn sign_and_verify_transfer() {
        let (sk, _vk) = crypto::generate_keypair();
        let tx = make_signed_transfer(&sk);
        assert!(tx.verify_signature());
    }

    #[test]
    fn wrong_key_rejects() {
        let (sk, _) = crypto::generate_keypair();
        let (_, wrong_vk) = crypto::generate_keypair();
        let tx = make_signed_transfer(&sk);
        // Tamper with the pub_key to use a wrong key
        let mut bad_tx = tx;
        if let Transaction::Transfer {
            ref mut pub_key, ..
        } = bad_tx
        {
            *pub_key = *wrong_vk.as_bytes();
        }
        // Should fail because derived address won't match sender
        assert!(!bad_tx.verify_signature());
    }

    #[test]
    fn verify_with_external_key() {
        let (sk, vk) = crypto::generate_keypair();
        let tx = make_signed_transfer(&sk);
        assert!(tx.verify_signature_with_key(&vk));

        let (_, wrong_vk) = crypto::generate_keypair();
        assert!(!tx.verify_signature_with_key(&wrong_vk));
    }

    #[test]
    fn hash_is_deterministic() {
        let (sk, _) = crypto::generate_keypair();
        let tx = make_signed_transfer(&sk);
        assert_eq!(tx.hash(), tx.hash());
    }

    #[test]
    fn sender_nonce_fee() {
        let (sk, vk) = crypto::generate_keypair();
        let tx = make_signed_transfer(&sk);
        assert_eq!(tx.sender(), Address::from_pubkey(&vk));
        assert_eq!(tx.nonce(), 0);
        assert_eq!(tx.fee(), 10);
    }
}
