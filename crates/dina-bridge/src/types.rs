use serde::{Deserialize, Serialize};

/// Identifies a blockchain network for cross-chain bridging.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainId {
    DinaTestnet,
    DinaMainnet,
    EthereumMainnet,
    EthereumSepolia,
    BaseMainnet,
    BaseSepolia,
    SolanaMainnet,
    SolanaDevnet,
    ArcMainnet,
    ArcTestnet,
}

impl std::fmt::Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainId::DinaTestnet => write!(f, "Dina Testnet"),
            ChainId::DinaMainnet => write!(f, "Dina Mainnet"),
            ChainId::EthereumMainnet => write!(f, "Ethereum Mainnet"),
            ChainId::EthereumSepolia => write!(f, "Ethereum Sepolia"),
            ChainId::BaseMainnet => write!(f, "Base Mainnet"),
            ChainId::BaseSepolia => write!(f, "Base Sepolia"),
            ChainId::SolanaMainnet => write!(f, "Solana Mainnet"),
            ChainId::SolanaDevnet => write!(f, "Solana Devnet"),
            ChainId::ArcMainnet => write!(f, "Arc Mainnet"),
            ChainId::ArcTestnet => write!(f, "Arc Testnet"),
        }
    }
}

/// Status of a cross-chain bridge transfer through the CCTP pipeline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BridgeStatus {
    /// Transfer initiated, waiting for source chain confirmation.
    Pending,
    /// Source chain transaction confirmed.
    SourceConfirmed,
    /// Circle attestation received for the burn event.
    AttestationReceived,
    /// Mint transaction confirmed on Dina Network.
    DinaConfirmed,
    /// Full round-trip completed successfully.
    Completed,
    /// Transfer failed with a reason.
    Failed(String),
}

/// Represents a single cross-chain bridge transfer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeTransfer {
    /// Unique identifier for this transfer (SHA-256 of transfer details).
    pub id: [u8; 32],
    /// The chain where funds are being sent from.
    pub from_chain: ChainId,
    /// The chain where funds are being received.
    pub to_chain: ChainId,
    /// Sender address in source chain format.
    pub sender: Vec<u8>,
    /// Recipient address as a Dina Ed25519 address (32 bytes).
    pub recipient: [u8; 32],
    /// Amount of USDC in micro-units (6 decimals, so 1 USDC = 1_000_000).
    pub amount: u64,
    /// Monotonically increasing nonce for ordering.
    pub nonce: u64,
    /// Current status of the transfer.
    pub status: BridgeStatus,
    /// Unix timestamp when the transfer was created.
    pub created_at: u64,
    /// Unix timestamp when the transfer completed, if applicable.
    pub completed_at: Option<u64>,
    /// Transaction hash on the source chain.
    pub source_tx_hash: Option<Vec<u8>>,
    /// Transaction hash on Dina Network.
    pub dina_tx_hash: Option<[u8; 32]>,
}

/// Aggregate statistics for the bridge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BridgeStats {
    /// Total USDC (micro-units) bridged into Dina Network.
    pub total_bridged_in: u64,
    /// Total USDC (micro-units) bridged out of Dina Network.
    pub total_bridged_out: u64,
    /// Number of transfers currently in progress.
    pub pending_count: usize,
    /// Number of completed transfers.
    pub completed_count: usize,
    /// Number of failed transfers.
    pub failed_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_id_display() {
        assert_eq!(ChainId::DinaTestnet.to_string(), "Dina Testnet");
        assert_eq!(ChainId::BaseMainnet.to_string(), "Base Mainnet");
        assert_eq!(ChainId::SolanaDevnet.to_string(), "Solana Devnet");
    }

    #[test]
    fn bridge_status_serialization_roundtrip() {
        let statuses = vec![
            BridgeStatus::Pending,
            BridgeStatus::SourceConfirmed,
            BridgeStatus::AttestationReceived,
            BridgeStatus::DinaConfirmed,
            BridgeStatus::Completed,
            BridgeStatus::Failed("timeout".to_string()),
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: BridgeStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn bridge_transfer_serialization_roundtrip() {
        let transfer = BridgeTransfer {
            id: [0xaa; 32],
            from_chain: ChainId::BaseMainnet,
            to_chain: ChainId::DinaMainnet,
            sender: vec![0x11; 20],
            recipient: [0x22; 32],
            amount: 100_000_000,
            nonce: 1,
            status: BridgeStatus::Pending,
            created_at: 1700000000,
            completed_at: None,
            source_tx_hash: Some(vec![0x33; 32]),
            dina_tx_hash: None,
        };
        let json = serde_json::to_string(&transfer).unwrap();
        let deserialized: BridgeTransfer = serde_json::from_str(&json).unwrap();
        assert_eq!(transfer, deserialized);
    }
}
