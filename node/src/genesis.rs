use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::info;

use dina_core::block::{Block, BlockHeader};
use dina_core::types::{Address, Hash};

/// Configuration for a genesis block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Chain identifier (e.g., "dina-testnet-1").
    pub chain_id: String,
    /// Genesis block timestamp (seconds since UNIX epoch).
    pub timestamp: u64,
    /// Accounts to include in the genesis state.
    pub initial_accounts: Vec<GenesisAccount>,
    /// Ed25519 public keys of the initial validator set.
    pub validators: Vec<[u8; 32]>,
}

/// An account created at genesis time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    /// Account address.
    pub address: Address,
    /// Initial balance in micro-USDC (6 decimals).
    pub balance: u64,
}

/// Create a genesis block from the given configuration.
///
/// The genesis block has height 0, a zero parent hash, and a state root
/// derived from the initial accounts.
pub fn create_genesis_block(config: &GenesisConfig) -> Block {
    // Compute state root from the initial accounts
    let state_root = compute_genesis_state_root(&config.initial_accounts);

    let header = BlockHeader {
        block_number: 0,
        timestamp: config.timestamp,
        parent_hash: Hash::ZERO,
        transactions_root: Hash::ZERO,
        state_root,
        proposer: Address::ZERO,
        proposer_pubkey: [0u8; 32],
        signature: [0u8; 64],
    };

    let block = Block {
        header,
        transactions: Vec::new(),
    };

    info!(
        chain_id = %config.chain_id,
        accounts = config.initial_accounts.len(),
        validators = config.validators.len(),
        block_hash = %block.hash(),
        "Genesis block created"
    );

    block
}

/// Load a genesis configuration from a JSON file on disk.
pub fn load_genesis_config(path: &str) -> Result<GenesisConfig> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read genesis config from '{path}'"))?;
    let config: GenesisConfig = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse genesis config from '{path}'"))?;

    info!(
        chain_id = %config.chain_id,
        accounts = config.initial_accounts.len(),
        validators = config.validators.len(),
        "Loaded genesis config from {path}"
    );

    Ok(config)
}

/// Create a default testnet genesis configuration.
///
/// Includes a faucet account holding 1 billion USDC (1_000_000_000 * 10^6
/// micro-USDC) for funding testnet accounts.
pub fn default_testnet_genesis() -> GenesisConfig {
    // Deterministic faucet address: SHA-256("dina-testnet-faucet")
    let mut hasher = Sha256::new();
    hasher.update(b"dina-testnet-faucet");
    let result = hasher.finalize();
    let mut faucet_bytes = [0u8; 32];
    faucet_bytes.copy_from_slice(&result);
    let faucet_address = Address(faucet_bytes);

    // 1 billion USDC with 6 decimals = 1_000_000_000_000_000 micro-USDC
    let faucet_balance: u64 = 1_000_000_000 * 1_000_000;

    let now = chrono::Utc::now().timestamp() as u64;

    GenesisConfig {
        chain_id: "dina-testnet-1".to_string(),
        timestamp: now,
        initial_accounts: vec![GenesisAccount {
            address: faucet_address,
            balance: faucet_balance,
        }],
        validators: Vec::new(),
    }
}

/// Save a genesis config to a JSON file.
#[allow(dead_code)]
pub fn save_genesis_config(config: &GenesisConfig, path: &str) -> Result<()> {
    let json =
        serde_json::to_string_pretty(config).context("failed to serialize genesis config")?;
    std::fs::write(path, json)
        .with_context(|| format!("failed to write genesis config to '{path}'"))?;
    info!("Saved genesis config to {path}");
    Ok(())
}

/// Compute a state root from the initial accounts by hashing their addresses
/// and balances together.
fn compute_genesis_state_root(accounts: &[GenesisAccount]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(b"DINA_GENESIS_STATE_ROOT");
    for account in accounts {
        hasher.update(account.address.as_bytes());
        hasher.update(account.balance.to_le_bytes());
    }
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    Hash(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_testnet_genesis_has_faucet() {
        let genesis = default_testnet_genesis();
        assert_eq!(genesis.chain_id, "dina-testnet-1");
        assert_eq!(genesis.initial_accounts.len(), 1);
        assert_eq!(
            genesis.initial_accounts[0].balance,
            1_000_000_000 * 1_000_000
        );
    }

    #[test]
    fn genesis_block_has_height_zero() {
        let genesis = default_testnet_genesis();
        let block = create_genesis_block(&genesis);
        assert_eq!(block.header.block_number, 0);
        assert_eq!(block.header.parent_hash, Hash::ZERO);
        assert!(block.transactions.is_empty());
    }

    #[test]
    fn genesis_block_hash_is_deterministic() {
        let mut genesis = default_testnet_genesis();
        genesis.timestamp = 1700000000;
        let block1 = create_genesis_block(&genesis);
        let block2 = create_genesis_block(&genesis);
        assert_eq!(block1.hash(), block2.hash());
    }

    #[test]
    fn genesis_config_roundtrip() {
        let config = default_testnet_genesis();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: GenesisConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.chain_id, config.chain_id);
        assert_eq!(parsed.initial_accounts.len(), config.initial_accounts.len());
    }
}
