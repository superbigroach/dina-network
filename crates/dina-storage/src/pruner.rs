use std::time::Instant;

use redb::ReadableTable;
use tracing::{debug, info};

use crate::db::{StorageError, StorageResult};
use crate::tables::{BLOCKS, BLOCK_HASHES};
use crate::DinaDB;

/// Configuration for state pruning behavior.
#[derive(Clone, Debug)]
pub struct PruneConfig {
    /// Number of recent blocks to always keep (default: 10_000, ~17 minutes).
    pub keep_recent_blocks: u64,
    /// Number of recent state snapshots to keep.
    pub keep_recent_states: u64,
    /// How often (in blocks) to evaluate pruning.
    pub prune_interval_blocks: u64,
    /// If true, never prune anything (archive node mode).
    pub archive_mode: bool,
}

impl Default for PruneConfig {
    fn default() -> Self {
        Self {
            keep_recent_blocks: 10_000,
            keep_recent_states: 1_000,
            prune_interval_blocks: 100,
            archive_mode: false,
        }
    }
}

/// Outcome of a prune operation.
#[derive(Clone, Debug)]
pub struct PruneResult {
    /// Number of blocks that were pruned.
    pub blocks_pruned: u64,
    /// Estimated bytes freed from the database.
    pub bytes_freed: u64,
    /// Duration of the prune operation in milliseconds.
    pub duration_ms: u64,
}

/// Estimated savings from a hypothetical prune at the current height.
#[derive(Clone, Debug)]
pub struct PruneSavingsEstimate {
    /// Number of blocks eligible for pruning.
    pub pruneable_blocks: u64,
    /// Estimated bytes that would be freed.
    pub estimated_bytes: u64,
}

/// Manages periodic pruning of old blocks and state from the database.
pub struct StatePruner {
    config: PruneConfig,
    last_pruned_height: u64,
}

impl StatePruner {
    /// Create a new `StatePruner` with the given configuration.
    pub fn new(config: PruneConfig) -> Self {
        Self {
            config,
            last_pruned_height: 0,
        }
    }

    /// Returns the current prune configuration.
    pub fn config(&self) -> &PruneConfig {
        &self.config
    }

    /// Returns the height at which pruning was last performed.
    pub fn last_pruned_height(&self) -> u64 {
        self.last_pruned_height
    }

    /// Check whether pruning should run at the given chain height.
    pub fn should_prune(&self, current_height: u64) -> bool {
        if self.config.archive_mode {
            return false;
        }
        if current_height <= self.config.keep_recent_blocks {
            return false;
        }
        if self.last_pruned_height == 0 {
            return true;
        }
        current_height >= self.last_pruned_height + self.config.prune_interval_blocks
    }

    /// Calculate the lowest block height that should be pruned.
    /// Returns `None` if nothing is pruneable.
    fn prune_cutoff(&self, current_height: u64) -> Option<u64> {
        if self.config.archive_mode || current_height <= self.config.keep_recent_blocks {
            return None;
        }
        Some(current_height.saturating_sub(self.config.keep_recent_blocks))
    }

    /// Prune old blocks and their hash index entries from the database.
    ///
    /// Blocks below `current_height - keep_recent_blocks` are removed.
    /// In archive mode this is a no-op.
    pub fn prune(&mut self, db: &DinaDB, current_height: u64) -> StorageResult<PruneResult> {
        let start = Instant::now();

        if self.config.archive_mode {
            info!("Archive mode enabled -- skipping prune");
            return Ok(PruneResult {
                blocks_pruned: 0,
                bytes_freed: 0,
                duration_ms: 0,
            });
        }

        let cutoff = match self.prune_cutoff(current_height) {
            Some(c) => c,
            None => {
                return Ok(PruneResult {
                    blocks_pruned: 0,
                    bytes_freed: 0,
                    duration_ms: 0,
                });
            }
        };

        let mut blocks_pruned: u64 = 0;
        let mut bytes_freed: u64 = 0;

        let write_txn = db.inner().begin_write().map_err(StorageError::Transaction)?;
        {
            let mut blocks_table = write_txn.open_table(BLOCKS).map_err(StorageError::Table)?;
            let mut hashes_table = write_txn
                .open_table(BLOCK_HASHES)
                .map_err(StorageError::Table)?;

            // Collect heights to prune by scanning the range [0, cutoff).
            let heights_to_prune: Vec<(u64, u64)> = {
                let range = blocks_table
                    .range(0..cutoff)
                    .map_err(StorageError::Storage)?;
                let mut heights = Vec::new();
                for entry in range {
                    let (key, value) = entry.map_err(StorageError::Storage)?;
                    let size = value.value().len() as u64;
                    heights.push((key.value(), size));
                }
                heights
            };

            for (height, size) in &heights_to_prune {
                // Before removing the block, deserialize it to get the hash for index cleanup.
                if let Some(block_bytes) = blocks_table
                    .remove(*height)
                    .map_err(StorageError::Storage)?
                {
                    bytes_freed += *size;
                    blocks_pruned += 1;

                    // Try to remove the corresponding block-hash entry.
                    // We deserialize the block just to get its hash.
                    if let Ok(block) =
                        bincode::deserialize::<dina_core::Block>(block_bytes.value())
                    {
                        let hash = block.hash();
                        let _ = hashes_table.remove(hash.as_bytes().as_slice());
                    }
                }
            }
        }
        write_txn.commit().map_err(StorageError::Commit)?;

        let duration_ms = start.elapsed().as_millis() as u64;
        self.last_pruned_height = current_height;

        info!(
            blocks_pruned,
            bytes_freed, duration_ms, cutoff, "Prune complete"
        );

        Ok(PruneResult {
            blocks_pruned,
            bytes_freed,
            duration_ms,
        })
    }

    /// Estimate how much data could be freed by pruning at the current height
    /// without actually deleting anything.
    pub fn estimate_savings(
        &self,
        db: &DinaDB,
        current_height: u64,
    ) -> StorageResult<PruneSavingsEstimate> {
        if self.config.archive_mode || current_height <= self.config.keep_recent_blocks {
            return Ok(PruneSavingsEstimate {
                pruneable_blocks: 0,
                estimated_bytes: 0,
            });
        }

        let cutoff = current_height.saturating_sub(self.config.keep_recent_blocks);
        let mut pruneable_blocks: u64 = 0;
        let mut estimated_bytes: u64 = 0;

        let read_txn = db.inner().begin_read().map_err(StorageError::Transaction)?;
        let table = match read_txn.open_table(BLOCKS) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => {
                return Ok(PruneSavingsEstimate {
                    pruneable_blocks: 0,
                    estimated_bytes: 0,
                });
            }
            Err(e) => return Err(StorageError::Table(e)),
        };

        let range = table.range(0..cutoff).map_err(StorageError::Storage)?;
        for entry in range {
            let (_key, value) = entry.map_err(StorageError::Storage)?;
            pruneable_blocks += 1;
            estimated_bytes += value.value().len() as u64;
        }

        debug!(pruneable_blocks, estimated_bytes, "Prune savings estimate");
        Ok(PruneSavingsEstimate {
            pruneable_blocks,
            estimated_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::block::BlockHeader;
    use dina_core::types::{Address, Hash};
    use dina_core::Block;

    fn test_db() -> DinaDB {
        DinaDB::open_in_memory().expect("failed to open in-memory db")
    }

    fn make_block(height: u64) -> Block {
        Block {
            header: BlockHeader {
                block_number: height,
                timestamp: 1_700_000_000 + height,
                parent_hash: Hash::ZERO,
                transactions_root: Hash::ZERO,
                state_root: Hash::ZERO,
                proposer: Address::ZERO,
                signature: [0u8; 64],
            },
            transactions: vec![],
        }
    }

    fn store_blocks(db: &DinaDB, range: std::ops::Range<u64>) {
        for h in range {
            db.store_block(&make_block(h)).unwrap();
        }
    }

    #[test]
    fn default_config_values() {
        let cfg = PruneConfig::default();
        assert_eq!(cfg.keep_recent_blocks, 10_000);
        assert_eq!(cfg.keep_recent_states, 1_000);
        assert_eq!(cfg.prune_interval_blocks, 100);
        assert!(!cfg.archive_mode);
    }

    #[test]
    fn archive_mode_never_prunes() {
        let pruner = StatePruner::new(PruneConfig {
            archive_mode: true,
            ..PruneConfig::default()
        });
        assert!(!pruner.should_prune(999_999));
    }

    #[test]
    fn should_not_prune_below_retention_window() {
        let pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 100,
            ..PruneConfig::default()
        });
        assert!(!pruner.should_prune(50));
        assert!(!pruner.should_prune(100));
    }

    #[test]
    fn should_prune_above_retention_window() {
        let pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 100,
            ..PruneConfig::default()
        });
        assert!(pruner.should_prune(101));
        assert!(pruner.should_prune(500));
    }

    #[test]
    fn should_prune_respects_interval() {
        let mut pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 10,
            prune_interval_blocks: 50,
            ..PruneConfig::default()
        });
        // First prune should trigger immediately.
        assert!(pruner.should_prune(100));
        pruner.last_pruned_height = 100;

        // 30 blocks later: too soon.
        assert!(!pruner.should_prune(130));
        // Exactly 50 blocks later: should trigger.
        assert!(pruner.should_prune(150));
    }

    #[test]
    fn prune_removes_old_blocks() {
        let db = test_db();
        store_blocks(&db, 0..20);

        let mut pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 10,
            prune_interval_blocks: 1,
            archive_mode: false,
            ..PruneConfig::default()
        });

        let result = pruner.prune(&db, 20).unwrap();
        assert_eq!(result.blocks_pruned, 10); // blocks 0..10
        assert!(result.bytes_freed > 0);

        // Blocks 0..10 should be gone.
        for h in 0..10 {
            assert!(db.get_block(h).unwrap().is_none(), "block {h} should be pruned");
        }
        // Blocks 10..20 should remain.
        for h in 10..20 {
            assert!(db.get_block(h).unwrap().is_some(), "block {h} should exist");
        }
    }

    #[test]
    fn prune_in_archive_mode_is_noop() {
        let db = test_db();
        store_blocks(&db, 0..20);

        let mut pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 5,
            archive_mode: true,
            ..PruneConfig::default()
        });

        let result = pruner.prune(&db, 20).unwrap();
        assert_eq!(result.blocks_pruned, 0);

        // All blocks should still exist.
        for h in 0..20 {
            assert!(db.get_block(h).unwrap().is_some());
        }
    }

    #[test]
    fn prune_updates_last_pruned_height() {
        let db = test_db();
        store_blocks(&db, 0..30);

        let mut pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 10,
            prune_interval_blocks: 1,
            archive_mode: false,
            ..PruneConfig::default()
        });

        assert_eq!(pruner.last_pruned_height(), 0);
        pruner.prune(&db, 25).unwrap();
        assert_eq!(pruner.last_pruned_height(), 25);
    }

    #[test]
    fn estimate_savings_correct() {
        let db = test_db();
        store_blocks(&db, 0..20);

        let pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 10,
            archive_mode: false,
            ..PruneConfig::default()
        });

        let estimate = pruner.estimate_savings(&db, 20).unwrap();
        assert_eq!(estimate.pruneable_blocks, 10);
        assert!(estimate.estimated_bytes > 0);
    }

    #[test]
    fn estimate_savings_archive_mode_returns_zero() {
        let db = test_db();
        store_blocks(&db, 0..20);

        let pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 5,
            archive_mode: true,
            ..PruneConfig::default()
        });

        let estimate = pruner.estimate_savings(&db, 20).unwrap();
        assert_eq!(estimate.pruneable_blocks, 0);
        assert_eq!(estimate.estimated_bytes, 0);
    }

    #[test]
    fn estimate_savings_empty_db() {
        let db = test_db();

        let pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 10,
            archive_mode: false,
            ..PruneConfig::default()
        });

        let estimate = pruner.estimate_savings(&db, 100).unwrap();
        assert_eq!(estimate.pruneable_blocks, 0);
        assert_eq!(estimate.estimated_bytes, 0);
    }

    #[test]
    fn prune_removes_block_hash_index() {
        let db = test_db();
        let block = make_block(5);
        let hash = block.hash();
        db.store_block(&block).unwrap();
        store_blocks(&db, 100..110);

        let mut pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 5,
            prune_interval_blocks: 1,
            archive_mode: false,
            ..PruneConfig::default()
        });

        pruner.prune(&db, 110).unwrap();

        // Block 5 should be gone.
        assert!(db.get_block(5).unwrap().is_none());
        // Hash index should also be cleaned up.
        assert!(db.get_block_by_hash(hash).unwrap().is_none());
    }

    #[test]
    fn successive_prunes_are_incremental() {
        let db = test_db();
        store_blocks(&db, 0..50);

        let mut pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 10,
            prune_interval_blocks: 1,
            archive_mode: false,
            ..PruneConfig::default()
        });

        // First prune at height 30: removes blocks 0..20.
        let r1 = pruner.prune(&db, 30).unwrap();
        assert_eq!(r1.blocks_pruned, 20);

        // Second prune at height 40: removes blocks 20..30.
        let r2 = pruner.prune(&db, 40).unwrap();
        assert_eq!(r2.blocks_pruned, 10);

        // Blocks 30..50 should still exist.
        for h in 30..50 {
            assert!(db.get_block(h).unwrap().is_some());
        }
    }

    #[test]
    fn prune_nothing_when_all_blocks_are_recent() {
        let db = test_db();
        store_blocks(&db, 0..5);

        let mut pruner = StatePruner::new(PruneConfig {
            keep_recent_blocks: 100,
            prune_interval_blocks: 1,
            archive_mode: false,
            ..PruneConfig::default()
        });

        let result = pruner.prune(&db, 5).unwrap();
        assert_eq!(result.blocks_pruned, 0);
    }
}
