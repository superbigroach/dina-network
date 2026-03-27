use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use jsonrpsee::core::async_trait;
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::ServerBuilder;
use jsonrpsee::types::ErrorObjectOwned;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::info;

use dina_core::block::Block;
use dina_core::device::{DeviceIdentity, DeviceType};
use dina_core::transaction::Transaction;
use dina_core::types::{Address, Hash};
use dina_core::account::AccountState;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Account information returned by the RPC.
#[derive(Clone, Debug, Serialize)]
pub struct AccountInfo {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
    pub has_code: bool,
}

/// Block information returned by the RPC.
#[derive(Clone, Debug, Serialize)]
pub struct BlockInfo {
    pub hash: String,
    pub block_number: u64,
    pub parent_hash: String,
    pub state_root: String,
    pub transactions_root: String,
    pub timestamp: u64,
    pub proposer: String,
    pub transaction_count: usize,
    pub transactions: Vec<String>,
}

/// Transaction information returned by the RPC.
#[derive(Clone, Debug, Serialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub sender: String,
    pub nonce: u64,
    pub fee: u64,
    pub tx_type: String,
    pub block_number: Option<u64>,
}

/// Device information returned by the RPC.
#[derive(Clone, Debug, Serialize)]
pub struct DeviceInfo {
    pub address: String,
    pub name: String,
    pub device_type: String,
    pub owner: String,
    pub active: bool,
    pub registered_at: u64,
}

/// Network information returned by the RPC.
#[derive(Clone, Debug, Serialize)]
pub struct NetworkInfo {
    pub chain_id: String,
    pub block_height: u64,
    pub peer_count: u32,
    pub version: String,
    pub protocol_version: u32,
}

// ---------------------------------------------------------------------------
// JSON-RPC trait definition
// ---------------------------------------------------------------------------

#[rpc(server)]
pub trait DinaRpc {
    /// Submit a signed transaction (hex-encoded) to the network.
    #[method(name = "dina_sendTransaction")]
    async fn send_transaction(&self, tx_hex: String) -> RpcResult<String>;

    /// Get the balance of an address.
    #[method(name = "dina_getBalance")]
    async fn get_balance(&self, address: String) -> RpcResult<u64>;

    /// Get full account information for an address.
    #[method(name = "dina_getAccount")]
    async fn get_account(&self, address: String) -> RpcResult<AccountInfo>;

    /// Get a block by its height.
    #[method(name = "dina_getBlock")]
    async fn get_block(&self, height: u64) -> RpcResult<BlockInfo>;

    /// Get a block by its hash.
    #[method(name = "dina_getBlockByHash")]
    async fn get_block_by_hash(&self, hash: String) -> RpcResult<BlockInfo>;

    /// Get the latest block.
    #[method(name = "dina_getLatestBlock")]
    async fn get_latest_block(&self) -> RpcResult<BlockInfo>;

    /// Get a transaction by its hash.
    #[method(name = "dina_getTransaction")]
    async fn get_transaction(&self, hash: String) -> RpcResult<TransactionInfo>;

    /// Get a device by its public key (hex-encoded).
    #[method(name = "dina_getDevice")]
    async fn get_device(&self, pubkey: String) -> RpcResult<DeviceInfo>;

    /// Get network information.
    #[method(name = "dina_networkInfo")]
    async fn network_info(&self) -> RpcResult<NetworkInfo>;

    /// Get the chain ID.
    #[method(name = "dina_chainId")]
    async fn chain_id(&self) -> RpcResult<String>;
}

// ---------------------------------------------------------------------------
// Shared node state
// ---------------------------------------------------------------------------

/// Shared state backing the RPC server. Holds the chain, accounts, mempool,
/// and device registry. All fields are behind `Arc<RwLock<>>` so they can be
/// read and written from multiple async tasks concurrently.
#[derive(Clone)]
pub struct NodeState {
    pub accounts: Arc<RwLock<AccountState>>,
    pub blocks: Arc<RwLock<Vec<Block>>>,
    pub block_index: Arc<RwLock<HashMap<Hash, usize>>>,
    pub tx_pool: Arc<RwLock<Vec<Transaction>>>,
    pub tx_index: Arc<RwLock<HashMap<Hash, (Transaction, Option<u64>)>>>,
    pub devices: Arc<RwLock<HashMap<String, DeviceIdentity>>>,
    pub peer_count: Arc<RwLock<u32>>,
    pub chain_id: String,
}

impl NodeState {
    /// Create a new `NodeState` with the provided chain ID and a genesis block.
    pub fn new(chain_id: String) -> Self {
        let genesis = Block::genesis(Address::ZERO, 0);
        let genesis_hash = genesis.hash();

        let mut block_index = HashMap::new();
        block_index.insert(genesis_hash, 0);

        Self {
            accounts: Arc::new(RwLock::new(AccountState::new())),
            blocks: Arc::new(RwLock::new(vec![genesis])),
            block_index: Arc::new(RwLock::new(block_index)),
            tx_pool: Arc::new(RwLock::new(Vec::new())),
            tx_index: Arc::new(RwLock::new(HashMap::new())),
            devices: Arc::new(RwLock::new(HashMap::new())),
            peer_count: Arc::new(RwLock::new(0)),
            chain_id,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: convert a Block into a BlockInfo
// ---------------------------------------------------------------------------

pub fn block_to_info(block: &Block) -> BlockInfo {
    let tx_hashes: Vec<String> = block
        .transactions
        .iter()
        .map(|tx| tx.hash().to_string())
        .collect();

    BlockInfo {
        hash: block.hash().to_string(),
        block_number: block.header.block_number,
        parent_hash: block.header.parent_hash.to_string(),
        state_root: block.header.state_root.to_string(),
        transactions_root: block.header.transactions_root.to_string(),
        timestamp: block.header.timestamp,
        proposer: block.header.proposer.to_string(),
        transaction_count: block.transactions.len(),
        transactions: tx_hashes,
    }
}

// ---------------------------------------------------------------------------
// Helper: build a JSON-RPC error
// ---------------------------------------------------------------------------

fn rpc_err(code: i32, msg: impl Into<String>) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(code, msg.into(), None::<()>)
}

const ERR_INVALID_PARAMS: i32 = -32602;
const ERR_NOT_FOUND: i32 = -32001;
const ERR_INTERNAL: i32 = -32603;

// ---------------------------------------------------------------------------
// RPC server implementation
// ---------------------------------------------------------------------------

/// The concrete implementation of the `DinaRpc` trait.
pub struct DinaRpcServerImpl {
    pub state: NodeState,
}

impl DinaRpcServerImpl {
    pub fn new(state: NodeState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl DinaRpcServer for DinaRpcServerImpl {
    async fn send_transaction(&self, tx_hex: String) -> RpcResult<String> {
        let raw = tx_hex.strip_prefix("0x").unwrap_or(&tx_hex);
        let bytes = hex::decode(raw)
            .map_err(|e| rpc_err(ERR_INVALID_PARAMS, format!("invalid hex: {e}")))?;

        let tx: Transaction = serde_json::from_slice(&bytes)
            .map_err(|e| rpc_err(ERR_INVALID_PARAMS, format!("invalid transaction: {e}")))?;

        let tx_hash = tx.hash();

        // Index the transaction (not yet in a block).
        {
            let mut idx = self.state.tx_index.write().await;
            idx.insert(tx_hash, (tx.clone(), None));
        }

        // Add to the mempool.
        {
            let mut pool = self.state.tx_pool.write().await;
            pool.push(tx);
        }

        info!(%tx_hash, "transaction submitted to mempool");
        Ok(tx_hash.to_string())
    }

    async fn get_balance(&self, address: String) -> RpcResult<u64> {
        let addr = Address::from_str(&address)
            .map_err(|e| rpc_err(ERR_INVALID_PARAMS, format!("invalid address: {e}")))?;

        let accounts = self.state.accounts.read().await;
        let balance = accounts
            .get_account(&addr)
            .map(|a| a.balance)
            .unwrap_or(0);

        Ok(balance)
    }

    async fn get_account(&self, address: String) -> RpcResult<AccountInfo> {
        let addr = Address::from_str(&address)
            .map_err(|e| rpc_err(ERR_INVALID_PARAMS, format!("invalid address: {e}")))?;

        let accounts = self.state.accounts.read().await;
        let account = accounts
            .get_account(&addr)
            .ok_or_else(|| rpc_err(ERR_NOT_FOUND, "account not found"))?;

        Ok(AccountInfo {
            address: account.address.to_string(),
            balance: account.balance,
            nonce: account.nonce,
            has_code: account.code_hash.is_some(),
        })
    }

    async fn get_block(&self, height: u64) -> RpcResult<BlockInfo> {
        let blocks = self.state.blocks.read().await;
        let block = blocks
            .get(height as usize)
            .ok_or_else(|| rpc_err(ERR_NOT_FOUND, format!("block {height} not found")))?;

        Ok(block_to_info(block))
    }

    async fn get_block_by_hash(&self, hash: String) -> RpcResult<BlockInfo> {
        let target = Hash::from_str(&hash)
            .map_err(|e| rpc_err(ERR_INVALID_PARAMS, format!("invalid hash: {e}")))?;

        let idx = self.state.block_index.read().await;
        let position = idx
            .get(&target)
            .ok_or_else(|| rpc_err(ERR_NOT_FOUND, "block not found"))?;

        let blocks = self.state.blocks.read().await;
        let block = blocks
            .get(*position)
            .ok_or_else(|| rpc_err(ERR_INTERNAL, "block index inconsistency"))?;

        Ok(block_to_info(block))
    }

    async fn get_latest_block(&self) -> RpcResult<BlockInfo> {
        let blocks = self.state.blocks.read().await;
        let block = blocks
            .last()
            .ok_or_else(|| rpc_err(ERR_INTERNAL, "no blocks in chain"))?;

        Ok(block_to_info(block))
    }

    async fn get_transaction(&self, hash: String) -> RpcResult<TransactionInfo> {
        let target = Hash::from_str(&hash)
            .map_err(|e| rpc_err(ERR_INVALID_PARAMS, format!("invalid hash: {e}")))?;

        let idx = self.state.tx_index.read().await;
        let (tx, block_num) = idx
            .get(&target)
            .ok_or_else(|| rpc_err(ERR_NOT_FOUND, "transaction not found"))?;

        let tx_type = match tx {
            Transaction::Transfer { .. } => "Transfer",
            Transaction::DeployContract { .. } => "DeployContract",
            Transaction::CallContract { .. } => "CallContract",
            Transaction::RegisterDevice { .. } => "RegisterDevice",
        };

        Ok(TransactionInfo {
            hash: tx.hash().to_string(),
            sender: tx.sender().to_string(),
            nonce: tx.nonce(),
            fee: tx.fee(),
            tx_type: tx_type.to_string(),
            block_number: *block_num,
        })
    }

    async fn get_device(&self, pubkey: String) -> RpcResult<DeviceInfo> {
        let key = pubkey.strip_prefix("0x").unwrap_or(&pubkey).to_lowercase();
        let devices = self.state.devices.read().await;
        let device = devices
            .get(&key)
            .ok_or_else(|| rpc_err(ERR_NOT_FOUND, "device not found"))?;

        let dtype = device.device_type.to_string();

        let name = device.metadata.name.clone().unwrap_or_default();

        Ok(DeviceInfo {
            address: device.id.to_string(),
            name,
            device_type: dtype,
            owner: device.owner.to_string(),
            active: device.active,
            registered_at: device.registered_at,
        })
    }

    async fn network_info(&self) -> RpcResult<NetworkInfo> {
        let blocks = self.state.blocks.read().await;
        let height = blocks.len().saturating_sub(1) as u64;
        let peers = *self.state.peer_count.read().await;

        Ok(NetworkInfo {
            chain_id: self.state.chain_id.clone(),
            block_height: height,
            peer_count: peers,
            version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: 1,
        })
    }

    async fn chain_id(&self) -> RpcResult<String> {
        Ok(self.state.chain_id.clone())
    }
}

/// Start the JSON-RPC server on the given address and return the server handle.
pub async fn start_jsonrpc_server(
    state: NodeState,
    bind_addr: &str,
) -> Result<jsonrpsee::server::ServerHandle, Box<dyn std::error::Error + Send + Sync>> {
    let server = ServerBuilder::default()
        .build(bind_addr)
        .await?;

    let rpc_impl = DinaRpcServerImpl::new(state);
    let handle = server.start(rpc_impl.into_rpc());

    info!("JSON-RPC server listening on {bind_addr}");
    Ok(handle)
}
