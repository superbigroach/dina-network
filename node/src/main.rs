mod chain_state;
mod genesis;
mod mempool;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tokio::signal;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use dina_core::account::AccountState;
use dina_core::block::{Block, BlockHeader};
use dina_core::executor::BlockExecutor;
use dina_core::types::{Address, Hash};
use dina_rpc::jsonrpc::NodeState;
use dina_rpc::server::{RpcConfig, RpcServer};
use dina_storage::DinaDB;

use crate::chain_state::ChainState;
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

    /// Block time in milliseconds (single-validator mode).
    #[arg(long, default_value_t = 2000)]
    block_time_ms: u64,
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
fn load_or_generate_identity(data_dir: &Path) -> Result<SigningKey> {
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
/// Returns the genesis block (either loaded from DB or freshly created).
fn initialize_genesis(
    db: &DinaDB,
    genesis_config: &GenesisConfig,
    account_state: &mut AccountState,
) -> Result<Block> {
    // Check if genesis block (height 0) already exists in the DB
    if let Some(existing_genesis) = db
        .get_block(0)
        .context("failed to check for genesis block")?
    {
        info!(
            hash = %existing_genesis.hash(),
            "Genesis block already exists, loading accounts from DB"
        );

        // Restore account state from the genesis accounts
        for ga in &genesis_config.initial_accounts {
            if let Ok(Some(acct)) = db.get_account(ga.address) {
                account_state.credit(&acct.address, acct.balance);
            } else {
                // Account not in DB yet — use genesis config values
                account_state.credit(&ga.address, ga.balance);
            }
        }

        return Ok(existing_genesis);
    }

    // First run: create and store genesis
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

    Ok(genesis_block)
}

/// Build a new block on top of the given parent, including transactions from
/// the mempool. Signs the block header with the validator key.
fn build_block(
    parent: &Block,
    transactions: Vec<dina_core::transaction::Transaction>,
    validator_key: &SigningKey,
) -> Block {
    let validator_address = Address::from_pubkey(&validator_key.verifying_key());
    let now = chrono::Utc::now().timestamp() as u64;
    // Ensure timestamp is strictly greater than parent
    let timestamp = now.max(parent.header.timestamp + 1);

    let header = BlockHeader {
        block_number: parent.header.block_number + 1,
        timestamp,
        parent_hash: parent.hash(),
        transactions_root: Hash::ZERO, // Filled after execution
        state_root: Hash::ZERO,        // Filled after execution
        proposer: validator_address,
        signature: [0u8; 64],          // Signed below
    };

    let mut block = Block {
        header,
        transactions,
    };

    // Sign the block header
    let header_hash = block.header.hash();
    block.header.signature = dina_core::crypto::sign(validator_key, header_hash.as_bytes());

    block
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // ── 1. Initialize tracing ───────────────────────────────────────────
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

    // ── 2. Expand and create data directory ─────────────────────────────
    let data_dir = expand_home(&cli.data_dir);
    std::fs::create_dir_all(&data_dir)
        .with_context(|| format!("failed to create data directory: {}", data_dir.display()))?;

    // ── 3. Load or generate node identity ───────────────────────────────
    let node_key = load_or_generate_identity(&data_dir)?;
    let node_address = Address::from_pubkey(&node_key.verifying_key());
    let node_pubkey = node_key.verifying_key().to_bytes();

    info!(
        address = %node_address,
        pubkey = %hex::encode(node_pubkey),
        "Node identity ready"
    );

    // ── 4. Open database ────────────────────────────────────────────────
    let db_path = data_dir.join("chain.redb");
    let db_path_str = db_path.to_string_lossy().to_string();
    let db = DinaDB::open(&db_path_str)
        .with_context(|| format!("failed to open database at {}", db_path.display()))?;

    info!(path = %db_path.display(), "Database opened");

    // ── 5. Load genesis config ──────────────────────────────────────────
    let genesis_config = if let Some(ref genesis_path) = cli.genesis {
        load_genesis_config(genesis_path)?
    } else {
        info!("Using default testnet genesis configuration");
        default_testnet_genesis()
    };

    // ── 6. Initialize genesis block and account state ───────────────────
    let mut account_state = AccountState::new();
    let genesis_block = initialize_genesis(&db, &genesis_config, &mut account_state)?;

    info!(
        genesis_hash = %genesis_block.hash(),
        accounts = genesis_config.initial_accounts.len(),
        "Genesis state initialized"
    );

    // ── 7. Create ChainState (in-memory canonical chain + accounts) ─────
    let chain_state = Arc::new(RwLock::new(
        ChainState::new(genesis_block.clone(), cli.chain_id.clone()),
    ));

    // Seed the chain state accounts from genesis
    {
        let mut cs = chain_state.write().await;
        for ga in &genesis_config.initial_accounts {
            cs.accounts.credit(&ga.address, ga.balance);
        }
    }

    // If the DB has blocks beyond genesis, replay them into ChainState.
    {
        let latest_height = db
            .get_latest_block_height()
            .context("failed to read latest block height")?;
        if latest_height > 0 {
            info!(
                latest_height,
                "Replaying stored blocks into in-memory chain state"
            );
            let mut cs = chain_state.write().await;
            for height in 1..=latest_height {
                if let Some(block) = db.get_block(height).context("failed to load block")? {
                    if let Err(e) = cs.apply_block(block) {
                        warn!(height, err = %e, "Failed to replay block (may already be applied)");
                    }
                }
            }
            info!(height = cs.current_height(), "Chain replay complete");
        }
    }

    // ── 8. Create shared NodeState for RPC ──────────────────────────────
    let node_state = NodeState::new(cli.chain_id.clone());

    // Replace the default genesis block with the real one
    {
        let mut blocks = node_state.blocks.write().await;
        let mut idx = node_state.block_index.write().await;
        blocks.clear();
        idx.clear();
        idx.insert(genesis_block.hash(), 0);
        blocks.push(genesis_block.clone());
    }

    // Populate the NodeState accounts from the chain state
    {
        let cs = chain_state.read().await;
        let mut accounts = node_state.accounts.write().await;
        for (addr, acct) in cs.accounts.iter() {
            accounts.credit(addr, acct.balance);
        }
    }

    // Replay blocks into NodeState as well (for RPC block queries)
    {
        let latest_height = db
            .get_latest_block_height()
            .context("failed to read latest block height")?;
        if latest_height > 0 {
            let mut blocks = node_state.blocks.write().await;
            let mut idx = node_state.block_index.write().await;
            for height in 1..=latest_height {
                if let Some(block) = db.get_block(height).context("failed to load block")? {
                    let hash = block.hash();
                    let pos = blocks.len();
                    idx.insert(hash, pos);
                    blocks.push(block);
                }
            }
        }
    }

    // ── 9. Initialize mempool ───────────────────────────────────────────
    let mempool = Arc::new(RwLock::new(Mempool::new()));

    // ── 10. Start RPC servers ───────────────────────────────────────────
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

    // ── 11. Bridge: drain RPC tx_pool into node Mempool ─────────────────
    //
    // The RPC server pushes submitted transactions into node_state.tx_pool.
    // This task drains that Vec and inserts them into the real Mempool so
    // the block production loop can pick them up.
    let rpc_tx_pool = node_state.tx_pool.clone();
    let mempool_bridge = mempool.clone();
    let bridge_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            // Drain the RPC tx_pool
            let txs = {
                let mut pool = rpc_tx_pool.write().await;
                if pool.is_empty() {
                    continue;
                }
                std::mem::take(&mut *pool)
            };
            // Insert into the real mempool
            let mut mp = mempool_bridge.write().await;
            for tx in txs {
                let hash = tx.hash();
                if let Err(e) = mp.add_transaction(tx) {
                    warn!(%hash, err = %e, "Failed to add transaction to mempool");
                }
            }
        }
    });

    // ── 12. Validator: single-validator block production loop ────────────
    let shutdown = tokio::sync::watch::channel(false);
    let (shutdown_tx, shutdown_rx) = (shutdown.0, shutdown.1);

    let validator_handle = if cli.validator {
        let validator_key = if let Some(ref key_path) = cli.validator_key {
            load_validator_key(key_path)?
        } else {
            info!("No validator key specified, using node key for validation");
            node_key.clone()
        };

        let validator_address = Address::from_pubkey(&validator_key.verifying_key());
        info!(
            address = %validator_address,
            pubkey = %hex::encode(validator_key.verifying_key().to_bytes()),
            block_time_ms = cli.block_time_ms,
            "Starting single-validator block production"
        );

        let block_time = tokio::time::Duration::from_millis(cli.block_time_ms);
        let chain_state_prod = chain_state.clone();
        let mempool_prod = mempool.clone();
        let node_state_prod = node_state.clone();
        let db_prod = db.clone();
        let mut shutdown_rx_prod = shutdown_rx.clone();

        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(block_time);
            // Skip the first immediate tick
            interval.tick().await;

            loop {
                tokio::select! {
                    _ = interval.tick() => {},
                    _ = shutdown_rx_prod.changed() => {
                        info!("Block production loop shutting down");
                        break;
                    }
                }

                // Collect pending transactions from mempool
                let pending_txs = {
                    let pool = mempool_prod.read().await;
                    pool.get_pending(500) // Up to 500 txs per block
                };

                // Build a new block
                let block = {
                    let cs = chain_state_prod.read().await;
                    let parent = cs.latest_block().clone();
                    build_block(&parent, pending_txs, &validator_key)
                };

                let block_height = block.header.block_number;
                let tx_count = block.transactions.len();

                // Execute the block via BlockExecutor to compute state changes
                let execution_result = {
                    let cs = chain_state_prod.read().await;
                    let mut executor = BlockExecutor::new(cs.accounts.clone());
                    executor.execute_block(&block)
                };

                let _exec_result = match execution_result {
                    Ok(r) => r,
                    Err(e) => {
                        error!(height = block_height, err = %e, "Block execution failed");
                        continue;
                    }
                };

                // Apply the block to ChainState (validates parent hash, height, timestamp)
                {
                    let mut cs = chain_state_prod.write().await;
                    match cs.apply_block(block.clone()) {
                        Ok(result) => {
                            info!(
                                height = block_height,
                                hash = %block.hash(),
                                txs = tx_count,
                                successful = result.successful_txs,
                                failed = result.failed_txs,
                                fees = result.total_fees,
                                "Block committed"
                            );
                        }
                        Err(e) => {
                            error!(height = block_height, err = %e, "Failed to apply block to chain state");
                            continue;
                        }
                    }
                }

                // Persist the block to the database
                if let Err(e) = db_prod.store_block(&block) {
                    error!(height = block_height, err = %e, "Failed to store block in database");
                }

                // Persist updated accounts to the database
                {
                    let cs = chain_state_prod.read().await;
                    for (addr, acct) in cs.accounts.iter() {
                        if let Err(e) = db_prod.set_account(*addr, acct) {
                            error!(address = %addr, err = %e, "Failed to persist account");
                        }
                    }
                }

                // Remove committed transactions from the mempool
                {
                    let tx_hashes: Vec<_> =
                        block.transactions.iter().map(|tx| tx.hash()).collect();
                    if !tx_hashes.is_empty() {
                        let mut pool = mempool_prod.write().await;
                        pool.remove_batch(&tx_hashes);
                    }
                }

                // Update NodeState so RPC sees the new block and accounts
                {
                    // Update blocks list
                    let mut blocks = node_state_prod.blocks.write().await;
                    let mut idx = node_state_prod.block_index.write().await;
                    let pos = blocks.len();
                    idx.insert(block.hash(), pos);
                    blocks.push(block.clone());
                }
                {
                    // Sync account state to RPC
                    let cs = chain_state_prod.read().await;
                    let mut accounts = node_state_prod.accounts.write().await;
                    // Replace the entire account state so RPC reflects
                    // the latest balances and nonces.
                    *accounts = cs.accounts.clone();
                }
                {
                    // Index transactions by hash for RPC lookups
                    let mut tx_idx = node_state_prod.tx_index.write().await;
                    for tx in &block.transactions {
                        tx_idx.insert(tx.hash(), (tx.clone(), Some(block_height)));
                    }
                }
            }
        }))
    } else {
        info!("Running as non-validator node (no block production)");
        None
    };

    // ── 13. Periodic mempool maintenance ────────────────────────────────
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

    // ── 14. Periodic status logging ─────────────────────────────────────
    let chain_state_status = chain_state.clone();
    let mempool_status = mempool.clone();
    let status_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let cs = chain_state_status.read().await;
            let pool = mempool_status.read().await;
            info!(
                height = cs.current_height(),
                mempool = pool.size(),
                "Node status"
            );
        }
    });

    info!(
        listen = %cli.listen,
        rpc = %rpc_config.jsonrpc_bind,
        rest = %rpc_config.rest_bind,
        "Dina node fully started and ready"
    );

    // ── 15. Wait for shutdown signal ────────────────────────────────────
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal, shutting down gracefully...");
        }
        Err(e) => {
            error!("Failed to listen for shutdown signal: {e}");
        }
    }

    // ── 16. Graceful shutdown ───────────────────────────────────────────
    // Signal the block production loop to stop
    let _ = shutdown_tx.send(true);

    info!("Stopping RPC servers...");
    jsonrpc_handle
        .stop()
        .map_err(|e| anyhow::anyhow!("failed to stop JSON-RPC: {e:?}"))?;
    rest_handle.abort();
    bridge_handle.abort();
    maintenance_handle.abort();
    status_handle.abort();

    if let Some(handle) = validator_handle {
        handle.abort();
    }

    // Log final state
    {
        let cs = chain_state.read().await;
        info!(
            final_height = cs.current_height(),
            "Dina node shutdown complete"
        );
    }

    Ok(())
}
