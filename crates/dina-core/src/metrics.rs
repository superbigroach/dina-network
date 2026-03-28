use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum number of block timestamps retained for avg-block-time calculation.
const MAX_BLOCK_TIMES: usize = 100;

/// Maximum number of TX-count samples retained for TPS calculation.
const MAX_TX_SAMPLES: usize = 3_600;

/// One minute in seconds.
const ONE_MINUTE_SECS: u64 = 60;

/// One hour in seconds.
const ONE_HOUR_SECS: u64 = 3_600;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Real-time metrics for a Dina Network node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub blocks_processed: u64,
    pub transactions_processed: u64,
    pub peers_connected: u64,
    pub avg_block_time_ms: u64,
    pub uptime_seconds: u64,
    pub memory_usage_bytes: u64,
    pub disk_usage_bytes: u64,
    pub pending_transactions: u64,
    pub consensus_round: u64,
    pub last_block_time: u64,
    pub start_time: u64,
    block_times: VecDeque<u64>,
    tx_counts: VecDeque<(u64, u64)>,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl NodeMetrics {
    /// Create a new metrics tracker with the given start timestamp (seconds).
    pub fn new(start_time: u64) -> Self {
        Self {
            blocks_processed: 0,
            transactions_processed: 0,
            peers_connected: 0,
            avg_block_time_ms: 0,
            uptime_seconds: 0,
            memory_usage_bytes: 0,
            disk_usage_bytes: 0,
            pending_transactions: 0,
            consensus_round: 0,
            last_block_time: 0,
            start_time,
            block_times: VecDeque::with_capacity(MAX_BLOCK_TIMES + 1),
            tx_counts: VecDeque::with_capacity(MAX_TX_SAMPLES + 1),
        }
    }

    /// Record a new block with its timestamp (in seconds) and number of
    /// transactions it contained.
    pub fn record_block(&mut self, timestamp: u64, tx_count: u64) {
        self.blocks_processed += 1;
        self.transactions_processed += tx_count;
        self.last_block_time = timestamp;
        self.uptime_seconds = timestamp.saturating_sub(self.start_time);

        // Track block timestamp
        self.block_times.push_back(timestamp);
        if self.block_times.len() > MAX_BLOCK_TIMES {
            self.block_times.pop_front();
        }

        // Track tx counts for TPS
        self.tx_counts.push_back((timestamp, tx_count));
        if self.tx_counts.len() > MAX_TX_SAMPLES {
            self.tx_counts.pop_front();
        }

        // Recalculate average block time
        self.avg_block_time_ms = self.calculate_avg_block_time();
    }

    /// Update the number of connected peers.
    pub fn update_peers(&mut self, count: u64) {
        self.peers_connected = count;
    }

    /// Update the number of pending (mempool) transactions.
    pub fn update_pending(&mut self, count: u64) {
        self.pending_transactions = count;
    }

    /// Update resource usage counters.
    pub fn update_resources(&mut self, memory_bytes: u64, disk_bytes: u64) {
        self.memory_usage_bytes = memory_bytes;
        self.disk_usage_bytes = disk_bytes;
    }

    /// Update the consensus round.
    pub fn update_consensus_round(&mut self, round: u64) {
        self.consensus_round = round;
    }

    /// Rolling transactions-per-second over the last 1 minute.
    pub fn tps_1m(&self) -> f64 {
        self.tps_over(ONE_MINUTE_SECS)
    }

    /// Rolling transactions-per-second over the last 1 hour.
    pub fn tps_1h(&self) -> f64 {
        self.tps_over(ONE_HOUR_SECS)
    }

    /// Average block time in milliseconds over the last `MAX_BLOCK_TIMES` blocks.
    pub fn avg_block_time(&self) -> u64 {
        self.avg_block_time_ms
    }

    /// Produce a Prometheus-compatible metrics string.
    pub fn to_prometheus(&self) -> String {
        let mut out = String::with_capacity(1024);

        out.push_str("# HELP dina_blocks_processed Total blocks processed\n");
        out.push_str("# TYPE dina_blocks_processed counter\n");
        out.push_str(&format!(
            "dina_blocks_processed {}\n",
            self.blocks_processed
        ));

        out.push_str("# HELP dina_transactions_processed Total transactions processed\n");
        out.push_str("# TYPE dina_transactions_processed counter\n");
        out.push_str(&format!(
            "dina_transactions_processed {}\n",
            self.transactions_processed
        ));

        out.push_str("# HELP dina_peers_connected Number of connected peers\n");
        out.push_str("# TYPE dina_peers_connected gauge\n");
        out.push_str(&format!("dina_peers_connected {}\n", self.peers_connected));

        out.push_str("# HELP dina_tps_1m Transactions per second (1 minute)\n");
        out.push_str("# TYPE dina_tps_1m gauge\n");
        out.push_str(&format!("dina_tps_1m {:.2}\n", self.tps_1m()));

        out.push_str("# HELP dina_tps_1h Transactions per second (1 hour)\n");
        out.push_str("# TYPE dina_tps_1h gauge\n");
        out.push_str(&format!("dina_tps_1h {:.2}\n", self.tps_1h()));

        out.push_str("# HELP dina_avg_block_time_ms Average block time in milliseconds\n");
        out.push_str("# TYPE dina_avg_block_time_ms gauge\n");
        out.push_str(&format!(
            "dina_avg_block_time_ms {}\n",
            self.avg_block_time_ms
        ));

        out.push_str("# HELP dina_uptime_seconds Node uptime in seconds\n");
        out.push_str("# TYPE dina_uptime_seconds counter\n");
        out.push_str(&format!("dina_uptime_seconds {}\n", self.uptime_seconds));

        out.push_str("# HELP dina_memory_usage_bytes Memory usage in bytes\n");
        out.push_str("# TYPE dina_memory_usage_bytes gauge\n");
        out.push_str(&format!(
            "dina_memory_usage_bytes {}\n",
            self.memory_usage_bytes
        ));

        out.push_str("# HELP dina_disk_usage_bytes Disk usage in bytes\n");
        out.push_str("# TYPE dina_disk_usage_bytes gauge\n");
        out.push_str(&format!(
            "dina_disk_usage_bytes {}\n",
            self.disk_usage_bytes
        ));

        out.push_str("# HELP dina_pending_transactions Pending mempool transactions\n");
        out.push_str("# TYPE dina_pending_transactions gauge\n");
        out.push_str(&format!(
            "dina_pending_transactions {}\n",
            self.pending_transactions
        ));

        out.push_str("# HELP dina_consensus_round Current consensus round\n");
        out.push_str("# TYPE dina_consensus_round gauge\n");
        out.push_str(&format!("dina_consensus_round {}\n", self.consensus_round));

        out
    }

    /// Produce a JSON representation of all metrics.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "blocks_processed": self.blocks_processed,
            "transactions_processed": self.transactions_processed,
            "peers_connected": self.peers_connected,
            "tps_1m": self.tps_1m(),
            "tps_1h": self.tps_1h(),
            "avg_block_time_ms": self.avg_block_time_ms,
            "uptime_seconds": self.uptime_seconds,
            "memory_usage_bytes": self.memory_usage_bytes,
            "disk_usage_bytes": self.disk_usage_bytes,
            "pending_transactions": self.pending_transactions,
            "consensus_round": self.consensus_round,
            "last_block_time": self.last_block_time,
            "start_time": self.start_time,
        })
    }

    // -- Internal -----------------------------------------------------------

    /// Calculate TPS over a rolling window of `window_secs` seconds.
    fn tps_over(&self, window_secs: u64) -> f64 {
        if self.tx_counts.is_empty() {
            return 0.0;
        }

        let cutoff = self.last_block_time.saturating_sub(window_secs);
        let total_tx: u64 = self
            .tx_counts
            .iter()
            .filter(|(ts, _)| *ts > cutoff)
            .map(|(_, count)| count)
            .sum();

        if let (Some(first), Some(last)) = (
            self.tx_counts
                .iter()
                .filter(|(ts, _)| *ts > cutoff)
                .map(|(ts, _)| *ts)
                .next(),
            self.tx_counts.back().map(|(ts, _)| *ts),
        ) {
            let elapsed = last.saturating_sub(first);
            if elapsed > 0 {
                return total_tx as f64 / elapsed as f64;
            }
        }

        // Only one data point in the window -- report total as TPS assuming
        // a 1-second window minimum.
        total_tx as f64
    }

    /// Average block time in milliseconds from the retained block timestamps.
    fn calculate_avg_block_time(&self) -> u64 {
        if self.block_times.len() < 2 {
            return 0;
        }

        let first = *self.block_times.front().unwrap();
        let last = *self.block_times.back().unwrap();
        let elapsed_secs = last.saturating_sub(first);
        let intervals = (self.block_times.len() - 1) as u64;

        if intervals == 0 {
            return 0;
        }

        // Convert seconds to milliseconds
        elapsed_secs * 1_000 / intervals
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_metrics_zeroed() {
        let m = NodeMetrics::new(1000);
        assert_eq!(m.blocks_processed, 0);
        assert_eq!(m.transactions_processed, 0);
        assert_eq!(m.start_time, 1000);
    }

    #[test]
    fn record_block_increments_counters() {
        let mut m = NodeMetrics::new(0);
        m.record_block(10, 5);
        assert_eq!(m.blocks_processed, 1);
        assert_eq!(m.transactions_processed, 5);
        assert_eq!(m.last_block_time, 10);
        assert_eq!(m.uptime_seconds, 10);
    }

    #[test]
    fn record_multiple_blocks() {
        let mut m = NodeMetrics::new(0);
        m.record_block(1, 10);
        m.record_block(2, 20);
        m.record_block(3, 30);
        assert_eq!(m.blocks_processed, 3);
        assert_eq!(m.transactions_processed, 60);
    }

    #[test]
    fn avg_block_time_single_block() {
        let mut m = NodeMetrics::new(0);
        m.record_block(10, 1);
        assert_eq!(m.avg_block_time(), 0); // need at least 2 blocks
    }

    #[test]
    fn avg_block_time_multiple_blocks() {
        let mut m = NodeMetrics::new(0);
        m.record_block(10, 1);
        m.record_block(12, 1); // 2 sec gap
        m.record_block(14, 1); // 2 sec gap
                               // total 4 sec over 2 intervals = 2 sec = 2000 ms
        assert_eq!(m.avg_block_time(), 2000);
    }

    #[test]
    fn tps_1m_basic() {
        let mut m = NodeMetrics::new(0);
        // 10 blocks over 10 seconds, 5 tx each = 50 tx / 9 sec gaps
        for i in 1..=10 {
            m.record_block(i, 5);
        }
        let tps = m.tps_1m();
        // 50 tx over 9 seconds
        assert!((tps - 50.0 / 9.0).abs() < 0.1);
    }

    #[test]
    fn tps_1h_basic() {
        let mut m = NodeMetrics::new(0);
        for i in 1..=10 {
            m.record_block(i, 5);
        }
        // Same data, all within 1 hour
        let tps = m.tps_1h();
        assert!(tps > 0.0);
    }

    #[test]
    fn tps_empty() {
        let m = NodeMetrics::new(0);
        assert_eq!(m.tps_1m(), 0.0);
        assert_eq!(m.tps_1h(), 0.0);
    }

    #[test]
    fn update_peers() {
        let mut m = NodeMetrics::new(0);
        m.update_peers(5);
        assert_eq!(m.peers_connected, 5);
    }

    #[test]
    fn update_pending() {
        let mut m = NodeMetrics::new(0);
        m.update_pending(42);
        assert_eq!(m.pending_transactions, 42);
    }

    #[test]
    fn update_resources() {
        let mut m = NodeMetrics::new(0);
        m.update_resources(1024 * 1024, 1024 * 1024 * 100);
        assert_eq!(m.memory_usage_bytes, 1024 * 1024);
        assert_eq!(m.disk_usage_bytes, 1024 * 1024 * 100);
    }

    #[test]
    fn update_consensus_round() {
        let mut m = NodeMetrics::new(0);
        m.update_consensus_round(7);
        assert_eq!(m.consensus_round, 7);
    }

    #[test]
    fn block_times_bounded() {
        let mut m = NodeMetrics::new(0);
        for i in 0..200u64 {
            m.record_block(i, 1);
        }
        assert!(m.block_times.len() <= MAX_BLOCK_TIMES);
    }

    #[test]
    fn to_prometheus_output() {
        let mut m = NodeMetrics::new(0);
        m.record_block(1, 10);
        m.update_peers(3);
        let prom = m.to_prometheus();
        assert!(prom.contains("dina_blocks_processed 1"));
        assert!(prom.contains("dina_transactions_processed 10"));
        assert!(prom.contains("dina_peers_connected 3"));
        assert!(prom.contains("dina_tps_1m"));
        assert!(prom.contains("# TYPE"));
        assert!(prom.contains("# HELP"));
    }

    #[test]
    fn to_json_output() {
        let mut m = NodeMetrics::new(100);
        m.record_block(110, 5);
        let json = m.to_json();
        assert_eq!(json["blocks_processed"], 1);
        assert_eq!(json["transactions_processed"], 5);
        assert_eq!(json["start_time"], 100);
        assert_eq!(json["uptime_seconds"], 10);
    }

    #[test]
    fn tps_1m_excludes_old_data() {
        let mut m = NodeMetrics::new(0);
        // Old block at t=10
        m.record_block(10, 100);
        // Recent blocks at t=1000..1010
        for i in 1000..=1010 {
            m.record_block(i, 5);
        }
        let tps = m.tps_1m();
        // Only recent 11 blocks (55 tx over 10 sec) should count.
        // Old block at t=10 is outside the 60-second window from t=1010.
        assert!(tps > 0.0);
        assert!(tps < 100.0); // should not reflect the 100 tx from old block
    }
}
