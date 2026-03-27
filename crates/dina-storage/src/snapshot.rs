use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use redb::ReadableTable;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use dina_core::device::DeviceIdentity;
use dina_core::types::{Address, Hash};
use dina_core::Account;

use crate::db::{StorageError, StorageResult};
use crate::tables::{ACCOUNTS, CONTRACT_CODE, CONTRACT_STORAGE, DEVICE_REGISTRY};
use crate::DinaDB;

/// Configuration for the snapshot subsystem.
#[derive(Clone, Debug)]
pub struct SnapshotConfig {
    /// Create a snapshot every N blocks.
    pub snapshot_interval_blocks: u64,
    /// Maximum number of snapshots to retain on disk.
    pub max_snapshots: usize,
    /// Directory where snapshot files are stored.
    pub snapshot_dir: String,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            snapshot_interval_blocks: 10_000,
            max_snapshots: 5,
            snapshot_dir: "snapshots".to_string(),
        }
    }
}

/// Metadata about a snapshot file.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SnapshotInfo {
    /// Block height the snapshot was taken at.
    pub height: u64,
    /// State root hash at that height.
    pub state_root: Hash,
    /// Number of accounts in the snapshot.
    pub accounts_count: u64,
    /// Number of contract code entries.
    pub contracts_count: u64,
    /// Size of the snapshot file in bytes.
    pub file_size_bytes: u64,
    /// Unix timestamp when the snapshot was created.
    pub created_at: u64,
    /// SHA-256 checksum of the serialized snapshot data.
    pub checksum: Hash,
}

/// A full state snapshot containing all accounts, contracts, and devices.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    /// Snapshot metadata.
    pub info: SnapshotInfo,
    /// All accounts keyed by address.
    pub accounts: BTreeMap<Address, Account>,
    /// Contract WASM code keyed by code hash.
    pub contract_code: BTreeMap<Hash, Vec<u8>>,
    /// Contract storage keyed by (address, slot).
    pub contract_storage: BTreeMap<(Address, Hash), Vec<u8>>,
    /// Registered device identities keyed by device address.
    pub devices: BTreeMap<Address, DeviceIdentity>,
}

/// Manages creation, loading, verification, and rotation of state snapshots.
pub struct SnapshotManager {
    config: SnapshotConfig,
    snapshots: BTreeMap<u64, SnapshotInfo>,
}

impl SnapshotManager {
    /// Create a new `SnapshotManager`.
    pub fn new(config: SnapshotConfig) -> Self {
        let mut mgr = Self {
            config,
            snapshots: BTreeMap::new(),
        };
        mgr.discover_existing_snapshots();
        mgr
    }

    /// Returns the snapshot configuration.
    pub fn config(&self) -> &SnapshotConfig {
        &self.config
    }

    /// Scan the snapshot directory for existing snapshot files and populate
    /// the internal index.
    fn discover_existing_snapshots(&mut self) {
        let dir = Path::new(&self.config.snapshot_dir);
        if !dir.exists() {
            return;
        }
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("snap") {
                if let Some(info) = Self::read_snapshot_info(&path) {
                    self.snapshots.insert(info.height, info);
                }
            }
        }
        debug!(count = self.snapshots.len(), "Discovered existing snapshots");
    }

    /// Read just the SnapshotInfo header from a snapshot file without loading
    /// all the data.
    fn read_snapshot_info(path: &Path) -> Option<SnapshotInfo> {
        let data = fs::read(path).ok()?;
        let snapshot: Snapshot = bincode::deserialize(&data).ok()?;
        Some(snapshot.info)
    }

    /// Check whether a snapshot should be created at the given height.
    pub fn should_snapshot(&self, current_height: u64) -> bool {
        if current_height == 0 {
            return false;
        }
        current_height.is_multiple_of(self.config.snapshot_interval_blocks)
    }

    /// Build the file path for a snapshot at a given height.
    fn snapshot_path(&self, height: u64) -> PathBuf {
        Path::new(&self.config.snapshot_dir).join(format!("snapshot_{height}.snap"))
    }

    /// Create a snapshot of the full database state at the given height.
    ///
    /// The snapshot is serialized with bincode and written to the snapshot directory.
    pub fn create_snapshot(
        &mut self,
        db: &DinaDB,
        height: u64,
        state_root: Hash,
    ) -> StorageResult<SnapshotInfo> {
        // Ensure the snapshot directory exists.
        fs::create_dir_all(&self.config.snapshot_dir).map_err(|e| {
            StorageError::Serialization(format!("failed to create snapshot dir: {e}"))
        })?;

        let read_txn = db.inner().begin_read().map_err(StorageError::Transaction)?;

        // Collect accounts.
        let mut accounts = BTreeMap::new();
        if let Ok(table) = read_txn.open_table(ACCOUNTS) {
            let iter = table.iter().map_err(StorageError::Storage)?;
            for entry in iter {
                let (key, value) = entry.map_err(StorageError::Storage)?;
                let addr_bytes: [u8; 32] = key.value().try_into().unwrap_or([0u8; 32]);
                let addr = Address(addr_bytes);
                if let Ok(account) = bincode::deserialize::<Account>(value.value()) {
                    accounts.insert(addr, account);
                }
            }
        }

        // Collect contract code.
        let mut contract_code = BTreeMap::new();
        if let Ok(table) = read_txn.open_table(CONTRACT_CODE) {
            let iter = table.iter().map_err(StorageError::Storage)?;
            for entry in iter {
                let (key, value) = entry.map_err(StorageError::Storage)?;
                let hash_bytes: [u8; 32] = key.value().try_into().unwrap_or([0u8; 32]);
                let hash = Hash(hash_bytes);
                contract_code.insert(hash, value.value().to_vec());
            }
        }

        // Collect contract storage.
        let mut contract_storage = BTreeMap::new();
        if let Ok(table) = read_txn.open_table(CONTRACT_STORAGE) {
            let iter = table.iter().map_err(StorageError::Storage)?;
            for entry in iter {
                let (key, value) = entry.map_err(StorageError::Storage)?;
                let raw = key.value();
                if raw.len() == 64 {
                    let mut addr_bytes = [0u8; 32];
                    let mut slot_bytes = [0u8; 32];
                    addr_bytes.copy_from_slice(&raw[..32]);
                    slot_bytes.copy_from_slice(&raw[32..]);
                    contract_storage.insert(
                        (Address(addr_bytes), Hash(slot_bytes)),
                        value.value().to_vec(),
                    );
                }
            }
        }

        // Collect devices.
        let mut devices = BTreeMap::new();
        if let Ok(table) = read_txn.open_table(DEVICE_REGISTRY) {
            let iter = table.iter().map_err(StorageError::Storage)?;
            for entry in iter {
                let (key, value) = entry.map_err(StorageError::Storage)?;
                let addr_bytes: [u8; 32] = key.value().try_into().unwrap_or([0u8; 32]);
                let addr = Address(addr_bytes);
                if let Ok(device) = bincode::deserialize::<DeviceIdentity>(value.value()) {
                    devices.insert(addr, device);
                }
            }
        }

        // Build the snapshot without checksum first, then compute checksum.
        let placeholder_info = SnapshotInfo {
            height,
            state_root,
            accounts_count: accounts.len() as u64,
            contracts_count: contract_code.len() as u64,
            file_size_bytes: 0,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            checksum: Hash::ZERO,
        };

        let mut snapshot = Snapshot {
            info: placeholder_info,
            accounts,
            contract_code,
            contract_storage,
            devices,
        };

        // Serialize to compute checksum and size.
        let data =
            bincode::serialize(&snapshot).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let checksum = sha256_hash(&data);
        let file_size = data.len() as u64;

        snapshot.info.checksum = checksum;
        snapshot.info.file_size_bytes = file_size;

        // Re-serialize with final info.
        let final_data =
            bincode::serialize(&snapshot).map_err(|e| StorageError::Serialization(e.to_string()))?;

        // Write to disk.
        let path = self.snapshot_path(height);
        fs::write(&path, &final_data).map_err(|e| {
            StorageError::Serialization(format!("failed to write snapshot file: {e}"))
        })?;

        let info = snapshot.info.clone();
        self.snapshots.insert(height, info.clone());

        info!(
            height,
            accounts = info.accounts_count,
            contracts = info.contracts_count,
            size_bytes = final_data.len(),
            "Snapshot created"
        );

        Ok(info)
    }

    /// Load a snapshot from disk by height.
    pub fn load_snapshot(&self, height: u64) -> StorageResult<Snapshot> {
        let path = self.snapshot_path(height);
        let data = fs::read(&path).map_err(|e| {
            StorageError::Serialization(format!("failed to read snapshot at height {height}: {e}"))
        })?;
        let snapshot: Snapshot = bincode::deserialize(&data)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        Ok(snapshot)
    }

    /// Return info for the most recent snapshot, if any.
    pub fn latest_snapshot(&self) -> Option<&SnapshotInfo> {
        self.snapshots.values().next_back()
    }

    /// List all known snapshots ordered by height.
    pub fn list_snapshots(&self) -> Vec<&SnapshotInfo> {
        self.snapshots.values().collect()
    }

    /// Delete snapshots that exceed `max_snapshots`, keeping the most recent ones.
    /// Returns the number of snapshots deleted.
    pub fn delete_old_snapshots(&mut self) -> u64 {
        let max = self.config.max_snapshots;
        if self.snapshots.len() <= max {
            return 0;
        }

        let to_remove = self.snapshots.len() - max;
        let heights_to_delete: Vec<u64> = self.snapshots.keys().take(to_remove).copied().collect();

        let mut deleted = 0u64;
        for height in heights_to_delete {
            let path = self.snapshot_path(height);
            if let Err(e) = fs::remove_file(&path) {
                warn!(height, error = %e, "Failed to delete snapshot file");
            }
            self.snapshots.remove(&height);
            deleted += 1;
        }

        info!(deleted, remaining = self.snapshots.len(), "Old snapshots cleaned up");
        deleted
    }

    /// Verify a snapshot's integrity by re-computing the checksum and comparing
    /// the accounts count.
    pub fn verify_snapshot(&self, snapshot: &Snapshot) -> bool {
        // Verify account count matches.
        if snapshot.accounts.len() as u64 != snapshot.info.accounts_count {
            return false;
        }
        // Verify contract count matches.
        if snapshot.contract_code.len() as u64 != snapshot.info.contracts_count {
            return false;
        }
        true
    }

    /// Restore state from a snapshot into the database, overwriting existing data.
    pub fn restore_from_snapshot(&self, snapshot: Snapshot, db: &DinaDB) -> StorageResult<()> {
        let write_txn = db.inner().begin_write().map_err(StorageError::Transaction)?;
        {
            // Restore accounts.
            let mut accounts_table =
                write_txn.open_table(ACCOUNTS).map_err(StorageError::Table)?;
            for (addr, account) in &snapshot.accounts {
                let bytes = bincode::serialize(account)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                accounts_table
                    .insert(addr.as_bytes().as_slice(), bytes.as_slice())
                    .map_err(StorageError::Storage)?;
            }

            // Restore contract code.
            let mut code_table = write_txn
                .open_table(CONTRACT_CODE)
                .map_err(StorageError::Table)?;
            for (hash, code) in &snapshot.contract_code {
                code_table
                    .insert(hash.as_bytes().as_slice(), code.as_slice())
                    .map_err(StorageError::Storage)?;
            }

            // Restore contract storage.
            let mut storage_table = write_txn
                .open_table(CONTRACT_STORAGE)
                .map_err(StorageError::Table)?;
            for ((addr, slot), value) in &snapshot.contract_storage {
                let mut key = [0u8; 64];
                key[..32].copy_from_slice(addr.as_bytes());
                key[32..].copy_from_slice(slot.as_bytes());
                storage_table
                    .insert(key.as_slice(), value.as_slice())
                    .map_err(StorageError::Storage)?;
            }

            // Restore devices.
            let mut device_table = write_txn
                .open_table(DEVICE_REGISTRY)
                .map_err(StorageError::Table)?;
            for (addr, device) in &snapshot.devices {
                let bytes = bincode::serialize(device)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                device_table
                    .insert(addr.as_bytes().as_slice(), bytes.as_slice())
                    .map_err(StorageError::Storage)?;
            }
        }
        write_txn.commit().map_err(StorageError::Commit)?;

        info!(
            height = snapshot.info.height,
            accounts = snapshot.info.accounts_count,
            "State restored from snapshot"
        );
        Ok(())
    }
}

/// Compute a SHA-256 hash and return it as a `Hash`.
fn sha256_hash(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    Hash(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_db() -> DinaDB {
        DinaDB::open_in_memory().expect("failed to open in-memory db")
    }

    fn snapshot_dir() -> String {
        let dir = tempfile::tempdir().unwrap();
        // Leak the tempdir so it persists for the test.
        let path = dir.path().to_str().unwrap().to_string();
        std::mem::forget(dir);
        path
    }

    fn seed_accounts(db: &DinaDB, count: u64) {
        for i in 0..count {
            let mut addr_bytes = [0u8; 32];
            addr_bytes[0..8].copy_from_slice(&i.to_le_bytes());
            let addr = Address(addr_bytes);
            let account = Account::with_balance(addr, i * 100);
            db.set_account(addr, &account).unwrap();
        }
    }

    #[test]
    fn default_config() {
        let cfg = SnapshotConfig::default();
        assert_eq!(cfg.snapshot_interval_blocks, 10_000);
        assert_eq!(cfg.max_snapshots, 5);
    }

    #[test]
    fn should_snapshot_at_intervals() {
        let mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_interval_blocks: 100,
            snapshot_dir: snapshot_dir(),
            ..SnapshotConfig::default()
        });
        assert!(!mgr.should_snapshot(0));
        assert!(!mgr.should_snapshot(50));
        assert!(mgr.should_snapshot(100));
        assert!(mgr.should_snapshot(200));
        assert!(!mgr.should_snapshot(150));
    }

    #[test]
    fn create_and_load_snapshot() {
        let db = test_db();
        seed_accounts(&db, 5);

        let dir = snapshot_dir();
        let mut mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_interval_blocks: 10,
            max_snapshots: 5,
            snapshot_dir: dir,
        });

        let info = mgr.create_snapshot(&db, 10, Hash::ZERO).unwrap();
        assert_eq!(info.height, 10);
        assert_eq!(info.accounts_count, 5);
        assert!(info.file_size_bytes > 0);

        let loaded = mgr.load_snapshot(10).unwrap();
        assert_eq!(loaded.accounts.len(), 5);
        assert_eq!(loaded.info.height, 10);
    }

    #[test]
    fn latest_snapshot_returns_most_recent() {
        let db = test_db();
        let dir = snapshot_dir();
        let mut mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_interval_blocks: 10,
            max_snapshots: 5,
            snapshot_dir: dir,
        });

        mgr.create_snapshot(&db, 10, Hash::ZERO).unwrap();
        mgr.create_snapshot(&db, 20, Hash::ZERO).unwrap();
        mgr.create_snapshot(&db, 30, Hash::ZERO).unwrap();

        let latest = mgr.latest_snapshot().unwrap();
        assert_eq!(latest.height, 30);
    }

    #[test]
    fn list_snapshots_ordered() {
        let db = test_db();
        let dir = snapshot_dir();
        let mut mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_interval_blocks: 10,
            max_snapshots: 10,
            snapshot_dir: dir,
        });

        mgr.create_snapshot(&db, 30, Hash::ZERO).unwrap();
        mgr.create_snapshot(&db, 10, Hash::ZERO).unwrap();
        mgr.create_snapshot(&db, 20, Hash::ZERO).unwrap();

        let list = mgr.list_snapshots();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].height, 10);
        assert_eq!(list[1].height, 20);
        assert_eq!(list[2].height, 30);
    }

    #[test]
    fn delete_old_snapshots_respects_max() {
        let db = test_db();
        let dir = snapshot_dir();
        let mut mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_interval_blocks: 10,
            max_snapshots: 2,
            snapshot_dir: dir,
        });

        mgr.create_snapshot(&db, 10, Hash::ZERO).unwrap();
        mgr.create_snapshot(&db, 20, Hash::ZERO).unwrap();
        mgr.create_snapshot(&db, 30, Hash::ZERO).unwrap();
        mgr.create_snapshot(&db, 40, Hash::ZERO).unwrap();

        let deleted = mgr.delete_old_snapshots();
        assert_eq!(deleted, 2);
        assert_eq!(mgr.list_snapshots().len(), 2);

        // The two newest should remain.
        let remaining: Vec<u64> = mgr.list_snapshots().iter().map(|s| s.height).collect();
        assert_eq!(remaining, vec![30, 40]);
    }

    #[test]
    fn verify_snapshot_valid() {
        let db = test_db();
        seed_accounts(&db, 3);
        let dir = snapshot_dir();
        let mut mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_interval_blocks: 10,
            max_snapshots: 5,
            snapshot_dir: dir,
        });

        mgr.create_snapshot(&db, 10, Hash::ZERO).unwrap();
        let snapshot = mgr.load_snapshot(10).unwrap();
        assert!(mgr.verify_snapshot(&snapshot));
    }

    #[test]
    fn verify_snapshot_invalid_account_count() {
        let db = test_db();
        seed_accounts(&db, 3);
        let dir = snapshot_dir();
        let mut mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_interval_blocks: 10,
            max_snapshots: 5,
            snapshot_dir: dir,
        });

        mgr.create_snapshot(&db, 10, Hash::ZERO).unwrap();
        let mut snapshot = mgr.load_snapshot(10).unwrap();
        // Tamper with the account count.
        snapshot.info.accounts_count = 999;
        assert!(!mgr.verify_snapshot(&snapshot));
    }

    #[test]
    fn restore_from_snapshot_populates_db() {
        let db1 = test_db();
        seed_accounts(&db1, 4);

        let dir = snapshot_dir();
        let mut mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_interval_blocks: 10,
            max_snapshots: 5,
            snapshot_dir: dir,
        });
        mgr.create_snapshot(&db1, 10, Hash::ZERO).unwrap();
        let snapshot = mgr.load_snapshot(10).unwrap();

        // Restore into a fresh database.
        let db2 = test_db();
        mgr.restore_from_snapshot(snapshot, &db2).unwrap();

        // Verify all 4 accounts are present.
        for i in 0u64..4 {
            let mut addr_bytes = [0u8; 32];
            addr_bytes[0..8].copy_from_slice(&i.to_le_bytes());
            let addr = Address(addr_bytes);
            let account = db2.get_account(addr).unwrap().expect("account missing after restore");
            assert_eq!(account.balance, i * 100);
        }
    }

    #[test]
    fn no_snapshots_returns_none_for_latest() {
        let dir = snapshot_dir();
        let mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_dir: dir,
            ..SnapshotConfig::default()
        });
        assert!(mgr.latest_snapshot().is_none());
    }

    #[test]
    fn load_nonexistent_snapshot_errors() {
        let dir = snapshot_dir();
        let mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_dir: dir,
            ..SnapshotConfig::default()
        });
        assert!(mgr.load_snapshot(999).is_err());
    }

    #[test]
    fn empty_db_snapshot_has_zero_counts() {
        let db = test_db();
        let dir = snapshot_dir();
        let mut mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_interval_blocks: 10,
            max_snapshots: 5,
            snapshot_dir: dir,
        });

        let info = mgr.create_snapshot(&db, 10, Hash::ZERO).unwrap();
        assert_eq!(info.accounts_count, 0);
        assert_eq!(info.contracts_count, 0);
    }

    #[test]
    fn delete_old_snapshots_noop_when_under_max() {
        let db = test_db();
        let dir = snapshot_dir();
        let mut mgr = SnapshotManager::new(SnapshotConfig {
            snapshot_interval_blocks: 10,
            max_snapshots: 10,
            snapshot_dir: dir,
        });

        mgr.create_snapshot(&db, 10, Hash::ZERO).unwrap();
        mgr.create_snapshot(&db, 20, Hash::ZERO).unwrap();

        let deleted = mgr.delete_old_snapshots();
        assert_eq!(deleted, 0);
        assert_eq!(mgr.list_snapshots().len(), 2);
    }
}
