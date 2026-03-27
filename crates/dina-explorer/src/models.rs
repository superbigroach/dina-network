use serde::{Deserialize, Serialize};
use serde_json::Value;

/// API response for a single block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockResponse {
    pub height: u64,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: u64,
    pub proposer: String,
    pub tx_count: usize,
    pub state_root: String,
}

/// API response for a single transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub hash: String,
    pub block_height: u64,
    pub tx_type: String,
    pub from: String,
    pub to: Option<String>,
    pub amount: Option<u64>,
    pub fee: u64,
    pub status: String,
    pub timestamp: u64,
}

/// API response for account information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountResponse {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
    pub tx_count: u64,
    pub first_seen: Option<u64>,
    pub last_active: Option<u64>,
}

/// Chain-wide statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStats {
    pub total_blocks: u64,
    pub total_transactions: u64,
    pub total_accounts: u64,
    pub total_devices: u64,
    pub avg_block_time_ms: f64,
    pub tps_1m: f64,
    pub tps_1h: f64,
}

/// Information about a validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub address: String,
    pub blocks_proposed: u64,
    pub uptime_pct: f64,
    pub last_proposed: Option<u64>,
}

/// Device information for the explorer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceResponse {
    pub id: String,
    pub owner: String,
    pub device_type: String,
    pub firmware_hash: String,
    pub registered_at: u64,
    pub active: bool,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
}

/// A search result that can be one of several entity types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub result_type: String,
    pub data: Value,
}

/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub page: u64,
    pub limit: u64,
    pub total: u64,
    pub has_more: bool,
}

/// Query parameters for paginated endpoints.
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

impl PaginationParams {
    pub fn page(&self) -> u64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn limit(&self) -> u64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }
}

/// Query parameters for the search endpoint.
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
}

/// Standard API error response body.
#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error: String,
    pub code: u16,
}
