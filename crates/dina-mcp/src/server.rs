use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info};

use crate::error::McpError;
use crate::handler::McpHandler;
use crate::tools::{McpTool, McpToolCall};

/// MCP protocol request envelope.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum McpRequest {
    /// List all available tools.
    #[serde(rename = "tools/list")]
    ListTools,
    /// Execute a tool call.
    #[serde(rename = "tools/call")]
    CallTool {
        /// The tool call parameters.
        params: McpToolCall,
    },
    /// Ping to check server health.
    #[serde(rename = "ping")]
    Ping,
}

/// MCP protocol response envelope.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC style id (echoed from request if provided).
    pub id: Option<serde_json::Value>,
    /// The response payload.
    pub result: serde_json::Value,
}

/// MCP server that listens for tool calls from Cognitum Seeds and routes
/// them to the Dina Network node via the McpHandler.
#[derive(Clone)]
pub struct McpServer {
    handler: Arc<McpHandler>,
    bind_addr: String,
}

impl McpServer {
    /// Create a new MCP server with the given handler and bind address.
    ///
    /// # Arguments
    /// * `handler` - The MCP handler that processes tool calls.
    /// * `bind_addr` - The address to bind the HTTP server to (e.g., "127.0.0.1:3100").
    pub fn new(handler: McpHandler, bind_addr: String) -> Self {
        Self {
            handler: Arc::new(handler),
            bind_addr,
        }
    }

    /// Start the MCP server. This method runs until the server is shut down.
    ///
    /// The server exposes:
    /// - `POST /mcp` - Handle MCP protocol messages (tools/list, tools/call, ping).
    /// - `GET /health` - Health check endpoint.
    /// - `GET /tools` - List available tools (convenience endpoint).
    pub async fn start(&self) -> Result<(), McpError> {
        let app = Self::build_router(self.handler.clone());

        let addr: SocketAddr = self
            .bind_addr
            .parse()
            .map_err(|e| McpError::ServerError(format!("invalid bind address '{}': {e}", self.bind_addr)))?;

        info!("MCP server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| McpError::ServerError(format!("failed to bind to {addr}: {e}")))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| McpError::ServerError(format!("server error: {e}")))?;

        Ok(())
    }

    /// Build the Axum router with all MCP endpoints.
    fn build_router(handler: Arc<McpHandler>) -> Router {
        Router::new()
            .route("/mcp", post(handle_mcp_request))
            .route("/health", get(handle_health))
            .route("/tools", get(handle_list_tools))
            .with_state(handler)
    }
}

/// Handle an incoming MCP protocol request on `POST /mcp`.
///
/// Accepts a JSON body conforming to the MCP protocol and routes to the
/// appropriate handler based on the `method` field.
async fn handle_mcp_request(
    State(handler): State<Arc<McpHandler>>,
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<McpResponse>) {
    // Extract the optional id field for JSON-RPC style correlation.
    let id = body.get("id").cloned();

    // Parse the method and route accordingly.
    let method = body.get("method").and_then(|m| m.as_str()).unwrap_or("");

    let result = match method {
        "tools/list" => {
            let tools = McpHandler::available_tools();
            json!({ "tools": tools })
        }
        "tools/call" => {
            let params = match body.get("params") {
                Some(p) => p,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(McpResponse {
                            id,
                            result: json!({
                                "error": "missing 'params' for tools/call"
                            }),
                        }),
                    );
                }
            };

            let tool_call: McpToolCall = match serde_json::from_value(params.clone()) {
                Ok(tc) => tc,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(McpResponse {
                            id,
                            result: json!({
                                "error": format!("invalid tool call params: {e}")
                            }),
                        }),
                    );
                }
            };

            let tool_result = handler.handle_tool_call(tool_call).await;
            serde_json::to_value(&tool_result).unwrap_or(json!({"error": "serialization failed"}))
        }
        "ping" => {
            json!({ "status": "ok" })
        }
        _ => {
            error!(method = %method, "unknown MCP method");
            return (
                StatusCode::BAD_REQUEST,
                Json(McpResponse {
                    id,
                    result: json!({
                        "error": format!("unknown method: {method}")
                    }),
                }),
            );
        }
    };

    (StatusCode::OK, Json(McpResponse { id, result }))
}

/// Handle GET /health for health checks.
async fn handle_health() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok", "service": "dina-mcp" })))
}

/// Handle GET /tools to list available MCP tools (convenience endpoint).
async fn handle_list_tools() -> (StatusCode, Json<Vec<McpTool>>) {
    (StatusCode::OK, Json(McpHandler::available_tools()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_server_creation() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let server = McpServer::new(handler, "127.0.0.1:3100".to_string());
        assert_eq!(server.bind_addr, "127.0.0.1:3100");
    }

    #[test]
    fn mcp_request_list_tools_deserialize() {
        let json_str = r#"{"method": "tools/list"}"#;
        let request: McpRequest = serde_json::from_str(json_str).unwrap();
        assert!(matches!(request, McpRequest::ListTools));
    }

    #[test]
    fn mcp_request_call_tool_deserialize() {
        let json_str = r#"{
            "method": "tools/call",
            "params": {
                "tool_name": "dina/balance",
                "arguments": {}
            }
        }"#;
        let request: McpRequest = serde_json::from_str(json_str).unwrap();
        assert!(matches!(request, McpRequest::CallTool { .. }));
    }

    #[test]
    fn mcp_request_ping_deserialize() {
        let json_str = r#"{"method": "ping"}"#;
        let request: McpRequest = serde_json::from_str(json_str).unwrap();
        assert!(matches!(request, McpRequest::Ping));
    }

    #[test]
    fn mcp_response_serialize() {
        let response = McpResponse {
            id: Some(json!(1)),
            result: json!({"status": "ok"}),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains("\"id\":1"));
        assert!(serialized.contains("\"status\":\"ok\""));
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let (status, Json(body)) = handle_health().await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["service"], "dina-mcp");
    }

    #[tokio::test]
    async fn list_tools_endpoint_returns_tools() {
        let (status, Json(tools)) = handle_list_tools().await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(tools.len(), 12);
    }

    #[tokio::test]
    async fn router_builds_without_panic() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let _router = McpServer::build_router(Arc::new(handler));
    }
}
