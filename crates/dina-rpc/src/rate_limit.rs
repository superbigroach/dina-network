use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use serde::Serialize;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the RPC rate limiter.
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum sustained requests per second per IP.
    pub requests_per_second: u32,
    /// Maximum requests per minute per IP.
    pub requests_per_minute: u32,
    /// Maximum burst size — the number of requests that can arrive
    /// instantaneously before throttling kicks in.
    pub burst_size: u32,
    /// IP addresses exempt from rate limiting (e.g. internal services).
    pub whitelist: Vec<String>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 50,
            requests_per_minute: 1000,
            burst_size: 100,
            whitelist: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/// The outcome of a rate-limit check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RateLimitResult {
    /// The request is allowed.
    Allowed,
    /// The request is denied — the caller should retry after `retry_after_ms`.
    Limited { retry_after_ms: u64 },
}

// ---------------------------------------------------------------------------
// Rate limiter
// ---------------------------------------------------------------------------

/// A sliding-window rate limiter keyed by IP address.
pub struct RateLimiter {
    limits: RateLimitConfig,
    /// Per-IP list of recent request timestamps.
    requests: HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limits: config,
            requests: HashMap::new(),
        }
    }

    /// Check whether a request from `ip` should be allowed.
    ///
    /// On `Allowed`, the request timestamp is recorded.
    /// On `Limited`, the caller receives a suggested retry delay.
    pub fn check(&mut self, ip: &str) -> RateLimitResult {
        if self.is_whitelisted(ip) {
            return RateLimitResult::Allowed;
        }

        let now = Instant::now();
        let one_second_ago = now - Duration::from_secs(1);
        let one_minute_ago = now - Duration::from_secs(60);

        let timestamps = self.requests.entry(ip.to_string()).or_default();

        // Prune entries older than one minute (they are irrelevant).
        timestamps.retain(|t| *t >= one_minute_ago);

        // Count requests in the last second and last minute.
        let in_last_second = timestamps.iter().filter(|t| **t >= one_second_ago).count() as u32;
        let in_last_minute = timestamps.len() as u32;

        // Check burst limit (instantaneous count in the last second).
        if in_last_second >= self.limits.burst_size {
            return RateLimitResult::Limited {
                retry_after_ms: 1000,
            };
        }

        // Check per-second limit.
        if in_last_second >= self.limits.requests_per_second {
            return RateLimitResult::Limited {
                retry_after_ms: 1000,
            };
        }

        // Check per-minute limit.
        if in_last_minute >= self.limits.requests_per_minute {
            // Suggest waiting until the oldest entry in the window expires.
            let oldest = timestamps.first().copied().unwrap_or(now);
            let wait = Duration::from_secs(60)
                .checked_sub(now.duration_since(oldest))
                .unwrap_or(Duration::from_secs(1));
            return RateLimitResult::Limited {
                retry_after_ms: wait.as_millis() as u64,
            };
        }

        // Allowed — record this request.
        timestamps.push(now);
        RateLimitResult::Allowed
    }

    /// Returns `true` if the given IP is whitelisted.
    pub fn is_whitelisted(&self, ip: &str) -> bool {
        self.limits.whitelist.iter().any(|w| w == ip)
    }

    /// Remove all entries older than one minute. Call this periodically to
    /// prevent unbounded memory growth from long-gone clients.
    pub fn cleanup(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(60);
        self.requests.retain(|_ip, timestamps| {
            timestamps.retain(|t| *t >= cutoff);
            !timestamps.is_empty()
        });
    }

    /// Number of tracked IPs (useful for monitoring).
    pub fn tracked_ips(&self) -> usize {
        self.requests.len()
    }
}

// ---------------------------------------------------------------------------
// Shared handle for the axum middleware
// ---------------------------------------------------------------------------

/// A thread-safe, shared handle to the rate limiter.
#[derive(Clone)]
pub struct SharedRateLimiter {
    inner: Arc<Mutex<RateLimiter>>,
}

impl SharedRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RateLimiter::new(config))),
        }
    }

    /// Check a request from `ip` and return the result.
    pub async fn check(&self, ip: &str) -> RateLimitResult {
        let mut limiter = self.inner.lock().await;
        limiter.check(ip)
    }

    /// Run periodic cleanup.
    pub async fn cleanup(&self) {
        let mut limiter = self.inner.lock().await;
        limiter.cleanup();
    }
}

// ---------------------------------------------------------------------------
// Axum middleware
// ---------------------------------------------------------------------------

/// JSON error body returned when a client is rate-limited.
#[derive(Serialize)]
struct RateLimitError {
    error: String,
    retry_after_ms: u64,
}

/// Axum middleware that rate-limits requests by source IP.
///
/// Attach to a router via:
/// ```ignore
/// let limiter = SharedRateLimiter::new(RateLimitConfig::default());
/// let app = Router::new()
///     .route("/v1/...", get(handler))
///     .layer(axum::middleware::from_fn_with_state(
///         limiter,
///         rate_limit_middleware,
///     ));
/// ```
pub async fn rate_limit_middleware(
    axum::extract::State(limiter): axum::extract::State<SharedRateLimiter>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Extract the client IP from the connection info or fall back to a header.
    let ip = request
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .or_else(|| {
            request
                .headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.split(',').next().unwrap_or("unknown").trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    match limiter.check(&ip).await {
        RateLimitResult::Allowed => next.run(request).await,
        RateLimitResult::Limited { retry_after_ms } => {
            let body = serde_json::to_string(&RateLimitError {
                error: "rate limit exceeded".to_string(),
                retry_after_ms,
            })
            .unwrap_or_else(|_| r#"{"error":"rate limit exceeded"}"#.to_string());

            Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header("Content-Type", "application/json")
                .header("Retry-After", (retry_after_ms / 1000).max(1).to_string())
                .body(Body::from(body))
                .unwrap()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_second: 5,
            requests_per_minute: 20,
            burst_size: 10,
            whitelist: vec!["127.0.0.1".to_string()],
        }
    }

    #[test]
    fn allowed_within_limit() {
        let mut limiter = RateLimiter::new(default_config());

        for _ in 0..5 {
            assert_eq!(limiter.check("10.0.0.1"), RateLimitResult::Allowed);
        }
    }

    #[test]
    fn blocked_over_per_second_limit() {
        let mut limiter = RateLimiter::new(default_config());

        // Use up the per-second allowance.
        for _ in 0..5 {
            assert_eq!(limiter.check("10.0.0.1"), RateLimitResult::Allowed);
        }

        // The 6th request within the same second should be limited.
        match limiter.check("10.0.0.1") {
            RateLimitResult::Limited { retry_after_ms } => {
                assert!(retry_after_ms > 0);
            }
            RateLimitResult::Allowed => panic!("should have been rate limited"),
        }
    }

    #[test]
    fn blocked_over_per_minute_limit() {
        let config = RateLimitConfig {
            requests_per_second: 100, // high per-second so we don't hit it
            requests_per_minute: 10,
            burst_size: 100,
            whitelist: vec![],
        };
        let mut limiter = RateLimiter::new(config);

        for _ in 0..10 {
            assert_eq!(limiter.check("10.0.0.2"), RateLimitResult::Allowed);
        }

        // The 11th request should be limited by the per-minute cap.
        match limiter.check("10.0.0.2") {
            RateLimitResult::Limited { retry_after_ms } => {
                assert!(retry_after_ms > 0);
            }
            RateLimitResult::Allowed => panic!("should have been rate limited"),
        }
    }

    #[test]
    fn whitelist_bypass() {
        let mut limiter = RateLimiter::new(default_config());

        // A whitelisted IP is never limited, even past the per-second limit.
        for _ in 0..100 {
            assert_eq!(limiter.check("127.0.0.1"), RateLimitResult::Allowed);
        }
    }

    #[test]
    fn is_whitelisted() {
        let limiter = RateLimiter::new(default_config());
        assert!(limiter.is_whitelisted("127.0.0.1"));
        assert!(!limiter.is_whitelisted("10.0.0.1"));
    }

    #[test]
    fn separate_ips_are_independent() {
        let mut limiter = RateLimiter::new(default_config());

        // Exhaust the limit for one IP.
        for _ in 0..5 {
            limiter.check("10.0.0.1");
        }
        assert!(matches!(
            limiter.check("10.0.0.1"),
            RateLimitResult::Limited { .. }
        ));

        // A different IP should still be allowed.
        assert_eq!(limiter.check("10.0.0.2"), RateLimitResult::Allowed);
    }

    #[test]
    fn cleanup_removes_stale_entries() {
        let mut limiter = RateLimiter::new(default_config());

        // Record a request — the IP should be tracked.
        limiter.check("10.0.0.99");
        assert_eq!(limiter.tracked_ips(), 1);

        // Manually clear the timestamps to simulate time passing.
        limiter.requests.get_mut("10.0.0.99").unwrap().clear();
        limiter.cleanup();

        // After cleanup, the empty entry should be removed.
        assert_eq!(limiter.tracked_ips(), 0);
    }

    #[test]
    fn burst_limit_enforcement() {
        let config = RateLimitConfig {
            requests_per_second: 100, // high so burst is the binding constraint
            requests_per_minute: 1000,
            burst_size: 3,
            whitelist: vec![],
        };
        let mut limiter = RateLimiter::new(config);

        for _ in 0..3 {
            assert_eq!(limiter.check("10.0.0.3"), RateLimitResult::Allowed);
        }

        // The 4th request exceeds the burst of 3.
        assert!(matches!(
            limiter.check("10.0.0.3"),
            RateLimitResult::Limited { .. }
        ));
    }

    #[test]
    fn default_config_values() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_second, 50);
        assert_eq!(config.requests_per_minute, 1000);
        assert_eq!(config.burst_size, 100);
        assert!(config.whitelist.is_empty());
    }
}
