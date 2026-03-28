use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use tokio::sync::RwLock;

use crate::alerting::AlertManager;
use crate::health::HealthChecker;
use crate::prometheus::PrometheusMetrics;

/// Shared state for the monitoring dashboard.
#[derive(Clone)]
pub struct DashboardState {
    pub metrics: Arc<RwLock<PrometheusMetrics>>,
    pub health: Arc<RwLock<HealthChecker>>,
    pub alerts: Arc<RwLock<AlertManager>>,
}

/// The monitoring dashboard provides HTTP endpoints for metrics, health checks,
/// and alerts. It is designed to integrate with Prometheus, Kubernetes probes,
/// and alerting systems.
pub struct MonitoringDashboard {
    state: DashboardState,
}

impl MonitoringDashboard {
    /// Create a new monitoring dashboard with default-initialized components.
    pub fn new() -> Self {
        Self {
            state: DashboardState {
                metrics: Arc::new(RwLock::new(PrometheusMetrics::new())),
                health: Arc::new(RwLock::new(HealthChecker::new("0.1.0"))),
                alerts: Arc::new(RwLock::new(AlertManager::new())),
            },
        }
    }

    /// Create a dashboard from existing shared state components.
    pub fn from_parts(
        metrics: Arc<RwLock<PrometheusMetrics>>,
        health: Arc<RwLock<HealthChecker>>,
        alerts: Arc<RwLock<AlertManager>>,
    ) -> Self {
        Self {
            state: DashboardState {
                metrics,
                health,
                alerts,
            },
        }
    }

    /// Get a reference to the shared metrics.
    pub fn metrics(&self) -> &Arc<RwLock<PrometheusMetrics>> {
        &self.state.metrics
    }

    /// Get a reference to the shared health checker.
    pub fn health(&self) -> &Arc<RwLock<HealthChecker>> {
        &self.state.health
    }

    /// Get a reference to the shared alert manager.
    pub fn alerts(&self) -> &Arc<RwLock<AlertManager>> {
        &self.state.alerts
    }

    /// Build the axum Router with all monitoring endpoints.
    ///
    /// Routes:
    /// - `GET /metrics`       — Prometheus text format metrics
    /// - `GET /health`        — Full JSON health report
    /// - `GET /health/live`   — Kubernetes liveness probe (200 or 503)
    /// - `GET /health/ready`  — Kubernetes readiness probe (200 or 503)
    /// - `GET /alerts`        — Active alerts as JSON
    /// - `GET /dashboard`     — Combined overview (metrics summary + health + alerts)
    pub fn router(&self) -> Router {
        Router::new()
            .route("/metrics", get(handle_metrics))
            .route("/health", get(handle_health))
            .route("/health/live", get(handle_liveness))
            .route("/health/ready", get(handle_readiness))
            .route("/alerts", get(handle_alerts))
            .route("/dashboard", get(handle_dashboard))
            .with_state(self.state.clone())
    }
}

impl Default for MonitoringDashboard {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /metrics — renders all metrics in Prometheus text exposition format.
async fn handle_metrics(State(state): State<DashboardState>) -> Response {
    let metrics = state.metrics.read().await;
    let body = metrics.render();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
        .into_response()
}

/// GET /health — returns a full JSON health report.
async fn handle_health(State(state): State<DashboardState>) -> Response {
    let health = state.health.read().await;
    let report = health.report();
    let status_code = match report.status {
        crate::health::OverallStatus::Healthy => StatusCode::OK,
        crate::health::OverallStatus::Degraded => StatusCode::OK,
        crate::health::OverallStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };
    (status_code, Json(report)).into_response()
}

/// GET /health/live — simple liveness probe for Kubernetes.
async fn handle_liveness(State(state): State<DashboardState>) -> Response {
    let health = state.health.read().await;
    if health.is_live() {
        (StatusCode::OK, Json(serde_json::json!({"status": "live"}))).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "not live"})),
        )
            .into_response()
    }
}

/// GET /health/ready — simple readiness probe for Kubernetes.
async fn handle_readiness(State(state): State<DashboardState>) -> Response {
    let health = state.health.read().await;
    if health.is_ready() {
        (StatusCode::OK, Json(serde_json::json!({"status": "ready"}))).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "not ready"})),
        )
            .into_response()
    }
}

/// GET /alerts — returns all active (unresolved) alerts as JSON.
async fn handle_alerts(State(state): State<DashboardState>) -> Response {
    let alerts = state.alerts.read().await;
    let active: Vec<_> = alerts.active_alerts().into_iter().cloned().collect();
    (StatusCode::OK, Json(active)).into_response()
}

/// GET /dashboard — combined overview with metrics summary, health, and alerts.
async fn handle_dashboard(State(state): State<DashboardState>) -> Response {
    let metrics = state.metrics.read().await;
    let health = state.health.read().await;
    let alerts = state.alerts.read().await;

    let report = health.report();
    let active_alerts: Vec<_> = alerts.active_alerts().into_iter().cloned().collect();

    let dashboard = serde_json::json!({
        "health": report,
        "alerts": {
            "active_count": active_alerts.len(),
            "alerts": active_alerts,
        },
        "metrics_summary": {
            "blocks_total": metrics.get_counter("dina_blocks_total"),
            "peers_connected": metrics.get_gauge("dina_peers_connected"),
            "mempool_size": metrics.get_gauge("dina_mempool_size"),
            "disk_usage_pct": metrics.get_gauge("dina_disk_usage_pct"),
        },
    });

    (StatusCode::OK, Json(dashboard)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;

    async fn body_string(response: Response) -> String {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    fn get_request(uri: &str) -> Request<Body> {
        Request::get(uri).body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn test_metrics_endpoint_empty() {
        let dashboard = MonitoringDashboard::new();
        let app = dashboard.router();

        let response = app.oneshot(get_request("/metrics")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(content_type.contains("text/plain"));
    }

    #[tokio::test]
    async fn test_metrics_endpoint_with_data() {
        let dashboard = MonitoringDashboard::new();
        {
            let mut metrics = dashboard.metrics().write().await;
            metrics.inc_counter("dina_blocks_total", &[]);
            metrics.set_gauge("dina_peers_connected", 3.0, &[]);
        }
        let app = dashboard.router();

        let response = app.oneshot(get_request("/metrics")).await.unwrap();

        let body = body_string(response).await;
        assert!(body.contains("dina_blocks_total 1"));
        assert!(body.contains("dina_peers_connected 3"));
    }

    #[tokio::test]
    async fn test_health_endpoint_healthy() {
        let dashboard = MonitoringDashboard::new();
        let app = dashboard.router();

        let response = app.oneshot(get_request("/health")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("Healthy"));
    }

    #[tokio::test]
    async fn test_health_endpoint_unhealthy() {
        let dashboard = MonitoringDashboard::new();
        {
            let mut health = dashboard.health().write().await;
            health.add_check(Box::new(crate::health::StorageHealthCheck::new(|| false)));
        }
        let app = dashboard.router();

        let response = app.oneshot(get_request("/health")).await.unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_liveness_endpoint() {
        let dashboard = MonitoringDashboard::new();
        let app = dashboard.router();

        let response = app.oneshot(get_request("/health/live")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("live"));
    }

    #[tokio::test]
    async fn test_readiness_endpoint_ready() {
        let dashboard = MonitoringDashboard::new();
        let app = dashboard.router();

        let response = app.oneshot(get_request("/health/ready")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("ready"));
    }

    #[tokio::test]
    async fn test_readiness_endpoint_not_ready() {
        let dashboard = MonitoringDashboard::new();
        {
            let mut health = dashboard.health().write().await;
            health.add_check(Box::new(crate::health::StorageHealthCheck::new(|| false)));
        }
        let app = dashboard.router();

        let response = app.oneshot(get_request("/health/ready")).await.unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = body_string(response).await;
        assert!(body.contains("not ready"));
    }

    #[tokio::test]
    async fn test_alerts_endpoint_empty() {
        let dashboard = MonitoringDashboard::new();
        let app = dashboard.router();

        let response = app.oneshot(get_request("/alerts")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert_eq!(body, "[]");
    }

    #[tokio::test]
    async fn test_alerts_endpoint_with_alerts() {
        let dashboard = MonitoringDashboard::new();
        // Set peers to 0 so low_peers alert triggers
        {
            let mut metrics = dashboard.metrics().write().await;
            metrics.set_gauge("dina_peers_connected", 0.0, &[]);
        }
        {
            let metrics = dashboard.metrics().read().await;
            let mut alerts = dashboard.alerts().write().await;
            alerts.evaluate(&metrics);
        }

        let app = dashboard.router();
        let response = app.oneshot(get_request("/alerts")).await.unwrap();

        let body = body_string(response).await;
        assert!(body.contains("low_peers"));
    }

    #[tokio::test]
    async fn test_dashboard_endpoint() {
        let dashboard = MonitoringDashboard::new();
        {
            let mut metrics = dashboard.metrics().write().await;
            metrics.inc_counter("dina_blocks_total", &[]);
            metrics.set_gauge("dina_peers_connected", 5.0, &[]);
        }
        let app = dashboard.router();

        let response = app.oneshot(get_request("/dashboard")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("health"));
        assert!(body.contains("alerts"));
        assert!(body.contains("metrics_summary"));
    }

    #[tokio::test]
    async fn test_from_parts_constructor() {
        let metrics = Arc::new(RwLock::new(PrometheusMetrics::new()));
        let health = Arc::new(RwLock::new(HealthChecker::new("2.0.0")));
        let alerts = Arc::new(RwLock::new(AlertManager::new()));

        let dashboard =
            MonitoringDashboard::from_parts(metrics.clone(), health.clone(), alerts.clone());

        let app = dashboard.router();
        let response = app.oneshot(get_request("/health")).await.unwrap();

        let body = body_string(response).await;
        assert!(body.contains("2.0.0"));
    }

    #[tokio::test]
    async fn test_default_impl() {
        let dashboard = MonitoringDashboard::default();
        let app = dashboard.router();
        let response = app.oneshot(get_request("/health/live")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
