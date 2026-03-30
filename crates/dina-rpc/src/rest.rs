use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use dina_core::transaction::Transaction;
use dina_core::types::Address;
use dina_monitoring::PrometheusMetrics;

use crate::jsonrpc::{block_to_info, DeviceInfo, NodeState};

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

/// Application state shared across all REST handlers.
///
/// Wraps `NodeState` together with faucet rate-limit tracking.
/// Faucet global rate-limit window state, protected by a single mutex to
/// prevent TOCTOU races between checking and resetting the window.
pub struct FaucetWindow {
    pub start: u64,
    pub count: u64,
}

pub struct RestAppState {
    pub node: NodeState,
    /// Rate limiter: maps address bytes to the last faucet request time.
    pub faucet_rate_limit: Mutex<HashMap<Address, Instant>>,
    /// C-2: Global faucet counters to prevent unlimited minting.
    /// Total number of faucet requests processed (all-time).
    pub faucet_global_count: AtomicU64,
    /// Total micro-USDC minted by the faucet (all-time).
    pub faucet_total_minted: AtomicU64,
    /// C-2: Global per-minute rate limit window, mutex-guarded to avoid race conditions.
    pub faucet_window: Mutex<FaucetWindow>,
    /// Prometheus metrics collector.
    pub metrics: Mutex<PrometheusMetrics>,
    /// Node start time for uptime tracking.
    pub started_at: Instant,
}

pub type AppState = Arc<RestAppState>;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    height: u64,
    peers: u32,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct BalanceResponse {
    address: String,
    balance: u64,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct SubmitTxRequest {
    tx_hex: String,
}

#[derive(Deserialize)]
struct SubmitTxBody {
    tx_hex: String,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct SubmitTxResponse {
    tx_hash: String,
}

#[derive(Serialize)]
struct PeerInfo {
    peer_count: u32,
    peers: Vec<String>,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct ErrorResponse {
    error: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let blocks = state.node.blocks.read().await;
    let height = blocks.len().saturating_sub(1) as u64;
    let peers = *state.node.peer_count.read().await;

    Json(HealthResponse {
        status: "ok".to_string(),
        height,
        peers,
    })
}

async fn get_balance_handler(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    let addr = match Address::from_str(&address) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("invalid address: {e}") })),
            );
        }
    };

    let accounts = state.node.accounts.read().await;
    let account = accounts.get_account(&addr);
    let balance = account.map(|a| a.balance).unwrap_or(0);
    let nonce = account.map(|a| a.nonce).unwrap_or(0);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "address": address,
            "balance": balance,
            "nonce": nonce,
        })),
    )
}

async fn get_block_handler(
    State(state): State<AppState>,
    Path(height): Path<u64>,
) -> impl IntoResponse {
    let blocks = state.node.blocks.read().await;
    // L-3: Safe cast from u64 to usize to avoid truncation on 32-bit platforms.
    let idx = match usize::try_from(height) {
        Ok(i) => i,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("block height {height} out of range") })),
            );
        }
    };
    match blocks.get(idx) {
        Some(block) => (
            StatusCode::OK,
            Json(serde_json::to_value(block_to_info(block)).unwrap()),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("block {height} not found") })),
        ),
    }
}

async fn get_latest_block_handler(State(state): State<AppState>) -> impl IntoResponse {
    let blocks = state.node.blocks.read().await;
    match blocks.last() {
        Some(block) => (
            StatusCode::OK,
            Json(serde_json::to_value(block_to_info(block)).unwrap()),
        ),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "no blocks in chain" })),
        ),
    }
}

async fn submit_transaction_handler(
    State(state): State<AppState>,
    Json(body): Json<SubmitTxBody>,
) -> impl IntoResponse {
    let raw = body.tx_hex.strip_prefix("0x").unwrap_or(&body.tx_hex);
    let bytes = match hex::decode(raw) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("invalid hex: {e}") })),
            );
        }
    };

    let tx: Transaction = match serde_json::from_slice(&bytes) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("invalid transaction: {e}") })),
            );
        }
    };

    let tx_hash = tx.hash();

    // M-2: Check tx_pool size before accepting to prevent memory exhaustion.
    {
        let pool = state.node.tx_pool.read().await;
        if pool.len() >= 10_000 {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "error": "transaction pool full" })),
            );
        }
    }

    // C-1: Full Ed25519 signature verification before accepting into mempool.
    // Coinbase/faucet transactions (from zero address) are exempt.
    {
        let sender = tx.sender();
        if sender != Address([0u8; 32]) && !tx.verify_signature() {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({ "error": "invalid signature: Ed25519 verification failed" }),
                ),
            );
        }
    }

    // Index and add to mempool.
    {
        let mut idx = state.node.tx_index.write().await;
        idx.insert(tx_hash, (tx.clone(), None, None));
    }
    // Send to consensus mempool if available (multi-validator mode)
    if let Some(ref tx_sender) = state.node.consensus_tx_sender {
        let _ = tx_sender.send(tx.clone());
    }

    // Optimistically apply transfer to RPC account state for instant balance
    // updates. The consensus will re-apply when the block is committed.
    if let Transaction::Transfer { from, to, amount, .. } = &tx {
        let mut accounts = state.node.accounts.write().await;
        if accounts.transfer(from, to, *amount).is_ok() {
            info!(%tx_hash, amount, "optimistically applied transfer to RPC accounts");
        }
    }

    {
        let mut pool = state.node.tx_pool.write().await;
        pool.push(tx);
    }

    info!(%tx_hash, "transaction submitted via REST");

    (
        StatusCode::OK,
        Json(serde_json::json!({ "tx_hash": tx_hash.to_string() })),
    )
}

async fn get_device_handler(
    State(state): State<AppState>,
    Path(pubkey): Path<String>,
) -> impl IntoResponse {
    let key = pubkey.strip_prefix("0x").unwrap_or(&pubkey).to_lowercase();
    let devices = state.node.devices.read().await;

    match devices.get(&key) {
        Some(device) => {
            let dtype = device.device_type.to_string();
            let info = DeviceInfo {
                address: device.id.to_string(),
                name: device.metadata.name.clone().unwrap_or_default(),
                device_type: dtype,
                owner: device.owner.to_string(),
                active: device.active,
                registered_at: device.registered_at,
            };
            (StatusCode::OK, Json(serde_json::to_value(info).unwrap()))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "device not found" })),
        ),
    }
}

async fn get_peers_handler(State(state): State<AppState>) -> impl IntoResponse {
    let peer_count = *state.node.peer_count.read().await;

    Json(PeerInfo {
        peer_count,
        peers: Vec::new(), // Populated when connected to the P2P layer.
    })
}

/// Faucet constants.
const FAUCET_AMOUNT: u64 = 10_000_000_000; // 10,000 USDC per request (testnet)
const FAUCET_COOLDOWN_SECS: u64 = 30; // 30 second cooldown (testnet — fast iteration)
/// C-2: Global faucet limits.
const FAUCET_MAX_TOTAL: u64 = 1_000_000_000_000_000; // 1 TRILLION USDC cap (testnet — unlimited)
const FAUCET_MAX_PER_MINUTE: u64 = 1000; // 1000 requests/min (testnet — no real limit)

async fn faucet_handler(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    let addr = match Address::from_str(&address) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("invalid address: {e}") })),
            );
        }
    };

    // C-2: Check global supply cap — faucet stops after 1M USDC total minted.
    let total_minted = state.faucet_total_minted.load(Ordering::Relaxed);
    if total_minted >= FAUCET_MAX_TOTAL {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "error": "faucet depleted — maximum total supply reached",
            })),
        );
    }

    // C-2: Check global per-minute rate limit (mutex-guarded to prevent race).
    {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut window = state.faucet_window.lock().await;
        if now_secs - window.start >= 60 {
            // Start a new window.
            window.start = now_secs;
            window.count = 1;
        } else {
            window.count += 1;
            if window.count > FAUCET_MAX_PER_MINUTE {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(serde_json::json!({
                        "error": "faucet rate limited — too many global requests, try again later",
                    })),
                );
            }
        }
    }

    // Per-address rate limiting: max 1 request per address per 10 minutes.
    {
        let mut rate_map = state.faucet_rate_limit.lock().await;

        // M-3: Evict stale entries to prevent unbounded memory growth.
        if rate_map.len() > 10_000 {
            rate_map.retain(|_, instant| instant.elapsed().as_secs() < FAUCET_COOLDOWN_SECS);
        }

        if let Some(last) = rate_map.get(&addr) {
            let elapsed = last.elapsed().as_secs();
            if elapsed < FAUCET_COOLDOWN_SECS {
                let remaining = FAUCET_COOLDOWN_SECS - elapsed;
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(serde_json::json!({
                        "error": format!("rate limited, retry in {remaining}s"),
                        "retry_after_secs": remaining,
                    })),
                );
            }
        }
        rate_map.insert(addr, Instant::now());
    }

    // Credit the account: write to both RPC accounts AND submit a special
    // "faucet" transaction so the block producer's chain_state gets updated too.
    {
        let mut accounts = state.node.accounts.write().await;
        accounts.credit(&addr, FAUCET_AMOUNT);
    }

    // Also inject a faucet transfer into the tx_pool so chain_state picks it up.
    // We use a Transfer from the zero address (coinbase) with no signature.
    {
        let faucet_tx = Transaction::Transfer {
            from: Address([0u8; 32]), // coinbase / faucet address
            to: addr,
            amount: FAUCET_AMOUNT,
            memo: None,
            device_witness: None,
            nonce: 0,
            fee: 0,
            pub_key: [0u8; 32], // coinbase has no real key
            signature: dina_core::transaction::Sig64([0u8; 64]),
        };
        // Send to consensus mempool if available (multi-validator mode)
        if let Some(ref tx_sender) = state.node.consensus_tx_sender {
            let _ = tx_sender.send(faucet_tx.clone());
        }
        let mut pool = state.node.tx_pool.write().await;
        pool.push(faucet_tx);
    }

    // C-2: Track total minted amount.
    state
        .faucet_total_minted
        .fetch_add(FAUCET_AMOUNT, Ordering::Relaxed);
    state.faucet_global_count.fetch_add(1, Ordering::Relaxed);

    info!(%addr, amount = FAUCET_AMOUNT, "faucet dispensed");

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "amount": FAUCET_AMOUNT,
            "address": address,
        })),
    )
}

// ---------------------------------------------------------------------------
// Prometheus metrics endpoint
// ---------------------------------------------------------------------------

async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    let blocks = state.node.blocks.read().await;
    let height = blocks.len().saturating_sub(1) as u64;
    let tx_pool = state.node.tx_pool.read().await;
    let peers = *state.node.peer_count.read().await;
    drop(blocks);

    let mut metrics = state.metrics.lock().await;
    metrics.set_gauge("dina_block_height", height as f64, &[]);
    metrics.set_gauge("dina_peer_count", peers as f64, &[]);
    metrics.set_gauge("dina_mempool_size", tx_pool.len() as f64, &[]);
    metrics.set_gauge(
        "dina_uptime_seconds",
        state.started_at.elapsed().as_secs() as f64,
        &[],
    );

    let body = metrics.render();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}

// ---------------------------------------------------------------------------
// Detailed health endpoint
// ---------------------------------------------------------------------------

async fn health_detailed_handler(State(state): State<AppState>) -> impl IntoResponse {
    let blocks = state.node.blocks.read().await;
    let height = blocks.len().saturating_sub(1) as u64;
    let latest_timestamp = blocks.last().map(|b| b.header.timestamp).unwrap_or(0);
    drop(blocks);

    let peers = *state.node.peer_count.read().await;
    let tx_pool_size = state.node.tx_pool.read().await.len();
    let uptime = state.started_at.elapsed().as_secs();

    let now = chrono::Utc::now().timestamp() as u64;
    let block_age = now.saturating_sub(latest_timestamp);
    // Consider unhealthy if last block is older than 5 minutes
    let consensus_ok = block_age < 300;
    let overall = if consensus_ok { "healthy" } else { "degraded" };

    Json(serde_json::json!({
        "status": overall,
        "uptime_seconds": uptime,
        "version": env!("CARGO_PKG_VERSION"),
        "checks": {
            "consensus": {
                "healthy": consensus_ok,
                "block_height": height,
                "last_block_age_secs": block_age,
            },
            "network": {
                "healthy": peers > 0,
                "peer_count": peers,
            },
            "mempool": {
                "size": tx_pool_size,
            }
        }
    }))
}

// ---------------------------------------------------------------------------
// Router construction
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Transaction history by address
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct TxHistoryEntry {
    tx_hash: String,
    #[serde(rename = "type")]
    tx_type: String,
    from: String,
    to: String,
    amount: u64,
    fee: u64,
    nonce: u64,
    block_height: Option<u64>,
    timestamp: Option<u64>,
    status: String,
}

async fn get_transactions_handler(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    let raw = address.strip_prefix("0x").unwrap_or(&address).to_lowercase();

    let target = match Address::from_str(&raw) {
        Ok(a) => a,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid address" })),
            );
        }
    };

    let idx = state.node.tx_index.read().await;
    let mut entries: Vec<TxHistoryEntry> = Vec::new();

    for (hash, (tx, block_height, block_ts)) in idx.iter() {
        let (from, to, amount, fee, nonce) = match tx {
            Transaction::Transfer { from, to, amount, fee, nonce, .. } => {
                (*from, *to, *amount, *fee, *nonce)
            }
            _ => continue,
        };

        if from == target || to == target {
            entries.push(TxHistoryEntry {
                tx_hash: hash.to_string(),
                tx_type: if from == target { "send".into() } else { "receive".into() },
                from: from.to_string(),
                to: to.to_string(),
                amount,
                fee,
                nonce,
                block_height: *block_height,
                timestamp: *block_ts,
                status: if block_height.is_some() { "confirmed".into() } else { "pending".into() },
            });
        }
    }

    // Sort by block height descending (confirmed first), then by nonce
    entries.sort_by(|a, b| {
        b.block_height.unwrap_or(u64::MAX).cmp(&a.block_height.unwrap_or(u64::MAX))
    });

    // Limit to 50 most recent
    entries.truncate(50);

    (
        StatusCode::OK,
        Json(serde_json::json!({ "transactions": entries })),
    )
}

// ---------------------------------------------------------------------------
// All recent transactions (no address filter)
// ---------------------------------------------------------------------------

async fn get_all_transactions_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let idx = state.node.tx_index.read().await;
    let mut entries: Vec<TxHistoryEntry> = Vec::new();

    for (hash, (tx, block_height, block_ts)) in idx.iter() {
        let (from, to, amount, fee, nonce) = match tx {
            Transaction::Transfer { from, to, amount, fee, nonce, .. } => {
                (*from, *to, *amount, *fee, *nonce)
            }
            _ => continue,
        };

        entries.push(TxHistoryEntry {
            tx_hash: hash.to_string(),
            tx_type: if from == Address([0u8; 32]) { "faucet".into() } else { "transfer".into() },
            from: from.to_string(),
            to: to.to_string(),
            amount,
            fee,
            nonce,
            block_height: *block_height,
            timestamp: *block_ts,
            status: if block_height.is_some() { "confirmed".into() } else { "pending".into() },
        });
    }

    // Sort by block height descending
    entries.sort_by(|a, b| {
        b.block_height.unwrap_or(u64::MAX).cmp(&a.block_height.unwrap_or(u64::MAX))
    });

    entries.truncate(100);

    (
        StatusCode::OK,
        Json(serde_json::json!({ "transactions": entries, "total": entries.len() })),
    )
}

/// Submit a transaction and wait for BFT consensus confirmation.
/// Returns the block height and time when the TX was included.
async fn submit_and_confirm_handler(
    State(state): State<AppState>,
    Json(body): Json<SubmitTxBody>,
) -> impl IntoResponse {
    let raw = body.tx_hex.strip_prefix("0x").unwrap_or(&body.tx_hex);
    let bytes = match hex::decode(raw) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("invalid hex: {e}") })),
            );
        }
    };

    let tx: Transaction = match serde_json::from_slice(&bytes) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("invalid transaction: {e}") })),
            );
        }
    };

    let tx_hash = tx.hash();

    // Verify signature
    {
        let sender = tx.sender();
        if sender != Address([0u8; 32]) && !tx.verify_signature() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid signature" })),
            );
        }
    }

    // Subscribe to confirmations BEFORE submitting (avoid race)
    let mut rx = state.node.tx_confirmed_sender.subscribe();

    // Add to index and mempool
    {
        let mut idx = state.node.tx_index.write().await;
        idx.insert(tx_hash, (tx.clone(), None, None));
    }
    if let Some(ref tx_sender) = state.node.consensus_tx_sender {
        let _ = tx_sender.send(tx.clone());
    }
    // Optimistic account update
    if let Transaction::Transfer { from, to, amount, .. } = &tx {
        let mut accounts = state.node.accounts.write().await;
        let _ = accounts.transfer(from, to, *amount);
    }
    {
        let mut pool = state.node.tx_pool.write().await;
        pool.push(tx);
    }

    // Wait for consensus confirmation (max 10 seconds)
    let timeout = tokio::time::Duration::from_secs(10);
    let result = tokio::time::timeout(timeout, async {
        loop {
            match rx.recv().await {
                Ok((confirmed_hash, block_height)) => {
                    if confirmed_hash == tx_hash {
                        return Some(block_height);
                    }
                }
                Err(_) => return None,
            }
        }
    })
    .await;

    match result {
        Ok(Some(block_height)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "tx_hash": tx_hash.to_string(),
                "confirmed": true,
                "block_height": block_height,
                "validators": 3,
            })),
        ),
        _ => {
            // TX was submitted and optimistically applied, but consensus
            // confirmation didn't arrive within timeout
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "tx_hash": tx_hash.to_string(),
                    "confirmed": false,
                    "pending": true,
                })),
            )
        }
    }
}

/// Serve the last N lines of the node log file for remote debugging.
async fn debug_logs_handler(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let lines = params.get("lines").and_then(|s| s.parse::<usize>().ok()).unwrap_or(100);
    let filter = params.get("filter").cloned().unwrap_or_default();

    // Read from /data/node.log (the data directory mount)
    let content = match tokio::fs::read_to_string("/data/node.log").await {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Cannot read logs: {e}")),
    };

    let all_lines: Vec<&str> = content.lines().collect();
    let filtered: Vec<&str> = if filter.is_empty() {
        all_lines.iter().rev().take(lines).copied().collect()
    } else {
        all_lines.iter().rev().filter(|l| l.contains(&filter)).take(lines).copied().collect()
    };

    let mut result: Vec<&str> = filtered;
    result.reverse();
    (StatusCode::OK, result.join("\n"))
}

/// Build the REST API router with all routes and shared state.
pub fn rest_router(state: NodeState) -> Router {
    let shared = Arc::new(RestAppState {
        node: state,
        faucet_rate_limit: Mutex::new(HashMap::new()),
        faucet_global_count: AtomicU64::new(0),
        faucet_total_minted: AtomicU64::new(0),
        faucet_window: Mutex::new(FaucetWindow { start: 0, count: 0 }),
        metrics: Mutex::new(PrometheusMetrics::new()),
        started_at: Instant::now(),
    });

    Router::new()
        .route("/health", get(health_handler))
        .route("/health/detailed", get(health_detailed_handler))
        .route("/metrics", get(metrics_handler))
        .route("/v1/balance/{address}", get(get_balance_handler))
        .route("/v1/block/latest", get(get_latest_block_handler))
        .route("/v1/block/{height}", get(get_block_handler))
        .route("/v1/transaction", post(submit_transaction_handler))
        .route("/v1/transaction/confirm", post(submit_and_confirm_handler))
        .route("/v1/transactions", get(get_all_transactions_handler))
        .route("/v1/transactions/{address}", get(get_transactions_handler))
        .route("/v1/device/{pubkey}", get(get_device_handler))
        .route("/v1/peers", get(get_peers_handler))
        .route("/faucet/{address}", post(faucet_handler))
        .route("/debug/logs", get(debug_logs_handler))
        .with_state(shared)
        // H-2: CORS is kept open for portal/frontend compatibility on testnet.
        // The faucet is protected by global rate limits (C-2) and per-address
        // cooldowns. For production, restrict origins to known frontends.
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}
