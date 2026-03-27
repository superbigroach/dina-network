//! GossipSub topic management and message publishing for the Dina network.
//!
//! Three topics partition network traffic:
//! - `dina/transactions/v1` -- new transactions
//! - `dina/blocks/v1`       -- new blocks
//! - `dina/consensus/v1`    -- proposals, votes, view-changes

use libp2p::gossipsub::{
    self, IdentTopic, MessageAuthenticity, MessageId, Topic, ValidationMode,
};
use libp2p::identity::Keypair;
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

use crate::message::{
    BlockPayload, NetworkMessage, Proposal, TransactionPayload, ViewChange, Vote,
};

/// The three canonical GossipSub topics.
pub const TOPIC_TRANSACTIONS: &str = "dina/transactions/v1";
pub const TOPIC_BLOCKS: &str = "dina/blocks/v1";
pub const TOPIC_CONSENSUS: &str = "dina/consensus/v1";

/// Create the GossipSub topics as `IdentTopic` instances.
pub fn topics() -> [IdentTopic; 3] {
    [
        IdentTopic::new(TOPIC_TRANSACTIONS),
        IdentTopic::new(TOPIC_BLOCKS),
        IdentTopic::new(TOPIC_CONSENSUS),
    ]
}

/// Build a configured `gossipsub::Behaviour` with Dina-specific settings.
///
/// Message IDs are derived from the SHA-256 of the message payload so that
/// duplicate messages from different relays are deduplicated automatically.
pub fn build_gossipsub(keypair: &Keypair) -> Result<gossipsub::Behaviour, String> {
    let message_id_fn = |message: &gossipsub::Message| {
        let mut hasher = Sha256::new();
        hasher.update(&message.data);
        let hash = hasher.finalize();
        MessageId::from(hash.to_vec())
    };

    let config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(std::time::Duration::from_secs(1))
        .validation_mode(ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .max_transmit_size(2 * 1024 * 1024) // 2 MiB max message
        .build()
        .map_err(|e| e.to_string())?;

    gossipsub::Behaviour::new(
        MessageAuthenticity::Signed(keypair.clone()),
        config,
    )
    .map_err(|e| e.to_string())
}

/// High-level wrapper for publishing typed messages to the right GossipSub topics.
pub struct DinaGossip;

impl DinaGossip {
    /// Publish a transaction to the transactions topic.
    pub fn publish_transaction(
        gossipsub: &mut gossipsub::Behaviour,
        tx: TransactionPayload,
    ) -> Result<MessageId, PublishError> {
        let msg = NetworkMessage::Transaction(tx);
        let data = msg
            .to_bytes()
            .map_err(|e| PublishError::Serialization(e.to_string()))?;
        let topic = IdentTopic::new(TOPIC_TRANSACTIONS);
        debug!(topic = TOPIC_TRANSACTIONS, bytes = data.len(), "publishing transaction");
        gossipsub
            .publish(topic, data)
            .map_err(|e| PublishError::Gossipsub(e.to_string()))
    }

    /// Publish a block to the blocks topic.
    pub fn publish_block(
        gossipsub: &mut gossipsub::Behaviour,
        block: BlockPayload,
    ) -> Result<MessageId, PublishError> {
        let msg = NetworkMessage::Block(block);
        let data = msg
            .to_bytes()
            .map_err(|e| PublishError::Serialization(e.to_string()))?;
        let topic = IdentTopic::new(TOPIC_BLOCKS);
        debug!(topic = TOPIC_BLOCKS, bytes = data.len(), "publishing block");
        gossipsub
            .publish(topic, data)
            .map_err(|e| PublishError::Gossipsub(e.to_string()))
    }

    /// Publish a consensus message (proposal, vote, or view-change) to the
    /// consensus topic.
    pub fn publish_consensus_message(
        gossipsub: &mut gossipsub::Behaviour,
        msg: NetworkMessage,
    ) -> Result<MessageId, PublishError> {
        match &msg {
            NetworkMessage::Proposal(_)
            | NetworkMessage::Vote(_)
            | NetworkMessage::ViewChange(_) => {}
            other => {
                warn!(label = other.label(), "attempted to publish non-consensus message on consensus topic");
                return Err(PublishError::WrongTopic(
                    "only Proposal, Vote, and ViewChange go on the consensus topic".into(),
                ));
            }
        }

        let data = msg
            .to_bytes()
            .map_err(|e| PublishError::Serialization(e.to_string()))?;
        let topic = IdentTopic::new(TOPIC_CONSENSUS);
        debug!(topic = TOPIC_CONSENSUS, bytes = data.len(), "publishing consensus message");
        gossipsub
            .publish(topic, data)
            .map_err(|e| PublishError::Gossipsub(e.to_string()))
    }

    /// Validate an incoming gossip message by checking basic structural
    /// integrity. Returns the deserialized `NetworkMessage` on success.
    ///
    /// This performs:
    /// 1. Bincode deserialization
    /// 2. Signature field presence checks (non-zero)
    ///
    /// Full cryptographic verification is deferred to the consensus/mempool
    /// layers so the network layer stays fast.
    pub fn validate_message(data: &[u8]) -> Result<NetworkMessage, PublishError> {
        let msg = NetworkMessage::from_bytes(data)
            .map_err(|e| PublishError::Serialization(e.to_string()))?;

        // Basic sanity: check that signed messages have non-zero signatures.
        match &msg {
            NetworkMessage::Proposal(p) => {
                if p.signature == [0u8; 64] {
                    return Err(PublishError::InvalidMessage(
                        "proposal has zero signature".into(),
                    ));
                }
            }
            NetworkMessage::Vote(v) => {
                if v.signature == [0u8; 64] {
                    return Err(PublishError::InvalidMessage(
                        "vote has zero signature".into(),
                    ));
                }
            }
            NetworkMessage::ViewChange(vc) => {
                if vc.signature == [0u8; 64] {
                    return Err(PublishError::InvalidMessage(
                        "view-change has zero signature".into(),
                    ));
                }
            }
            // Transactions are validated by the mempool, blocks by the chain.
            _ => {}
        }

        Ok(msg)
    }
}

/// Errors that can occur when publishing or validating gossip messages.
#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("gossipsub publish error: {0}")]
    Gossipsub(String),
    #[error("wrong topic for message: {0}")]
    WrongTopic(String),
    #[error("invalid message: {0}")]
    InvalidMessage(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{TransactionPayload, Vote, VoteType};
    use dina_core::types::Hash;

    #[test]
    fn validate_rejects_zero_signature_vote() {
        let vote_msg = NetworkMessage::Vote(Vote {
            view: 1,
            block_hash: Hash([0xaa; 32]),
            vote_type: VoteType::Prevote,
            signature: [0u8; 64],
            voter: [0x01; 32],
        });
        let data = vote_msg.to_bytes().unwrap();
        let err = DinaGossip::validate_message(&data).unwrap_err();
        assert!(err.to_string().contains("zero signature"));
    }

    #[test]
    fn validate_accepts_valid_transaction() {
        let tx_msg = NetworkMessage::Transaction(TransactionPayload {
            data: vec![1, 2, 3],
            hash: Hash([0xbb; 32]),
        });
        let data = tx_msg.to_bytes().unwrap();
        let result = DinaGossip::validate_message(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn topics_are_correct() {
        let t = topics();
        assert_eq!(t[0].hash(), Topic::new(TOPIC_TRANSACTIONS).hash());
        assert_eq!(t[1].hash(), Topic::new(TOPIC_BLOCKS).hash());
        assert_eq!(t[2].hash(), Topic::new(TOPIC_CONSENSUS).hash());
    }
}
