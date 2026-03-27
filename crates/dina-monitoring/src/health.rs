use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Result of a single health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub details: serde_json::Value,
}

impl HealthStatus {
    /// Create a healthy status with a message.
    pub fn healthy(message: impl Into<String>) -> Self {
        Self {
            healthy: true,
            message: message.into(),
            details: serde_json::Value::Null,
        }
    }

    /// Create an unhealthy status with a message.
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: message.into(),
            details: serde_json::Value::Null,
        }
    }

    /// Attach JSON details to the status.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }
}

/// Overall status of the node, derived from individual health checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverallStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl std::fmt::Display for OverallStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverallStatus::Healthy => write!(f, "healthy"),
            OverallStatus::Degraded => write!(f, "degraded"),
            OverallStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Aggregated health report containing results of all checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub status: OverallStatus,
    pub checks: Vec<(String, HealthStatus)>,
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub version: String,
}

/// Trait for implementing a health check.
pub trait HealthCheck: Send + Sync {
    /// Human-readable name of this check.
    fn name(&self) -> &str;
    /// Execute the check and return a status.
    fn check(&self) -> HealthStatus;
}

/// Manages a collection of health checks and produces aggregate reports.
pub struct HealthChecker {
    checks: Vec<Box<dyn HealthCheck>>,
    started_at: Instant,
    version: String,
}

impl HealthChecker {
    /// Create a new health checker with no checks registered.
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            checks: Vec::new(),
            started_at: Instant::now(),
            version: version.into(),
        }
    }

    /// Register a health check.
    pub fn add_check(&mut self, check: Box<dyn HealthCheck>) {
        self.checks.push(check);
    }

    /// Run all checks and produce a report.
    pub fn report(&self) -> HealthReport {
        let mut results = Vec::new();
        let mut any_unhealthy = false;
        let mut all_healthy = true;

        for check in &self.checks {
            let status = check.check();
            if !status.healthy {
                all_healthy = false;
                // Determine if this is a critical check (consensus, storage) or optional
                let name = check.name();
                if name == "consensus" || name == "storage" {
                    any_unhealthy = true;
                }
            }
            results.push((check.name().to_string(), status));
        }

        let overall = if all_healthy {
            OverallStatus::Healthy
        } else if any_unhealthy {
            OverallStatus::Unhealthy
        } else {
            OverallStatus::Degraded
        };

        let now = chrono::Utc::now().timestamp() as u64;
        let uptime = self.started_at.elapsed().as_secs();

        HealthReport {
            status: overall,
            checks: results,
            timestamp: now,
            uptime_seconds: uptime,
            version: self.version.clone(),
        }
    }

    /// Quick liveness check: returns true if the node process is running
    /// (always true if this code is executing).
    pub fn is_live(&self) -> bool {
        true
    }

    /// Readiness check: returns true if all critical checks pass.
    pub fn is_ready(&self) -> bool {
        let report = self.report();
        report.status != OverallStatus::Unhealthy
    }
}

// ---------------------------------------------------------------------------
// Built-in health checks
// ---------------------------------------------------------------------------

/// Checks whether consensus is running and the last block is recent.
pub struct ConsensusHealthCheck {
    /// Returns (is_running, last_block_timestamp_epoch_secs)
    state_fn: Box<dyn Fn() -> (bool, u64) + Send + Sync>,
    max_block_age_secs: u64,
}

impl ConsensusHealthCheck {
    pub fn new(
        state_fn: impl Fn() -> (bool, u64) + Send + Sync + 'static,
        max_block_age_secs: u64,
    ) -> Self {
        Self {
            state_fn: Box::new(state_fn),
            max_block_age_secs,
        }
    }
}

impl HealthCheck for ConsensusHealthCheck {
    fn name(&self) -> &str {
        "consensus"
    }

    fn check(&self) -> HealthStatus {
        let (is_running, last_block_ts) = (self.state_fn)();
        if !is_running {
            return HealthStatus::unhealthy("Consensus is not running").with_details(
                serde_json::json!({ "running": false }),
            );
        }

        let now = chrono::Utc::now().timestamp() as u64;
        let age = now.saturating_sub(last_block_ts);

        if age > self.max_block_age_secs {
            HealthStatus::unhealthy(format!(
                "Last block is {}s old (max {}s)",
                age, self.max_block_age_secs
            ))
            .with_details(serde_json::json!({
                "running": true,
                "last_block_age_secs": age,
                "max_block_age_secs": self.max_block_age_secs,
            }))
        } else {
            HealthStatus::healthy("Consensus is running and up to date").with_details(
                serde_json::json!({
                    "running": true,
                    "last_block_age_secs": age,
                }),
            )
        }
    }
}

/// Checks whether the node has at least one connected peer.
pub struct NetworkHealthCheck {
    /// Returns the current peer count.
    peer_count_fn: Box<dyn Fn() -> u64 + Send + Sync>,
    min_peers: u64,
}

impl NetworkHealthCheck {
    pub fn new(
        peer_count_fn: impl Fn() -> u64 + Send + Sync + 'static,
        min_peers: u64,
    ) -> Self {
        Self {
            peer_count_fn: Box::new(peer_count_fn),
            min_peers,
        }
    }
}

impl HealthCheck for NetworkHealthCheck {
    fn name(&self) -> &str {
        "network"
    }

    fn check(&self) -> HealthStatus {
        let count = (self.peer_count_fn)();
        if count >= self.min_peers {
            HealthStatus::healthy(format!("{} peers connected", count)).with_details(
                serde_json::json!({ "peer_count": count, "min_peers": self.min_peers }),
            )
        } else {
            HealthStatus::unhealthy(format!(
                "Only {} peers connected (min {})",
                count, self.min_peers
            ))
            .with_details(
                serde_json::json!({ "peer_count": count, "min_peers": self.min_peers }),
            )
        }
    }
}

/// Checks whether the database/storage layer is readable.
pub struct StorageHealthCheck {
    /// Returns true if the storage is readable.
    storage_fn: Box<dyn Fn() -> bool + Send + Sync>,
}

impl StorageHealthCheck {
    pub fn new(storage_fn: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            storage_fn: Box::new(storage_fn),
        }
    }
}

impl HealthCheck for StorageHealthCheck {
    fn name(&self) -> &str {
        "storage"
    }

    fn check(&self) -> HealthStatus {
        if (self.storage_fn)() {
            HealthStatus::healthy("Storage is readable")
        } else {
            HealthStatus::unhealthy("Storage is not readable")
        }
    }
}

/// Checks that the mempool is not full (below capacity threshold).
pub struct MempoolHealthCheck {
    /// Returns (current_size, max_capacity).
    mempool_fn: Box<dyn Fn() -> (u64, u64) + Send + Sync>,
    threshold_pct: u8,
}

impl MempoolHealthCheck {
    pub fn new(
        mempool_fn: impl Fn() -> (u64, u64) + Send + Sync + 'static,
        threshold_pct: u8,
    ) -> Self {
        Self {
            mempool_fn: Box::new(mempool_fn),
            threshold_pct,
        }
    }
}

impl HealthCheck for MempoolHealthCheck {
    fn name(&self) -> &str {
        "mempool"
    }

    fn check(&self) -> HealthStatus {
        let (current, max) = (self.mempool_fn)();
        if max == 0 {
            return HealthStatus::healthy("Mempool has no capacity limit");
        }

        let usage_pct = ((current as f64 / max as f64) * 100.0) as u8;
        if usage_pct >= self.threshold_pct {
            HealthStatus::unhealthy(format!(
                "Mempool is {}% full ({}/{})",
                usage_pct, current, max
            ))
            .with_details(serde_json::json!({
                "current": current,
                "max": max,
                "usage_pct": usage_pct,
                "threshold_pct": self.threshold_pct,
            }))
        } else {
            HealthStatus::healthy(format!("Mempool is {}% full", usage_pct)).with_details(
                serde_json::json!({
                    "current": current,
                    "max": max,
                    "usage_pct": usage_pct,
                }),
            )
        }
    }
}

/// Checks whether the node is synced with the network (within N blocks of network height).
pub struct SyncHealthCheck {
    /// Returns (local_height, network_height).
    sync_fn: Box<dyn Fn() -> (u64, u64) + Send + Sync>,
    max_behind_blocks: u64,
}

impl SyncHealthCheck {
    pub fn new(
        sync_fn: impl Fn() -> (u64, u64) + Send + Sync + 'static,
        max_behind_blocks: u64,
    ) -> Self {
        Self {
            sync_fn: Box::new(sync_fn),
            max_behind_blocks,
        }
    }
}

impl HealthCheck for SyncHealthCheck {
    fn name(&self) -> &str {
        "sync"
    }

    fn check(&self) -> HealthStatus {
        let (local_height, network_height) = (self.sync_fn)();
        let behind = network_height.saturating_sub(local_height);

        if behind <= self.max_behind_blocks {
            HealthStatus::healthy(format!(
                "Node is synced (local={}, network={}, behind={})",
                local_height, network_height, behind
            ))
            .with_details(serde_json::json!({
                "local_height": local_height,
                "network_height": network_height,
                "blocks_behind": behind,
            }))
        } else {
            HealthStatus::unhealthy(format!(
                "Node is {} blocks behind (local={}, network={})",
                behind, local_height, network_height
            ))
            .with_details(serde_json::json!({
                "local_height": local_height,
                "network_height": network_height,
                "blocks_behind": behind,
                "max_behind": self.max_behind_blocks,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_healthy() {
        let status = HealthStatus::healthy("All good");
        assert!(status.healthy);
        assert_eq!(status.message, "All good");
    }

    #[test]
    fn test_health_status_unhealthy() {
        let status = HealthStatus::unhealthy("Bad");
        assert!(!status.healthy);
        assert_eq!(status.message, "Bad");
    }

    #[test]
    fn test_health_status_with_details() {
        let status = HealthStatus::healthy("ok")
            .with_details(serde_json::json!({"key": "value"}));
        assert!(status.details.is_object());
        assert_eq!(status.details["key"], "value");
    }

    #[test]
    fn test_overall_status_display() {
        assert_eq!(OverallStatus::Healthy.to_string(), "healthy");
        assert_eq!(OverallStatus::Degraded.to_string(), "degraded");
        assert_eq!(OverallStatus::Unhealthy.to_string(), "unhealthy");
    }

    #[test]
    fn test_health_checker_no_checks() {
        let checker = HealthChecker::new("1.0.0");
        let report = checker.report();
        assert_eq!(report.status, OverallStatus::Healthy);
        assert!(report.checks.is_empty());
        assert_eq!(report.version, "1.0.0");
    }

    #[test]
    fn test_health_checker_all_healthy() {
        let mut checker = HealthChecker::new("1.0.0");
        checker.add_check(Box::new(NetworkHealthCheck::new(|| 5, 1)));
        checker.add_check(Box::new(StorageHealthCheck::new(|| true)));
        let report = checker.report();
        assert_eq!(report.status, OverallStatus::Healthy);
        assert_eq!(report.checks.len(), 2);
    }

    #[test]
    fn test_health_checker_degraded_non_critical() {
        let mut checker = HealthChecker::new("1.0.0");
        // Network failing is degraded (not critical)
        checker.add_check(Box::new(NetworkHealthCheck::new(|| 0, 1)));
        checker.add_check(Box::new(StorageHealthCheck::new(|| true)));
        let report = checker.report();
        assert_eq!(report.status, OverallStatus::Degraded);
    }

    #[test]
    fn test_health_checker_unhealthy_critical() {
        let mut checker = HealthChecker::new("1.0.0");
        // Storage failing is critical -> Unhealthy
        checker.add_check(Box::new(StorageHealthCheck::new(|| false)));
        let report = checker.report();
        assert_eq!(report.status, OverallStatus::Unhealthy);
    }

    #[test]
    fn test_liveness_always_true() {
        let checker = HealthChecker::new("1.0.0");
        assert!(checker.is_live());
    }

    #[test]
    fn test_readiness_healthy() {
        let mut checker = HealthChecker::new("1.0.0");
        checker.add_check(Box::new(StorageHealthCheck::new(|| true)));
        assert!(checker.is_ready());
    }

    #[test]
    fn test_readiness_unhealthy() {
        let mut checker = HealthChecker::new("1.0.0");
        checker.add_check(Box::new(StorageHealthCheck::new(|| false)));
        assert!(!checker.is_ready());
    }

    #[test]
    fn test_consensus_health_check_running() {
        let now = chrono::Utc::now().timestamp() as u64;
        let check = ConsensusHealthCheck::new(move || (true, now), 10);
        let status = check.check();
        assert!(status.healthy);
    }

    #[test]
    fn test_consensus_health_check_not_running() {
        let check = ConsensusHealthCheck::new(|| (false, 0), 10);
        let status = check.check();
        assert!(!status.healthy);
        assert!(status.message.contains("not running"));
    }

    #[test]
    fn test_consensus_health_check_stale_block() {
        // Block from 60 seconds ago, max age is 10s
        let old_ts = (chrono::Utc::now().timestamp() as u64).saturating_sub(60);
        let check = ConsensusHealthCheck::new(move || (true, old_ts), 10);
        let status = check.check();
        assert!(!status.healthy);
    }

    #[test]
    fn test_network_health_check_enough_peers() {
        let check = NetworkHealthCheck::new(|| 5, 1);
        let status = check.check();
        assert!(status.healthy);
        assert!(status.message.contains("5 peers"));
    }

    #[test]
    fn test_network_health_check_no_peers() {
        let check = NetworkHealthCheck::new(|| 0, 1);
        let status = check.check();
        assert!(!status.healthy);
    }

    #[test]
    fn test_storage_health_check_ok() {
        let check = StorageHealthCheck::new(|| true);
        assert!(check.check().healthy);
        assert_eq!(check.name(), "storage");
    }

    #[test]
    fn test_storage_health_check_fail() {
        let check = StorageHealthCheck::new(|| false);
        assert!(!check.check().healthy);
    }

    #[test]
    fn test_mempool_health_check_ok() {
        let check = MempoolHealthCheck::new(|| (50, 1000), 90);
        let status = check.check();
        assert!(status.healthy);
    }

    #[test]
    fn test_mempool_health_check_full() {
        let check = MempoolHealthCheck::new(|| (950, 1000), 90);
        let status = check.check();
        assert!(!status.healthy);
        assert!(status.message.contains("95%"));
    }

    #[test]
    fn test_mempool_health_check_zero_capacity() {
        let check = MempoolHealthCheck::new(|| (0, 0), 90);
        let status = check.check();
        assert!(status.healthy);
    }

    #[test]
    fn test_sync_health_check_synced() {
        let check = SyncHealthCheck::new(|| (100, 102), 5);
        let status = check.check();
        assert!(status.healthy);
    }

    #[test]
    fn test_sync_health_check_behind() {
        let check = SyncHealthCheck::new(|| (50, 100), 5);
        let status = check.check();
        assert!(!status.healthy);
        assert!(status.message.contains("50 blocks behind"));
    }

    #[test]
    fn test_sync_health_check_exact_threshold() {
        let check = SyncHealthCheck::new(|| (95, 100), 5);
        let status = check.check();
        assert!(status.healthy);
    }

    #[test]
    fn test_uptime_positive() {
        let checker = HealthChecker::new("1.0.0");
        // Uptime should be >= 0 (just created)
        let report = checker.report();
        assert!(report.uptime_seconds < 5);
    }
}
