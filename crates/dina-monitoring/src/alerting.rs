use serde::{Deserialize, Serialize};

use crate::prometheus::{MetricValue, PrometheusMetrics};

/// Alert severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

/// Conditions that can trigger an alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// Block processing time exceeds max_ms milliseconds.
    BlockTimeTooSlow { max_ms: u64 },
    /// Connected peer count drops below min.
    PeerCountLow { min: u64 },
    /// Mempool usage exceeds threshold_pct percent.
    MempoolFull { threshold_pct: u8 },
    /// No new block for max_seconds seconds.
    ConsensusStalled { max_seconds: u64 },
    /// Disk usage exceeds threshold_pct percent.
    DiskUsageHigh { threshold_pct: u8 },
    /// Validator missed max_consecutive consecutive blocks.
    ValidatorMissedBlocks { max_consecutive: u64 },
}

/// Defines an alert rule with a condition, severity, and cooldown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub condition: AlertCondition,
    pub severity: Severity,
    /// Minimum seconds between re-triggering this alert.
    pub cooldown_secs: u64,
}

/// A triggered alert instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub rule_name: String,
    pub severity: Severity,
    pub message: String,
    pub triggered_at: u64,
    pub resolved: bool,
}

/// Manages alert rules and tracks active alerts.
pub struct AlertManager {
    rules: Vec<AlertRule>,
    active_alerts: Vec<Alert>,
    /// Tracks when each rule last fired to enforce cooldown.
    last_fired: std::collections::HashMap<String, u64>,
}

impl AlertManager {
    /// Create a new AlertManager with a set of sensible default rules.
    pub fn new() -> Self {
        let rules = vec![
            AlertRule {
                name: "block_time_slow".into(),
                condition: AlertCondition::BlockTimeTooSlow { max_ms: 5000 },
                severity: Severity::Warning,
                cooldown_secs: 60,
            },
            AlertRule {
                name: "low_peers".into(),
                condition: AlertCondition::PeerCountLow { min: 1 },
                severity: Severity::Critical,
                cooldown_secs: 30,
            },
            AlertRule {
                name: "mempool_full".into(),
                condition: AlertCondition::MempoolFull { threshold_pct: 90 },
                severity: Severity::Warning,
                cooldown_secs: 120,
            },
            AlertRule {
                name: "consensus_stalled".into(),
                condition: AlertCondition::ConsensusStalled { max_seconds: 30 },
                severity: Severity::Critical,
                cooldown_secs: 60,
            },
            AlertRule {
                name: "disk_usage_high".into(),
                condition: AlertCondition::DiskUsageHigh { threshold_pct: 90 },
                severity: Severity::Warning,
                cooldown_secs: 300,
            },
            AlertRule {
                name: "validator_missed_blocks".into(),
                condition: AlertCondition::ValidatorMissedBlocks { max_consecutive: 3 },
                severity: Severity::Critical,
                cooldown_secs: 60,
            },
        ];

        Self {
            rules,
            active_alerts: Vec::new(),
            last_fired: std::collections::HashMap::new(),
        }
    }

    /// Create an AlertManager with custom rules (no defaults).
    pub fn with_rules(rules: Vec<AlertRule>) -> Self {
        Self {
            rules,
            active_alerts: Vec::new(),
            last_fired: std::collections::HashMap::new(),
        }
    }

    /// Add a rule to the manager.
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }

    /// Evaluate all rules against the current metrics and return any newly triggered alerts.
    pub fn evaluate(&mut self, metrics: &PrometheusMetrics) -> Vec<Alert> {
        let now = chrono::Utc::now().timestamp() as u64;
        let mut new_alerts = Vec::new();

        for rule in &self.rules {
            // Check cooldown
            if let Some(&last) = self.last_fired.get(&rule.name) {
                if now.saturating_sub(last) < rule.cooldown_secs {
                    continue;
                }
            }

            let triggered = self.check_condition(&rule.condition, metrics);

            if triggered {
                let message = self.format_message(&rule.condition, metrics);
                let alert = Alert {
                    rule_name: rule.name.clone(),
                    severity: rule.severity,
                    message,
                    triggered_at: now,
                    resolved: false,
                };
                new_alerts.push(alert.clone());
                self.active_alerts.push(alert);
                self.last_fired.insert(rule.name.clone(), now);
            }
        }

        new_alerts
    }

    /// Check whether a condition is triggered given the current metrics.
    fn check_condition(&self, condition: &AlertCondition, metrics: &PrometheusMetrics) -> bool {
        match condition {
            AlertCondition::BlockTimeTooSlow { max_ms } => {
                // Check the histogram for block time observations exceeding max_ms
                if let Some(MetricValue::Histogram(obs)) = metrics.get("dina_block_time_seconds") {
                    let max_secs = *max_ms as f64 / 1000.0;
                    // Alert if the latest observation exceeds threshold
                    obs.last().is_some_and(|&v| v > max_secs)
                } else {
                    false
                }
            }
            AlertCondition::PeerCountLow { min } => {
                let peers = metrics.get_gauge("dina_peers_connected");
                (peers as u64) < *min
            }
            AlertCondition::MempoolFull { threshold_pct } => {
                let current = metrics.get_gauge("dina_mempool_size");
                let max = metrics.get_gauge("dina_mempool_capacity");
                if max <= 0.0 {
                    return false;
                }
                let pct = (current / max * 100.0) as u8;
                pct >= *threshold_pct
            }
            AlertCondition::ConsensusStalled { max_seconds } => {
                let last_block_ts = metrics.get_gauge("dina_last_block_timestamp");
                if last_block_ts <= 0.0 {
                    return false; // No data yet, don't alert
                }
                let now = chrono::Utc::now().timestamp() as f64;
                let age = now - last_block_ts;
                age > *max_seconds as f64
            }
            AlertCondition::DiskUsageHigh { threshold_pct } => {
                let usage = metrics.get_gauge("dina_disk_usage_pct");
                usage as u8 >= *threshold_pct
            }
            AlertCondition::ValidatorMissedBlocks { max_consecutive } => {
                let missed = metrics.get_gauge("dina_validator_missed_consecutive");
                missed as u64 >= *max_consecutive
            }
        }
    }

    /// Format a human-readable message for a triggered condition.
    fn format_message(&self, condition: &AlertCondition, metrics: &PrometheusMetrics) -> String {
        match condition {
            AlertCondition::BlockTimeTooSlow { max_ms } => {
                if let Some(MetricValue::Histogram(obs)) = metrics.get("dina_block_time_seconds") {
                    let last = obs.last().copied().unwrap_or(0.0);
                    format!(
                        "Block processing time {:.0}ms exceeds {}ms threshold",
                        last * 1000.0,
                        max_ms
                    )
                } else {
                    format!("Block processing time exceeds {}ms threshold", max_ms)
                }
            }
            AlertCondition::PeerCountLow { min } => {
                let peers = metrics.get_gauge("dina_peers_connected") as u64;
                format!("Peer count {} is below minimum {}", peers, min)
            }
            AlertCondition::MempoolFull { threshold_pct } => {
                let current = metrics.get_gauge("dina_mempool_size") as u64;
                let max = metrics.get_gauge("dina_mempool_capacity") as u64;
                format!(
                    "Mempool is {}/{}  ({}% threshold)",
                    current, max, threshold_pct
                )
            }
            AlertCondition::ConsensusStalled { max_seconds } => {
                format!("No new block in over {} seconds", max_seconds)
            }
            AlertCondition::DiskUsageHigh { threshold_pct } => {
                let usage = metrics.get_gauge("dina_disk_usage_pct");
                format!("Disk usage at {:.0}% (threshold {}%)", usage, threshold_pct)
            }
            AlertCondition::ValidatorMissedBlocks { max_consecutive } => {
                let missed = metrics.get_gauge("dina_validator_missed_consecutive") as u64;
                format!(
                    "Validator missed {} consecutive blocks (max {})",
                    missed, max_consecutive
                )
            }
        }
    }

    /// Return all currently active (unresolved) alerts.
    pub fn active_alerts(&self) -> Vec<&Alert> {
        self.active_alerts.iter().filter(|a| !a.resolved).collect()
    }

    /// Resolve all alerts for a given rule name.
    pub fn resolve(&mut self, rule_name: &str) {
        for alert in &mut self.active_alerts {
            if alert.rule_name == rule_name && !alert.resolved {
                alert.resolved = true;
            }
        }
    }

    /// Clear all resolved alerts from the list.
    pub fn clear_resolved(&mut self) {
        self.active_alerts.retain(|a| !a.resolved);
    }

    /// Return a reference to all rules.
    pub fn rules(&self) -> &[AlertRule] {
        &self.rules
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_metrics_with_peers(peer_count: f64) -> PrometheusMetrics {
        let mut m = PrometheusMetrics::new();
        m.set_gauge("dina_peers_connected", peer_count, &[]);
        m
    }

    #[test]
    fn test_default_rules_exist() {
        let am = AlertManager::new();
        assert!(am.rules().len() >= 6);
    }

    #[test]
    fn test_no_alerts_when_healthy() {
        let mut am = AlertManager::new();
        let metrics = make_metrics_with_peers(5.0);
        let alerts = am.evaluate(&metrics);
        // Only peer-related rules should be checked; peers are healthy
        let peer_alerts: Vec<_> = alerts
            .iter()
            .filter(|a| a.rule_name == "low_peers")
            .collect();
        assert!(peer_alerts.is_empty());
    }

    #[test]
    fn test_peer_count_low_alert() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "low_peers".into(),
            condition: AlertCondition::PeerCountLow { min: 1 },
            severity: Severity::Critical,
            cooldown_secs: 0,
        }]);
        let metrics = make_metrics_with_peers(0.0);
        let alerts = am.evaluate(&metrics);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_name, "low_peers");
        assert_eq!(alerts[0].severity, Severity::Critical);
    }

    #[test]
    fn test_mempool_full_alert() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "mempool_full".into(),
            condition: AlertCondition::MempoolFull { threshold_pct: 90 },
            severity: Severity::Warning,
            cooldown_secs: 0,
        }]);
        let mut metrics = PrometheusMetrics::new();
        metrics.set_gauge("dina_mempool_size", 950.0, &[]);
        metrics.set_gauge("dina_mempool_capacity", 1000.0, &[]);
        let alerts = am.evaluate(&metrics);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_name, "mempool_full");
    }

    #[test]
    fn test_mempool_ok_no_alert() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "mempool_full".into(),
            condition: AlertCondition::MempoolFull { threshold_pct: 90 },
            severity: Severity::Warning,
            cooldown_secs: 0,
        }]);
        let mut metrics = PrometheusMetrics::new();
        metrics.set_gauge("dina_mempool_size", 100.0, &[]);
        metrics.set_gauge("dina_mempool_capacity", 1000.0, &[]);
        let alerts = am.evaluate(&metrics);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_block_time_slow_alert() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "block_time_slow".into(),
            condition: AlertCondition::BlockTimeTooSlow { max_ms: 5000 },
            severity: Severity::Warning,
            cooldown_secs: 0,
        }]);
        let mut metrics = PrometheusMetrics::new();
        // 6 seconds = 6000ms, exceeds 5000ms threshold
        metrics.observe_histogram("dina_block_time_seconds", 6.0);
        let alerts = am.evaluate(&metrics);
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_block_time_ok_no_alert() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "block_time_slow".into(),
            condition: AlertCondition::BlockTimeTooSlow { max_ms: 5000 },
            severity: Severity::Warning,
            cooldown_secs: 0,
        }]);
        let mut metrics = PrometheusMetrics::new();
        metrics.observe_histogram("dina_block_time_seconds", 2.0);
        let alerts = am.evaluate(&metrics);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_disk_usage_high_alert() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "disk_high".into(),
            condition: AlertCondition::DiskUsageHigh { threshold_pct: 90 },
            severity: Severity::Warning,
            cooldown_secs: 0,
        }]);
        let mut metrics = PrometheusMetrics::new();
        metrics.set_gauge("dina_disk_usage_pct", 95.0, &[]);
        let alerts = am.evaluate(&metrics);
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_validator_missed_blocks_alert() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "validator_missed".into(),
            condition: AlertCondition::ValidatorMissedBlocks { max_consecutive: 3 },
            severity: Severity::Critical,
            cooldown_secs: 0,
        }]);
        let mut metrics = PrometheusMetrics::new();
        metrics.set_gauge("dina_validator_missed_consecutive", 5.0, &[]);
        let alerts = am.evaluate(&metrics);
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_resolve_alert() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "low_peers".into(),
            condition: AlertCondition::PeerCountLow { min: 1 },
            severity: Severity::Critical,
            cooldown_secs: 0,
        }]);
        let metrics = make_metrics_with_peers(0.0);
        am.evaluate(&metrics);
        assert_eq!(am.active_alerts().len(), 1);

        am.resolve("low_peers");
        assert!(am.active_alerts().is_empty());
    }

    #[test]
    fn test_clear_resolved() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "low_peers".into(),
            condition: AlertCondition::PeerCountLow { min: 1 },
            severity: Severity::Critical,
            cooldown_secs: 0,
        }]);
        let metrics = make_metrics_with_peers(0.0);
        am.evaluate(&metrics);
        am.resolve("low_peers");
        am.clear_resolved();
        // Internal list should be empty now
        assert!(am.active_alerts.is_empty());
    }

    #[test]
    fn test_cooldown_prevents_re_trigger() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "low_peers".into(),
            condition: AlertCondition::PeerCountLow { min: 1 },
            severity: Severity::Critical,
            cooldown_secs: 9999, // Very long cooldown
        }]);
        let metrics = make_metrics_with_peers(0.0);

        let first = am.evaluate(&metrics);
        assert_eq!(first.len(), 1);

        // Second evaluation should be blocked by cooldown
        let second = am.evaluate(&metrics);
        assert!(second.is_empty());
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Info.to_string(), "info");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Critical.to_string(), "critical");
    }

    #[test]
    fn test_add_custom_rule() {
        let mut am = AlertManager::with_rules(vec![]);
        am.add_rule(AlertRule {
            name: "custom".into(),
            condition: AlertCondition::PeerCountLow { min: 10 },
            severity: Severity::Info,
            cooldown_secs: 0,
        });
        assert_eq!(am.rules().len(), 1);

        let metrics = make_metrics_with_peers(5.0);
        let alerts = am.evaluate(&metrics);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, Severity::Info);
    }

    #[test]
    fn test_consensus_stalled_no_data_no_alert() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "consensus_stalled".into(),
            condition: AlertCondition::ConsensusStalled { max_seconds: 30 },
            severity: Severity::Critical,
            cooldown_secs: 0,
        }]);
        // No dina_last_block_timestamp set => no alert
        let metrics = PrometheusMetrics::new();
        let alerts = am.evaluate(&metrics);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_consensus_stalled_alert() {
        let mut am = AlertManager::with_rules(vec![AlertRule {
            name: "consensus_stalled".into(),
            condition: AlertCondition::ConsensusStalled { max_seconds: 30 },
            severity: Severity::Critical,
            cooldown_secs: 0,
        }]);
        let mut metrics = PrometheusMetrics::new();
        // Set last block timestamp to 120 seconds ago
        let old_ts = chrono::Utc::now().timestamp() as f64 - 120.0;
        metrics.set_gauge("dina_last_block_timestamp", old_ts, &[]);
        let alerts = am.evaluate(&metrics);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_name, "consensus_stalled");
    }

    #[test]
    fn test_multiple_alerts_same_evaluation() {
        let mut am = AlertManager::with_rules(vec![
            AlertRule {
                name: "low_peers".into(),
                condition: AlertCondition::PeerCountLow { min: 1 },
                severity: Severity::Critical,
                cooldown_secs: 0,
            },
            AlertRule {
                name: "disk_high".into(),
                condition: AlertCondition::DiskUsageHigh { threshold_pct: 90 },
                severity: Severity::Warning,
                cooldown_secs: 0,
            },
        ]);
        let mut metrics = PrometheusMetrics::new();
        metrics.set_gauge("dina_peers_connected", 0.0, &[]);
        metrics.set_gauge("dina_disk_usage_pct", 95.0, &[]);
        let alerts = am.evaluate(&metrics);
        assert_eq!(alerts.len(), 2);
    }
}
