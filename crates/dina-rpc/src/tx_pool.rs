use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Serialize;

use dina_core::transaction::Transaction;
use dina_core::types::{Address, Hash};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the enhanced transaction pool.
#[derive(Clone, Debug)]
pub struct TxPoolConfig {
    /// Maximum number of pending (ready-to-mine) transactions.
    pub max_pending: usize,
    /// Maximum queued transactions per account (future nonce).
    pub max_queued_per_account: usize,
    /// Maximum serialized transaction size in bytes.
    pub max_tx_size_bytes: usize,
    /// Minimum fee (micro-USDC) to accept a transaction.
    pub min_fee: u64,
    /// How long (in seconds) before an un-mined transaction is evicted.
    pub eviction_interval_secs: u64,
}

impl Default for TxPoolConfig {
    fn default() -> Self {
        Self {
            max_pending: 10_000,
            max_queued_per_account: 100,
            max_tx_size_bytes: 1_048_576, // 1 MB
            min_fee: 100,                 // $0.0001
            eviction_interval_secs: 300,  // 5 minutes
        }
    }
}

// ---------------------------------------------------------------------------
// Pool entry
// ---------------------------------------------------------------------------

/// A transaction stored in the pool with metadata.
#[derive(Clone, Debug)]
pub struct PoolEntry {
    /// The transaction itself.
    pub tx: Transaction,
    /// Unix timestamp (seconds) when the transaction was received.
    pub received_at: u64,
    /// Estimated gas units for this transaction.
    pub gas_estimate: u64,
}

// ---------------------------------------------------------------------------
// Pool status
// ---------------------------------------------------------------------------

/// Summary of the current transaction pool state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TxPoolStatus {
    /// Number of pending (ready-to-mine) transactions.
    pub pending: usize,
    /// Number of queued (future nonce) transactions.
    pub queued: usize,
    /// Total USDC value (micro-USDC) across all pending transfer transactions.
    pub total_value: u64,
}

// ---------------------------------------------------------------------------
// Transaction pool
// ---------------------------------------------------------------------------

/// Enhanced transaction pool with fee-priority ordering, duplicate detection,
/// future-nonce queuing, and time-based eviction.
pub struct TxPool {
    /// Pending transactions sorted by fee (highest first).
    /// Key is `(u64::MAX - fee, insertion_seq)` so BTreeMap gives descending fee order.
    pending: BTreeMap<(u64, u64), PoolEntry>,
    /// Transactions with future nonces, keyed by sender address.
    queued: HashMap<Address, Vec<Transaction>>,
    /// Set of all known transaction hashes (pending + queued) for dedup.
    known_hashes: HashSet<Hash>,
    /// Pool configuration.
    config: TxPoolConfig,
    /// Monotonically increasing sequence number to break fee ties.
    seq: u64,
}

/// Errors that can occur when adding a transaction to the pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxPoolError {
    /// Transaction already exists in the pool.
    AlreadyKnown,
    /// Transaction fee is below the minimum.
    FeeTooLow { min: u64, got: u64 },
    /// Transaction is too large.
    TooLarge { max: usize, got: usize },
    /// The pool is full and this transaction's fee is not high enough to evict.
    PoolFull,
    /// The account's queued transaction limit has been reached.
    QueuedLimitReached { address: Address },
}

impl std::fmt::Display for TxPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyKnown => write!(f, "transaction already known"),
            Self::FeeTooLow { min, got } => {
                write!(f, "fee too low: minimum {min}, got {got}")
            }
            Self::TooLarge { max, got } => {
                write!(f, "transaction too large: max {max} bytes, got {got}")
            }
            Self::PoolFull => write!(f, "transaction pool is full"),
            Self::QueuedLimitReached { address } => {
                write!(f, "queued limit reached for account {address}")
            }
        }
    }
}

impl std::error::Error for TxPoolError {}

impl TxPool {
    /// Create a new, empty transaction pool.
    pub fn new(config: TxPoolConfig) -> Self {
        Self {
            pending: BTreeMap::new(),
            queued: HashMap::new(),
            known_hashes: HashSet::new(),
            config,
            seq: 0,
        }
    }

    /// Add a transaction to the pool.
    ///
    /// Returns the transaction hash on success.
    pub fn add(&mut self, tx: Transaction) -> Result<Hash, TxPoolError> {
        let hash = tx.hash();

        // Reject duplicates.
        if self.known_hashes.contains(&hash) {
            return Err(TxPoolError::AlreadyKnown);
        }

        // Enforce minimum fee.
        let fee = tx.fee();
        if fee < self.config.min_fee {
            return Err(TxPoolError::FeeTooLow {
                min: self.config.min_fee,
                got: fee,
            });
        }

        // Enforce max transaction size.
        let size = serde_json::to_vec(&tx).map(|v| v.len()).unwrap_or(0);
        if size > self.config.max_tx_size_bytes {
            return Err(TxPoolError::TooLarge {
                max: self.config.max_tx_size_bytes,
                got: size,
            });
        }

        // Check pending capacity.
        if self.pending.len() >= self.config.max_pending {
            // Check if this tx has a higher fee than the lowest in the pool.
            if let Some((&lowest_key, _)) = self.pending.last_key_value() {
                let lowest_fee = u64::MAX - lowest_key.0;
                if fee <= lowest_fee {
                    return Err(TxPoolError::PoolFull);
                }
                // Evict the lowest-fee entry to make room.
                let evicted = self.pending.remove(&lowest_key).unwrap();
                self.known_hashes.remove(&evicted.tx.hash());
            }
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = PoolEntry {
            tx: tx.clone(),
            received_at: now,
            gas_estimate: fee, // Use fee as gas estimate proxy.
        };

        // Insert into pending, sorted by fee descending.
        let key = (u64::MAX - fee, self.seq);
        self.seq += 1;
        self.pending.insert(key, entry);
        self.known_hashes.insert(hash);

        Ok(hash)
    }

    /// Remove a transaction from the pool by hash.
    pub fn remove(&mut self, hash: &Hash) {
        if !self.known_hashes.remove(hash) {
            return;
        }

        // Search pending.
        self.pending.retain(|_, entry| entry.tx.hash() != *hash);

        // Search queued.
        for txs in self.queued.values_mut() {
            txs.retain(|tx| tx.hash() != *hash);
        }
        self.queued.retain(|_, txs| !txs.is_empty());
    }

    /// Get up to `limit` pending transactions, ordered by fee descending.
    pub fn get_pending(&self, limit: usize) -> Vec<&Transaction> {
        self.pending
            .values()
            .take(limit)
            .map(|entry| &entry.tx)
            .collect()
    }

    /// Promote queued transactions for `address` whose nonce is now current.
    ///
    /// Call this after a transaction from `address` is mined with `nonce`.
    /// Any queued transaction with `nonce + 1` becomes promotable.
    pub fn promote_queued(&mut self, address: &Address, nonce: u64) {
        let next_nonce = nonce + 1;
        let txs = match self.queued.get_mut(address) {
            Some(txs) => txs,
            None => return,
        };

        // Find and remove transactions that are now ready.
        let mut promoted = Vec::new();
        txs.retain(|tx| {
            if tx.nonce() == next_nonce {
                promoted.push(tx.clone());
                false
            } else {
                true
            }
        });

        if txs.is_empty() {
            self.queued.remove(address);
        }

        // Add promoted transactions to pending.
        for tx in promoted {
            let fee = tx.fee();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let entry = PoolEntry {
                tx,
                received_at: now,
                gas_estimate: fee,
            };

            let key = (u64::MAX - fee, self.seq);
            self.seq += 1;
            self.pending.insert(key, entry);
        }
    }

    /// Add a transaction to the queued set (future nonce).
    pub fn queue(&mut self, tx: Transaction) -> Result<Hash, TxPoolError> {
        let hash = tx.hash();
        let sender = tx.sender();

        if self.known_hashes.contains(&hash) {
            return Err(TxPoolError::AlreadyKnown);
        }

        let queue = self.queued.entry(sender).or_default();
        if queue.len() >= self.config.max_queued_per_account {
            return Err(TxPoolError::QueuedLimitReached { address: sender });
        }

        queue.push(tx);
        self.known_hashes.insert(hash);
        Ok(hash)
    }

    /// Get the current pool status.
    pub fn status(&self) -> TxPoolStatus {
        let queued: usize = self.queued.values().map(|v| v.len()).sum();

        let total_value: u64 = self
            .pending
            .values()
            .map(|entry| match &entry.tx {
                Transaction::Transfer { amount, .. } => *amount,
                Transaction::CallContract { usdc_attached, .. } => *usdc_attached,
                _ => 0,
            })
            .sum();

        TxPoolStatus {
            pending: self.pending.len(),
            queued,
            total_value,
        }
    }

    /// Evict transactions older than the eviction interval.
    pub fn evict_expired(&mut self, current_time: u64) {
        let cutoff = current_time.saturating_sub(self.config.eviction_interval_secs);

        let to_remove: Vec<(u64, u64)> = self
            .pending
            .iter()
            .filter(|(_, entry)| entry.received_at < cutoff)
            .map(|(key, _)| *key)
            .collect();

        for key in to_remove {
            if let Some(entry) = self.pending.remove(&key) {
                self.known_hashes.remove(&entry.tx.hash());
            }
        }
    }

    /// Check whether a transaction with this hash is already in the pool.
    pub fn contains(&self, hash: &Hash) -> bool {
        self.known_hashes.contains(hash)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::transaction::Sig64;
    use dina_core::types::Address;

    fn dummy_transfer(from_byte: u8, nonce: u64, fee: u64, amount: u64) -> Transaction {
        Transaction::Transfer {
            from: Address([from_byte; 32]),
            to: Address([0xBB; 32]),
            amount,
            memo: None,
            device_witness: None,
            nonce,
            fee,
            signature: Sig64([0u8; 64]),
        }
    }

    fn default_pool() -> TxPool {
        TxPool::new(TxPoolConfig::default())
    }

    // -- Basic add / remove -------------------------------------------------

    #[test]
    fn add_and_contains() {
        let mut pool = default_pool();
        let tx = dummy_transfer(0x01, 0, 200, 1_000_000);
        let hash = pool.add(tx.clone()).unwrap();
        assert!(pool.contains(&hash));
    }

    #[test]
    fn remove_transaction() {
        let mut pool = default_pool();
        let tx = dummy_transfer(0x01, 0, 200, 1_000_000);
        let hash = pool.add(tx).unwrap();
        assert!(pool.contains(&hash));

        pool.remove(&hash);
        assert!(!pool.contains(&hash));
    }

    #[test]
    fn remove_nonexistent_is_noop() {
        let mut pool = default_pool();
        pool.remove(&Hash::ZERO); // should not panic
    }

    // -- Duplicate detection ------------------------------------------------

    #[test]
    fn reject_duplicate() {
        let mut pool = default_pool();
        let tx = dummy_transfer(0x01, 0, 200, 1_000_000);
        pool.add(tx.clone()).unwrap();

        assert_eq!(pool.add(tx).unwrap_err(), TxPoolError::AlreadyKnown);
    }

    // -- Fee enforcement ----------------------------------------------------

    #[test]
    fn reject_low_fee() {
        let mut pool = default_pool();
        let tx = dummy_transfer(0x01, 0, 10, 1_000_000); // fee < min_fee (100)
        assert_eq!(
            pool.add(tx).unwrap_err(),
            TxPoolError::FeeTooLow { min: 100, got: 10 }
        );
    }

    // -- Size enforcement ---------------------------------------------------

    #[test]
    fn reject_oversized_transaction() {
        let config = TxPoolConfig {
            max_tx_size_bytes: 10, // absurdly small
            ..TxPoolConfig::default()
        };
        let mut pool = TxPool::new(config);
        let tx = dummy_transfer(0x01, 0, 200, 1_000_000);

        match pool.add(tx) {
            Err(TxPoolError::TooLarge { .. }) => {} // expected
            other => panic!("expected TooLarge, got {:?}", other),
        }
    }

    // -- Fee-priority ordering ----------------------------------------------

    #[test]
    fn get_pending_ordered_by_fee_desc() {
        let mut pool = default_pool();
        let tx_low = dummy_transfer(0x01, 0, 200, 100);
        let tx_high = dummy_transfer(0x02, 0, 10_000, 100);
        let tx_mid = dummy_transfer(0x03, 0, 1_000, 100);

        pool.add(tx_low).unwrap();
        pool.add(tx_high).unwrap();
        pool.add(tx_mid).unwrap();

        let pending = pool.get_pending(10);
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].fee(), 10_000);
        assert_eq!(pending[1].fee(), 1_000);
        assert_eq!(pending[2].fee(), 200);
    }

    #[test]
    fn get_pending_respects_limit() {
        let mut pool = default_pool();
        for i in 0..5u8 {
            let tx = dummy_transfer(i, 0, 200 + i as u64, 100);
            pool.add(tx).unwrap();
        }

        let pending = pool.get_pending(2);
        assert_eq!(pending.len(), 2);
    }

    // -- Pool capacity eviction ---------------------------------------------

    #[test]
    fn evicts_lowest_fee_when_full() {
        let config = TxPoolConfig {
            max_pending: 2,
            min_fee: 100,
            ..TxPoolConfig::default()
        };
        let mut pool = TxPool::new(config);

        let tx_low = dummy_transfer(0x01, 0, 200, 100);
        let tx_mid = dummy_transfer(0x02, 0, 500, 100);
        let tx_high = dummy_transfer(0x03, 0, 1_000, 100);

        let hash_low = pool.add(tx_low).unwrap();
        pool.add(tx_mid).unwrap();

        // Pool is full (2). Adding a higher-fee tx should evict the lowest.
        pool.add(tx_high).unwrap();

        assert!(!pool.contains(&hash_low)); // evicted
        assert_eq!(pool.status().pending, 2);
    }

    #[test]
    fn rejects_when_full_and_fee_too_low() {
        let config = TxPoolConfig {
            max_pending: 2,
            min_fee: 100,
            ..TxPoolConfig::default()
        };
        let mut pool = TxPool::new(config);

        pool.add(dummy_transfer(0x01, 0, 500, 100)).unwrap();
        pool.add(dummy_transfer(0x02, 0, 500, 100)).unwrap();

        // Try to add a tx with equal fee — should be rejected.
        let result = pool.add(dummy_transfer(0x03, 0, 200, 100));
        assert_eq!(result.unwrap_err(), TxPoolError::PoolFull);
    }

    // -- Status -------------------------------------------------------------

    #[test]
    fn status_reflects_pool_state() {
        let mut pool = default_pool();
        let tx1 = dummy_transfer(0x01, 0, 200, 5_000_000);
        let tx2 = dummy_transfer(0x02, 0, 300, 3_000_000);

        pool.add(tx1).unwrap();
        pool.add(tx2).unwrap();

        let status = pool.status();
        assert_eq!(status.pending, 2);
        assert_eq!(status.queued, 0);
        assert_eq!(status.total_value, 8_000_000);
    }

    // -- Time-based eviction ------------------------------------------------

    #[test]
    fn evict_expired_removes_old_entries() {
        let config = TxPoolConfig {
            eviction_interval_secs: 300,
            ..TxPoolConfig::default()
        };
        let mut pool = TxPool::new(config);

        let tx = dummy_transfer(0x01, 0, 200, 100);
        let hash = pool.add(tx).unwrap();

        // Force the received_at to an old time.
        for entry in pool.pending.values_mut() {
            entry.received_at = 1000;
        }

        // current_time = 1400 => cutoff = 1100, entry at 1000 < 1100 => evicted
        pool.evict_expired(1400);
        assert!(!pool.contains(&hash));
        assert_eq!(pool.status().pending, 0);
    }

    #[test]
    fn evict_expired_keeps_recent() {
        let mut pool = default_pool();
        let tx = dummy_transfer(0x01, 0, 200, 100);
        pool.add(tx).unwrap();

        // Force a recent timestamp.
        for entry in pool.pending.values_mut() {
            entry.received_at = 5000;
        }

        // current_time = 5100 => cutoff = 4800, entry at 5000 >= 4800 => kept
        pool.evict_expired(5100);
        assert_eq!(pool.status().pending, 1);
    }

    // -- Queued transactions ------------------------------------------------

    #[test]
    fn queue_and_promote() {
        let mut pool = default_pool();
        let addr = Address([0x01; 32]);
        let future_tx = dummy_transfer(0x01, 5, 500, 100);
        let hash = pool.queue(future_tx).unwrap();

        assert!(pool.contains(&hash));
        assert_eq!(pool.status().queued, 1);
        assert_eq!(pool.status().pending, 0);

        // Promote: nonce 4 was just mined, so nonce 5 becomes ready.
        pool.promote_queued(&addr, 4);
        assert_eq!(pool.status().queued, 0);
        assert_eq!(pool.status().pending, 1);
    }

    #[test]
    fn queue_limit_per_account() {
        let config = TxPoolConfig {
            max_queued_per_account: 2,
            ..TxPoolConfig::default()
        };
        let mut pool = TxPool::new(config);

        pool.queue(dummy_transfer(0x01, 10, 200, 100)).unwrap();
        pool.queue(dummy_transfer(0x01, 11, 200, 100)).unwrap();

        let result = pool.queue(dummy_transfer(0x01, 12, 200, 100));
        match result {
            Err(TxPoolError::QueuedLimitReached { .. }) => {} // expected
            other => panic!("expected QueuedLimitReached, got {:?}", other),
        }
    }

    // -- Default config values ----------------------------------------------

    #[test]
    fn default_config() {
        let cfg = TxPoolConfig::default();
        assert_eq!(cfg.max_pending, 10_000);
        assert_eq!(cfg.max_queued_per_account, 100);
        assert_eq!(cfg.max_tx_size_bytes, 1_048_576);
        assert_eq!(cfg.min_fee, 100);
        assert_eq!(cfg.eviction_interval_secs, 300);
    }
}
