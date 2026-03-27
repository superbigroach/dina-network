use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Represents a single metric value in Prometheus format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
}

/// Metadata attached to each metric for rendering HELP and TYPE lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricMeta {
    help: String,
    value: MetricValue,
}

/// Prometheus-compatible metrics collector that stores counters, gauges, and histograms
/// and renders them in the standard Prometheus text exposition format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusMetrics {
    metrics: BTreeMap<String, MetricMeta>,
}

impl PrometheusMetrics {
    /// Create a new empty metrics collector.
    pub fn new() -> Self {
        Self {
            metrics: BTreeMap::new(),
        }
    }

    /// Build a metric key from a base name and a set of labels.
    /// e.g. `("dina_blocks_total", &[("chain", "main")])` -> `"dina_blocks_total{chain=\"main\"}"`
    fn make_key(name: &str, labels: &[(&str, &str)]) -> String {
        if labels.is_empty() {
            name.to_string()
        } else {
            let label_str: Vec<String> = labels
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();
            format!("{}{{{}}}", name, label_str.join(","))
        }
    }

    /// Extract the base metric name from a key (strips labels).
    fn base_name(key: &str) -> &str {
        key.split('{').next().unwrap_or(key)
    }

    /// Increment a counter by 1. Creates the counter if it does not exist.
    pub fn inc_counter(&mut self, name: &str, labels: &[(&str, &str)]) {
        let key = Self::make_key(name, labels);
        let entry = self.metrics.entry(key).or_insert_with(|| MetricMeta {
            help: format!("{} counter", name),
            value: MetricValue::Counter(0),
        });
        if let MetricValue::Counter(ref mut c) = entry.value {
            *c += 1;
        }
    }

    /// Increment a counter by a specific amount. Creates the counter if it does not exist.
    pub fn inc_counter_by(&mut self, name: &str, labels: &[(&str, &str)], amount: u64) {
        let key = Self::make_key(name, labels);
        let entry = self.metrics.entry(key).or_insert_with(|| MetricMeta {
            help: format!("{} counter", name),
            value: MetricValue::Counter(0),
        });
        if let MetricValue::Counter(ref mut c) = entry.value {
            *c += amount;
        }
    }

    /// Set a gauge to a specific value. Creates the gauge if it does not exist.
    pub fn set_gauge(&mut self, name: &str, value: f64, labels: &[(&str, &str)]) {
        let key = Self::make_key(name, labels);
        let entry = self.metrics.entry(key).or_insert_with(|| MetricMeta {
            help: format!("{} gauge", name),
            value: MetricValue::Gauge(0.0),
        });
        entry.value = MetricValue::Gauge(value);
    }

    /// Record an observation into a histogram. Creates the histogram if it does not exist.
    pub fn observe_histogram(&mut self, name: &str, value: f64) {
        let key = name.to_string();
        let entry = self.metrics.entry(key).or_insert_with(|| MetricMeta {
            help: format!("{} histogram", name),
            value: MetricValue::Histogram(Vec::new()),
        });
        if let MetricValue::Histogram(ref mut observations) = entry.value {
            observations.push(value);
        }
    }

    /// Retrieve the current value of a metric by its full key (name + labels).
    pub fn get(&self, name: &str) -> Option<&MetricValue> {
        self.metrics.get(name).map(|m| &m.value)
    }

    /// Retrieve a counter value by name (no labels). Returns 0 if not found.
    pub fn get_counter(&self, name: &str) -> u64 {
        match self.metrics.get(name).map(|m| &m.value) {
            Some(MetricValue::Counter(c)) => *c,
            _ => 0,
        }
    }

    /// Retrieve a gauge value by name (no labels). Returns 0.0 if not found.
    pub fn get_gauge(&self, name: &str) -> f64 {
        match self.metrics.get(name).map(|m| &m.value) {
            Some(MetricValue::Gauge(g)) => *g,
            _ => 0.0,
        }
    }

    /// Render all metrics in the Prometheus text exposition format.
    ///
    /// Each metric group includes a `# HELP` line, a `# TYPE` line, and one or more
    /// sample lines. Histograms are rendered with standard `_bucket`, `_sum`, and `_count`
    /// suffixes using default bucket boundaries.
    pub fn render(&self) -> String {
        let mut output = String::new();
        // Track which base names we've already emitted HELP/TYPE for
        let mut emitted_bases: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Default histogram bucket boundaries
        let default_buckets: &[f64] = &[
            0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ];

        for (key, meta) in &self.metrics {
            let base = Self::base_name(key).to_string();

            match &meta.value {
                MetricValue::Counter(val) => {
                    if emitted_bases.insert(base.clone()) {
                        output.push_str(&format!("# HELP {} {}\n", base, meta.help));
                        output.push_str(&format!("# TYPE {} counter\n", base));
                    }
                    output.push_str(&format!("{} {}\n", key, val));
                }
                MetricValue::Gauge(val) => {
                    if emitted_bases.insert(base.clone()) {
                        output.push_str(&format!("# HELP {} {}\n", base, meta.help));
                        output.push_str(&format!("# TYPE {} gauge\n", base));
                    }
                    output.push_str(&format!("{} {}\n", key, val));
                }
                MetricValue::Histogram(observations) => {
                    if emitted_bases.insert(base.clone()) {
                        output.push_str(&format!("# HELP {} {}\n", base, meta.help));
                        output.push_str(&format!("# TYPE {} histogram\n", base));
                    }

                    let count = observations.len() as u64;
                    let sum: f64 = observations.iter().sum();

                    // Emit bucket lines
                    for bucket in default_buckets {
                        let bucket_count = observations.iter().filter(|&&v| v <= *bucket).count();
                        output.push_str(&format!(
                            "{}_bucket{{le=\"{}\"}} {}\n",
                            base, bucket, bucket_count
                        ));
                    }
                    // +Inf bucket
                    output.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", base, count));
                    output.push_str(&format!("{}_sum {}\n", base, sum));
                    output.push_str(&format!("{}_count {}\n", base, count));
                }
            }
        }

        output
    }
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PrometheusMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_metrics_empty() {
        let metrics = PrometheusMetrics::new();
        assert!(metrics.render().is_empty());
    }

    #[test]
    fn test_inc_counter_no_labels() {
        let mut metrics = PrometheusMetrics::new();
        metrics.inc_counter("dina_blocks_total", &[]);
        metrics.inc_counter("dina_blocks_total", &[]);
        metrics.inc_counter("dina_blocks_total", &[]);
        assert_eq!(metrics.get_counter("dina_blocks_total"), 3);
    }

    #[test]
    fn test_inc_counter_with_labels() {
        let mut metrics = PrometheusMetrics::new();
        metrics.inc_counter("dina_tx_total", &[("type", "transfer")]);
        metrics.inc_counter("dina_tx_total", &[("type", "transfer")]);
        metrics.inc_counter("dina_tx_total", &[("type", "deploy")]);

        let key_transfer = r#"dina_tx_total{type="transfer"}"#;
        let key_deploy = r#"dina_tx_total{type="deploy"}"#;

        match metrics.get(key_transfer) {
            Some(MetricValue::Counter(c)) => assert_eq!(*c, 2),
            _ => panic!("Expected counter for transfer"),
        }
        match metrics.get(key_deploy) {
            Some(MetricValue::Counter(c)) => assert_eq!(*c, 1),
            _ => panic!("Expected counter for deploy"),
        }
    }

    #[test]
    fn test_inc_counter_by() {
        let mut metrics = PrometheusMetrics::new();
        metrics.inc_counter_by("dina_bytes_received", &[], 1024);
        metrics.inc_counter_by("dina_bytes_received", &[], 2048);
        assert_eq!(metrics.get_counter("dina_bytes_received"), 3072);
    }

    #[test]
    fn test_set_gauge() {
        let mut metrics = PrometheusMetrics::new();
        metrics.set_gauge("dina_peers_connected", 5.0, &[]);
        assert!((metrics.get_gauge("dina_peers_connected") - 5.0).abs() < f64::EPSILON);

        // Overwrite
        metrics.set_gauge("dina_peers_connected", 3.0, &[]);
        assert!((metrics.get_gauge("dina_peers_connected") - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_set_gauge_with_labels() {
        let mut metrics = PrometheusMetrics::new();
        metrics.set_gauge("dina_cpu_usage", 75.5, &[("core", "0")]);
        metrics.set_gauge("dina_cpu_usage", 60.2, &[("core", "1")]);

        let key0 = r#"dina_cpu_usage{core="0"}"#;
        let key1 = r#"dina_cpu_usage{core="1"}"#;

        match metrics.get(key0) {
            Some(MetricValue::Gauge(v)) => assert!((v - 75.5).abs() < f64::EPSILON),
            _ => panic!("Expected gauge for core 0"),
        }
        match metrics.get(key1) {
            Some(MetricValue::Gauge(v)) => assert!((v - 60.2).abs() < f64::EPSILON),
            _ => panic!("Expected gauge for core 1"),
        }
    }

    #[test]
    fn test_observe_histogram() {
        let mut metrics = PrometheusMetrics::new();
        metrics.observe_histogram("dina_block_time_seconds", 0.03);
        metrics.observe_histogram("dina_block_time_seconds", 0.07);
        metrics.observe_histogram("dina_block_time_seconds", 0.15);

        match metrics.get("dina_block_time_seconds") {
            Some(MetricValue::Histogram(obs)) => {
                assert_eq!(obs.len(), 3);
                assert!((obs[0] - 0.03).abs() < f64::EPSILON);
                assert!((obs[1] - 0.07).abs() < f64::EPSILON);
                assert!((obs[2] - 0.15).abs() < f64::EPSILON);
            }
            _ => panic!("Expected histogram"),
        }
    }

    #[test]
    fn test_render_counter() {
        let mut metrics = PrometheusMetrics::new();
        metrics.inc_counter("dina_blocks_total", &[]);
        let rendered = metrics.render();
        assert!(rendered.contains("# HELP dina_blocks_total"));
        assert!(rendered.contains("# TYPE dina_blocks_total counter"));
        assert!(rendered.contains("dina_blocks_total 1"));
    }

    #[test]
    fn test_render_gauge() {
        let mut metrics = PrometheusMetrics::new();
        metrics.set_gauge("dina_peers_connected", 5.0, &[]);
        let rendered = metrics.render();
        assert!(rendered.contains("# HELP dina_peers_connected"));
        assert!(rendered.contains("# TYPE dina_peers_connected gauge"));
        assert!(rendered.contains("dina_peers_connected 5"));
    }

    #[test]
    fn test_render_histogram_buckets() {
        let mut metrics = PrometheusMetrics::new();
        metrics.observe_histogram("dina_block_time_seconds", 0.03);
        metrics.observe_histogram("dina_block_time_seconds", 0.07);
        metrics.observe_histogram("dina_block_time_seconds", 1.5);

        let rendered = metrics.render();
        assert!(rendered.contains("# TYPE dina_block_time_seconds histogram"));
        // 0.03 fits in le=0.05 bucket
        assert!(rendered.contains("dina_block_time_seconds_bucket{le=\"0.05\"} 1"));
        // 0.03 and 0.07 fit in le=0.1 bucket
        assert!(rendered.contains("dina_block_time_seconds_bucket{le=\"0.1\"} 2"));
        // All 3 fit in le=2.5 bucket
        assert!(rendered.contains("dina_block_time_seconds_bucket{le=\"2.5\"} 3"));
        // +Inf
        assert!(rendered.contains("dina_block_time_seconds_bucket{le=\"+Inf\"} 3"));
        assert!(rendered.contains("dina_block_time_seconds_count 3"));
    }

    #[test]
    fn test_render_multiple_label_variants() {
        let mut metrics = PrometheusMetrics::new();
        metrics.inc_counter("dina_requests", &[("method", "GET")]);
        metrics.inc_counter("dina_requests", &[("method", "POST")]);
        let rendered = metrics.render();
        assert!(rendered.contains(r#"dina_requests{method="GET"} 1"#));
        assert!(rendered.contains(r#"dina_requests{method="POST"} 1"#));
    }

    #[test]
    fn test_get_missing_counter_returns_zero() {
        let metrics = PrometheusMetrics::new();
        assert_eq!(metrics.get_counter("nonexistent"), 0);
    }

    #[test]
    fn test_get_missing_gauge_returns_zero() {
        let metrics = PrometheusMetrics::new();
        assert!((metrics.get_gauge("nonexistent")).abs() < f64::EPSILON);
    }

    #[test]
    fn test_display_impl() {
        let mut metrics = PrometheusMetrics::new();
        metrics.inc_counter("test_counter", &[]);
        let display = format!("{}", metrics);
        assert!(display.contains("test_counter 1"));
    }

    #[test]
    fn test_default_impl() {
        let metrics = PrometheusMetrics::default();
        assert!(metrics.render().is_empty());
    }
}
