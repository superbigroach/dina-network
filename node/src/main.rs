mod genesis;
mod mempool;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tokio::signal;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info};

use dina_core::account::AccountState;
use dina_core::types::Address;
use dina_rpc::jsonrpc::NodeState;
use dina_rpc::server::{RpcConfig, RpcServer};
use dina_storage::DinaDB;
use dina_wasm::runtime::{RuntimeConfig, WasmRuntime};

use crate::genesis::{
    create_genesis_block, default_testnet_genesis, load_genesis_config, GenesisConfig,
};
use crate::mempool::Mempool;

/// Dina Network node binary.
#[derive(Parser, Debug)]
#[command(name = "dina-node", version, about = "Dina Network blockchain node")]
struct Cli {
    /// Data directory for blockchain storage and keys.
    #[arg(long, default_value = "~/.dina")]
    data_dir: String,

    /// Listen address for P2P networking (multiaddr format).
    #[arg(long, default_value = "/ip4/0.0.0.0/tcp/9944")]
    listen: String,

    /// Port for the JSON-RPC server.
    #[arg(long, default_value_t = 8545)]
    rpc_port: u16,

    /// Port for the REST API server.
    #[arg(long, default_value_t = 8080)]
    rest_port: u16,

    /// Run this node as a validator.
    #[arg(long)]
    validator: bool,

    /// Path to the validator Ed25519 signing key file.
    #[arg(long)]
    validator_key: Option<String>,

    /// Bootstrap peer multiaddresses (can be specified multiple times).
    #[arg(long)]
    bootstrap: Vec<String>,

    /// Chain identifier.
    #[arg(long, default_value = "dina-testnet-1")]
    chain_id: String,

    /// Path to a genesis configuration JSON file.
    #[arg(long)]
    genesis: Option<String>,

    /// Log level filter (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    log_level: String,
}

/// Expand ~ to the user home directory.
fn expand_home(path: &str) -> PathBuf {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

/// Cross-platform home directory detection.
fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Load or generate the node identity (Ed25519 keypair).
///
/// If a key file exists at `<data_dir>/node_key`, it is loaded.
/// Otherwise a new keypair is generated and saved.
fn load_or_generate_identity(data_dir: &PathBuf) -> Result<SigningKey> {
    let key_path = data_dir.join("node_key");

    if key_path.exists() {
        let bytes = std::fs::read(&key_path)
            .with_context(|| format!("failed to read node key from {}", key_path.display()))?;

        if bytes.len() != 32 {
            anyhow::bail!(
                "node key file {} has invalid length: expected 32 bytes, got {}",
                key_path.display(),
                bytes.len()
            );
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);

        let address = Address::from_pubkey(&signing_key.verifying_key());
        info!(
            address = %address,
            path = %key_path.display(),
            "Loaded node identity"
        );

        Ok(signing_key)
    } else {
        let signing_key = SigningKey::generate(&mut OsRng);

        std::fs::write(&key_path, signing_key.as_bytes())
            .with_context(|| format!("failed to write node key to {}", key_path.display()))?;

        let address = Address::from_pubkey(&signing_key.verifying_key());
        info!(
            address = %address,
            path = %key_path.display(),
            "Generated new node identity"
        );

        Ok(signing_key)
    }
}

/// Load a validator signing key from a file.
fn load_validator_key(path: &str) -> Result<SigningKey> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read validator key from '{path}'"))?;

    if bytes.len() != 32 {
        anyhow::bail!(
            "validator key file has invalid length: expected 32 bytes, got {}",
            bytes.len()
        );
    }

    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&bytes);
    let signing_key = SigningKey::from_bytes(&key_bytes);

    let address = Address::from_pubkey(&signing_key.verifying_key());
    info!(address = %address, "Loaded validator key");

    Ok(signing_key)
}

/// Initialize the genesis state if the database has no blocks.
fn initialize_genesis(
    db: &DinaDB,
    genesis_config: &GenesisConfig,
    account_state: &mut AccountState,
) -> Result<()> {
    let latest_height = db
        .get_latest_block_height()
        .context("failed to check latest block height")?;

    if latest_height > 0 {
        info!(
            height = latest_height,
            "Chain already initialized, skipping genesis"
        );
        return Ok(());
    }

    // Check if genesis block (height 0) already exists
    if db
        .get_block(0)
        .context("failed to check for genesis block")?
        .is_some()
    {
        info!("Genesis block already exists");
        return Ok(());
    }

    let genesis_block = create_genesis_block(genesis_config);

    // Set up initial account balances
    for account in &genesis_config.initial_accounts {
        account_state.credit(&account.address, account.balance);
        db.set_account(
            account.address,
            &dina_core::Account::with_balance(account.address, account.balance),
        )
        .context("failed to persist genesis account")?;
        info!(
            address = %account.address,
            balance = account.balance,
            "Initialized genesis account"
        );
    }

    db.store_block(&genesis_block)
        .context("failed to store genesis block")?;

    info!(
        hash = %genesis_block.hash(),
        "Genesis block stored at height 0"
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing subscriber with the requested log level.
    let filter = tracing_subscriber::EnvFilter::try_new(&cli.log_level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        chain_id = %cli.chain_id,
        data_dir = %cli.data_dir,
        validator = cli.validator,
        "Starting Dina Network node"
    );

    // 1. Expand and create data directory
    let data_dir = expand_home(&cli.data_dir);
    std::fs::create_dir_all(&data_dir)
        .with_context(|| format!("failed to create data directory: {}", data_dir.display()))?;

    // 2. Load or generate node identity
    let node_key = load_or_generate_identity(&data_dir)?;
    let node_address = Address::from_pubkey(&node_key.verifying_key());
    let node_pubkey = node_key.verifying_key().to_bytes();

    info!(
        address = %node_address,
        pubkey = %hex::encode(node_pubkey),
        "Node identity ready"
    );

    // 3. Open database
    let db_path = data_dir.join("chain.redb");
    let db_path_str = db_path.to_string_lossy().to_string();
    let db = DinaDB::open(&db_path_str)
        .with_context(|| format!("failed to open database at {}", db_path.display()))?;

    info!(path = %db_path.display(), "Database opened");

    // 4. Load genesis config
    let genesis_config = if let Some(ref genesis_path) = cli.genesis {
        load_genesis_config(genesis_path)?
    } else {
        info!("Using default testnet genesis configuration");
        default_testnet_genesis()
    };

    // 5. Initialize genesis block if first run
    let mut account_state = AccountState::new();
    initialize_genesis(&db, &genesis_config, &mut account_state)?;

    // 6. Create shared node state for RPC
    let node_state = NodeState::new(cli.chain_id.clone());

    // Apply genesis account balances to the in-memory state
    {
        let mut accounts = node_state.accounts.write().await;
        for ga in &genesis_config.initial_accounts {
            accounts.credit(&ga.address, ga.balance);
        }
    }

    // 7. Initialize mempool
    let mempool = Arc::new(RwLock::new(Mempool::new()));

    // 8. Initialize WASM runtime
    let _wasm_runtime = WasmRuntime::new(RuntimeConfig::default());
    info!("WASM runtime initialized");

    // 9. Start RPC servers
    let rpc_config = RpcConfig {
        jsonrpc_bind: format!("127.0.0.1:{}", cli.rpc_port),
        rest_bind: format!("0.0.0.0:{}", cli.rest_port),
    };

    let rpc_server = RpcServer::new(rpc_config.clone(), node_state.clone());
    let (jsonrpc_handle, rest_handle) = rpc_server
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("failed to start RPC servers: {e}"))?;

    info!(
        jsonrpc = %rpc_config.jsonrpc_bind,
        rest = %rpc_config.rest_bind,
        "RPC servers started"
    );

    // 10. Start consensus engine if running as validator
    let consensus_handles = if cli.validator {
        let validator_key = if let Some(ref key_path) = cli.validator_key {
            load_validator_key(key_path)?
        } else {
            info!("No validator key specified, using node key for validation");
            node_key.clone()
        };

        let validator_pubkey = validator_key.verifying_key().to_bytes();
        let validator_address = Address::from_pubkey(&validator_key.verifying_key());

        info!(
            address = %validator_address,
            pubkey = %hex::encode(validator_pubkey),
            "Starting consensus engine as validator"
        );

        let consensus_config = dina_consensus::ConsensusConfig {
            validator_keys: genesis_config
                .validators
                .iter()
                .copied()
                .chain(std::iter::once(validator_pubkey))
                .collect(),
            block_time_ms: 2000,
            timeout_ms: 10000,
        };

        let (output_tx, mut output_rx) = mpsc::unbounded_channel();
        let (tx_tx, tx_rx) = mpsc::unbounded_channel();

        let mut consensus =
            dina_consensus::TurboBFT::new(consensus_config, validator_key, output_tx);

        // Feed pending transactions from the mempool to consensus
        let mempool_feeder = mempool.clone();
        let tx_sender = tx_tx.clone();
        let feeder_handle = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_millis(1000));
            loop {
                interval.tick().await;
                let pool = mempool_feeder.read().await;
                let pending = pool.get_pending(100);
                if !pending.is_empty() {
                    if tx_sender.send(pending).is_err() {
                        break;
                    }
                }
            }
        });

        // Process consensus outputs (committed blocks)
        let node_state_consensus = node_state.clone();
        let db_consensus = db.clone();
        let mempool_pruner = mempool.clone();
        let output_handle = tokio::spawn(async move {
            while let Some(output) = output_rx.recv().await {
                match output {
                    dina_consensus::turbobft::ConsensusOutput::BlockCommitted {
                        block, ..
                    } => {
                        info!(
                            height = block.header.block_number,
                            hash = %block.hash(),
                            txs = block.transactions.len(),
                            "Block committed by consensus"
                        );

                        // Store block in database
                        if let Err(e) = db_consensus.store_block(&block) {
                            error!("Failed to store committed block: {e}");
                            continue;
                        }

                        // Remove committed transactions from mempool
                        let tx_hashes: Vec<_> =
                            block.transactions.iter().map(|tx| tx.hash()).collect();
                        {
                            let mut pool = mempool_pruner.write().await;
                            pool.remove_batch(&tx_hashes);
                        }

                        // Update in-memory block list
                        {
                            let mut blocks = node_state_consensus.blocks.write().await;
                            let mut idx =
                                node_state_consensus.block_index.write().await;
                            let pos = blocks.len();
                            idx.insert(block.hash(), pos);
                            blocks.push(block);
                        }
                    }
                    _ => {
                        // BroadcastProposal, BroadcastVote, BroadcastViewChange
                        // would be sent over P2P in production
                    }
                }
            }
        });

        // Run the consensus loop
        let consensus_handle = tokio::spawn(async move {
            consensus.start(tx_rx).await;
        });

        Some((consensus_handle, output_handle, feeder_handle))
    } else {
        info!("Running as non-validator node (no consensus participation)");
        None
    };

    // 11. Periodic mempool maintenance
    let mempool_maintenance = mempool.clone();
    let maintenance_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let mut pool = mempool_maintenance.write().await;
            let expired = pool.clear_expired();
            if expired > 0 {
                info!(
                    expired,
                    remaining = pool.size(),
                    "Mempool maintenance complete"
                );
            }
        }
    });

    info!(
        listen = %cli.listen,
        "Dina node fully started and ready"
    );

    // 12. Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal, shutting down gracefully...");
        }
        Err(e) => {
            error!("Failed to listen for shutdown signal: {e}");
        }
    }

    // Graceful shutdown
    info!("Stopping RPC servers...");
    jsonrpc_handle
        .stop()
        .map_err(|e| anyhow::anyhow!("failed to stop JSON-RPC: {e:?}"))?;
    rest_handle.abort();
    maintenance_handle.abort();

    if let Some((consensus, output, feeder)) = consensus_handles {
        consensus.abort();
        output.abort();
        feeder.abort();
    }

    info!("Dina node shutdown complete");
    Ok(())
}
