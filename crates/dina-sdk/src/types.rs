use borsh::{BorshDeserialize, BorshSerialize};
use core::fmt;
use serde::{Deserialize, Serialize};

/// A 32-byte account address on the Dina network.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct Address(pub [u8; 32]);

impl Address {
    pub const ZERO: Self = Self([0u8; 32]);

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display as hex with 0x prefix
        write!(f, "0x")?;
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self)
    }
}

impl From<[u8; 32]> for Address {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// A 32-byte hash value (e.g., SHA-256 digest).
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub const ZERO: Self = Self([0u8; 32]);

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x")?;
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", self)
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Identifier for a physical device registered on the network.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct DeviceId(pub Vec<u8>);

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl fmt::Debug for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeviceId({})", self)
    }
}

/// Identifier for a credential (e.g., passkey, DID credential).
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct CredentialId(pub Vec<u8>);

impl fmt::Display for CredentialId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// Identifier for a service agreement between agents/devices.
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct AgreementId(pub [u8; 32]);

impl fmt::Display for AgreementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// Identifier for a swarm of collaborating agents.
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct SwarmId(pub [u8; 32]);

impl fmt::Display for SwarmId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// Identifier for a contract interface (like ERC-165 interface IDs).
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct InterfaceId(pub u32);

impl fmt::Display for InterfaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08x}", self.0)
    }
}

impl fmt::Debug for InterfaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InterfaceId({})", self)
    }
}

/// Result of a transaction execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum TxResult {
    /// Transaction succeeded.
    Success,
    /// Transaction failed with a reason.
    Failed(String),
    /// Transaction is pending a timelock expiry.
    Timelocked,
}

impl TxResult {
    pub fn is_success(&self) -> bool {
        matches!(self, TxResult::Success)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, TxResult::Failed(_))
    }

    /// Unwrap a success, panicking with the failure message if failed.
    pub fn expect_success(self, msg: &str) {
        match self {
            TxResult::Success => {}
            TxResult::Failed(reason) => panic!("{}: {}", msg, reason),
            TxResult::Timelocked => panic!("{}: transaction is timelocked", msg),
        }
    }
}

/// Per-address spending limits for smart wallet contracts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct SpendingLimits {
    /// Maximum amount per single transaction (in USDC micro-units).
    pub per_tx: u64,
    /// Maximum amount per 24-hour rolling window.
    pub daily: u64,
    /// Maximum amount per 30-day rolling window.
    pub monthly: u64,
}

impl Default for SpendingLimits {
    fn default() -> Self {
        Self {
            per_tx: u64::MAX,
            daily: u64::MAX,
            monthly: u64::MAX,
        }
    }
}

/// Tracks spending against limits within rolling windows.
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct SpendingStats {
    /// Total spent in the current daily window.
    pub spent_daily: u64,
    /// Total spent in the current monthly window.
    pub spent_monthly: u64,
    /// Timestamp when the daily window resets (Unix seconds).
    pub daily_reset_at: u64,
    /// Timestamp when the monthly window resets (Unix seconds).
    pub monthly_reset_at: u64,
}
