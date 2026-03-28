use std::net::SocketAddr;

use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::jsonrpc::{self, NodeState};
use crate::rest;

/// Configuration for the RPC server (both JSON-RPC and REST).
///
/// # TLS
///
/// For production deployments, TLS should be enabled by providing paths to a
/// PEM-encoded certificate chain and private key via `tls_cert_path` and
/// `tls_key_path`. When both are provided, the REST API server will listen
/// over HTTPS using `rustls`.
///
/// **Note**: The JSON-RPC (jsonrpsee) server does not directly support TLS in
/// this configuration. For production, it is recommended to terminate TLS at
/// a reverse proxy (nginx, envoy, caddy) that fronts both endpoints, or to
/// use the REST API exclusively when TLS is required end-to-end.
#[derive(Debug, Clone)]
pub struct RpcConfig {
    /// Bind address for the JSON-RPC server (e.g., "127.0.0.1:8545").
    pub jsonrpc_bind: String,
    /// Bind address for the REST API server (e.g., "0.0.0.0:8080").
    pub rest_bind: String,
    /// Optional path to a PEM-encoded TLS certificate chain for the REST API.
    pub tls_cert_path: Option<String>,
    /// Optional path to a PEM-encoded TLS private key for the REST API.
    pub tls_key_path: Option<String>,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            jsonrpc_bind: "127.0.0.1:8545".to_string(),
            rest_bind: "127.0.0.1:8080".to_string(),
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

/// Manages both the JSON-RPC and REST API servers.
pub struct RpcServer {
    config: RpcConfig,
    state: NodeState,
}

impl RpcServer {
    /// Create a new RPC server with the given configuration and node state.
    pub fn new(config: RpcConfig, state: NodeState) -> Self {
        Self { config, state }
    }

    /// Start both the JSON-RPC and REST servers.
    ///
    /// Returns handles for the JSON-RPC server and the REST server task.
    ///
    /// When `tls_cert_path` and `tls_key_path` are both set in the config,
    /// the REST API will serve over HTTPS. Otherwise, it falls back to
    /// plaintext HTTP.
    pub async fn start(
        self,
    ) -> Result<
        (jsonrpsee::server::ServerHandle, JoinHandle<()>),
        Box<dyn std::error::Error + Send + Sync>,
    > {
        // Start JSON-RPC server (plaintext — use a reverse proxy for TLS)
        if self.config.tls_cert_path.is_some() || self.config.tls_key_path.is_some() {
            warn!(
                "TLS is configured for the REST API but the JSON-RPC server \
                 (jsonrpsee) will run without TLS. Use a reverse proxy (nginx, \
                 envoy, caddy) to terminate TLS for the JSON-RPC endpoint."
            );
        }

        let jsonrpc_handle =
            jsonrpc::start_jsonrpc_server(self.state.clone(), &self.config.jsonrpc_bind).await?;

        // Start REST server (with optional TLS)
        let rest_bind = self.config.rest_bind.clone();
        let rest_state = self.state.clone();
        let tls_cert_path = self.config.tls_cert_path.clone();
        let tls_key_path = self.config.tls_key_path.clone();

        let rest_handle = tokio::spawn(async move {
            let router = rest::rest_router(rest_state);
            let addr: SocketAddr = match rest_bind.parse() {
                Ok(a) => a,
                Err(e) => {
                    error!("invalid REST bind address '{}': {}", rest_bind, e);
                    return;
                }
            };

            // If both TLS cert and key are provided, serve over HTTPS.
            if let (Some(cert_path), Some(key_path)) = (tls_cert_path, tls_key_path) {
                let tls_config = match axum_server::tls_rustls::RustlsConfig::from_pem_file(
                    &cert_path, &key_path,
                )
                .await
                {
                    Ok(c) => c,
                    Err(e) => {
                        error!(
                            "failed to load TLS certificate/key ({}, {}): {}",
                            cert_path, key_path, e
                        );
                        return;
                    }
                };

                info!("REST API server listening on {} (HTTPS/TLS)", addr);

                if let Err(e) = axum_server::bind_rustls(addr, tls_config)
                    .serve(router.into_make_service())
                    .await
                {
                    error!("REST server (TLS) error: {}", e);
                }
            } else {
                // Plaintext HTTP fallback.
                let listener = match tokio::net::TcpListener::bind(addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        error!("failed to bind REST server to {}: {}", addr, e);
                        return;
                    }
                };

                info!("REST API server listening on {} (HTTP)", addr);

                if let Err(e) = axum::serve(listener, router).await {
                    error!("REST server error: {}", e);
                }
            }
        });

        Ok((jsonrpc_handle, rest_handle))
    }
}
