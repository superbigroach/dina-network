use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

/// Event types that can be subscribed to via WebSocket.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscriptionTopic {
    /// New blocks as they are committed.
    NewBlocks,
    /// New transactions entering the mempool.
    NewTransactions,
    /// Consensus state changes.
    ConsensusUpdates,
}

/// A WebSocket event pushed to subscribers.
#[derive(Debug, Clone)]
pub struct WsEvent {
    pub topic: SubscriptionTopic,
    pub payload: String,
}

/// Shared state for WebSocket subscription management.
///
/// Maintains broadcast channels for each subscription topic so that
/// multiple WebSocket clients can receive real-time updates.
#[derive(Clone)]
pub struct DinaWsState {
    senders: Arc<HashMap<SubscriptionTopic, broadcast::Sender<WsEvent>>>,
    #[allow(dead_code)]
    subscriber_count: Arc<RwLock<u32>>,
}

impl DinaWsState {
    /// Create a new WebSocket state with broadcast channels for each topic.
    pub fn new() -> Self {
        let mut senders = HashMap::new();

        // Each channel has a buffer of 256 events; slow consumers will drop old events.
        let (blocks_tx, _) = broadcast::channel(256);
        let (txs_tx, _) = broadcast::channel(256);
        let (consensus_tx, _) = broadcast::channel(256);

        senders.insert(SubscriptionTopic::NewBlocks, blocks_tx);
        senders.insert(SubscriptionTopic::NewTransactions, txs_tx);
        senders.insert(SubscriptionTopic::ConsensusUpdates, consensus_tx);

        info!("WebSocket subscription state initialized");

        Self {
            senders: Arc::new(senders),
            subscriber_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Subscribe to a topic. Returns a receiver that yields events.
    pub fn subscribe(&self, topic: &SubscriptionTopic) -> Option<broadcast::Receiver<WsEvent>> {
        self.senders.get(topic).map(|tx| {
            debug!(?topic, "new WebSocket subscriber");
            tx.subscribe()
        })
    }

    /// Publish an event to all subscribers of the given topic.
    pub fn publish(&self, event: WsEvent) {
        if let Some(tx) = self.senders.get(&event.topic) {
            let receivers = tx.receiver_count();
            if receivers > 0 {
                let _ = tx.send(event);
            }
        }
    }

    /// Get the number of active subscribers across all topics.
    pub fn total_subscribers(&self) -> usize {
        self.senders.values().map(|tx| tx.receiver_count()).sum()
    }
}

impl Default for DinaWsState {
    fn default() -> Self {
        Self::new()
    }
}
