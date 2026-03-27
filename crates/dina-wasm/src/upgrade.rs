use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use dina_core::error::DinaError;
use dina_core::types::{Address, Hash};

/// Record of a single contract upgrade (or rollback).
#[derive(Debug, Clone)]
pub struct UpgradeRecord {
    /// SHA-256 hash of the previous WASM bytecode.
    pub old_code_hash: Hash,
    /// SHA-256 hash of the new WASM bytecode.
    pub new_code_hash: Hash,
    /// Address of the account that performed the upgrade.
    pub upgraded_by: Address,
    /// Block timestamp at which the upgrade occurred.
    pub upgraded_at: u64,
    /// Optional migration data passed to the new contract's `__migrate` entry.
    pub migration_data: Option<Vec<u8>>,
}

/// Manages contract code upgrades, history tracking, and rollbacks.
///
/// Only the original deployer of a contract (the "owner") may upgrade it.
/// Each upgrade is recorded so the full history is auditable, and a single
/// rollback to the previous version is supported.
pub struct ContractUpgrader {
    /// Per-contract upgrade history, ordered chronologically.
    upgrade_history: BTreeMap<Address, Vec<UpgradeRecord>>,
    /// Tracks the current WASM bytecode for each upgradeable contract.
    current_code: BTreeMap<Address, Vec<u8>>,
    /// Tracks the owner (deployer) of each contract.
    owners: BTreeMap<Address, Address>,
}

impl ContractUpgrader {
    /// Create a new empty upgrader.
    pub fn new() -> Self {
        Self {
            upgrade_history: BTreeMap::new(),
            current_code: BTreeMap::new(),
            owners: BTreeMap::new(),
        }
    }

    /// Register a contract so it can be upgraded later.
    ///
    /// Must be called after initial deployment to track the owner and code.
    pub fn register_contract(
        &mut self,
        contract: Address,
        owner: Address,
        wasm_bytes: &[u8],
    ) {
        self.owners.insert(contract, owner);
        self.current_code.insert(contract, wasm_bytes.to_vec());
    }

    /// Check whether `caller` is authorised to upgrade the contract.
    pub fn can_upgrade(&self, contract: Address, caller: Address) -> bool {
        self.owners.get(&contract) == Some(&caller)
    }

    /// Perform an upgrade: replace the contract's WASM bytecode.
    ///
    /// Returns the `UpgradeRecord` on success. Fails if the caller is not the
    /// owner, if the contract is not registered, or if the new code hashes to
    /// the same value as the old code (no-op upgrade).
    pub fn upgrade(
        &mut self,
        contract: Address,
        new_wasm: &[u8],
        migration_data: Option<Vec<u8>>,
        caller: Address,
        block_time: u64,
    ) -> Result<UpgradeRecord, DinaError> {
        if !self.can_upgrade(contract, caller) {
            return Err(DinaError::WasmExecutionError(
                "caller is not authorised to upgrade this contract".into(),
            ));
        }

        let old_code = self
            .current_code
            .get(&contract)
            .ok_or_else(|| DinaError::ContractNotFound(contract.to_string()))?;

        let old_code_hash = Self::hash_code(old_code);
        let new_code_hash = Self::hash_code(new_wasm);

        if old_code_hash == new_code_hash {
            return Err(DinaError::WasmExecutionError(
                "new code is identical to current code".into(),
            ));
        }

        let record = UpgradeRecord {
            old_code_hash,
            new_code_hash,
            upgraded_by: caller,
            upgraded_at: block_time,
            migration_data,
        };

        self.current_code.insert(contract, new_wasm.to_vec());
        self.upgrade_history
            .entry(contract)
            .or_default()
            .push(record.clone());

        Ok(record)
    }

    /// Return the full upgrade history for a contract.
    pub fn upgrade_history(&self, contract: Address) -> Vec<&UpgradeRecord> {
        self.upgrade_history
            .get(&contract)
            .map(|records| records.iter().collect())
            .unwrap_or_default()
    }

    /// Rollback a contract to its previous version.
    ///
    /// This effectively re-applies the old code from the most recent upgrade
    /// record and appends a new record documenting the rollback.
    pub fn rollback(
        &mut self,
        contract: Address,
        caller: Address,
        block_time: u64,
    ) -> Result<UpgradeRecord, DinaError> {
        if !self.can_upgrade(contract, caller) {
            return Err(DinaError::WasmExecutionError(
                "caller is not authorised to rollback this contract".into(),
            ));
        }

        let history = self
            .upgrade_history
            .get(&contract)
            .ok_or_else(|| {
                DinaError::WasmExecutionError("no upgrade history to rollback".into())
            })?;

        let last_upgrade = history.last().ok_or_else(|| {
            DinaError::WasmExecutionError("no upgrade history to rollback".into())
        })?;

        // The rollback swaps new and old: the current code hash becomes "old",
        // and the previous code hash becomes "new".
        let current_hash = Self::hash_code(
            self.current_code
                .get(&contract)
                .ok_or_else(|| DinaError::ContractNotFound(contract.to_string()))?,
        );

        // We don't have the actual old bytecode stored here (that would be in
        // the runtime's contract store). We record the intent; the runtime is
        // responsible for swapping the actual bytecode.
        let record = UpgradeRecord {
            old_code_hash: current_hash,
            new_code_hash: last_upgrade.old_code_hash,
            upgraded_by: caller,
            upgraded_at: block_time,
            migration_data: None,
        };

        self.upgrade_history
            .entry(contract)
            .or_default()
            .push(record.clone());

        Ok(record)
    }

    /// Get the current code for a registered contract.
    pub fn current_code(&self, contract: Address) -> Option<&[u8]> {
        self.current_code.get(&contract).map(|v| v.as_slice())
    }

    /// Compute the SHA-256 hash of WASM bytecode.
    fn hash_code(code: &[u8]) -> Hash {
        let result = Sha256::digest(code);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        Hash(bytes)
    }
}

impl Default for ContractUpgrader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(byte: u8) -> Address {
        Address([byte; 32])
    }

    fn wasm_v1() -> Vec<u8> {
        vec![0x00, 0x61, 0x73, 0x6d, 0x01]
    }

    fn wasm_v2() -> Vec<u8> {
        vec![0x00, 0x61, 0x73, 0x6d, 0x02]
    }

    fn wasm_v3() -> Vec<u8> {
        vec![0x00, 0x61, 0x73, 0x6d, 0x03]
    }

    #[test]
    fn new_upgrader_is_empty() {
        let upgrader = ContractUpgrader::new();
        assert!(upgrader.upgrade_history(addr(1)).is_empty());
    }

    #[test]
    fn register_and_can_upgrade() {
        let mut upgrader = ContractUpgrader::new();
        let contract = addr(1);
        let owner = addr(2);
        upgrader.register_contract(contract, owner, &wasm_v1());

        assert!(upgrader.can_upgrade(contract, owner));
        assert!(!upgrader.can_upgrade(contract, addr(3)));
    }

    #[test]
    fn upgrade_succeeds_for_owner() {
        let mut upgrader = ContractUpgrader::new();
        let contract = addr(1);
        let owner = addr(2);
        upgrader.register_contract(contract, owner, &wasm_v1());

        let record = upgrader
            .upgrade(contract, &wasm_v2(), None, owner, 1000)
            .unwrap();

        assert_eq!(record.upgraded_by, owner);
        assert_eq!(record.upgraded_at, 1000);
        assert_ne!(record.old_code_hash, record.new_code_hash);
        assert!(record.migration_data.is_none());
    }

    #[test]
    fn upgrade_fails_for_non_owner() {
        let mut upgrader = ContractUpgrader::new();
        let contract = addr(1);
        upgrader.register_contract(contract, addr(2), &wasm_v1());

        let result = upgrader.upgrade(contract, &wasm_v2(), None, addr(99), 1000);
        assert!(result.is_err());
    }

    #[test]
    fn upgrade_fails_with_identical_code() {
        let mut upgrader = ContractUpgrader::new();
        let contract = addr(1);
        let owner = addr(2);
        let code = wasm_v1();
        upgrader.register_contract(contract, owner, &code);

        let result = upgrader.upgrade(contract, &code, None, owner, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn upgrade_history_tracks_all_upgrades() {
        let mut upgrader = ContractUpgrader::new();
        let contract = addr(1);
        let owner = addr(2);
        upgrader.register_contract(contract, owner, &wasm_v1());

        upgrader
            .upgrade(contract, &wasm_v2(), None, owner, 100)
            .unwrap();
        upgrader
            .upgrade(contract, &wasm_v3(), Some(b"migrate".to_vec()), owner, 200)
            .unwrap();

        let history = upgrader.upgrade_history(contract);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].upgraded_at, 100);
        assert_eq!(history[1].upgraded_at, 200);
        assert_eq!(history[1].migration_data, Some(b"migrate".to_vec()));
    }

    #[test]
    fn rollback_creates_record() {
        let mut upgrader = ContractUpgrader::new();
        let contract = addr(1);
        let owner = addr(2);
        upgrader.register_contract(contract, owner, &wasm_v1());

        let upgrade_record = upgrader
            .upgrade(contract, &wasm_v2(), None, owner, 100)
            .unwrap();

        let rollback_record = upgrader.rollback(contract, owner, 200).unwrap();

        // The rollback's new_code_hash should be the upgrade's old_code_hash
        assert_eq!(rollback_record.new_code_hash, upgrade_record.old_code_hash);
        assert_eq!(rollback_record.upgraded_at, 200);
    }

    #[test]
    fn rollback_fails_without_history() {
        let mut upgrader = ContractUpgrader::new();
        let contract = addr(1);
        let owner = addr(2);
        upgrader.register_contract(contract, owner, &wasm_v1());

        let result = upgrader.rollback(contract, owner, 100);
        assert!(result.is_err());
    }

    #[test]
    fn rollback_fails_for_non_owner() {
        let mut upgrader = ContractUpgrader::new();
        let contract = addr(1);
        let owner = addr(2);
        upgrader.register_contract(contract, owner, &wasm_v1());
        upgrader
            .upgrade(contract, &wasm_v2(), None, owner, 100)
            .unwrap();

        let result = upgrader.rollback(contract, addr(99), 200);
        assert!(result.is_err());
    }

    #[test]
    fn current_code_updates_after_upgrade() {
        let mut upgrader = ContractUpgrader::new();
        let contract = addr(1);
        let owner = addr(2);
        let v1 = wasm_v1();
        let v2 = wasm_v2();
        upgrader.register_contract(contract, owner, &v1);

        assert_eq!(upgrader.current_code(contract), Some(v1.as_slice()));
        upgrader
            .upgrade(contract, &v2, None, owner, 100)
            .unwrap();
        assert_eq!(upgrader.current_code(contract), Some(v2.as_slice()));
    }

    #[test]
    fn unregistered_contract_cannot_upgrade() {
        let upgrader = ContractUpgrader::new();
        assert!(!upgrader.can_upgrade(addr(99), addr(1)));
    }
}
