use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use tracing::{debug, info};

use dina_core::transaction::Transaction;
use dina_core::types::Hash;

/// Maximum age (in seconds) before a transaction is considered expired.
const DEFAULT_TX_EXPIRY_SECS: u64 = 3600; // 1 hour

/// Maximum number of transactions in the mempool.
const DEFAULT_MAX_SIZE: usize = 10_000;

/// A transaction mempool that orders pending transactions by fee (highest first).
///
/// Uses a BTreeMap keyed by (fee descending, arrival order) for efficient
/// retrieval of the highest-fee transactions, and a HashMap for O(1) lookup
/// and removal by hash.
pub struct Mempool {
    /// Transactions ordered by (Reverse(fee), insertion_order) for highest-fee-first iteration.
    by_fee: BTreeMap<FeeKey, MempoolEntry>,
    /// Hash -> FeeKey index for O(1) removal by hash.
    hash_index: HashMap<Hash, FeeKey>,
    /// Monotonically increasing counter for insertion ordering.
    insertion_counter: u64,
    /// Maximum number of transactions allowed.
    max_size: usize,
    /// Maximum age of a transaction before it is expired.
    expiry_duration: Duration,
}

/// Composite key for the BTreeMap: (reverse fee, insertion order).
/// Using negated fee so that BTreeMap natural ordering gives highest fees first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct FeeKey {
    /// Fee stored as negated value so that BTreeMap ordering gives highest first.
    neg_fee: i128,
    /// Insertion counter for FIFO among same-fee transactions.
    order: u64,
}

/// An entry in the mempool.
#[derive(Debug, Clone)]
struct MempoolEntry {
    transaction: Transaction,
    hash: Hash,
    added_at: Instant,
}

impl Mempool {
    /// Create a new empty mempool with default limits.
    pub fn new() -> Self {
        Self {
            by_fee: BTreeMap::new(),
            hash_index: HashMap::new(),
            insertion_counter: 0,
            max_size: DEFAULT_MAX_SIZE,
            expiry_duration: Duration::from_secs(DEFAULT_TX_EXPIRY_SECS),
        }
    }

    /// Create a mempool with custom size and expiry limits.
    pub fn with_limits(max_size: usize, expiry_secs: u64) -> Self {
        Self {
            by_fee: BTreeMap::new(),
            hash_index: HashMap::new(),
            insertion_counter: 0,
            max_size,
            expiry_duration: Duration::from_secs(expiry_secs),
        }
    }

    /// Add a transaction to the mempool.
    ///
    /// Returns an error if:
    /// - The transaction is already in the mempool.
    /// - The mempool is full and the new transaction fee is not higher than
    ///   the lowest-fee transaction.
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<()> {
        let hash = tx.hash();

        // Check for duplicates
        if self.hash_index.contains_key(&hash) {
            bail!("transaction {} already in mempool", hash);
        }

        // Check capacity
        if self.by_fee.len() >= self.max_size {
            // Evict the lowest-fee transaction if the new one pays more
            if let Some((&lowest_key, lowest_entry)) = self.by_fee.last_key_value() {
                let lowest_fee = lowest_entry.transaction.fee();
                if tx.fee() <= lowest_fee {
                    bail!(
                        "mempool full ({} txs) and new tx fee {} <= lowest fee {}",
                        self.max_size,
                        tx.fee(),
                        lowest_fee
                    );
                }
                // Evict the lowest-fee transaction
                let evicted_hash = lowest_entry.hash;
                debug!(
                    %evicted_hash,
                    fee = lowest_fee,
                    "evicting lowest-fee transaction to make room"
                );
                self.hash_index.remove(&evicted_hash);
                self.by_fee.remove(&lowest_key);
            }
        }

        let fee_key = FeeKey {
            neg_fee: -(tx.fee() as i128),
            order: self.insertion_counter,
        };
        self.insertion_counter += 1;

        self.hash_index.insert(hash, fee_key);
        self.by_fee.insert(
            fee_key,
            MempoolEntry {
                transaction: tx,
                hash,
                added_at: Instant::now(),
            },
        );

        debug!(%hash, size = self.by_fee.len(), "transaction added to mempool");
        Ok(())
    }

    /// Remove a transaction by its hash (e.g., after it has been included in a block).
    pub fn remove_transaction(&mut self, hash: &Hash) {
        if let Some(fee_key) = self.hash_index.remove(hash) {
            self.by_fee.remove(&fee_key);
            debug!(%hash, size = self.by_fee.len(), "transaction removed from mempool");
        }
    }

    /// Get the top N pending transactions ordered by fee (highest first).
    pub fn get_pending(&self, limit: usize) -> Vec<Transaction> {
        self.by_fee
            .values()
            .take(limit)
            .map(|entry| entry.transaction.clone())
            .collect()
    }

    /// Return the number of transactions in the mempool.
    pub fn size(&self) -> usize {
        self.by_fee.len()
    }

    /// Return true if the mempool is empty.
    pub fn is_empty(&self) -> bool {
        self.by_fee.is_empty()
    }

    /// Check if a transaction with the given hash is in the mempool.
    pub fn contains(&self, hash: &Hash) -> bool {
        self.hash_index.contains_key(hash)
    }

    /// Remove all transactions that are older than the expiry duration.
    /// Returns the number of transactions removed.
    pub fn clear_expired(&mut self) -> usize {
        let expired_keys: Vec<(FeeKey, Hash)> = self
            .by_fee
            .iter()
            .filter(|(_, entry)| entry.added_at.elapsed() > self.expiry_duration)
            .map(|(&key, entry)| (key, entry.hash))
            .collect();

        let count = expired_keys.len();
        for (key, hash) in expired_keys {
            self.by_fee.remove(&key);
            self.hash_index.remove(&hash);
        }

        if count > 0 {
            info!(
                expired = count,
                remaining = self.by_fee.len(),
                "cleared expired transactions from mempool"
            );
        }

        count
    }

    /// Remove all transactions from the mempool.
    pub fn clear(&mut self) {
        let size = self.by_fee.len();
        self.by_fee.clear();
        self.hash_index.clear();
        if size > 0 {
            info!(removed = size, "mempool cleared");
        }
    }

    /// Remove a batch of transactions by their hashes.
    pub fn remove_batch(&mut self, hashes: &[Hash]) {
        for hash in hashes {
            self.remove_transaction(hash);
        }
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::crypto;
    use dina_core::transaction::Sig64;
    use dina_core::types::Address;

    fn make_transfer(fee: u64, nonce: u64) -> Transaction {
        let (sk, _vk) = crypto::generate_keypair();
        let from = Address::from_pubkey(&sk.verifying_key());
        let to = Address([0xbb; 32]);

        let mut tx = Transaction::Transfer {
            from,
            to,
            amount: 1000,
            memo: None,
            device_witness: None,
            nonce,
            fee,
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(&sk, &msg);

        if let Transaction::Transfer {
            ref mut signature, ..
        } = tx
        {
            *signature = Sig64(sig);
        }

        tx
    }

    #[test]
    fn add_and_get_pending() {
        let mut pool = Mempool::new();
        let tx1 = make_transfer(10, 0);
        let tx2 = make_transfer(50, 1);
        let tx3 = make_transfer(30, 2);

        pool.add_transaction(tx1).unwrap();
        pool.add_transaction(tx2).unwrap();
        pool.add_transaction(tx3).unwrap();

        assert_eq!(pool.size(), 3);

        let pending = pool.get_pending(10);
        assert_eq!(pending.len(), 3);

        // First transaction should have the highest fee (50)
        assert_eq!(pending[0].fee(), 50);
        assert_eq!(pending[1].fee(), 30);
        assert_eq!(pending[2].fee(), 10);
    }

    #[test]
    fn remove_transaction_works() {
        let mut pool = Mempool::new();
        let tx = make_transfer(10, 0);
        let hash = tx.hash();

        pool.add_transaction(tx).unwrap();
        assert_eq!(pool.size(), 1);
        assert!(pool.contains(&hash));

        pool.remove_transaction(&hash);
        assert_eq!(pool.size(), 0);
        assert!(!pool.contains(&hash));
    }

    #[test]
    fn reject_duplicate() {
        let mut pool = Mempool::new();
        let tx = make_transfer(10, 0);

        pool.add_transaction(tx.clone()).unwrap();
        let result = pool.add_transaction(tx);
        assert!(result.is_err());
    }

    #[test]
    fn evict_lowest_when_full() {
        let mut pool = Mempool::with_limits(2, 3600);
        let tx1 = make_transfer(10, 0);
        let tx2 = make_transfer(20, 1);
        let tx3 = make_transfer(30, 2);

        pool.add_transaction(tx1).unwrap();
        pool.add_transaction(tx2).unwrap();
        assert_eq!(pool.size(), 2);

        // tx3 has higher fee than the lowest (10), so lowest gets evicted
        pool.add_transaction(tx3).unwrap();
        assert_eq!(pool.size(), 2);

        let pending = pool.get_pending(10);
        assert_eq!(pending[0].fee(), 30);
        assert_eq!(pending[1].fee(), 20);
    }

    #[test]
    fn reject_when_full_and_fee_too_low() {
        let mut pool = Mempool::with_limits(2, 3600);
        let tx1 = make_transfer(20, 0);
        let tx2 = make_transfer(30, 1);
        let tx3 = make_transfer(5, 2);

        pool.add_transaction(tx1).unwrap();
        pool.add_transaction(tx2).unwrap();

        // tx3 fee (5) is less than the lowest (20), should be rejected
        let result = pool.add_transaction(tx3);
        assert!(result.is_err());
        assert_eq!(pool.size(), 2);
    }

    #[test]
    fn get_pending_with_limit() {
        let mut pool = Mempool::new();
        for i in 0..5 {
            pool.add_transaction(make_transfer(i * 10 + 10, i)).unwrap();
        }

        let pending = pool.get_pending(3);
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].fee(), 50);
        assert_eq!(pending[1].fee(), 40);
        assert_eq!(pending[2].fee(), 30);
    }

    #[test]
    fn clear_empties_mempool() {
        let mut pool = Mempool::new();
        pool.add_transaction(make_transfer(10, 0)).unwrap();
        pool.add_transaction(make_transfer(20, 1)).unwrap();
        pool.clear();
        assert!(pool.is_empty());
    }
}
