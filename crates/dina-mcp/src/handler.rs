use serde_json::json;
use tracing::{debug, info, warn};

use crate::error::{McpError, McpResult};
use crate::tools::{all_tools, McpTool, McpToolCall, McpToolResult};

/// Handles incoming MCP tool calls by routing them to the appropriate
/// JSON-RPC endpoint on the Dina Network node.
#[derive(Clone, Debug)]
pub struct McpHandler {
    /// JSON-RPC endpoint URL for the Dina node (e.g., "http://127.0.0.1:8545").
    rpc_endpoint: String,
    /// HTTP client for making JSON-RPC calls.
    client: reqwest::Client,
}

impl McpHandler {
    /// Create a new MCP handler that routes tool calls to the given RPC endpoint.
    pub fn new(rpc_endpoint: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");

        Self {
            rpc_endpoint,
            client,
        }
    }

    /// Returns the list of all available MCP tools.
    pub fn available_tools() -> Vec<McpTool> {
        all_tools()
    }

    /// Process an incoming MCP tool call and return the result.
    ///
    /// Routes the call to the appropriate JSON-RPC method on the Dina node,
    /// translating between MCP tool arguments and JSON-RPC parameters.
    pub async fn handle_tool_call(&self, call: McpToolCall) -> McpToolResult {
        info!(tool = %call.tool_name, "handling MCP tool call");
        debug!(arguments = %call.arguments, "tool call arguments");

        match call.tool_name.as_str() {
            "dina/transfer" => self.handle_transfer(&call.arguments).await,
            "dina/balance" => self.handle_balance(&call.arguments).await,
            "dina/deploy_contract" => self.handle_deploy_contract(&call.arguments).await,
            "dina/call_contract" => self.handle_call_contract(&call.arguments).await,
            "dina/register_device" => self.handle_register_device(&call.arguments).await,
            "dina/verify_device" => self.handle_verify_device(&call.arguments).await,
            "dina/channel_open" => self.handle_channel_open(&call.arguments).await,
            "dina/channel_pay" => self.handle_channel_pay(&call.arguments).await,
            "dina/channel_close" => self.handle_channel_close(&call.arguments).await,
            "dina/peers" => self.handle_peers(&call.arguments).await,
            "dina/block_info" => self.handle_block_info(&call.arguments).await,
            "dina/network_status" => self.handle_network_status(&call.arguments).await,
            unknown => {
                warn!(tool = %unknown, "unknown MCP tool requested");
                McpToolResult::err(format!("unknown tool: {unknown}"))
            }
        }
    }

    /// Send a JSON-RPC request to the Dina node and return the result.
    async fn rpc_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> McpResult<serde_json::Value> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        debug!(method = %method, "sending JSON-RPC request");

        let response = self
            .client
            .post(&self.rpc_endpoint)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| McpError::RpcError(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown".to_string());
            return Err(McpError::RpcError(format!(
                "RPC returned HTTP {status}: {body}"
            )));
        }

        let rpc_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| McpError::RpcError(format!("failed to parse RPC response: {e}")))?;

        if let Some(err) = rpc_response.get("error") {
            let message = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown RPC error");
            let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            return Err(McpError::RpcError(format!(
                "RPC error {code}: {message}"
            )));
        }

        Ok(rpc_response
            .get("result")
            .cloned()
            .unwrap_or(json!(null)))
    }

    /// Handle `dina/transfer` tool call.
    async fn handle_transfer(&self, args: &serde_json::Value) -> McpToolResult {
        let to = match args.get("to").and_then(|v| v.as_str()) {
            Some(addr) => addr,
            None => return McpToolResult::err("missing required argument: 'to'"),
        };

        let amount = match args.get("amount").and_then(|v| v.as_u64()) {
            Some(amt) => amt,
            None => return McpToolResult::err("missing or invalid argument: 'amount'"),
        };

        let memo = args.get("memo").and_then(|v| v.as_str());
        let fee = args.get("fee").and_then(|v| v.as_u64()).unwrap_or(10);

        let mut params = json!({
            "to": to,
            "amount": amount,
            "fee": fee,
        });
        if let Some(m) = memo {
            params["memo"] = json!(m);
        }

        match self.rpc_call("dina_sendTransfer", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/balance` tool call.
    async fn handle_balance(&self, args: &serde_json::Value) -> McpToolResult {
        let params = if let Some(addr) = args.get("address").and_then(|v| v.as_str()) {
            json!({ "address": addr })
        } else {
            json!({})
        };

        match self.rpc_call("dina_getBalance", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/deploy_contract` tool call.
    async fn handle_deploy_contract(&self, args: &serde_json::Value) -> McpToolResult {
        let wasm_bytecode = match args.get("wasm_bytecode").and_then(|v| v.as_str()) {
            Some(code) => code,
            None => return McpToolResult::err("missing required argument: 'wasm_bytecode'"),
        };

        let init_args = args
            .get("init_args")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fee = args.get("fee").and_then(|v| v.as_u64()).unwrap_or(100);

        let params = json!({
            "wasm_bytecode": wasm_bytecode,
            "init_args": init_args,
            "fee": fee,
        });

        match self.rpc_call("dina_deployContract", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/call_contract` tool call.
    async fn handle_call_contract(&self, args: &serde_json::Value) -> McpToolResult {
        let contract = match args.get("contract").and_then(|v| v.as_str()) {
            Some(addr) => addr,
            None => return McpToolResult::err("missing required argument: 'contract'"),
        };

        let method = match args.get("method").and_then(|v| v.as_str()) {
            Some(m) => m,
            None => return McpToolResult::err("missing required argument: 'method'"),
        };

        let call_args = args
            .get("args")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let usdc_attached = args.get("usdc_attached").and_then(|v| v.as_u64()).unwrap_or(0);
        let fee = args.get("fee").and_then(|v| v.as_u64()).unwrap_or(10);

        let params = json!({
            "contract": contract,
            "method": method,
            "args": call_args,
            "usdc_attached": usdc_attached,
            "fee": fee,
        });

        match self.rpc_call("dina_callContract", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/register_device` tool call.
    async fn handle_register_device(&self, args: &serde_json::Value) -> McpToolResult {
        let device_pubkey = match args.get("device_pubkey").and_then(|v| v.as_str()) {
            Some(pk) => pk,
            None => return McpToolResult::err("missing required argument: 'device_pubkey'"),
        };

        let owner = match args.get("owner").and_then(|v| v.as_str()) {
            Some(addr) => addr,
            None => return McpToolResult::err("missing required argument: 'owner'"),
        };

        let firmware_hash = match args.get("firmware_hash").and_then(|v| v.as_str()) {
            Some(h) => h,
            None => return McpToolResult::err("missing required argument: 'firmware_hash'"),
        };

        let attestation_signature =
            match args.get("attestation_signature").and_then(|v| v.as_str()) {
                Some(sig) => sig,
                None => {
                    return McpToolResult::err(
                        "missing required argument: 'attestation_signature'",
                    )
                }
            };

        let witness_root = args
            .get("witness_root")
            .and_then(|v| v.as_str())
            .unwrap_or("0000000000000000000000000000000000000000000000000000000000000000");
        let fee = args.get("fee").and_then(|v| v.as_u64()).unwrap_or(100);

        let params = json!({
            "device_pubkey": device_pubkey,
            "owner": owner,
            "firmware_hash": firmware_hash,
            "witness_root": witness_root,
            "attestation_signature": attestation_signature,
            "fee": fee,
        });

        match self.rpc_call("dina_registerDevice", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/verify_device` tool call.
    async fn handle_verify_device(&self, args: &serde_json::Value) -> McpToolResult {
        let device_id = match args.get("device_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return McpToolResult::err("missing required argument: 'device_id'"),
        };

        let mut params = json!({ "device_id": device_id });

        if let Some(pk) = args.get("attestation_pubkey").and_then(|v| v.as_str()) {
            params["attestation_pubkey"] = json!(pk);
        }
        if let Some(fh) = args.get("firmware_hash").and_then(|v| v.as_str()) {
            params["firmware_hash"] = json!(fh);
        }
        if let Some(wr) = args.get("witness_root").and_then(|v| v.as_str()) {
            params["witness_root"] = json!(wr);
        }
        if let Some(sig) = args.get("attestation_signature").and_then(|v| v.as_str()) {
            params["attestation_signature"] = json!(sig);
        }

        match self.rpc_call("dina_verifyDevice", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/channel_open` tool call.
    async fn handle_channel_open(&self, args: &serde_json::Value) -> McpToolResult {
        let counterparty = match args.get("counterparty").and_then(|v| v.as_str()) {
            Some(addr) => addr,
            None => return McpToolResult::err("missing required argument: 'counterparty'"),
        };

        let deposit = match args.get("deposit").and_then(|v| v.as_u64()) {
            Some(d) => d,
            None => return McpToolResult::err("missing or invalid argument: 'deposit'"),
        };

        let fee = args.get("fee").and_then(|v| v.as_u64()).unwrap_or(10);

        let params = json!({
            "counterparty": counterparty,
            "deposit": deposit,
            "fee": fee,
        });

        match self.rpc_call("dina_channelOpen", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/channel_pay` tool call.
    async fn handle_channel_pay(&self, args: &serde_json::Value) -> McpToolResult {
        let channel_id = match args.get("channel_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return McpToolResult::err("missing required argument: 'channel_id'"),
        };

        let amount = match args.get("amount").and_then(|v| v.as_u64()) {
            Some(amt) => amt,
            None => return McpToolResult::err("missing or invalid argument: 'amount'"),
        };

        let params = json!({
            "channel_id": channel_id,
            "amount": amount,
        });

        match self.rpc_call("dina_channelPay", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/channel_close` tool call.
    async fn handle_channel_close(&self, args: &serde_json::Value) -> McpToolResult {
        let channel_id = match args.get("channel_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return McpToolResult::err("missing required argument: 'channel_id'"),
        };

        let fee = args.get("fee").and_then(|v| v.as_u64()).unwrap_or(10);

        let params = json!({
            "channel_id": channel_id,
            "fee": fee,
        });

        match self.rpc_call("dina_channelClose", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/peers` tool call.
    async fn handle_peers(&self, args: &serde_json::Value) -> McpToolResult {
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50);
        let params = json!({ "limit": limit });

        match self.rpc_call("dina_getPeers", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/block_info` tool call.
    async fn handle_block_info(&self, args: &serde_json::Value) -> McpToolResult {
        let mut params = json!({});

        if let Some(hash) = args.get("block_hash").and_then(|v| v.as_str()) {
            params["block_hash"] = json!(hash);
        } else if let Some(num) = args.get("block_number").and_then(|v| v.as_u64()) {
            params["block_number"] = json!(num);
        }
        // If neither provided, the node returns the latest block.

        match self.rpc_call("dina_getBlock", params).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }

    /// Handle `dina/network_status` tool call.
    async fn handle_network_status(&self, _args: &serde_json::Value) -> McpToolResult {
        match self.rpc_call("dina_networkStatus", json!({})).await {
            Ok(result) => McpToolResult::ok(result),
            Err(e) => McpToolResult::err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_creation() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        assert_eq!(handler.rpc_endpoint, "http://127.0.0.1:8545");
    }

    #[test]
    fn available_tools_returns_all() {
        let tools = McpHandler::available_tools();
        assert_eq!(tools.len(), 12);
    }

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let call = McpToolCall {
            tool_name: "dina/nonexistent".to_string(),
            arguments: json!({}),
        };
        let result = handler.handle_tool_call(call).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("unknown tool"));
    }

    #[tokio::test]
    async fn transfer_missing_to_returns_error() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let call = McpToolCall {
            tool_name: "dina/transfer".to_string(),
            arguments: json!({"amount": 100}),
        };
        let result = handler.handle_tool_call(call).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("'to'"));
    }

    #[tokio::test]
    async fn transfer_missing_amount_returns_error() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let call = McpToolCall {
            tool_name: "dina/transfer".to_string(),
            arguments: json!({"to": "0xabab"}),
        };
        let result = handler.handle_tool_call(call).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("'amount'"));
    }

    #[tokio::test]
    async fn balance_no_args_is_valid() {
        // balance with no address should not fail on argument validation
        // (it will fail on RPC connection, but that's expected in tests)
        let handler = McpHandler::new("http://127.0.0.1:1".to_string());
        let call = McpToolCall {
            tool_name: "dina/balance".to_string(),
            arguments: json!({}),
        };
        let result = handler.handle_tool_call(call).await;
        // Connection refused is expected -- but it should not be an argument error
        assert!(!result.success);
        let err = result.error.unwrap();
        assert!(!err.contains("missing required argument"));
    }

    #[tokio::test]
    async fn deploy_contract_missing_bytecode_returns_error() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let call = McpToolCall {
            tool_name: "dina/deploy_contract".to_string(),
            arguments: json!({}),
        };
        let result = handler.handle_tool_call(call).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("'wasm_bytecode'"));
    }

    #[tokio::test]
    async fn call_contract_missing_fields_returns_error() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let call = McpToolCall {
            tool_name: "dina/call_contract".to_string(),
            arguments: json!({"contract": "0xaabb"}),
        };
        let result = handler.handle_tool_call(call).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("'method'"));
    }

    #[tokio::test]
    async fn register_device_missing_fields_returns_error() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let call = McpToolCall {
            tool_name: "dina/register_device".to_string(),
            arguments: json!({"device_pubkey": "ab"}),
        };
        let result = handler.handle_tool_call(call).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("'owner'"));
    }

    #[tokio::test]
    async fn channel_open_missing_counterparty_returns_error() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let call = McpToolCall {
            tool_name: "dina/channel_open".to_string(),
            arguments: json!({"deposit": 1000}),
        };
        let result = handler.handle_tool_call(call).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("'counterparty'"));
    }

    #[tokio::test]
    async fn channel_pay_missing_channel_id_returns_error() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let call = McpToolCall {
            tool_name: "dina/channel_pay".to_string(),
            arguments: json!({"amount": 500}),
        };
        let result = handler.handle_tool_call(call).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("'channel_id'"));
    }

    #[tokio::test]
    async fn channel_close_missing_channel_id_returns_error() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let call = McpToolCall {
            tool_name: "dina/channel_close".to_string(),
            arguments: json!({}),
        };
        let result = handler.handle_tool_call(call).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("'channel_id'"));
    }

    #[tokio::test]
    async fn verify_device_missing_device_id_returns_error() {
        let handler = McpHandler::new("http://127.0.0.1:8545".to_string());
        let call = McpToolCall {
            tool_name: "dina/verify_device".to_string(),
            arguments: json!({}),
        };
        let result = handler.handle_tool_call(call).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("'device_id'"));
    }
}
