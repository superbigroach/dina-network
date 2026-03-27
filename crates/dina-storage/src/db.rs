use std::sync::Arc;

use redb::Database;
use tracing::{debug, error};

use dina_core::types::{Address, Hash};
use dina_core::{Account, Block};

use crate::migration;
use crate::tables::{ACCOUNTS, BLOCKS, BLOCK_HASHES, STATE_METADATA};

/// Storage error type for the database layer.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("redb error: {0}")]
    Redb(#[from] redb::DatabaseError),

    #[error("redb transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),

    #[error("redb table error: {0}")]
    Table(#[from] redb::TableError),

    #[error("redb storage error: {0}")]
    Storage(#[from] redb::StorageError),

    #[error("redb commit error: {0}")]
    Commit(#[from] redb::CommitError),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("migration error: {0}")]
    Migration(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

/// The main database handle for Dina storage, wrapping a redb::Database.
#[derive(Clone)]
pub struct DinaDB {
    db: Arc<Database>,
}

impl DinaDB {
    /// Open (or create) a database at the given file path.
    /// Runs any pending schema migrations after opening.
    pub fn open(path: &str) -> StorageResult<Self> {
        let db = Database::create(path).map_err(StorageError::Redb)?;
        let instance = Self { db: Arc::new(db) };
        migration::migrate(&instance)?;
        debug!("DinaDB opened at {path}");
        Ok(instance)
    }

    /// Open an in-memory database (backed by a temporary file).
    /// Useful for tests.
    pub fn open_in_memory() -> StorageResult<Self> {
        let temp = tempfile::NamedTempFile::new()
            .map_err(|e| StorageError::Migration(format!("failed to create temp file: {e}")))?;
        let path = temp.path().to_owned();
        // Keep the NamedTempFile alive long enough to get the path, then
        // let redb manage the file. We persist it so redb can open it.
        let _keep = temp.into_temp_path();
        let db =
            Database::create(path.to_str().unwrap()).map_err(StorageError::Redb)?;
        let instance = Self { db: Arc::new(db) };
        migration::migrate(&instance)?;
        debug!("DinaDB opened in-memory (temp file)");
        Ok(instance)
    }

    /// Get a reference to the underlying redb::Database.
    pub(crate) fn inner(&self) -> &Database {
        &self.db
    }

    /// Look up an account by its address.
    pub fn get_account(&self, address: Address) -> StorageResult<Option<Account>> {
        let read_txn = self.db.begin_read().map_err(StorageError::Transaction)?;
        let table = match read_txn.open_table(ACCOUNTS) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(StorageError::Table(e)),
        };

        match table.get(address.as_bytes().as_slice()) {
            Ok(Some(value)) => {
                let account: Account = bincode::deserialize(value.value())
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(account))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Storage(e)),
        }
    }

    /// Insert or update an account.
    pub fn set_account(&self, address: Address, account: &Account) -> StorageResult<()> {
        let bytes =
            bincode::serialize(account).map_err(|e| StorageError::Serialization(e.to_string()))?;

        let write_txn = self.db.begin_write().map_err(StorageError::Transaction)?;
        {
            let mut table = write_txn.open_table(ACCOUNTS).map_err(StorageError::Table)?;
            table
                .insert(address.as_bytes().as_slice(), bytes.as_slice())
                .map_err(StorageError::Storage)?;
        }
        write_txn.commit().map_err(StorageError::Commit)?;
        Ok(())
    }

    /// Retrieve a block by its height (block number).
    pub fn get_block(&self, height: u64) -> StorageResult<Option<Block>> {
        let read_txn = self.db.begin_read().map_err(StorageError::Transaction)?;
        let table = match read_txn.open_table(BLOCKS) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(StorageError::Table(e)),
        };

        match table.get(height) {
            Ok(Some(value)) => {
                let block: Block = bincode::deserialize(value.value())
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(block))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Storage(e)),
        }
    }

    /// Store a block and update the block-hash index and latest-height metadata.
    pub fn store_block(&self, block: &Block) -> StorageResult<()> {
        let block_bytes =
            bincode::serialize(block).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let height = block.header.block_number;
        let block_hash = block.hash();

        let write_txn = self.db.begin_write().map_err(StorageError::Transaction)?;
        {
            // Store block by height.
            let mut blocks_table = write_txn.open_table(BLOCKS).map_err(StorageError::Table)?;
            blocks_table
                .insert(height, block_bytes.as_slice())
                .map_err(StorageError::Storage)?;

            // Store block hash -> height index.
            let mut hashes_table = write_txn
                .open_table(BLOCK_HASHES)
                .map_err(StorageError::Table)?;
            hashes_table
                .insert(block_hash.as_bytes().as_slice(), height)
                .map_err(StorageError::Storage)?;

            // Update latest block height in metadata.
            let mut meta_table = write_txn
                .open_table(STATE_METADATA)
                .map_err(StorageError::Table)?;
            let height_bytes = height.to_le_bytes();
            meta_table
                .insert("latest_block_height", height_bytes.as_slice())
                .map_err(StorageError::Storage)?;
        }
        write_txn.commit().map_err(StorageError::Commit)?;

        debug!("Stored block at height {height}");
        Ok(())
    }

    /// Get the height of the latest stored block, or 0 if no blocks exist.
    pub fn get_latest_block_height(&self) -> StorageResult<u64> {
        let read_txn = self.db.begin_read().map_err(StorageError::Transaction)?;
        let table = match read_txn.open_table(STATE_METADATA) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(0),
            Err(e) => return Err(StorageError::Table(e)),
        };

        match table.get("latest_block_height") {
            Ok(Some(value)) => {
                let bytes = value.value();
                if bytes.len() == 8 {
                    Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
                } else {
                    error!("corrupt latest_block_height metadata: expected 8 bytes");
                    Ok(0)
                }
            }
            Ok(None) => Ok(0),
            Err(e) => Err(StorageError::Storage(e)),
        }
    }

    /// Look up a block by its hash using the block-hash index.
    pub fn get_block_by_hash(&self, hash: Hash) -> StorageResult<Option<Block>> {
        let read_txn = self.db.begin_read().map_err(StorageError::Transaction)?;

        // First, look up the height from the hash index.
        let hashes_table = match read_txn.open_table(BLOCK_HASHES) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(StorageError::Table(e)),
        };

        let height = match hashes_table.get(hash.as_bytes().as_slice()) {
            Ok(Some(v)) => v.value(),
            Ok(None) => return Ok(None),
            Err(e) => return Err(StorageError::Storage(e)),
        };

        // Then, load the block by height.
        let blocks_table = match read_txn.open_table(BLOCKS) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(StorageError::Table(e)),
        };

        match blocks_table.get(height) {
            Ok(Some(value)) => {
                let block: Block = bincode::deserialize(value.value())
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(block))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Storage(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::block::BlockHeader;
    use dina_core::types::Hash;

    fn test_db() -> DinaDB {
        DinaDB::open_in_memory().expect("failed to open in-memory db")
    }

    #[test]
    fn account_round_trip() {
        let db = test_db();
        let addr = Address([0xAA; 32]);
        let account = Account::with_balance(addr, 1_000_000);

        db.set_account(addr, &account).unwrap();
        let loaded = db.get_account(addr).unwrap().expect("account not found");

        assert_eq!(loaded.address, addr);
        assert_eq!(loaded.balance, 1_000_000);
        assert_eq!(loaded.nonce, 0);
    }

    #[test]
    fn get_missing_account_returns_none() {
        let db = test_db();
        let addr = Address([0xBB; 32]);
        assert!(db.get_account(addr).unwrap().is_none());
    }

    #[test]
    fn block_store_and_retrieve() {
        let db = test_db();
        let block = Block {
            header: BlockHeader {
                block_number: 1,
                timestamp: 1700000000,
                parent_hash: Hash::ZERO,
                transactions_root: Hash::ZERO,
                state_root: Hash::ZERO,
                proposer: Address::ZERO,
                signature: [0u8; 64],
            },
            transactions: vec![],
        };

        db.store_block(&block).unwrap();

        let loaded = db.get_block(1).unwrap().expect("block not found");
        assert_eq!(loaded.header.block_number, 1);
        assert_eq!(loaded.header.timestamp, 1700000000);
    }

    #[test]
    fn get_block_by_hash_works() {
        let db = test_db();
        let block = Block {
            header: BlockHeader {
                block_number: 5,
                timestamp: 1700000000,
                parent_hash: Hash::ZERO,
                transactions_root: Hash::ZERO,
                state_root: Hash::ZERO,
                proposer: Address::ZERO,
                signature: [0u8; 64],
            },
            transactions: vec![],
        };

        let hash = block.hash();
        db.store_block(&block).unwrap();

        let loaded = db
            .get_block_by_hash(hash)
            .unwrap()
            .expect("block not found by hash");
        assert_eq!(loaded.header.block_number, 5);
    }

    #[test]
    fn latest_block_height_updates() {
        let db = test_db();
        assert_eq!(db.get_latest_block_height().unwrap(), 0);

        let block = Block {
            header: BlockHeader {
                block_number: 42,
                timestamp: 1700000000,
                parent_hash: Hash::ZERO,
                transactions_root: Hash::ZERO,
                state_root: Hash::ZERO,
                proposer: Address::ZERO,
                signature: [0u8; 64],
            },
            transactions: vec![],
        };
        db.store_block(&block).unwrap();

        assert_eq!(db.get_latest_block_height().unwrap(), 42);
    }

    #[test]
    fn open_in_memory_succeeds() {
        let db = DinaDB::open_in_memory();
        assert!(db.is_ok(), "open_in_memory should succeed");
    }

    #[test]
    fn get_nonexistent_block_returns_none() {
        let db = test_db();
        assert!(db.get_block(999).unwrap().is_none());
    }

    #[test]
    fn get_block_by_nonexistent_hash_returns_none() {
        let db = test_db();
        let fake_hash = Hash([0xDE; 32]);
        assert!(db.get_block_by_hash(fake_hash).unwrap().is_none());
    }

    #[test]
    fn account_update_overwrites_previous() {
        let db = test_db();
        let addr = Address([0xCC; 32]);

        let account1 = Account::with_balance(addr, 100);
        db.set_account(addr, &account1).unwrap();

        let account2 = Account::with_balance(addr, 999);
        db.set_account(addr, &account2).unwrap();

        let loaded = db.get_account(addr).unwrap().unwrap();
        assert_eq!(loaded.balance, 999);
    }

    #[test]
    fn store_multiple_blocks_updates_latest_height() {
        let db = test_db();

        for height in [1, 5, 10] {
            let block = Block {
                header: BlockHeader {
                    block_number: height,
                    timestamp: 1700000000 + height,
                    parent_hash: Hash::ZERO,
                    transactions_root: Hash::ZERO,
                    state_root: Hash::ZERO,
                    proposer: Address::ZERO,
                    signature: [0u8; 64],
                },
                transactions: vec![],
            };
            db.store_block(&block).unwrap();
        }

        assert_eq!(db.get_latest_block_height().unwrap(), 10);

        // All blocks should be retrievable
        assert!(db.get_block(1).unwrap().is_some());
        assert!(db.get_block(5).unwrap().is_some());
        assert!(db.get_block(10).unwrap().is_some());
        assert!(db.get_block(2).unwrap().is_none());
    }

    #[test]
    fn block_hash_is_consistent() {
        let db = test_db();
        let block = Block {
            header: BlockHeader {
                block_number: 7,
                timestamp: 1700000000,
                parent_hash: Hash::ZERO,
                transactions_root: Hash::ZERO,
                state_root: Hash::ZERO,
                proposer: Address::ZERO,
                signature: [0u8; 64],
            },
            transactions: vec![],
        };

        let hash = block.hash();
        db.store_block(&block).unwrap();

        // Retrieve by hash and verify contents match
        let loaded = db.get_block_by_hash(hash).unwrap().unwrap();
        assert_eq!(loaded.header.block_number, 7);
        assert_eq!(loaded.hash(), hash);
    }
}
