use std::collections::{BTreeMap, VecDeque};

use dina_core::block::Block;
use dina_core::device::DeviceIdentity;
use dina_core::transaction::Transaction;
use dina_core::types::{Address, Hash};

/// Statistics tracked per account by the indexer.
#[derive(Debug, Clone)]
pub struct AccountStats {
    pub first_seen: u64,
    pub last_active: u64,
    pub tx_count: u64,
}

/// In-memory transaction indexer that builds searchable indexes from committed blocks.
///
/// Maintains mappings from addresses and block heights to transaction hashes,
/// tracks per-account statistics, validator proposal counts, device registrations,
/// and rolling TPS windows.
pub struct ExplorerIndexer {
    /// address -> list of tx hashes involving that address
    tx_by_address: BTreeMap<Address, Vec<Hash>>,
    /// block height -> list of tx hashes in that block
    tx_by_block: BTreeMap<u64, Vec<Hash>>,
    /// address -> block height when first seen
    account_first_seen: BTreeMap<Address, u64>,
    /// address -> block height when last active
    account_last_active: BTreeMap<Address, u64>,
    /// address -> total transaction count
    account_tx_count: BTreeMap<Address, u64>,
    /// Total number of indexed transactions
    pub total_transactions: u64,
    /// Total number of indexed blocks
    pub total_blocks: u64,
    /// Validator address -> number of blocks proposed
    validator_blocks: BTreeMap<Address, u64>,
    /// Validator address -> block height of last proposal
    validator_last_proposed: BTreeMap<Address, u64>,
    /// Registered devices
    devices: BTreeMap<Address, DeviceIdentity>,
    /// Block timestamps for computing average block time (recent window)
    block_timestamps: VecDeque<u64>,
    /// Transaction timestamps for TPS calculation (unix seconds -> count)
    tx_counts_by_second: BTreeMap<u64, u64>,
    /// Cached transaction details: tx_hash -> (block_height, Transaction)
    tx_cache: BTreeMap<Hash, (u64, Transaction)>,
}

impl ExplorerIndexer {
    /// Create a new empty indexer.
    pub fn new() -> Self {
        Self {
            tx_by_address: BTreeMap::new(),
            tx_by_block: BTreeMap::new(),
            account_first_seen: BTreeMap::new(),
            account_last_active: BTreeMap::new(),
            account_tx_count: BTreeMap::new(),
            total_transactions: 0,
            total_blocks: 0,
            validator_blocks: BTreeMap::new(),
            validator_last_proposed: BTreeMap::new(),
            devices: BTreeMap::new(),
            block_timestamps: VecDeque::new(),
            tx_counts_by_second: BTreeMap::new(),
            tx_cache: BTreeMap::new(),
        }
    }

    /// Index a committed block, extracting all transactions and updating indexes.
    pub fn index_block(&mut self, block: &Block) {
        let height = block.header.block_number;
        let timestamp = block.header.timestamp;
        let proposer = block.header.proposer;

        self.total_blocks += 1;

        // Track block timestamp for avg block time
        self.block_timestamps.push_back(timestamp);
        // Keep only the last 1000 timestamps
        if self.block_timestamps.len() > 1000 {
            self.block_timestamps.pop_front();
        }

        // Track validator stats
        *self.validator_blocks.entry(proposer).or_insert(0) += 1;
        self.validator_last_proposed.insert(proposer, height);

        // Touch proposer as seen
        self.touch_account(proposer, height);

        let mut block_tx_hashes = Vec::with_capacity(block.transactions.len());

        for tx in &block.transactions {
            let tx_hash = tx.hash();
            block_tx_hashes.push(tx_hash);
            self.total_transactions += 1;

            // Track TPS
            *self.tx_counts_by_second.entry(timestamp).or_insert(0) += 1;

            // Cache the transaction
            self.tx_cache.insert(tx_hash, (height, tx.clone()));

            // Index addresses involved in this transaction
            let addresses = Self::extract_addresses(tx);
            for addr in &addresses {
                self.tx_by_address
                    .entry(*addr)
                    .or_default()
                    .push(tx_hash);
                self.touch_account(*addr, height);
            }

            // Track device registrations
            if let Transaction::RegisterDevice {
                device_pubkey,
                owner,
                attestation,
                ..
            } = tx
            {
                let device = DeviceIdentity::new(
                    *device_pubkey,
                    *owner,
                    dina_core::device::DeviceType::Custom("Unknown".to_string()),
                    attestation.firmware_hash,
                    attestation.witness_root,
                    timestamp,
                );
                self.devices.insert(device.id, device);
            }
        }

        self.tx_by_block.insert(height, block_tx_hashes);

        // Prune old TPS data (keep last 2 hours)
        let cutoff = timestamp.saturating_sub(7200);
        self.tx_counts_by_second = self.tx_counts_by_second.split_off(&cutoff);
    }

    /// Get all transaction hashes involving a given address.
    pub fn transactions_for_address(&self, addr: &Address) -> Vec<Hash> {
        self.tx_by_address.get(addr).cloned().unwrap_or_default()
    }

    /// Get all transaction hashes in a given block.
    pub fn transactions_in_block(&self, height: u64) -> Vec<Hash> {
        self.tx_by_block.get(&height).cloned().unwrap_or_default()
    }

    /// Get account statistics for a given address.
    pub fn account_info(&self, addr: &Address) -> Option<AccountStats> {
        let first_seen = self.account_first_seen.get(addr)?;
        let last_active = self.account_last_active.get(addr)?;
        let tx_count = self.account_tx_count.get(addr).copied().unwrap_or(0);

        Some(AccountStats {
            first_seen: *first_seen,
            last_active: *last_active,
            tx_count,
        })
    }

    /// Get a cached transaction by its hash.
    pub fn get_transaction(&self, hash: &Hash) -> Option<&(u64, Transaction)> {
        self.tx_cache.get(hash)
    }

    /// Return the number of unique accounts seen.
    pub fn total_accounts(&self) -> u64 {
        self.account_first_seen.len() as u64
    }

    /// Return the total number of registered devices.
    pub fn total_devices(&self) -> u64 {
        self.devices.len() as u64
    }

    /// Iterate over all registered devices.
    pub fn devices(&self) -> impl Iterator<Item = (&Address, &DeviceIdentity)> {
        self.devices.iter()
    }

    /// Get a device by its ID.
    pub fn get_device(&self, id: &Address) -> Option<&DeviceIdentity> {
        self.devices.get(id)
    }

    /// Return all validator info.
    pub fn validators(&self) -> Vec<(Address, u64, Option<u64>)> {
        self.validator_blocks
            .iter()
            .map(|(addr, count)| {
                let last = self.validator_last_proposed.get(addr).copied();
                (*addr, *count, last)
            })
            .collect()
    }

    /// Compute the average block time in milliseconds from the recent timestamp window.
    pub fn avg_block_time_ms(&self) -> f64 {
        if self.block_timestamps.len() < 2 {
            return 0.0;
        }
        let first = *self.block_timestamps.front().unwrap();
        let last = *self.block_timestamps.back().unwrap();
        let duration_secs = last.saturating_sub(first) as f64;
        let intervals = (self.block_timestamps.len() - 1) as f64;
        if intervals == 0.0 {
            return 0.0;
        }
        (duration_secs / intervals) * 1000.0
    }

    /// Compute transactions per second over the last `window_secs` seconds.
    pub fn tps(&self, current_timestamp: u64, window_secs: u64) -> f64 {
        if window_secs == 0 {
            return 0.0;
        }
        let cutoff = current_timestamp.saturating_sub(window_secs);
        let total: u64 = self
            .tx_counts_by_second
            .range(cutoff..=current_timestamp)
            .map(|(_, count)| count)
            .sum();
        total as f64 / window_secs as f64
    }

    /// Touch an account: update first_seen, last_active, and tx_count.
    fn touch_account(&mut self, addr: Address, height: u64) {
        self.account_first_seen.entry(addr).or_insert(height);
        self.account_last_active
            .entry(addr)
            .and_modify(|h| {
                if height > *h {
                    *h = height;
                }
            })
            .or_insert(height);
        *self.account_tx_count.entry(addr).or_insert(0) += 1;
    }

    /// Extract all addresses involved in a transaction.
    fn extract_addresses(tx: &Transaction) -> Vec<Address> {
        match tx {
            Transaction::Transfer { from, to, .. } => vec![*from, *to],
            Transaction::DeployContract { from, .. } => vec![*from],
            Transaction::CallContract { from, contract, .. } => vec![*from, *contract],
            Transaction::RegisterDevice { owner, .. } => vec![*owner],
        }
    }
}

impl Default for ExplorerIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::block::BlockHeader;
    use dina_core::transaction::Sig64;
    use dina_core::types::Hash;

    fn make_block(height: u64, timestamp: u64, txs: Vec<Transaction>) -> Block {
        Block {
            header: BlockHeader {
                block_number: height,
                parent_hash: Hash::ZERO,
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp,
                proposer: Address([0x01; 32]),
                signature: [0u8; 64],
            },
            transactions: txs,
        }
    }

    fn make_transfer(from_byte: u8, to_byte: u8, amount: u64) -> Transaction {
        Transaction::Transfer {
            from: Address([from_byte; 32]),
            to: Address([to_byte; 32]),
            amount,
            memo: None,
            device_witness: None,
            nonce: 0,
            fee: 10,
            signature: Sig64([0u8; 64]),
        }
    }

    #[test]
    fn index_empty_block() {
        let mut indexer = ExplorerIndexer::new();
        let block = make_block(0, 1000, vec![]);
        indexer.index_block(&block);
        assert_eq!(indexer.total_blocks, 1);
        assert_eq!(indexer.total_transactions, 0);
    }

    #[test]
    fn index_block_with_transfers() {
        let mut indexer = ExplorerIndexer::new();
        let tx1 = make_transfer(0xAA, 0xBB, 100);
        let tx2 = make_transfer(0xAA, 0xCC, 200);
        let block = make_block(1, 1000, vec![tx1, tx2]);
        indexer.index_block(&block);

        assert_eq!(indexer.total_transactions, 2);
        let aa_txs = indexer.transactions_for_address(&Address([0xAA; 32]));
        assert_eq!(aa_txs.len(), 2);
        let bb_txs = indexer.transactions_for_address(&Address([0xBB; 32]));
        assert_eq!(bb_txs.len(), 1);
    }

    #[test]
    fn account_info_tracks_stats() {
        let mut indexer = ExplorerIndexer::new();
        let tx = make_transfer(0xAA, 0xBB, 100);
        let block = make_block(5, 1000, vec![tx]);
        indexer.index_block(&block);

        let info = indexer.account_info(&Address([0xAA; 32])).unwrap();
        assert_eq!(info.first_seen, 5);
        assert_eq!(info.last_active, 5);
        assert_eq!(info.tx_count, 1);
    }

    #[test]
    fn validators_tracked() {
        let mut indexer = ExplorerIndexer::new();
        let block = make_block(1, 1000, vec![]);
        indexer.index_block(&block);

        let validators = indexer.validators();
        assert_eq!(validators.len(), 1);
        assert_eq!(validators[0].1, 1); // 1 block proposed
    }
}
