use anyhow::{Context, Result};
use serde_json::{json, Value};

/// A minimal JSON-RPC client for communicating with a Dina node.
///
/// Uses raw HTTP POST requests to avoid pulling in the full jsonrpsee client
/// dependency, keeping the CLI binary lightweight.
pub struct RpcClient {
    url: String,
    client: reqwest::Client,
    request_id: std::sync::atomic::AtomicU64,
}

impl RpcClient {
    /// Create a new RPC client pointing at the given URL.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            client: reqwest::Client::new(),
            request_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Send a raw JSON-RPC request and return the result field.
    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let request_body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let response = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .with_context(|| {
                format!(
                    "failed to connect to node at {}. Is the node running?",
                    self.url
                )
            })?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .context("failed to parse JSON-RPC response")?;

        if !status.is_success() {
            anyhow::bail!("RPC request failed with HTTP {status}: {body}");
        }

        // Check for JSON-RPC error
        if let Some(error) = body.get("error") {
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("RPC error {code}: {message}");
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("RPC response missing 'result' field"))
    }

    /// Submit a signed transaction (hex-encoded) to the network.
    pub async fn send_transaction(&self, tx_hex: &str) -> Result<String> {
        let result = self.call("dina_sendTransaction", json!([tx_hex])).await?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("unexpected response format for sendTransaction"))
    }

    /// Get the balance of an address.
    pub async fn get_balance(&self, address: &str) -> Result<u64> {
        let result = self.call("dina_getBalance", json!([address])).await?;
        result
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("unexpected response format for getBalance"))
    }

    /// Get full account information.
    pub async fn get_account(&self, address: &str) -> Result<Value> {
        self.call("dina_getAccount", json!([address])).await
    }

    /// Get a block by height.
    pub async fn get_block(&self, height: u64) -> Result<Value> {
        self.call("dina_getBlock", json!([height])).await
    }

    /// Get a block by hash.
    #[allow(dead_code)]
    pub async fn get_block_by_hash(&self, hash: &str) -> Result<Value> {
        self.call("dina_getBlockByHash", json!([hash])).await
    }

    /// Get the latest block.
    pub async fn get_latest_block(&self) -> Result<Value> {
        self.call("dina_getLatestBlock", json!([])).await
    }

    /// Get a transaction by hash.
    pub async fn get_transaction(&self, hash: &str) -> Result<Value> {
        self.call("dina_getTransaction", json!([hash])).await
    }

    /// Get a device by its public key.
    pub async fn get_device(&self, pubkey: &str) -> Result<Value> {
        self.call("dina_getDevice", json!([pubkey])).await
    }

    /// Get network information.
    pub async fn network_info(&self) -> Result<Value> {
        self.call("dina_networkInfo", json!([])).await
    }

    /// Get the chain ID.
    #[allow(dead_code)]
    pub async fn chain_id(&self) -> Result<String> {
        let result = self.call("dina_chainId", json!([])).await?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("unexpected response format for chainId"))
    }

    /// Get contract information by address.
    pub async fn get_contract_info(&self, address: &str) -> Result<Value> {
        self.call("dina_getContractInfo", json!([address])).await
    }

    /// List open payment channels.
    pub async fn list_channels(&self) -> Result<Value> {
        self.call("dina_listChannels", json!([])).await
    }

    /// Request testnet USDC from the faucet.
    pub async fn request_faucet(&self, address: &str) -> Result<Value> {
        self.call("dina_faucet", json!([address])).await
    }

    /// List active validators.
    pub async fn list_validators(&self) -> Result<Value> {
        self.call("dina_getValidators", json!([])).await
    }

    /// Get validator details by address.
    pub async fn get_validator(&self, address: &str) -> Result<Value> {
        self.call("dina_getValidator", json!([address])).await
    }

    /// Get recent blocks (for the explorer).
    pub async fn get_recent_blocks(&self, limit: u64) -> Result<Value> {
        self.call("dina_getRecentBlocks", json!([limit])).await
    }

    /// Search blocks and transactions (for the explorer).
    pub async fn explorer_search(&self, query: &str) -> Result<Value> {
        self.call("dina_search", json!([query])).await
    }
}
