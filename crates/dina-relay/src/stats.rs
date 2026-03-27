//! Relay statistics tracking — records relay activity for display in
//! the integrating app's UI (e.g., "23 relays this month, $0.0023 earned").

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tracks cumulative relay statistics for the local device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayStats {
    /// Total number of blobs successfully relayed.
    pub total_relayed: u64,
    /// Total relay fees earned in micro-USDC (1 USDC = 1_000_000).
    pub total_fees_earned: u64,
    /// Total bytes transmitted across all relays.
    pub bytes_transmitted: u64,
    /// Unix timestamp of the first relay (0 if none).
    pub first_relay_at: u64,
    /// Unix timestamp of the most recent relay (0 if none).
    pub last_relay_at: u64,
    /// Number of relays grouped by day (key format: "YYYY-MM-DD").
    pub relays_by_day: BTreeMap<String, u64>,
}

impl RelayStats {
    /// Create a new empty stats tracker.
    pub fn new() -> Self {
        Self {
            total_relayed: 0,
            total_fees_earned: 0,
            bytes_transmitted: 0,
            first_relay_at: 0,
            last_relay_at: 0,
            relays_by_day: BTreeMap::new(),
        }
    }

    /// Record a successful relay, updating all counters.
    ///
    /// # Arguments
    /// * `fee` - Relay fee earned in micro-USDC
    /// * `bytes` - Number of bytes transmitted for this relay
    pub fn record_relay(&mut self, fee: u64, bytes: usize) {
        self.total_relayed += 1;
        self.total_fees_earned += fee;
        self.bytes_transmitted += bytes as u64;

        let now = current_unix_timestamp();

        if self.first_relay_at == 0 {
            self.first_relay_at = now;
        }
        self.last_relay_at = now;

        let day_key = unix_to_date_string(now);
        *self.relays_by_day.entry(day_key).or_insert(0) += 1;
    }

    /// Return the total number of blobs relayed.
    pub fn total_relayed(&self) -> u64 {
        self.total_relayed
    }

    /// Return the total relay fees earned in micro-USDC.
    pub fn total_fees_earned(&self) -> u64 {
        self.total_fees_earned
    }

    /// Return the number of relays performed today.
    pub fn relays_today(&self) -> u64 {
        let today = unix_to_date_string(current_unix_timestamp());
        self.relays_by_day.get(&today).copied().unwrap_or(0)
    }

    /// Format stats into a human-readable display string.
    ///
    /// Example: "23 relays | $0.0023 earned | 4.6 KB transmitted"
    pub fn to_display_string(&self) -> String {
        let fees_usdc = self.total_fees_earned as f64 / 1_000_000.0;
        let bytes_display = format_bytes(self.bytes_transmitted);

        // Count relays in the current calendar month
        let now = current_unix_timestamp();
        let month_prefix = unix_to_month_prefix(now);
        let monthly_relays: u64 = self
            .relays_by_day
            .iter()
            .filter(|(k, _)| k.starts_with(&month_prefix))
            .map(|(_, v)| v)
            .sum();

        format!(
            "{monthly_relays} relays this month, ${fees_usdc:.4} earned, {bytes_display} transmitted"
        )
    }

    /// Return the number of distinct days with relay activity.
    pub fn active_days(&self) -> usize {
        self.relays_by_day.len()
    }

    /// Return average relays per active day.
    pub fn avg_relays_per_day(&self) -> f64 {
        if self.relays_by_day.is_empty() {
            return 0.0;
        }
        self.total_relayed as f64 / self.relays_by_day.len() as f64
    }
}

impl Default for RelayStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the current Unix timestamp in seconds.
fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Convert a Unix timestamp to a "YYYY-MM-DD" date string.
fn unix_to_date_string(timestamp: u64) -> String {
    // Calculate date components from Unix timestamp
    // Days since epoch
    let days = (timestamp / 86400) as i64;

    // Algorithm from Howard Hinnant's chrono-compatible date library
    // https://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}")
}

/// Extract the "YYYY-MM" prefix from a Unix timestamp.
fn unix_to_month_prefix(timestamp: u64) -> String {
    let date = unix_to_date_string(timestamp);
    // "YYYY-MM-DD" -> "YYYY-MM"
    date[..7].to_string()
}

/// Format a byte count into a human-readable string.
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stats_are_zero() {
        let stats = RelayStats::new();
        assert_eq!(stats.total_relayed(), 0);
        assert_eq!(stats.total_fees_earned(), 0);
        assert_eq!(stats.bytes_transmitted, 0);
        assert_eq!(stats.first_relay_at, 0);
        assert_eq!(stats.last_relay_at, 0);
    }

    #[test]
    fn record_relay_increments_counters() {
        let mut stats = RelayStats::new();
        stats.record_relay(100, 256);
        assert_eq!(stats.total_relayed(), 1);
        assert_eq!(stats.total_fees_earned(), 100);
        assert_eq!(stats.bytes_transmitted, 256);
        assert!(stats.first_relay_at > 0);
        assert!(stats.last_relay_at > 0);
    }

    #[test]
    fn multiple_relays_accumulate() {
        let mut stats = RelayStats::new();
        stats.record_relay(10, 100);
        stats.record_relay(20, 200);
        stats.record_relay(30, 300);
        assert_eq!(stats.total_relayed(), 3);
        assert_eq!(stats.total_fees_earned(), 60);
        assert_eq!(stats.bytes_transmitted, 600);
    }

    #[test]
    fn display_string_format() {
        let mut stats = RelayStats::new();
        stats.record_relay(2300, 4710);
        let display = stats.to_display_string();
        assert!(display.contains("earned"), "display was: {display}");
        assert!(display.contains("transmitted"), "display was: {display}");
    }

    #[test]
    fn unix_to_date_known_values() {
        // 2023-11-14 00:00:00 UTC = 1699920000
        assert_eq!(unix_to_date_string(1699920000), "2023-11-14");
        // 2024-01-01 00:00:00 UTC = 1704067200
        assert_eq!(unix_to_date_string(1704067200), "2024-01-01");
        // Unix epoch
        assert_eq!(unix_to_date_string(0), "1970-01-01");
    }

    #[test]
    fn format_bytes_units() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1_500_000), "1.4 MB");
        assert_eq!(format_bytes(2_000_000_000), "1.9 GB");
    }

    #[test]
    fn active_days_and_avg() {
        let mut stats = RelayStats::new();
        // All relays happen "today" since we use current time
        stats.record_relay(10, 100);
        stats.record_relay(20, 200);
        assert_eq!(stats.active_days(), 1);
        assert!((stats.avg_relays_per_day() - 2.0).abs() < f64::EPSILON);
    }
}
