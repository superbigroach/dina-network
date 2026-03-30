//! Network message types for the Dina P2P layer.
//!
//! These messages are serialized with bincode for efficient wire transfer
//! between nodes. Each variant maps to a GossipSub topic or a direct
//! request/response exchange.

use dina_core::types::Hash;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

/// A serialized transaction ready for network propagation.
///
/// We use an opaque bytes representation so that the network layer does not
/// need to depend on the full transaction validation logic. The inner bytes
/// are a bincode-encoded `dina_core::Transaction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPayload {
    /// Bincode-encoded transaction bytes.
    pub data: Vec<u8>,
    /// SHA-256 hash of the transaction for deduplication.
    pub hash: Hash,
}

/// A serialized block ready for network propagation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPayload {
    /// Bincode-encoded block bytes.
    pub data: Vec<u8>,
    /// Block height for quick filtering during sync.
    pub height: u64,
    /// Block hash for deduplication.
    pub hash: Hash,
}

/// A consensus proposal from the current round leader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Block height being proposed.
    pub height: u64,
    /// Consensus round within this height.
    pub round: u32,
    /// The proposed block payload.
    pub block: BlockPayload,
    /// Ed25519 signature from the proposer.
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    /// Proposer's public key bytes.
    pub proposer: [u8; 32],
}

/// A validator vote on a proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Block height this vote is for.
    pub height: u64,
    /// Consensus round this vote is for.
    pub round: u32,
    /// Hash of the block being voted on.
    pub block_hash: Hash,
    /// Whether this is a prevote or precommit.
    pub vote_type: VoteType,
    /// Ed25519 signature over (height || round || block_hash || vote_type).
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    /// Voter's public key bytes.
    pub voter: [u8; 32],
}

/// The phase of a consensus vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteType {
    Prevote,
    Precommit,
}

/// A view-change request triggered when the leader is unresponsive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChange {
    /// Block height for this view change.
    pub height: u64,
    /// The round being abandoned.
    pub old_round: u32,
    /// The proposed new round.
    pub new_round: u32,
    /// Ed25519 signature over (height || old_round || new_round).
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    /// Requester's public key bytes.
    pub requester: [u8; 32],
}

/// Top-level network message enum.
///
/// Serialized with bincode and published over GossipSub topics or sent
/// via direct request/response streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// A new transaction to propagate.
    Transaction(TransactionPayload),
    /// A new block to propagate.
    Block(BlockPayload),
    /// A consensus proposal from the round leader.
    Proposal(Proposal),
    /// A validator vote (prevote or precommit).
    Vote(Vote),
    /// A view-change request.
    ViewChange(ViewChange),
    /// Request blocks starting from a given height (sync protocol).
    SyncRequest { from_height: u64 },
    /// Response with a batch of blocks (sync protocol).
    SyncResponse { blocks: Vec<BlockPayload> },
}

impl NetworkMessage {
    /// Serialize this message to bincode bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize a message from bincode bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }

    /// Returns a human-readable label for logging.
    pub fn label(&self) -> &'static str {
        match self {
            NetworkMessage::Transaction(_) => "transaction",
            NetworkMessage::Block(_) => "block",
            NetworkMessage::Proposal(_) => "proposal",
            NetworkMessage::Vote(_) => "vote",
            NetworkMessage::ViewChange(_) => "view_change",
            NetworkMessage::SyncRequest { .. } => "sync_request",
            NetworkMessage::SyncResponse { .. } => "sync_response",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_transaction_message() {
        let msg = NetworkMessage::Transaction(TransactionPayload {
            data: vec![1, 2, 3, 4],
            hash: Hash([0xab; 32]),
        });
        let bytes = msg.to_bytes().unwrap();
        let decoded = NetworkMessage::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.label(), "transaction");
    }

    #[test]
    fn roundtrip_sync_request() {
        let msg = NetworkMessage::SyncRequest { from_height: 42 };
        let bytes = msg.to_bytes().unwrap();
        let decoded = NetworkMessage::from_bytes(&bytes).unwrap();
        match decoded {
            NetworkMessage::SyncRequest { from_height } => assert_eq!(from_height, 42),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn roundtrip_vote() {
        let msg = NetworkMessage::Vote(Vote {
            height: 10,
            round: 0,
            block_hash: Hash([0xcd; 32]),
            vote_type: VoteType::Precommit,
            signature: [0xff; 64],
            voter: [0x01; 32],
        });
        let bytes = msg.to_bytes().unwrap();
        let decoded = NetworkMessage::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.label(), "vote");
    }
}
