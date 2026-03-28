use redb::{ReadableTable, WriteTransaction};
use tracing::debug;

use dina_core::types::Address;
use dina_core::Account;

use crate::db::{StorageError, StorageResult};
use crate::tables::{ACCOUNTS, CONTRACT_CODE, CONTRACT_STORAGE};
use crate::DinaDB;

/// Provides transactional access to blockchain state.
/// All reads and writes within a `StateTransaction` are atomic:
/// either all changes apply on `commit()`, or none do if the
/// transaction is dropped.
pub struct StateStore {
    db: DinaDB,
}

impl StateStore {
    /// Create a new `StateStore` backed by the given database.
    pub fn new(db: DinaDB) -> Self {
        Self { db }
    }

    /// Begin a new write transaction for atomic state updates.
    pub fn begin_transaction(&self) -> StorageResult<StateTransaction> {
        let txn = self
            .db
            .inner()
            .begin_write()
            .map_err(StorageError::Transaction)?;
        Ok(StateTransaction { txn })
    }
}

/// A write transaction over blockchain state.
///
/// Wrap multiple reads and writes, then call `commit()` to persist
/// atomically. If this struct is dropped without calling `commit()`,
/// all changes are rolled back.
pub struct StateTransaction {
    txn: WriteTransaction,
}

impl StateTransaction {
    /// Look up an account within this transaction.
    pub fn get_account(&self, address: &Address) -> StorageResult<Option<Account>> {
        let table = match self.txn.open_table(ACCOUNTS) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(StorageError::Table(e)),
        };

        let result = match table.get(address.as_bytes().as_slice()) {
            Ok(Some(value)) => {
                let account: Account = bincode::deserialize(value.value())
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(account))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Storage(e)),
        };
        result
    }

    /// Insert or update an account within this transaction.
    pub fn set_account(&self, address: &Address, account: &Account) -> StorageResult<()> {
        let bytes =
            bincode::serialize(account).map_err(|e| StorageError::Serialization(e.to_string()))?;

        let mut table = self.txn.open_table(ACCOUNTS).map_err(StorageError::Table)?;
        table
            .insert(address.as_bytes().as_slice(), bytes.as_slice())
            .map_err(StorageError::Storage)?;
        Ok(())
    }

    /// Retrieve WASM contract code by its code hash.
    pub fn get_contract_code(&self, code_hash: &[u8; 32]) -> StorageResult<Option<Vec<u8>>> {
        let table = match self.txn.open_table(CONTRACT_CODE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(StorageError::Table(e)),
        };

        let result = match table.get(code_hash.as_slice()) {
            Ok(Some(value)) => Ok(Some(value.value().to_vec())),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Storage(e)),
        };
        result
    }

    /// Store WASM contract code keyed by its code hash.
    pub fn set_contract_code(&self, code_hash: &[u8; 32], code: &[u8]) -> StorageResult<()> {
        let mut table = self
            .txn
            .open_table(CONTRACT_CODE)
            .map_err(StorageError::Table)?;
        table
            .insert(code_hash.as_slice(), code)
            .map_err(StorageError::Storage)?;
        Ok(())
    }

    /// Read a value from a contract's storage.
    /// The key is a composite of the contract address and the storage slot.
    pub fn get_contract_storage(
        &self,
        address: &Address,
        slot: &[u8; 32],
    ) -> StorageResult<Option<Vec<u8>>> {
        let composite_key = compose_storage_key(address, slot);

        let table = match self.txn.open_table(CONTRACT_STORAGE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(StorageError::Table(e)),
        };

        let result = match table.get(composite_key.as_slice()) {
            Ok(Some(value)) => Ok(Some(value.value().to_vec())),
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Storage(e)),
        };
        result
    }

    /// Write a value to a contract's storage.
    pub fn set_contract_storage(
        &self,
        address: &Address,
        slot: &[u8; 32],
        value: &[u8],
    ) -> StorageResult<()> {
        let composite_key = compose_storage_key(address, slot);

        let mut table = self
            .txn
            .open_table(CONTRACT_STORAGE)
            .map_err(StorageError::Table)?;
        table
            .insert(composite_key.as_slice(), value)
            .map_err(StorageError::Storage)?;
        Ok(())
    }

    /// Commit all pending changes atomically.
    /// If this returns an error, no changes are persisted.
    pub fn commit(self) -> StorageResult<()> {
        self.txn.commit().map_err(StorageError::Commit)?;
        debug!("State transaction committed");
        Ok(())
    }
}

/// Build a 64-byte composite key from a 32-byte address and a 32-byte storage slot.
fn compose_storage_key(address: &Address, slot: &[u8; 32]) -> [u8; 64] {
    let mut key = [0u8; 64];
    key[..32].copy_from_slice(address.as_bytes());
    key[32..].copy_from_slice(slot);
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::account::AccountState;

    fn test_store() -> StateStore {
        let db = DinaDB::open_in_memory().expect("failed to open in-memory db");
        StateStore::new(db)
    }

    #[test]
    fn atomic_account_update() {
        let store = test_store();
        let addr = Address([0x11; 32]);
        let account = Account::with_balance(addr, 500);

        let txn = store.begin_transaction().unwrap();
        txn.set_account(&addr, &account).unwrap();
        txn.commit().unwrap();

        // Verify via a new transaction.
        let txn2 = store.begin_transaction().unwrap();
        let loaded = txn2.get_account(&addr).unwrap().expect("account missing");
        assert_eq!(loaded.balance, 500);
    }

    #[test]
    fn dropped_transaction_does_not_persist() {
        let store = test_store();
        let addr = Address([0x22; 32]);
        let account = Account::with_balance(addr, 999);

        {
            let txn = store.begin_transaction().unwrap();
            txn.set_account(&addr, &account).unwrap();
            // Drop without commit.
        }

        let txn2 = store.begin_transaction().unwrap();
        assert!(txn2.get_account(&addr).unwrap().is_none());
    }

    #[test]
    fn contract_code_round_trip() {
        let store = test_store();
        let code_hash = [0xCC; 32];
        let wasm_code = b"\x00asm\x01\x00\x00\x00fake_wasm_module";

        let txn = store.begin_transaction().unwrap();
        txn.set_contract_code(&code_hash, wasm_code).unwrap();
        txn.commit().unwrap();

        let txn2 = store.begin_transaction().unwrap();
        let loaded = txn2
            .get_contract_code(&code_hash)
            .unwrap()
            .expect("code missing");
        assert_eq!(loaded, wasm_code);
    }

    #[test]
    fn contract_storage_round_trip() {
        let store = test_store();
        let addr = Address([0x33; 32]);
        let slot = [0x01; 32];
        let value = b"hello_storage";

        let txn = store.begin_transaction().unwrap();
        txn.set_contract_storage(&addr, &slot, value).unwrap();
        txn.commit().unwrap();

        let txn2 = store.begin_transaction().unwrap();
        let loaded = txn2
            .get_contract_storage(&addr, &slot)
            .unwrap()
            .expect("storage value missing");
        assert_eq!(loaded, value);
    }

    #[test]
    fn multiple_operations_in_one_transaction_are_atomic() {
        let store = test_store();
        let addr1 = Address([0x41; 32]);
        let addr2 = Address([0x42; 32]);
        let code_hash = [0xDE; 32];
        let contract_addr = Address([0x43; 32]);
        let slot = [0x01; 32];

        // Perform multiple operations in a single transaction
        let txn = store.begin_transaction().unwrap();
        txn.set_account(&addr1, &Account::with_balance(addr1, 100))
            .unwrap();
        txn.set_account(&addr2, &Account::with_balance(addr2, 200))
            .unwrap();
        txn.set_contract_code(&code_hash, b"wasm_bytecode").unwrap();
        txn.set_contract_storage(&contract_addr, &slot, b"slot_value")
            .unwrap();
        txn.commit().unwrap();

        // All four writes should be visible
        let txn2 = store.begin_transaction().unwrap();
        assert_eq!(txn2.get_account(&addr1).unwrap().unwrap().balance, 100);
        assert_eq!(txn2.get_account(&addr2).unwrap().unwrap().balance, 200);
        assert_eq!(
            txn2.get_contract_code(&code_hash).unwrap().unwrap(),
            b"wasm_bytecode"
        );
        assert_eq!(
            txn2.get_contract_storage(&contract_addr, &slot)
                .unwrap()
                .unwrap(),
            b"slot_value"
        );
    }

    #[test]
    fn dropped_multi_op_transaction_rolls_back_all() {
        let store = test_store();
        let addr1 = Address([0x51; 32]);
        let addr2 = Address([0x52; 32]);

        {
            let txn = store.begin_transaction().unwrap();
            txn.set_account(&addr1, &Account::with_balance(addr1, 100))
                .unwrap();
            txn.set_account(&addr2, &Account::with_balance(addr2, 200))
                .unwrap();
            // Drop without commit
        }

        // Neither account should exist
        let txn2 = store.begin_transaction().unwrap();
        assert!(txn2.get_account(&addr1).unwrap().is_none());
        assert!(txn2.get_account(&addr2).unwrap().is_none());
    }

    #[test]
    fn get_nonexistent_contract_code_returns_none() {
        let store = test_store();
        let txn = store.begin_transaction().unwrap();
        assert!(txn.get_contract_code(&[0xFF; 32]).unwrap().is_none());
    }

    #[test]
    fn get_nonexistent_contract_storage_returns_none() {
        let store = test_store();
        let txn = store.begin_transaction().unwrap();
        let addr = Address([0x60; 32]);
        let slot = [0x01; 32];
        assert!(txn.get_contract_storage(&addr, &slot).unwrap().is_none());
    }

    #[test]
    fn contract_storage_different_slots_are_independent() {
        let store = test_store();
        let addr = Address([0x70; 32]);
        let slot_a = [0x01; 32];
        let slot_b = [0x02; 32];

        let txn = store.begin_transaction().unwrap();
        txn.set_contract_storage(&addr, &slot_a, b"value_a")
            .unwrap();
        txn.set_contract_storage(&addr, &slot_b, b"value_b")
            .unwrap();
        txn.commit().unwrap();

        let txn2 = store.begin_transaction().unwrap();
        assert_eq!(
            txn2.get_contract_storage(&addr, &slot_a).unwrap().unwrap(),
            b"value_a"
        );
        assert_eq!(
            txn2.get_contract_storage(&addr, &slot_b).unwrap().unwrap(),
            b"value_b"
        );
    }

    #[test]
    fn contract_storage_different_contracts_are_independent() {
        let store = test_store();
        let addr_a = Address([0x80; 32]);
        let addr_b = Address([0x81; 32]);
        let slot = [0x01; 32];

        let txn = store.begin_transaction().unwrap();
        txn.set_contract_storage(&addr_a, &slot, b"from_a").unwrap();
        txn.set_contract_storage(&addr_b, &slot, b"from_b").unwrap();
        txn.commit().unwrap();

        let txn2 = store.begin_transaction().unwrap();
        assert_eq!(
            txn2.get_contract_storage(&addr_a, &slot).unwrap().unwrap(),
            b"from_a"
        );
        assert_eq!(
            txn2.get_contract_storage(&addr_b, &slot).unwrap().unwrap(),
            b"from_b"
        );
    }

    #[test]
    fn overwrite_account_in_same_transaction() {
        let store = test_store();
        let addr = Address([0x90; 32]);

        let txn = store.begin_transaction().unwrap();
        txn.set_account(&addr, &Account::with_balance(addr, 100))
            .unwrap();
        txn.set_account(&addr, &Account::with_balance(addr, 500))
            .unwrap();
        txn.commit().unwrap();

        let txn2 = store.begin_transaction().unwrap();
        assert_eq!(txn2.get_account(&addr).unwrap().unwrap().balance, 500);
    }
}
