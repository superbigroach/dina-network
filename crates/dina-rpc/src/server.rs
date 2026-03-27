use std::net::SocketAddr;

use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::jsonrpc::{self, NodeState};
use crate::rest;

/// Configuration for the RPC server (both JSON-RPC and REST).
#[derive(Debug, Clone)]
pub struct RpcConfig {
    /// Bind address for the JSON-RPC server (e.g., "127.0.0.1:8545").
    pub jsonrpc_bind: String,
    /// Bind address for the REST API server (e.g., "0.0.0.0:8080").
    pub rest_bind: String,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            jsonrpc_bind: "127.0.0.1:8545".to_string(),
            rest_bind: "0.0.0.0:8080".to_string(),
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
    pub async fn start(
        self,
    ) -> Result<(jsonrpsee::server::ServerHandle, JoinHandle<()>), Box<dyn std::error::Error + Send + Sync>>
    {
        // Start JSON-RPC server
        let jsonrpc_handle =
            jsonrpc::start_jsonrpc_server(self.state.clone(), &self.config.jsonrpc_bind).await?;

        // Start REST server
        let rest_bind = self.config.rest_bind.clone();
        let rest_state = self.state.clone();

        let rest_handle = tokio::spawn(async move {
            let router = rest::rest_router(rest_state);
            let addr: SocketAddr = match rest_bind.parse() {
                Ok(a) => a,
                Err(e) => {
                    error!("invalid REST bind address '{}': {}", rest_bind, e);
                    return;
                }
            };

            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    error!("failed to bind REST server to {}: {}", addr, e);
                    return;
                }
            };

            info!("REST API server listening on {}", addr);

            if let Err(e) = axum::serve(listener, router).await {
                error!("REST server error: {}", e);
            }
        });

        Ok((jsonrpc_handle, rest_handle))
    }
}
