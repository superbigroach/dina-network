use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{error, info};

use dina_core::Address;

use crate::faucet::{Faucet, FaucetError};

/// Shared faucet state wrapped for concurrent access.
type SharedFaucet = Arc<Mutex<Faucet>>;

/// Build the faucet HTTP router with all endpoints.
///
/// Mount this under `/faucet` in your main application:
/// ```ignore
/// let app = Router::new().nest("/faucet", faucet_router(faucet));
/// ```
pub fn faucet_router(faucet: Faucet) -> Router {
    let state: SharedFaucet = Arc::new(Mutex::new(faucet));

    Router::new()
        .route("/request", post(handle_request))
        .route("/status/{address}", get(handle_status))
        .route("/stats", get(handle_stats))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// POST /faucet/request body.
#[derive(Debug, Deserialize)]
struct FaucetRequestBody {
    /// Hex-encoded Dina address (with or without 0x prefix).
    address: String,
}

/// Successful drip response.
#[derive(Debug, Serialize)]
struct FaucetRequestResponse {
    /// Whether the request was successful.
    success: bool,
    /// Amount dispensed in USDC micro-units.
    amount: u64,
    /// Human-readable amount (e.g., "100.000000 USDC").
    amount_display: String,
    /// Hex-encoded recipient address.
    address: String,
    /// Unix timestamp of the request.
    timestamp: u64,
}

/// GET /faucet/status/:address response.
#[derive(Debug, Serialize)]
struct FaucetStatusResponse {
    /// Hex-encoded address being queried.
    address: String,
    /// Whether the address can currently request funds.
    can_request: bool,
    /// Seconds until the next request is allowed (0 if ready).
    seconds_until_next: u64,
    /// Total USDC dispensed to this address (micro-units).
    total_received: u64,
    /// Number of requests from this address.
    request_count: usize,
}

/// GET /faucet/stats response.
#[derive(Debug, Serialize)]
struct FaucetStatsResponse {
    /// Total USDC dispensed (micro-units).
    total_dispensed: u64,
    /// Human-readable total (e.g., "1234.560000 USDC").
    total_dispensed_display: String,
    /// Number of unique addresses served.
    unique_addresses: usize,
    /// Total number of drip requests.
    total_requests: usize,
    /// Amount per drip (micro-units).
    drip_amount: u64,
    /// Maximum per address per day (micro-units).
    max_per_address_per_day: u64,
    /// Cooldown between requests (seconds).
    cooldown_seconds: u64,
}

/// Error response body.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /faucet/request — Request testnet USDC.
async fn handle_request(
    State(faucet): State<SharedFaucet>,
    Json(body): Json<FaucetRequestBody>,
) -> impl IntoResponse {
    let address = match parse_address(&body.address) {
        Ok(addr) => addr,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::to_value(ErrorResponse {
                        success: false,
                        error: e,
                    })
                    .unwrap(),
                ),
            );
        }
    };

    let current_time = current_unix_timestamp();

    let mut faucet = faucet.lock().await;
    match faucet.request_funds(address, current_time) {
        Ok(req) => {
            info!(address = %address, amount = req.amount, "faucet request served via HTTP");

            let response = FaucetRequestResponse {
                success: true,
                amount: req.amount,
                amount_display: format_usdc(req.amount),
                address: format!("{}", address),
                timestamp: req.timestamp,
            };

            (
                StatusCode::OK,
                Json(serde_json::to_value(response).unwrap()),
            )
        }
        Err(FaucetError::CooldownActive { remaining_seconds }) => {
            let response = ErrorResponse {
                success: false,
                error: format!(
                    "Please wait {} seconds before requesting again",
                    remaining_seconds
                ),
            };
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::to_value(response).unwrap()),
            )
        }
        Err(FaucetError::DailyLimitExceeded { dispensed, limit }) => {
            let response = ErrorResponse {
                success: false,
                error: format!(
                    "Daily limit reached: {} of {} USDC dispensed today",
                    format_usdc(dispensed),
                    format_usdc(limit)
                ),
            };
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::to_value(response).unwrap()),
            )
        }
        Err(e) => {
            error!(error = %e, "faucet request failed");
            let response = ErrorResponse {
                success: false,
                error: e.to_string(),
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(response).unwrap()),
            )
        }
    }
}

/// GET /faucet/status/:address — Check faucet status for an address.
async fn handle_status(
    State(faucet): State<SharedFaucet>,
    Path(address_hex): Path<String>,
) -> impl IntoResponse {
    let address = match parse_address(&address_hex) {
        Ok(addr) => addr,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::to_value(ErrorResponse {
                        success: false,
                        error: e,
                    })
                    .unwrap(),
                ),
            );
        }
    };

    let current_time = current_unix_timestamp();
    let faucet = faucet.lock().await;

    let can_request = faucet.can_request(&address, current_time);
    let seconds_until_next = faucet.time_until_next(&address, current_time);
    let history = faucet.history(&address);
    let total_received: u64 = history.iter().map(|r| r.amount).sum();

    let response = FaucetStatusResponse {
        address: format!("{}", address),
        can_request,
        seconds_until_next,
        total_received,
        request_count: history.len(),
    };

    (
        StatusCode::OK,
        Json(serde_json::to_value(response).unwrap()),
    )
}

/// GET /faucet/stats — Faucet aggregate statistics.
async fn handle_stats(State(faucet): State<SharedFaucet>) -> impl IntoResponse {
    let faucet = faucet.lock().await;
    let stats = faucet.stats();

    let response = FaucetStatsResponse {
        total_dispensed: stats.total_dispensed,
        total_dispensed_display: format_usdc(stats.total_dispensed),
        unique_addresses: stats.unique_addresses,
        total_requests: stats.total_requests,
        drip_amount: stats.drip_amount,
        max_per_address_per_day: stats.max_per_address_per_day,
        cooldown_seconds: stats.cooldown_seconds,
    };

    (
        StatusCode::OK,
        Json(serde_json::to_value(response).unwrap()),
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a hex-encoded address string into an `Address`.
fn parse_address(s: &str) -> Result<Address, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).map_err(|e| format!("invalid hex address: {e}"))?;
    if bytes.len() != 32 {
        return Err(format!(
            "address must be 32 bytes (64 hex chars), got {} bytes",
            bytes.len()
        ));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(Address(arr))
}

/// Format USDC micro-units as a human-readable string.
fn format_usdc(micro_units: u64) -> String {
    let whole = micro_units / 1_000_000;
    let frac = micro_units % 1_000_000;
    format!("{}.{:06} USDC", whole, frac)
}

/// Get the current Unix timestamp in seconds.
fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_faucet() -> Faucet {
        Faucet::new(Address([0x01; 32]), 100_000_000) // 100 USDC per drip
    }

    fn test_address_hex() -> String {
        hex::encode([0x02; 32])
    }

    #[test]
    fn parse_address_with_0x_prefix() {
        let hex_str = format!("0x{}", hex::encode([0xaa; 32]));
        let addr = parse_address(&hex_str).unwrap();
        assert_eq!(addr, Address([0xaa; 32]));
    }

    #[test]
    fn parse_address_without_prefix() {
        let hex_str = hex::encode([0xbb; 32]);
        let addr = parse_address(&hex_str).unwrap();
        assert_eq!(addr, Address([0xbb; 32]));
    }

    #[test]
    fn parse_address_wrong_length() {
        let result = parse_address("aabb");
        assert!(result.is_err());
    }

    #[test]
    fn parse_address_invalid_hex() {
        let result = parse_address("not_hex_at_all_gggg");
        assert!(result.is_err());
    }

    #[test]
    fn format_usdc_whole() {
        assert_eq!(format_usdc(100_000_000), "100.000000 USDC");
    }

    #[test]
    fn format_usdc_fractional() {
        assert_eq!(format_usdc(1_500_000), "1.500000 USDC");
    }

    #[test]
    fn format_usdc_zero() {
        assert_eq!(format_usdc(0), "0.000000 USDC");
    }

    #[tokio::test]
    async fn post_request_success() {
        let app = faucet_router(test_faucet());

        let body = serde_json::json!({ "address": test_address_hex() });
        let request = Request::builder()
            .method("POST")
            .uri("/request")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["amount"], 100_000_000);
    }

    #[tokio::test]
    async fn post_request_invalid_address() {
        let app = faucet_router(test_faucet());

        let body = serde_json::json!({ "address": "not_valid" });
        let request = Request::builder()
            .method("POST")
            .uri("/request")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_status_no_history() {
        let app = faucet_router(test_faucet());
        let addr = test_address_hex();

        let request = Request::builder()
            .method("GET")
            .uri(format!("/status/{}", addr))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["can_request"], true);
        assert_eq!(json["seconds_until_next"], 0);
        assert_eq!(json["request_count"], 0);
    }

    #[tokio::test]
    async fn get_stats_empty_faucet() {
        let app = faucet_router(test_faucet());

        let request = Request::builder()
            .method("GET")
            .uri("/stats")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total_dispensed"], 0);
        assert_eq!(json["unique_addresses"], 0);
        assert_eq!(json["total_requests"], 0);
        assert_eq!(json["drip_amount"], 100_000_000);
    }
}
