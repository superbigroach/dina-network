use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use tokio::sync::RwLock;
use tracing::warn;

use dina_core::types::{Address, Hash};
use dina_storage::DinaDB;

use crate::indexer::ExplorerIndexer;
use crate::models::*;

/// Shared application state for all API handlers.
#[derive(Clone)]
pub struct AppState {
    pub indexer: Arc<RwLock<ExplorerIndexer>>,
    pub db: Arc<DinaDB>,
}

/// Build the axum router with all explorer API routes.
pub fn explorer_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/blocks", get(list_blocks))
        .route("/api/blocks/{height}", get(get_block_by_height))
        .route("/api/blocks/hash/{hash}", get(get_block_by_hash))
        .route("/api/transactions/{hash}", get(get_transaction))
        .route("/api/accounts/{address}", get(get_account))
        .route(
            "/api/accounts/{address}/transactions",
            get(get_account_transactions),
        )
        .route("/api/search", get(search))
        .route("/api/stats", get(get_stats))
        .route("/api/validators", get(list_validators))
        .route("/api/devices", get(list_devices))
        .route("/api/devices/{id}", get(get_device))
        .with_state(state)
}

/// Helper to produce a JSON error response with a given status code.
fn error_response(status: StatusCode, message: &str) -> impl IntoResponse {
    (
        status,
        Json(ApiErrorBody {
            error: message.to_string(),
            code: status.as_u16(),
        }),
    )
}

/// Convert a `dina_core::Block` into a `BlockResponse`.
fn block_to_response(block: &dina_core::Block) -> BlockResponse {
    BlockResponse {
        height: block.header.block_number,
        hash: block.hash().to_string(),
        parent_hash: block.header.parent_hash.to_string(),
        timestamp: block.header.timestamp,
        proposer: block.header.proposer.to_string(),
        tx_count: block.transactions.len(),
        state_root: block.header.state_root.to_string(),
    }
}

/// Convert a `dina_core::Transaction` into a `TransactionResponse`.
fn tx_to_response(
    tx: &dina_core::Transaction,
    block_height: u64,
    timestamp: u64,
) -> TransactionResponse {
    use dina_core::transaction::Transaction;

    let (tx_type, from, to, amount) = match tx {
        Transaction::Transfer {
            from, to, amount, ..
        } => ("transfer".to_string(), from.to_string(), Some(to.to_string()), Some(*amount)),
        Transaction::DeployContract { from, .. } => {
            ("deploy_contract".to_string(), from.to_string(), None, None)
        }
        Transaction::CallContract {
            from,
            contract,
            usdc_attached,
            ..
        } => (
            "call_contract".to_string(),
            from.to_string(),
            Some(contract.to_string()),
            if *usdc_attached > 0 {
                Some(*usdc_attached)
            } else {
                None
            },
        ),
        Transaction::RegisterDevice { owner, .. } => {
            ("register_device".to_string(), owner.to_string(), None, None)
        }
    };

    TransactionResponse {
        hash: tx.hash().to_string(),
        block_height,
        tx_type,
        from,
        to,
        amount,
        fee: tx.fee(),
        status: "confirmed".to_string(),
        timestamp,
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/blocks?page=1&limit=20
/// List recent blocks in descending order of height.
async fn list_blocks(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let page = params.page();
    let limit = params.limit();

    let latest_height = match state.db.get_latest_block_height() {
        Ok(h) => h,
        Err(e) => {
            warn!("failed to get latest block height: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error")
                .into_response();
        }
    };

    if latest_height == 0 {
        return Json(PaginatedResponse::<BlockResponse> {
            data: vec![],
            page,
            limit,
            total: 0,
            has_more: false,
        })
        .into_response();
    }

    // Total blocks is latest_height + 1 (genesis is block 0)
    let total = latest_height + 1;
    let skip = (page - 1) * limit;

    let mut blocks = Vec::new();
    let mut collected = 0u64;
    let mut skipped = 0u64;

    // Iterate from latest down to 0
    let mut height = latest_height as i64;
    while height >= 0 && collected < limit {
        if skipped < skip {
            skipped += 1;
            height -= 1;
            continue;
        }
        match state.db.get_block(height as u64) {
            Ok(Some(block)) => {
                blocks.push(block_to_response(&block));
                collected += 1;
            }
            Ok(None) => {
                // Gap in block heights, skip
            }
            Err(e) => {
                warn!("failed to load block at height {height}: {e}");
            }
        }
        height -= 1;
    }

    let has_more = skip + collected < total;

    Json(PaginatedResponse {
        data: blocks,
        page,
        limit,
        total,
        has_more,
    })
    .into_response()
}

/// GET /api/blocks/:height
async fn get_block_by_height(
    State(state): State<AppState>,
    Path(height): Path<u64>,
) -> impl IntoResponse {
    match state.db.get_block(height) {
        Ok(Some(block)) => Json(block_to_response(&block)).into_response(),
        Ok(None) => {
            error_response(StatusCode::NOT_FOUND, "block not found").into_response()
        }
        Err(e) => {
            warn!("failed to load block {height}: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response()
        }
    }
}

/// GET /api/blocks/hash/:hash
async fn get_block_by_hash(
    State(state): State<AppState>,
    Path(hash_str): Path<String>,
) -> impl IntoResponse {
    let hash = match Hash::from_str(&hash_str) {
        Ok(h) => h,
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, "invalid hash format").into_response();
        }
    };

    match state.db.get_block_by_hash(hash) {
        Ok(Some(block)) => Json(block_to_response(&block)).into_response(),
        Ok(None) => {
            error_response(StatusCode::NOT_FOUND, "block not found").into_response()
        }
        Err(e) => {
            warn!("failed to load block by hash: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response()
        }
    }
}

/// GET /api/transactions/:hash
async fn get_transaction(
    State(state): State<AppState>,
    Path(hash_str): Path<String>,
) -> impl IntoResponse {
    let hash = match Hash::from_str(&hash_str) {
        Ok(h) => h,
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, "invalid hash format").into_response();
        }
    };

    let indexer = state.indexer.read().await;
    match indexer.get_transaction(&hash) {
        Some((block_height, tx)) => {
            // Look up the block timestamp
            let timestamp = match state.db.get_block(*block_height) {
                Ok(Some(block)) => block.header.timestamp,
                _ => 0,
            };
            Json(tx_to_response(tx, *block_height, timestamp)).into_response()
        }
        None => {
            error_response(StatusCode::NOT_FOUND, "transaction not found").into_response()
        }
    }
}

/// GET /api/accounts/:address
async fn get_account(
    State(state): State<AppState>,
    Path(address_str): Path<String>,
) -> impl IntoResponse {
    let address = match Address::from_str(&address_str) {
        Ok(a) => a,
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, "invalid address format")
                .into_response();
        }
    };

    let (balance, nonce) = match state.db.get_account(address) {
        Ok(Some(account)) => (account.balance, account.nonce),
        Ok(None) => (0, 0),
        Err(e) => {
            warn!("failed to load account: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error")
                .into_response();
        }
    };

    let indexer = state.indexer.read().await;
    let stats = indexer.account_info(&address);

    Json(AccountResponse {
        address: address.to_string(),
        balance,
        nonce,
        tx_count: stats.as_ref().map_or(0, |s| s.tx_count),
        first_seen: stats.as_ref().map(|s| s.first_seen),
        last_active: stats.as_ref().map(|s| s.last_active),
    })
    .into_response()
}

/// GET /api/accounts/:address/transactions?page=1&limit=20
async fn get_account_transactions(
    State(state): State<AppState>,
    Path(address_str): Path<String>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let address = match Address::from_str(&address_str) {
        Ok(a) => a,
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, "invalid address format")
                .into_response();
        }
    };

    let page = params.page();
    let limit = params.limit();

    let indexer = state.indexer.read().await;
    let all_hashes = indexer.transactions_for_address(&address);
    let total = all_hashes.len() as u64;
    let skip = ((page - 1) * limit) as usize;

    let page_hashes: Vec<_> = all_hashes
        .iter()
        .rev() // most recent first
        .skip(skip)
        .take(limit as usize)
        .collect();

    let mut txs = Vec::with_capacity(page_hashes.len());
    for hash in page_hashes {
        if let Some((block_height, tx)) = indexer.get_transaction(hash) {
            let timestamp = match state.db.get_block(*block_height) {
                Ok(Some(block)) => block.header.timestamp,
                _ => 0,
            };
            txs.push(tx_to_response(tx, *block_height, timestamp));
        }
    }

    let has_more = (skip as u64) + (txs.len() as u64) < total;

    Json(PaginatedResponse {
        data: txs,
        page,
        limit,
        total,
        has_more,
    })
    .into_response()
}

/// GET /api/search?q=<query>
/// Search by block height (numeric), tx hash (0x + 64 hex chars), or address (0x + 64 hex chars).
async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let query = params.q.trim();

    if query.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "query parameter 'q' is required")
            .into_response();
    }

    // Try parsing as block height (numeric)
    if let Ok(height) = query.parse::<u64>() {
        if let Ok(Some(block)) = state.db.get_block(height) {
            let response = block_to_response(&block);
            return Json(SearchResult {
                result_type: "block".to_string(),
                data: serde_json::to_value(response).unwrap_or_default(),
            })
            .into_response();
        }
    }

    // Try parsing as a hash (could be tx hash or block hash)
    if let Ok(hash) = Hash::from_str(query) {
        // Check transaction index first
        let indexer = state.indexer.read().await;
        if let Some((block_height, tx)) = indexer.get_transaction(&hash) {
            let timestamp = match state.db.get_block(*block_height) {
                Ok(Some(block)) => block.header.timestamp,
                _ => 0,
            };
            let response = tx_to_response(tx, *block_height, timestamp);
            return Json(SearchResult {
                result_type: "transaction".to_string(),
                data: serde_json::to_value(response).unwrap_or_default(),
            })
            .into_response();
        }
        drop(indexer);

        // Check block hash
        if let Ok(Some(block)) = state.db.get_block_by_hash(hash) {
            let response = block_to_response(&block);
            return Json(SearchResult {
                result_type: "block".to_string(),
                data: serde_json::to_value(response).unwrap_or_default(),
            })
            .into_response();
        }
    }

    // Try parsing as an address
    if let Ok(address) = Address::from_str(query) {
        // Check if we have any record of this address
        let indexer = state.indexer.read().await;
        if indexer.account_info(&address).is_some() {
            let (balance, nonce) = match state.db.get_account(address) {
                Ok(Some(account)) => (account.balance, account.nonce),
                _ => (0, 0),
            };
            let stats = indexer.account_info(&address);
            let response = AccountResponse {
                address: address.to_string(),
                balance,
                nonce,
                tx_count: stats.as_ref().map_or(0, |s| s.tx_count),
                first_seen: stats.as_ref().map(|s| s.first_seen),
                last_active: stats.as_ref().map(|s| s.last_active),
            };
            return Json(SearchResult {
                result_type: "account".to_string(),
                data: serde_json::to_value(response).unwrap_or_default(),
            })
            .into_response();
        }
        drop(indexer);

        // Check if it exists as a device ID
        let indexer = state.indexer.read().await;
        if let Some(device) = indexer.get_device(&address) {
            let response = device_to_response(device);
            return Json(SearchResult {
                result_type: "device".to_string(),
                data: serde_json::to_value(response).unwrap_or_default(),
            })
            .into_response();
        }
    }

    error_response(StatusCode::NOT_FOUND, "no results found").into_response()
}

/// GET /api/stats
async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    let indexer = state.indexer.read().await;

    let now = chrono::Utc::now().timestamp() as u64;
    let total_blocks = indexer.total_blocks;
    let total_transactions = indexer.total_transactions;
    let total_accounts = indexer.total_accounts();
    let total_devices = indexer.total_devices();
    let avg_block_time_ms = indexer.avg_block_time_ms();
    let tps_1m = indexer.tps(now, 60);
    let tps_1h = indexer.tps(now, 3600);

    Json(ChainStats {
        total_blocks,
        total_transactions,
        total_accounts,
        total_devices,
        avg_block_time_ms,
        tps_1m,
        tps_1h,
    })
}

/// GET /api/validators
async fn list_validators(State(state): State<AppState>) -> impl IntoResponse {
    let indexer = state.indexer.read().await;
    let validators = indexer.validators();
    let total_blocks = indexer.total_blocks;

    let infos: Vec<ValidatorInfo> = validators
        .iter()
        .map(|(addr, blocks_proposed, last_proposed)| {
            let uptime_pct = if total_blocks > 0 {
                (*blocks_proposed as f64 / total_blocks as f64) * 100.0
            } else {
                0.0
            };
            ValidatorInfo {
                address: addr.to_string(),
                blocks_proposed: *blocks_proposed,
                uptime_pct,
                last_proposed: *last_proposed,
            }
        })
        .collect();

    Json(infos)
}

/// Convert a `DeviceIdentity` to a `DeviceResponse`.
fn device_to_response(device: &dina_core::device::DeviceIdentity) -> DeviceResponse {
    DeviceResponse {
        id: device.id.to_string(),
        owner: device.owner.to_string(),
        device_type: device.device_type.to_string(),
        firmware_hash: device.firmware_hash.to_string(),
        registered_at: device.registered_at,
        active: device.active,
        name: device.metadata.name.clone(),
        manufacturer: device.metadata.manufacturer.clone(),
        model: device.metadata.model.clone(),
    }
}

/// GET /api/devices?page=1&limit=20
async fn list_devices(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let page = params.page();
    let limit = params.limit();

    let indexer = state.indexer.read().await;
    let all_devices: Vec<DeviceResponse> = indexer
        .devices()
        .map(|(_, device)| device_to_response(device))
        .collect();

    let total = all_devices.len() as u64;
    let skip = ((page - 1) * limit) as usize;
    let data: Vec<DeviceResponse> = all_devices.into_iter().skip(skip).take(limit as usize).collect();
    let has_more = (skip as u64) + (data.len() as u64) < total;

    Json(PaginatedResponse {
        data,
        page,
        limit,
        total,
        has_more,
    })
}

/// GET /api/devices/:id
async fn get_device(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> impl IntoResponse {
    let id = match Address::from_str(&id_str) {
        Ok(a) => a,
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, "invalid device ID format")
                .into_response();
        }
    };

    let indexer = state.indexer.read().await;
    match indexer.get_device(&id) {
        Some(device) => Json(device_to_response(device)).into_response(),
        None => error_response(StatusCode::NOT_FOUND, "device not found").into_response(),
    }
}
