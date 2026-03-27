use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{DinaError, DinaResult};
use crate::types::{Address, Hash};

/// ABI-level mutability of a contract method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mutability {
    /// Read-only, does not modify state.
    View,
    /// Modifies on-chain state.
    Mutable,
    /// Accepts USDC value transfer.
    Payable,
}

/// A single parameter in a method ABI definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamAbi {
    pub name: String,
    pub param_type: String,
}

/// ABI description of a single contract method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodAbi {
    pub name: String,
    pub params: Vec<ParamAbi>,
    pub returns: String,
    pub mutability: Mutability,
}

/// Additional metadata attached to a registered contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractMetadata {
    pub description: String,
    pub repository: String,
    pub license: String,
    pub abi: Vec<MethodAbi>,
}

/// Full information about a deployed contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractInfo {
    pub address: Address,
    pub deployer: Address,
    pub code_hash: Hash,
    pub wasm_size: usize,
    pub name: Option<String>,
    pub version: String,
    pub interfaces: Vec<u32>,
    pub deployed_at: u64,
    pub verified: bool,
    pub source_hash: Option<Hash>,
    pub metadata: ContractMetadata,
}

/// On-chain registry of deployed contracts with metadata and secondary indices.
pub struct ContractRegistry {
    contracts: BTreeMap<Address, ContractInfo>,
    name_index: BTreeMap<String, Address>,
    deployer_index: BTreeMap<Address, Vec<Address>>,
}

impl ContractRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            contracts: BTreeMap::new(),
            name_index: BTreeMap::new(),
            deployer_index: BTreeMap::new(),
        }
    }

    /// Register a new contract. Fails if the address is already registered or
    /// the name (if provided) is already taken.
    pub fn register_contract(&mut self, info: ContractInfo) -> DinaResult<()> {
        if self.contracts.contains_key(&info.address) {
            return Err(DinaError::RegistryError(format!(
                "contract already registered at {}",
                info.address
            )));
        }

        if let Some(ref name) = info.name {
            if self.name_index.contains_key(name) {
                return Err(DinaError::RegistryError(format!(
                    "contract name '{}' already taken",
                    name
                )));
            }
            self.name_index.insert(name.clone(), info.address);
        }

        self.deployer_index
            .entry(info.deployer)
            .or_default()
            .push(info.address);

        self.contracts.insert(info.address, info);
        Ok(())
    }

    /// Look up a contract by its address.
    pub fn get_contract(&self, address: &Address) -> Option<&ContractInfo> {
        self.contracts.get(address)
    }

    /// Look up a contract address by its registered name.
    pub fn find_by_name(&self, name: &str) -> Option<Address> {
        self.name_index.get(name).copied()
    }

    /// Return all contract addresses deployed by a given deployer.
    pub fn contracts_by_deployer(&self, deployer: &Address) -> Vec<Address> {
        self.deployer_index
            .get(deployer)
            .cloned()
            .unwrap_or_default()
    }

    /// Return all contract addresses that declare support for a DRC interface ID.
    pub fn contracts_by_interface(&self, interface_id: u32) -> Vec<Address> {
        self.contracts
            .values()
            .filter(|c| c.interfaces.contains(&interface_id))
            .map(|c| c.address)
            .collect()
    }

    /// Mark a contract as verified by recording its source hash. Only succeeds
    /// if the contract exists and is not already verified.
    pub fn verify_contract(&mut self, address: &Address, source_hash: Hash) -> DinaResult<()> {
        let info = self
            .contracts
            .get_mut(address)
            .ok_or_else(|| DinaError::ContractNotFound(address.to_string()))?;

        if info.verified {
            return Err(DinaError::RegistryError(format!(
                "contract {} is already verified",
                address
            )));
        }

        info.verified = true;
        info.source_hash = Some(source_hash);
        Ok(())
    }

    /// Update metadata for a contract. Only the original deployer may do this.
    pub fn update_metadata(
        &mut self,
        address: &Address,
        deployer: &Address,
        metadata: ContractMetadata,
    ) -> DinaResult<()> {
        let info = self
            .contracts
            .get_mut(address)
            .ok_or_else(|| DinaError::ContractNotFound(address.to_string()))?;

        if info.deployer != *deployer {
            return Err(DinaError::RegistryError(
                "only the deployer can update metadata".to_string(),
            ));
        }

        info.metadata = metadata;
        Ok(())
    }

    /// Total number of registered contracts.
    pub fn total_contracts(&self) -> usize {
        self.contracts.len()
    }
}

impl Default for ContractRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_address(byte: u8) -> Address {
        Address([byte; 32])
    }

    fn make_hash(byte: u8) -> Hash {
        Hash([byte; 32])
    }

    fn make_metadata() -> ContractMetadata {
        ContractMetadata {
            description: "test contract".to_string(),
            repository: "https://github.com/dina/test".to_string(),
            license: "MIT".to_string(),
            abi: vec![MethodAbi {
                name: "transfer".to_string(),
                params: vec![
                    ParamAbi { name: "to".to_string(), param_type: "Address".to_string() },
                    ParamAbi { name: "amount".to_string(), param_type: "u64".to_string() },
                ],
                returns: "bool".to_string(),
                mutability: Mutability::Mutable,
            }],
        }
    }

    fn make_contract_info(addr_byte: u8, deployer_byte: u8, name: Option<&str>) -> ContractInfo {
        ContractInfo {
            address: make_address(addr_byte),
            deployer: make_address(deployer_byte),
            code_hash: make_hash(addr_byte),
            wasm_size: 1024,
            name: name.map(|n| n.to_string()),
            version: "1.0.0".to_string(),
            interfaces: vec![20, 721],
            deployed_at: 100,
            verified: false,
            source_hash: None,
            metadata: make_metadata(),
        }
    }

    #[test]
    fn register_and_get_contract() {
        let mut reg = ContractRegistry::new();
        let info = make_contract_info(1, 10, Some("token"));
        reg.register_contract(info.clone()).unwrap();
        let retrieved = reg.get_contract(&make_address(1)).unwrap();
        assert_eq!(retrieved.address, make_address(1));
        assert_eq!(retrieved.wasm_size, 1024);
    }

    #[test]
    fn register_duplicate_address_fails() {
        let mut reg = ContractRegistry::new();
        reg.register_contract(make_contract_info(1, 10, Some("a"))).unwrap();
        let result = reg.register_contract(make_contract_info(1, 10, Some("b")));
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("already registered"));
    }

    #[test]
    fn register_duplicate_name_fails() {
        let mut reg = ContractRegistry::new();
        reg.register_contract(make_contract_info(1, 10, Some("token"))).unwrap();
        let result = reg.register_contract(make_contract_info(2, 10, Some("token")));
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("already taken"));
    }

    #[test]
    fn find_by_name() {
        let mut reg = ContractRegistry::new();
        reg.register_contract(make_contract_info(1, 10, Some("mytoken"))).unwrap();
        assert_eq!(reg.find_by_name("mytoken"), Some(make_address(1)));
        assert_eq!(reg.find_by_name("nonexistent"), None);
    }

    #[test]
    fn contracts_by_deployer() {
        let mut reg = ContractRegistry::new();
        reg.register_contract(make_contract_info(1, 10, Some("a"))).unwrap();
        reg.register_contract(make_contract_info(2, 10, Some("b"))).unwrap();
        reg.register_contract(make_contract_info(3, 20, Some("c"))).unwrap();
        let deployer_10 = reg.contracts_by_deployer(&make_address(10));
        assert_eq!(deployer_10.len(), 2);
        let deployer_20 = reg.contracts_by_deployer(&make_address(20));
        assert_eq!(deployer_20.len(), 1);
        let deployer_99 = reg.contracts_by_deployer(&make_address(99));
        assert!(deployer_99.is_empty());
    }

    #[test]
    fn contracts_by_interface() {
        let mut reg = ContractRegistry::new();
        let mut info = make_contract_info(1, 10, Some("a"));
        info.interfaces = vec![20, 721];
        reg.register_contract(info).unwrap();

        let mut info2 = make_contract_info(2, 10, Some("b"));
        info2.interfaces = vec![20];
        reg.register_contract(info2).unwrap();

        assert_eq!(reg.contracts_by_interface(20).len(), 2);
        assert_eq!(reg.contracts_by_interface(721).len(), 1);
        assert_eq!(reg.contracts_by_interface(999).len(), 0);
    }

    #[test]
    fn verify_contract_success() {
        let mut reg = ContractRegistry::new();
        reg.register_contract(make_contract_info(1, 10, Some("a"))).unwrap();
        let src_hash = make_hash(0xAA);
        reg.verify_contract(&make_address(1), src_hash).unwrap();
        let info = reg.get_contract(&make_address(1)).unwrap();
        assert!(info.verified);
        assert_eq!(info.source_hash, Some(src_hash));
    }

    #[test]
    fn verify_contract_already_verified_fails() {
        let mut reg = ContractRegistry::new();
        reg.register_contract(make_contract_info(1, 10, Some("a"))).unwrap();
        reg.verify_contract(&make_address(1), make_hash(0xAA)).unwrap();
        let result = reg.verify_contract(&make_address(1), make_hash(0xBB));
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("already verified"));
    }

    #[test]
    fn verify_nonexistent_contract_fails() {
        let mut reg = ContractRegistry::new();
        let result = reg.verify_contract(&make_address(99), make_hash(0xAA));
        assert!(result.is_err());
    }

    #[test]
    fn update_metadata_by_deployer() {
        let mut reg = ContractRegistry::new();
        reg.register_contract(make_contract_info(1, 10, None)).unwrap();
        let new_meta = ContractMetadata {
            description: "updated".to_string(),
            repository: "https://new.repo".to_string(),
            license: "Apache-2.0".to_string(),
            abi: vec![],
        };
        reg.update_metadata(&make_address(1), &make_address(10), new_meta).unwrap();
        let info = reg.get_contract(&make_address(1)).unwrap();
        assert_eq!(info.metadata.description, "updated");
        assert_eq!(info.metadata.license, "Apache-2.0");
    }

    #[test]
    fn update_metadata_wrong_deployer_fails() {
        let mut reg = ContractRegistry::new();
        reg.register_contract(make_contract_info(1, 10, None)).unwrap();
        let new_meta = ContractMetadata {
            description: "hacked".to_string(),
            repository: String::new(),
            license: String::new(),
            abi: vec![],
        };
        let result = reg.update_metadata(&make_address(1), &make_address(99), new_meta);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("deployer"));
    }

    #[test]
    fn total_contracts() {
        let mut reg = ContractRegistry::new();
        assert_eq!(reg.total_contracts(), 0);
        reg.register_contract(make_contract_info(1, 10, Some("a"))).unwrap();
        assert_eq!(reg.total_contracts(), 1);
        reg.register_contract(make_contract_info(2, 10, Some("b"))).unwrap();
        assert_eq!(reg.total_contracts(), 2);
    }

    #[test]
    fn get_nonexistent_contract_returns_none() {
        let reg = ContractRegistry::new();
        assert!(reg.get_contract(&make_address(1)).is_none());
    }

    #[test]
    fn register_without_name_skips_name_index() {
        let mut reg = ContractRegistry::new();
        reg.register_contract(make_contract_info(1, 10, None)).unwrap();
        assert!(reg.get_contract(&make_address(1)).is_some());
        // No name should be in the index
        assert_eq!(reg.name_index.len(), 0);
    }

    #[test]
    fn default_creates_empty_registry() {
        let reg = ContractRegistry::default();
        assert_eq!(reg.total_contracts(), 0);
    }
}
