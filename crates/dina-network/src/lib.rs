//! `dina-network` -- P2P networking layer for the Dina blockchain.
//!
//! This crate provides:
//! - [`node::DinaNode`] -- the main network node with libp2p swarm
//! - [`gossip`] -- GossipSub topic management and message publishing
//! - [`discovery`] -- mDNS + Kademlia peer discovery
//! - [`peer`] -- peer tracking, scoring, and ban management
//! - [`message`] -- network message types with bincode serialization

pub mod discovery;
pub mod gossip;
pub mod message;
pub mod node;
pub mod peer;
pub mod sync;

// Re-export primary types for convenience.
pub use discovery::{DiscoveryConfig, DiscoveryState};
pub use gossip::{DinaGossip, PublishError};
pub use message::{
    BlockPayload, NetworkMessage, Proposal, TransactionPayload, ViewChange, Vote, VoteType,
};
pub use node::{CommandHandle, DinaBehaviour, DinaNode, DinaNodeHandle, NodeCommand, NodeEvent};
pub use peer::{PeerInfo, PeerManager, PeerManagerConfig};
pub use sync::{SyncManager, SyncRequest, SyncState};
