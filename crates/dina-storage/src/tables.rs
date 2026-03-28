use redb::TableDefinition;

/// Account data keyed by 32-byte address.
pub const ACCOUNTS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("accounts");

/// Block data keyed by block height.
pub const BLOCKS: TableDefinition<u64, &[u8]> = TableDefinition::new("blocks");

/// Reverse index: block hash -> block height.
pub const BLOCK_HASHES: TableDefinition<&[u8], u64> = TableDefinition::new("block_hashes");

/// Transaction data keyed by transaction hash.
pub const TRANSACTIONS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("transactions");

/// WASM contract code keyed by code hash.
pub const CONTRACT_CODE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("contract_code");

/// Contract storage keyed by composite key (address + slot).
pub const CONTRACT_STORAGE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("contract_storage");

/// Device identity records keyed by device address.
pub const DEVICE_REGISTRY: TableDefinition<&[u8], &[u8]> = TableDefinition::new("device_registry");

/// Global key-value metadata (e.g., schema version, latest block height).
pub const STATE_METADATA: TableDefinition<&str, &[u8]> = TableDefinition::new("state_metadata");
