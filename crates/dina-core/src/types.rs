use std::fmt;
use std::str::FromStr;

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::DinaError;

/// A 32-byte address derived from an Ed25519 public key via SHA-256.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Address(pub [u8; 32]);

impl Address {
    pub const ZERO: Address = Address([0u8; 32]);

    /// Derive an address from an Ed25519 verifying (public) key by hashing it with SHA-256.
    pub fn from_pubkey(pubkey: &VerifyingKey) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(pubkey.as_bytes());
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        Address(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address(0x{})", hex::encode(self.0))
    }
}

impl FromStr for Address {
    type Err = DinaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s)
            .map_err(|e| DinaError::SerializationError(format!("invalid hex address: {e}")))?;
        if bytes.len() != 32 {
            return Err(DinaError::SerializationError(format!(
                "address must be 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Address(arr))
    }
}

/// A 32-byte hash (SHA-256 output).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub const ZERO: Hash = Hash([0u8; 32]);

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash(0x{})", &hex::encode(self.0)[..16])
    }
}

impl FromStr for Hash {
    type Err = DinaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s)
            .map_err(|e| DinaError::SerializationError(format!("invalid hex hash: {e}")))?;
        if bytes.len() != 32 {
            return Err(DinaError::SerializationError(format!(
                "hash must be 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Hash(arr))
    }
}

/// Unique identifier for a registered device (derived from device pubkey).
pub type DeviceId = Address;

/// Unique identifier for a verifiable credential.
pub type CredentialId = Hash;

/// Unique identifier for a service agreement between agents.
pub type AgreementId = Hash;

/// Unique identifier for an encrypted communication session.
pub type SessionId = Hash;

/// Unique identifier for a swarm of cooperating agents.
pub type SwarmId = Hash;

/// Unique identifier for a service listing on the marketplace.
pub type ListingId = Hash;

/// Unique identifier for a sensor/device attestation.
pub type AttestationId = Hash;

/// Unique identifier for a social recovery request.
pub type RecoveryId = Hash;

/// Compact identifier for a device hardware interface (e.g., camera, GPS, motor).
pub type InterfaceId = u32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_display_and_from_str_roundtrip() {
        let addr = Address([0xab; 32]);
        let s = addr.to_string();
        let parsed: Address = s.parse().unwrap();
        assert_eq!(addr, parsed);
    }

    #[test]
    fn hash_display_and_from_str_roundtrip() {
        let h = Hash([0xcd; 32]);
        let s = h.to_string();
        let parsed: Hash = s.parse().unwrap();
        assert_eq!(h, parsed);
    }

    #[test]
    fn address_from_pubkey() {
        use ed25519_dalek::SigningKey;
        let signing = SigningKey::from_bytes(&[1u8; 32]);
        let verifying = signing.verifying_key();
        let addr = Address::from_pubkey(&verifying);
        // Address should be SHA-256 of the public key bytes
        let mut hasher = Sha256::new();
        hasher.update(verifying.as_bytes());
        let expected: [u8; 32] = hasher.finalize().into();
        assert_eq!(addr.0, expected);
    }
}
