//! Block synchronization protocol for the Dina network.
//!
//! When a new node joins the network or falls behind, the [`SyncManager`]
//! coordinates fetching missing blocks in batches from peers. It tracks
//! outstanding requests, handles timeouts, and buffers received blocks
//! until they can be applied to the chain in order.

use std::collections::BTreeMap;

use dina_core::Block;

/// The current synchronization state of a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncState {
    /// Not syncing; the node is idle or already caught up.
    Idle,
    /// Actively syncing blocks in the range `[from, to]`.
    Syncing { from: u64, to: u64 },
    /// Fully caught up with the network.
    CaughtUp,
}

/// A request for a batch of blocks from a peer.
#[derive(Debug, Clone)]
pub struct SyncRequest {
    /// Start of the requested block range (inclusive).
    pub from_height: u64,
    /// End of the requested block range (inclusive).
    pub to_height: u64,
    /// Peer identifier (e.g. serialized PeerId).
    pub peer: Vec<u8>,
    /// Unix timestamp (millis) when the request was created.
    pub requested_at: u64,
    /// Timeout duration in milliseconds before re-requesting.
    pub timeout_ms: u64,
}

/// Manages block synchronization for a node that needs to catch up.
///
/// The sync manager maintains:
/// - A target height representing the known network height.
/// - A set of pending (outstanding) requests keyed by their `from_height`.
/// - A buffer of received blocks waiting to be applied in order.
pub struct SyncManager {
    /// The network height we are syncing towards.
    target_height: u64,
    /// The height up to which blocks have been applied to the chain.
    current_height: u64,
    /// Outstanding sync requests, keyed by `from_height`.
    pending_requests: BTreeMap<u64, SyncRequest>,
    /// Blocks received but not yet applied, keyed by block height.
    received_blocks: BTreeMap<u64, Block>,
    /// Current sync state.
    sync_state: SyncState,
}

impl SyncManager {
    /// Create a new sync manager starting from the given chain height.
    pub fn new(current_height: u64) -> Self {
        SyncManager {
            target_height: current_height,
            current_height,
            pending_requests: BTreeMap::new(),
            received_blocks: BTreeMap::new(),
            sync_state: SyncState::Idle,
        }
    }

    /// Check whether the node needs to sync to reach `network_height`.
    pub fn needs_sync(&self, network_height: u64) -> bool {
        network_height > self.current_height
    }

    /// Create batch sync requests to cover the range from the current height
    /// up to `target_height`.
    ///
    /// Blocks are requested in chunks of `batch_size`. Each returned
    /// `SyncRequest` has an empty `peer` field -- the caller is responsible
    /// for assigning a peer and recording `requested_at`.
    pub fn create_sync_requests(
        &mut self,
        target_height: u64,
        batch_size: u64,
    ) -> Vec<SyncRequest> {
        self.target_height = target_height;

        if target_height <= self.current_height {
            self.sync_state = SyncState::CaughtUp;
            return Vec::new();
        }

        self.sync_state = SyncState::Syncing {
            from: self.current_height + 1,
            to: target_height,
        };

        let mut requests = Vec::new();
        let mut from = self.current_height + 1;

        while from <= target_height {
            // Skip ranges we already have a pending request for
            if self.pending_requests.contains_key(&from) {
                from += batch_size;
                continue;
            }

            let to = (from + batch_size - 1).min(target_height);

            let req = SyncRequest {
                from_height: from,
                to_height: to,
                peer: Vec::new(),
                requested_at: 0,
                timeout_ms: 5_000,
            };

            self.pending_requests.insert(from, req.clone());
            requests.push(req);
            from = to + 1;
        }

        requests
    }

    /// Handle a batch of blocks received from a peer.
    ///
    /// Blocks are buffered internally. This method returns all blocks that
    /// are now contiguously available starting from `current_height + 1`,
    /// ready to be applied to the chain in order.
    pub fn on_blocks_received(
        &mut self,
        from_height: u64,
        blocks: Vec<Block>,
    ) -> Vec<Block> {
        // Remove the corresponding pending request
        self.pending_requests.remove(&from_height);

        // Buffer all received blocks
        for block in blocks {
            let height = block.header.block_number;
            self.received_blocks.insert(height, block);
        }

        // Collect contiguous blocks starting from the next expected height
        let mut ready = Vec::new();
        let mut next = self.current_height + 1;

        while let Some(block) = self.received_blocks.remove(&next) {
            ready.push(block);
            next += 1;
        }

        ready
    }

    /// Handle a sync request timeout.
    ///
    /// If a pending request exists for the given `from_height`, it is removed
    /// and returned so the caller can re-issue it to a different peer.
    pub fn on_sync_timeout(&mut self, from_height: u64) -> Option<SyncRequest> {
        self.pending_requests.remove(&from_height)
    }

    /// Return the current sync progress as `(current_height, target_height)`.
    pub fn sync_progress(&self) -> (u64, u64) {
        (self.current_height, self.target_height)
    }

    /// Check whether the node is fully synced.
    pub fn is_synced(&self) -> bool {
        self.current_height >= self.target_height
    }

    /// Mark a height as applied to the chain, advancing the sync cursor.
    ///
    /// If the current height reaches the target, the state transitions to
    /// `CaughtUp`.
    pub fn mark_applied(&mut self, height: u64) {
        if height > self.current_height {
            self.current_height = height;
        }

        if self.current_height >= self.target_height
            && self.pending_requests.is_empty()
        {
            self.sync_state = SyncState::CaughtUp;
        }
    }

    /// Return the current sync state.
    pub fn sync_state(&self) -> &SyncState {
        &self.sync_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::types::{Address, Hash};

    /// Helper: create a dummy block at the given height with a given parent hash.
    fn make_block(height: u64, parent_hash: Hash, timestamp: u64) -> Block {
        Block {
            header: dina_core::BlockHeader {
                block_number: height,
                parent_hash,
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp,
                proposer: Address::ZERO,
                signature: [0u8; 64],
            },
            transactions: Vec::new(),
        }
    }

    #[test]
    fn new_sync_manager_is_idle() {
        let sm = SyncManager::new(0);
        assert_eq!(sm.sync_progress(), (0, 0));
        assert!(sm.is_synced());
        assert_eq!(*sm.sync_state(), SyncState::Idle);
    }

    #[test]
    fn needs_sync_detects_gap() {
        let sm = SyncManager::new(5);
        assert!(sm.needs_sync(10));
        assert!(!sm.needs_sync(5));
        assert!(!sm.needs_sync(3));
    }

    #[test]
    fn create_sync_requests_batches() {
        let mut sm = SyncManager::new(0);
        let requests = sm.create_sync_requests(10, 3);

        // Should create 4 batches: [1-3], [4-6], [7-9], [10-10]
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[0].from_height, 1);
        assert_eq!(requests[0].to_height, 3);
        assert_eq!(requests[1].from_height, 4);
        assert_eq!(requests[1].to_height, 6);
        assert_eq!(requests[2].from_height, 7);
        assert_eq!(requests[2].to_height, 9);
        assert_eq!(requests[3].from_height, 10);
        assert_eq!(requests[3].to_height, 10);

        assert_eq!(sm.sync_progress(), (0, 10));
        assert!(!sm.is_synced());
        assert!(matches!(sm.sync_state(), SyncState::Syncing { from: 1, to: 10 }));
    }

    #[test]
    fn create_sync_requests_when_already_caught_up() {
        let mut sm = SyncManager::new(10);
        let requests = sm.create_sync_requests(10, 5);
        assert!(requests.is_empty());
        assert_eq!(*sm.sync_state(), SyncState::CaughtUp);
    }

    #[test]
    fn on_blocks_received_returns_contiguous() {
        let mut sm = SyncManager::new(0);
        sm.create_sync_requests(5, 3);

        // Receive blocks 1-3
        let blocks_1_3 = vec![
            make_block(1, Hash::ZERO, 100),
            make_block(2, Hash::ZERO, 200),
            make_block(3, Hash::ZERO, 300),
        ];
        let ready = sm.on_blocks_received(1, blocks_1_3);
        assert_eq!(ready.len(), 3);
        assert_eq!(ready[0].header.block_number, 1);
        assert_eq!(ready[2].header.block_number, 3);
    }

    #[test]
    fn on_blocks_received_out_of_order() {
        let mut sm = SyncManager::new(0);
        sm.create_sync_requests(6, 3);

        // Receive blocks 4-6 first (out of order)
        let blocks_4_6 = vec![
            make_block(4, Hash::ZERO, 400),
            make_block(5, Hash::ZERO, 500),
            make_block(6, Hash::ZERO, 600),
        ];
        let ready = sm.on_blocks_received(4, blocks_4_6);
        // Nothing ready because blocks 1-3 are missing
        assert!(ready.is_empty());

        // Now receive blocks 1-3
        let blocks_1_3 = vec![
            make_block(1, Hash::ZERO, 100),
            make_block(2, Hash::ZERO, 200),
            make_block(3, Hash::ZERO, 300),
        ];
        let ready = sm.on_blocks_received(1, blocks_1_3);
        // All 6 blocks should be ready now
        assert_eq!(ready.len(), 6);
    }

    #[test]
    fn mark_applied_advances_cursor() {
        let mut sm = SyncManager::new(0);
        sm.create_sync_requests(5, 5);

        // Simulate receiving and applying all blocks
        let blocks: Vec<Block> = (1..=5)
            .map(|h| make_block(h, Hash::ZERO, h * 100))
            .collect();
        let _ = sm.on_blocks_received(1, blocks);

        for h in 1..=5 {
            sm.mark_applied(h);
        }

        assert!(sm.is_synced());
        assert_eq!(*sm.sync_state(), SyncState::CaughtUp);
        assert_eq!(sm.sync_progress(), (5, 5));
    }

    #[test]
    fn on_sync_timeout_returns_request() {
        let mut sm = SyncManager::new(0);
        sm.create_sync_requests(10, 5);

        let timed_out = sm.on_sync_timeout(1);
        assert!(timed_out.is_some());
        let req = timed_out.unwrap();
        assert_eq!(req.from_height, 1);
        assert_eq!(req.to_height, 5);

        // Second call returns None since it was already removed
        assert!(sm.on_sync_timeout(1).is_none());
    }

    #[test]
    fn duplicate_create_sync_requests_skips_pending() {
        let mut sm = SyncManager::new(0);
        let first = sm.create_sync_requests(10, 5);
        assert_eq!(first.len(), 2);

        // Calling again should produce no new requests (all are still pending)
        let second = sm.create_sync_requests(10, 5);
        assert!(second.is_empty());
    }
}
