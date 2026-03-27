use std::collections::HashMap;
use std::str::FromStr;
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

use crate::jsonrpc::{block_to_info, DeviceInfo, NodeState};

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

/// Application state shared across all REST handlers.
///
/// Wraps `NodeState` together with faucet rate-limit tracking.
pub struct RestAppState {
    pub node: NodeState,
    /// Rate limiter: maps address bytes to the last faucet request time.
    pub faucet_rate_limit: Mutex<HashMap<Address, Instant>>,
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
    let balance = accounts
        .get_account(&addr)
        .map(|a| a.balance)
        .unwrap_or(0);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "address": address,
            "balance": balance,
        })),
    )
}

async fn get_block_handler(
    State(state): State<AppState>,
    Path(height): Path<u64>,
) -> impl IntoResponse {
    let blocks = state.node.blocks.read().await;
    match blocks.get(height as usize) {
        Some(block) => (StatusCode::OK, Json(serde_json::to_value(block_to_info(block)).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("block {height} not found") })),
        ),
    }
}

async fn get_latest_block_handler(State(state): State<AppState>) -> impl IntoResponse {
    let blocks = state.node.blocks.read().await;
    match blocks.last() {
        Some(block) => (StatusCode::OK, Json(serde_json::to_value(block_to_info(block)).unwrap())),
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

    // Index and add to mempool.
    {
        let mut idx = state.node.tx_index.write().await;
        idx.insert(tx_hash, (tx.clone(), None));
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
const FAUCET_AMOUNT: u64 = 1_000_000_000; // 1,000 USDC in micro-USDC
const FAUCET_COOLDOWN_SECS: u64 = 600; // 10 minutes

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

    // Rate limiting: max 1 request per address per 10 minutes.
    {
        let mut rate_map = state.faucet_rate_limit.lock().await;
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

    // Credit the account directly (testnet only).
    {
        let mut accounts = state.node.accounts.write().await;
        accounts.credit(&addr, FAUCET_AMOUNT);
    }

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
// Router construction
// ---------------------------------------------------------------------------

/// Build the REST API router with all routes and shared state.
pub fn rest_router(state: NodeState) -> Router {
    let shared = Arc::new(RestAppState {
        node: state,
        faucet_rate_limit: Mutex::new(HashMap::new()),
    });

    Router::new()
        .route("/health", get(health_handler))
        .route("/v1/balance/{address}", get(get_balance_handler))
        .route("/v1/block/latest", get(get_latest_block_handler))
        .route("/v1/block/{height}", get(get_block_handler))
        .route("/v1/transaction", post(submit_transaction_handler))
        .route("/v1/device/{pubkey}", get(get_device_handler))
        .route("/v1/peers", get(get_peers_handler))
        .route("/faucet/{address}", post(faucet_handler))
        .with_state(shared)
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
}
