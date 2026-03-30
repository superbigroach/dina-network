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
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use dina_core::account::AccountState;
use dina_core::block::{Block, BlockHeader};
use dina_core::executor::BlockExecutor;
use dina_core::types::{Address, Hash};
use dina_consensus::{
    ConsensusConfig, ConsensusOutput, InboundMessage, TurboBFT,
};
use dina_network::message::NetworkMessage;
use dina_network::node::{DinaNode, NodeEvent};
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

    /// Bind address for RPC servers (default: 127.0.0.1, use 0.0.0.0 for external access).
    #[arg(long, default_value = "127.0.0.1")]
    rpc_bind: String,

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
    #[arg(long, default_value_t = 100)]
    block_time_ms: u64,

    /// Consensus round timeout in milliseconds before triggering a view change
    /// (multi-validator mode). Default: 2000ms.
    #[arg(long, default_value_t = 2000)]
    consensus_timeout_ms: u64,

    /// Path to a PEM-encoded TLS certificate chain for the REST API.
    /// When both --tls-cert and --tls-key are provided, the REST server
    /// will listen over HTTPS. The JSON-RPC endpoint remains plaintext;
    /// use a reverse proxy (nginx/envoy/caddy) for full TLS coverage.
    #[arg(long)]
    tls_cert: Option<String>,

    /// Path to a PEM-encoded TLS private key for the REST API.
    #[arg(long)]
    tls_key: Option<String>,
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

        // M-1: Set restrictive permissions on the key file (owner read/write only).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
                .with_context(|| format!("failed to set permissions on {}", key_path.display()))?;
        }

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
        proposer_pubkey: *validator_key.verifying_key().as_bytes(),
        signature: [0u8; 64], // Signed below
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

// ── Type conversions between consensus and network message types ─────

/// Convert a consensus Proposal to a network Proposal.
fn consensus_proposal_to_network(p: &dina_consensus::Proposal) -> dina_network::message::Proposal {
    let block_bytes = match bincode::serialize(&p.block) {
        Ok(bytes) => {
            debug!(
                height = p.height,
                round = p.round,
                block_bytes = bytes.len(),
                "serialized proposal block for network"
            );
            bytes
        }
        Err(e) => {
            error!(
                height = p.height,
                round = p.round,
                err = %e,
                "CRITICAL: failed to serialize block in proposal — consensus will stall"
            );
            vec![]
        }
    };
    let block_hash = p.block.hash();
    dina_network::message::Proposal {
        height: p.height,
        round: p.round,
        block: dina_network::message::BlockPayload {
            data: block_bytes,
            height: p.block.header.block_number,
            hash: block_hash,
        },
        signature: p.signature,
        proposer: p.proposer,
    }
}

/// Convert a network Proposal to a consensus Proposal.
fn network_proposal_to_consensus(
    p: &dina_network::message::Proposal,
) -> Option<dina_consensus::Proposal> {
    let block: Block = match bincode::deserialize(&p.block.data) {
        Ok(b) => b,
        Err(e) => {
            error!(
                height = p.height,
                round = p.round,
                data_len = p.block.data.len(),
                err = %e,
                "failed to deserialize block from network proposal"
            );
            return None;
        }
    };
    Some(dina_consensus::Proposal {
        height: p.height,
        round: p.round,
        block,
        proposer: p.proposer,
        signature: p.signature,
    })
}

/// Convert a consensus Vote to a network Vote.
fn consensus_vote_to_network(v: &dina_consensus::Vote) -> dina_network::message::Vote {
    dina_network::message::Vote {
        height: v.height,
        round: v.round,
        block_hash: v.block_hash,
        vote_type: match v.vote_type {
            dina_consensus::VoteType::Prevote => dina_network::message::VoteType::Prevote,
            dina_consensus::VoteType::Precommit => dina_network::message::VoteType::Precommit,
        },
        signature: v.signature,
        voter: v.voter,
    }
}

/// Convert a network Vote to a consensus Vote.
fn network_vote_to_consensus(v: &dina_network::message::Vote) -> dina_consensus::Vote {
    dina_consensus::Vote {
        height: v.height,
        round: v.round,
        block_hash: v.block_hash,
        vote_type: match v.vote_type {
            dina_network::message::VoteType::Prevote => dina_consensus::VoteType::Prevote,
            dina_network::message::VoteType::Precommit => dina_consensus::VoteType::Precommit,
        },
        voter: v.voter,
        signature: v.signature,
    }
}

/// Convert a consensus ViewChange to a network ViewChange.
fn consensus_vc_to_network(
    vc: &dina_consensus::ViewChange,
) -> dina_network::message::ViewChange {
    dina_network::message::ViewChange {
        height: vc.height,
        old_round: vc.old_round,
        new_round: vc.new_round,
        signature: vc.signature,
        requester: vc.voter,
    }
}

/// Convert a network ViewChange to a consensus ViewChange.
fn network_vc_to_consensus(
    vc: &dina_network::message::ViewChange,
) -> dina_consensus::ViewChange {
    dina_consensus::ViewChange {
        height: vc.height,
        old_round: vc.old_round,
        new_round: vc.new_round,
        voter: vc.requester,
        signature: vc.signature,
    }
}

/// Convert an ed25519_dalek SigningKey to a libp2p identity Keypair.
fn dalek_key_to_libp2p(signing_key: &SigningKey) -> Result<libp2p::identity::Keypair> {
    // ed25519_dalek uses 32-byte secret key; libp2p expects the same.
    let secret_bytes = signing_key.to_bytes();
    let libp2p_keypair = libp2p::identity::Keypair::ed25519_from_bytes(secret_bytes)
        .map_err(|e| anyhow::anyhow!("failed to convert ed25519 key to libp2p keypair: {e}"))?;
    Ok(libp2p_keypair)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // ── 1. Initialize tracing (log to both stderr and file for debugging) ──
    let filter = tracing_subscriber::EnvFilter::try_new(&cli.log_level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // Also write logs to data_dir/node.log for remote debugging via REST
    let log_path = std::path::Path::new(&cli.data_dir).join("node.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .ok();

    if let Some(file) = log_file {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;
        let file_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_writer(std::sync::Mutex::new(file));
        let stderr_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true);
        tracing_subscriber::registry()
            .with(filter)
            .with(stderr_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .init();
    }

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
    let chain_state = Arc::new(RwLock::new(ChainState::new(
        genesis_block.clone(),
        cli.chain_id.clone(),
    )));

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
    let mut node_state = NodeState::new(cli.chain_id.clone());

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
        jsonrpc_bind: format!("{}:{}", cli.rpc_bind, cli.rpc_port),
        rest_bind: format!("{}:{}", cli.rpc_bind, cli.rest_port),
        tls_cert_path: cli.tls_cert.clone(),
        tls_key_path: cli.tls_key.clone(),
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

    // ── 12. Validator / consensus mode ────────────────────────────────────
    let shutdown = tokio::sync::watch::channel(false);
    let (shutdown_tx, shutdown_rx) = (shutdown.0, shutdown.1);

    // Track extra spawned handles for graceful shutdown.
    let mut extra_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    let validator_handle = if cli.validator {
        let validator_key = if let Some(ref key_path) = cli.validator_key {
            load_validator_key(key_path)?
        } else {
            info!("No validator key specified, using node key for validation");
            node_key.clone()
        };

        let validator_address = Address::from_pubkey(&validator_key.verifying_key());

        if !cli.bootstrap.is_empty() {
            // ── Multi-validator TurboBFT mode ──────────────────────────────
            info!(
                address = %validator_address,
                pubkey = %hex::encode(validator_key.verifying_key().to_bytes()),
                bootstrap_peers = cli.bootstrap.len(),
                consensus_timeout_ms = cli.consensus_timeout_ms,
                "Starting multi-validator TurboBFT consensus"
            );

            // Parse bootstrap multiaddrs
            let bootstrap_addrs: Vec<libp2p::Multiaddr> = cli
                .bootstrap
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();

            // Convert ed25519_dalek key to libp2p keypair
            let libp2p_keypair = dalek_key_to_libp2p(&node_key)?;
            let listen_addr: libp2p::Multiaddr = cli.listen.parse()
                .context("failed to parse listen address as multiaddr")?;

            // Create the P2P node
            let (dina_node, mut node_handle) =
                DinaNode::new(libp2p_keypair, listen_addr, bootstrap_addrs)?;

            // Take the event receiver from the handle
            let node_event_rx = node_handle.take_event_rx()
                .expect("event_rx should be available");
            let node_command_handle = node_handle.command_handle();

            // Spawn the P2P node event loop
            let p2p_handle = tokio::spawn(async move {
                if let Err(e) = dina_node.start().await {
                    error!(err = %e, "P2P node event loop exited with error");
                }
            });
            extra_handles.push(p2p_handle);

            // Create consensus channels
            let (consensus_output_tx, mut consensus_output_rx) =
                mpsc::unbounded_channel::<ConsensusOutput>();
            let (consensus_inbound_tx, consensus_inbound_rx) =
                mpsc::unbounded_channel::<InboundMessage>();
            let (tx_feed_tx, tx_feed_rx) =
                mpsc::unbounded_channel::<Vec<dina_core::Transaction>>();

            // Build TurboBFT consensus engine
            // Include our own pubkey in the validator set (in case genesis doesn't have it)
            let mut validator_keys = genesis_config.validators.clone();
            let my_pubkey = validator_key.verifying_key().to_bytes();
            if !validator_keys.contains(&my_pubkey) {
                validator_keys.push(my_pubkey);
                info!("Added own pubkey to validator set (not in genesis)");
            }
            let consensus_config = ConsensusConfig {
                validator_keys,
                block_time_ms: cli.block_time_ms,
                timeout_ms: cli.consensus_timeout_ms,
            };
            let mut turbobft =
                TurboBFT::new(consensus_config, validator_key.clone(), consensus_output_tx);

            // Set the latest committed block (genesis) so proposals have a parent
            {
                let cs = chain_state.read().await;
                turbobft.set_latest_committed_block(cs.latest_block().clone());
            }

            // Create a channel for RPC → mempool transaction submission
            let (rpc_tx_sender, mut rpc_tx_receiver) =
                mpsc::unbounded_channel::<dina_core::Transaction>();
            node_state.consensus_tx_sender = Some(rpc_tx_sender);

            // Spawn: RPC → mempool bridge
            let mempool_rpc = mempool.clone();
            let rpc_bridge_handle = tokio::spawn(async move {
                while let Some(tx) = rpc_tx_receiver.recv().await {
                    let mut pool = mempool_rpc.write().await;
                    let _ = pool.add_transaction(tx);
                }
            });
            extra_handles.push(rpc_bridge_handle);

            // Spawn: feed transactions from mempool to consensus
            let mempool_feed = mempool.clone();
            let feed_handle = tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(tokio::time::Duration::from_millis(200));
                loop {
                    interval.tick().await;
                    let pending = {
                        let pool = mempool_feed.read().await;
                        pool.get_pending(500)
                    };
                    // Always send (even empty) so the leader can propose empty blocks
                    if tx_feed_tx.send(pending).is_err() {
                        break;
                    }
                }
            });
            extra_handles.push(feed_handle);

            // Spawn: Network -> Consensus bridge (also updates RPC peer count)
            let inbound_tx = consensus_inbound_tx;
            let mempool_net = mempool.clone();
            let node_state_net = node_state.clone();
            let node_cmd_peer_query = node_handle.command_handle();
            let net_to_consensus_handle = tokio::spawn(async move {
                let mut event_rx = node_event_rx;
                while let Some(event) = event_rx.recv().await {
                    match event {
                        NodeEvent::PeerConnected(peer_id) => {
                            // Query actual peer count from P2P layer
                            if let Ok(count) = node_cmd_peer_query.peer_count().await {
                                let mut pc = node_state_net.peer_count.write().await;
                                *pc = count as u32;
                            }
                            info!(%peer_id, "P2P peer connected (RPC peer_count updated)");
                        }
                        NodeEvent::PeerDisconnected(peer_id) => {
                            if let Ok(count) = node_cmd_peer_query.peer_count().await {
                                let mut pc = node_state_net.peer_count.write().await;
                                *pc = count as u32;
                            }
                            info!(%peer_id, "P2P peer disconnected (RPC peer_count updated)");
                        }
                        NodeEvent::MessageReceived { message, .. } => {
                            match message {
                                NetworkMessage::Proposal(p) => {
                                    info!(
                                        height = p.height,
                                        round = p.round,
                                        proposer = hex::encode(&p.proposer[..8]),
                                        "received proposal from network"
                                    );
                                    if let Some(cp) = network_proposal_to_consensus(&p) {
                                        let _ = inbound_tx.send(InboundMessage::Proposal(cp));
                                    } else {
                                        warn!(
                                            height = p.height,
                                            round = p.round,
                                            block_data_len = p.block.data.len(),
                                            "dropped proposal: failed to deserialize block"
                                        );
                                    }
                                }
                                NetworkMessage::Vote(v) => {
                                    debug!(
                                        height = v.height,
                                        round = v.round,
                                        voter = hex::encode(&v.voter[..8]),
                                        vote_type = ?v.vote_type,
                                        "received vote from network"
                                    );
                                    let cv = network_vote_to_consensus(&v);
                                    let _ = inbound_tx.send(InboundMessage::Vote(cv));
                                }
                                NetworkMessage::ViewChange(vc) => {
                                    info!(
                                        height = vc.height,
                                        old_round = vc.old_round,
                                        new_round = vc.new_round,
                                        requester = hex::encode(&vc.requester[..8]),
                                        "received view change from network"
                                    );
                                    let cvc = network_vc_to_consensus(&vc);
                                    let _ = inbound_tx.send(InboundMessage::ViewChange(cvc));
                                }
                                NetworkMessage::Transaction(tx_payload) => {
                                    // Deserialize and add to mempool
                                    if let Ok(tx) = bincode::deserialize::<
                                        dina_core::Transaction,
                                    >(&tx_payload.data)
                                    {
                                        let mut pool = mempool_net.write().await;
                                        let _ = pool.add_transaction(tx);
                                    }
                                }
                                _ => {} // Block, SyncRequest, SyncResponse handled elsewhere
                            }
                        }
                    }
                }
            });
            extra_handles.push(net_to_consensus_handle);

            // Spawn: Consensus -> Network bridge + block application
            let chain_state_con = chain_state.clone();
            let node_state_con = node_state.clone();
            let db_con = db.clone();
            let mempool_con = mempool.clone();
            let consensus_to_net_handle = tokio::spawn(async move {
                while let Some(output) = consensus_output_rx.recv().await {
                    match output {
                        ConsensusOutput::BroadcastProposal(p) => {
                            info!(
                                height = p.height,
                                round = p.round,
                                txs = p.block.transactions.len(),
                                "broadcasting proposal to network"
                            );
                            let net_proposal = consensus_proposal_to_network(&p);
                            let msg = NetworkMessage::Proposal(net_proposal);
                            if let Err(e) = node_command_handle.broadcast_consensus(msg).await {
                                error!(height = p.height, err = %e, "failed to broadcast proposal");
                            }
                        }
                        ConsensusOutput::BroadcastVote(v) => {
                            debug!(
                                height = v.height,
                                round = v.round,
                                vote_type = ?v.vote_type,
                                "broadcasting vote to network"
                            );
                            let net_vote = consensus_vote_to_network(&v);
                            let msg = NetworkMessage::Vote(net_vote);
                            if let Err(e) = node_command_handle.broadcast_consensus(msg).await {
                                error!(height = v.height, err = %e, "failed to broadcast vote");
                            }
                        }
                        ConsensusOutput::BroadcastViewChange(vc) => {
                            info!(
                                height = vc.height,
                                old_round = vc.old_round,
                                new_round = vc.new_round,
                                "broadcasting view change to network"
                            );
                            let net_vc = consensus_vc_to_network(&vc);
                            let msg = NetworkMessage::ViewChange(net_vc);
                            if let Err(e) = node_command_handle.broadcast_consensus(msg).await {
                                error!(height = vc.height, err = %e, "failed to broadcast view change");
                            }
                        }
                        ConsensusOutput::BlockCommitted { block, certificate } => {
                            let block_height = block.header.block_number;
                            let tx_count = block.transactions.len();

                            // Execute the block
                            let execution_result = {
                                let cs = chain_state_con.read().await;
                                let mut executor = BlockExecutor::new(cs.accounts.clone());
                                executor.execute_block(&block)
                            };

                            let _exec_result = match execution_result {
                                Ok(r) => r,
                                Err(e) => {
                                    error!(
                                        height = block_height,
                                        err = %e,
                                        "Consensus block execution failed"
                                    );
                                    continue;
                                }
                            };

                            // Apply to chain state.
                            // For consensus-committed blocks (3/4+ validator signatures),
                            // we force-apply even if the chain state is behind, since the
                            // consensus guarantees validity.
                            {
                                let mut cs = chain_state_con.write().await;
                                match cs.apply_block(block.clone()) {
                                    Ok(result) => {
                                        info!(
                                            height = block_height,
                                            hash = %block.hash(),
                                            txs = tx_count,
                                            successful = result.successful_txs,
                                            failed = result.failed_txs,
                                            fees = result.total_fees,
                                            cert_votes = certificate.votes.len(),
                                            "Block committed via TurboBFT consensus"
                                        );
                                    }
                                    Err(e) => {
                                        // Chain state may be behind — force-apply by
                                        // fast-forwarding the chain height and accounts.
                                        warn!(
                                            height = block_height,
                                            chain_height = cs.chain.current_height(),
                                            err = %e,
                                            "Chain state behind consensus — force-applying block"
                                        );
                                        cs.force_apply_block(block.clone());
                                        info!(
                                            height = block_height,
                                            cert_votes = certificate.votes.len(),
                                            "Force-applied consensus block"
                                        );
                                    }
                                }
                            }

                            // Persist block
                            if let Err(e) = db_con.store_block(&block) {
                                error!(
                                    height = block_height,
                                    err = %e,
                                    "Failed to store consensus block"
                                );
                            }

                            // Persist accounts
                            {
                                let cs = chain_state_con.read().await;
                                for (addr, acct) in cs.accounts.iter() {
                                    if let Err(e) = db_con.set_account(*addr, acct) {
                                        error!(
                                            address = %addr, err = %e,
                                            "Failed to persist account"
                                        );
                                    }
                                }
                            }

                            // Remove committed txs from mempool
                            {
                                let tx_hashes: Vec<_> =
                                    block.transactions.iter().map(|tx| tx.hash()).collect();
                                if !tx_hashes.is_empty() {
                                    let mut pool = mempool_con.write().await;
                                    pool.remove_batch(&tx_hashes);
                                }
                            }

                            // Update NodeState for RPC
                            {
                                let mut blocks = node_state_con.blocks.write().await;
                                let mut idx = node_state_con.block_index.write().await;
                                let pos = blocks.len();
                                idx.insert(block.hash(), pos);
                                blocks.push(block.clone());
                            }
                            node_state_con.prune_old_blocks().await;
                            {
                                let cs = chain_state_con.read().await;
                                let mut accounts = node_state_con.accounts.write().await;
                                *accounts = cs.accounts.clone();
                            }
                            {
                                let block_ts = block.header.timestamp;
                                let mut tx_idx = node_state_con.tx_index.write().await;
                                for tx in &block.transactions {
                                    let hash = tx.hash();
                                    tx_idx.insert(hash, (tx.clone(), Some(block_height), Some(block_ts)));
                                    // Notify any waiting /v1/transaction/confirm handlers
                                    let _ = node_state_con.tx_confirmed_sender.send((hash, block_height));
                                }
                            }
                        }
                    }
                }
            });
            extra_handles.push(consensus_to_net_handle);

            // Spawn the TurboBFT consensus loop itself.
            // Wait for P2P peers to connect before starting consensus so that
            // the first proposal isn't rejected with InsufficientPeers.
            let peer_wait_state = node_state.clone();
            Some(tokio::spawn(async move {
                info!("Waiting for P2P peers before starting consensus...");
                loop {
                    let pc = *peer_wait_state.peer_count.read().await;
                    if pc > 0 {
                        info!(peers = pc, "Peers connected — starting TurboBFT consensus");
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                // Give GossipSub mesh time to form after peers connect
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                turbobft.start(tx_feed_rx, consensus_inbound_rx).await;
            }))
        } else {
            // ── Single-validator block production loop (existing) ───────────
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

                    // Apply the block to ChainState
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
                        let mut blocks = node_state_prod.blocks.write().await;
                        let mut idx = node_state_prod.block_index.write().await;
                        let pos = blocks.len();
                        idx.insert(block.hash(), pos);
                        blocks.push(block.clone());
                    }
                    node_state_prod.prune_old_blocks().await;
                    {
                        let cs = chain_state_prod.read().await;
                        let mut accounts = node_state_prod.accounts.write().await;
                        *accounts = cs.accounts.clone();
                    }
                    {
                        let block_ts = block.header.timestamp;
                        let mut tx_idx = node_state_prod.tx_index.write().await;
                        for tx in &block.transactions {
                            tx_idx.insert(tx.hash(), (tx.clone(), Some(block_height), Some(block_ts)));
                        }
                    }
                }
            }))
        }
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

    // ── 15. Wait for shutdown signal (SIGINT or SIGTERM) ─────────────────
    {
        let ctrl_c = signal::ctrl_c();

        #[cfg(unix)]
        {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .context("failed to register SIGTERM handler")?;
            tokio::select! {
                _ = ctrl_c => {
                    info!("Received SIGINT, shutting down gracefully...");
                }
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, shutting down gracefully...");
                }
            }
        }

        #[cfg(not(unix))]
        {
            match ctrl_c.await {
                Ok(()) => {
                    info!("Received shutdown signal, shutting down gracefully...");
                }
                Err(e) => {
                    error!("Failed to listen for shutdown signal: {e}");
                }
            }
        }
    }

    // ── 16. Graceful shutdown ───────────────────────────────────────────
    // Signal the block production loop to stop
    let _ = shutdown_tx.send(true);

    info!("Stopping RPC servers...");
    jsonrpc_handle
        .stop()
        .map_err(|e| anyhow::anyhow!("failed to stop JSON-RPC: {e:?}"))?;

    // Give tasks a brief window to finish current work before aborting.
    let graceful_timeout = tokio::time::Duration::from_secs(5);
    let _ = tokio::time::timeout(graceful_timeout, async {
        // Wait for the bridge task to notice shutdown and exit naturally.
        // If it doesn't, the timeout will trigger and we'll abort below.
        tokio::task::yield_now().await;
    })
    .await;

    rest_handle.abort();
    bridge_handle.abort();
    maintenance_handle.abort();
    status_handle.abort();

    if let Some(handle) = validator_handle {
        handle.abort();
    }

    // Abort any extra handles (P2P node, consensus bridges, etc.)
    for handle in extra_handles {
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
