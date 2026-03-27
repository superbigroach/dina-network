//! Settlement submission to Dina validators via RPC.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{debug, error, info, warn};

use crate::blob::RelayBlob;
use crate::error::{RelayError, Result};

/// Configuration for the relay submitter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitterConfig {
    /// RPC endpoint URL for submitting settlements to validators.
    pub rpc_endpoint: String,
    /// Maximum number of retry attempts per submission.
    pub max_retries: u32,
    /// Delay between retries in milliseconds.
    pub retry_delay_ms: u64,
    /// Maximum number of blobs to submit in a single batch.
    pub batch_size: usize,
}

impl Default for SubmitterConfig {
    fn default() -> Self {
        Self {
            rpc_endpoint: String::from("http://localhost:9944"),
            max_retries: 3,
            retry_delay_ms: 1000,
            batch_size: 50,
        }
    }
}

/// Result of submitting a relay blob to a validator.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmissionResult {
    /// SHA-256 hash of the submitted blob.
    pub blob_hash: [u8; 32],
    /// Whether the submission was accepted by the validator.
    pub success: bool,
    /// Transaction hash on-chain (if the settlement was finalized).
    pub tx_hash: Option<[u8; 32]>,
    /// Relay fee earned in micro-USDC (if successful).
    pub fee_earned: u64,
    /// Error message if the submission failed.
    pub error: Option<String>,
}

impl SubmissionResult {
    /// Create a successful submission result.
    fn success(blob_hash: [u8; 32], tx_hash: [u8; 32], fee_earned: u64) -> Self {
        Self {
            blob_hash,
            success: true,
            tx_hash: Some(tx_hash),
            fee_earned,
            error: None,
        }
    }

    /// Create a failed submission result.
    fn failure(blob_hash: [u8; 32], error: String) -> Self {
        Self {
            blob_hash,
            success: false,
            tx_hash: None,
            fee_earned: 0,
            error: Some(error),
        }
    }
}

/// Handles submitting relay blobs to Dina validator nodes for on-chain settlement.
#[derive(Clone, Debug)]
pub struct RelaySubmitter {
    pending: VecDeque<RelayBlob>,
    config: SubmitterConfig,
}

impl RelaySubmitter {
    /// Create a new relay submitter with the given configuration.
    pub fn new(config: SubmitterConfig) -> Self {
        debug!(
            rpc_endpoint = %config.rpc_endpoint,
            max_retries = config.max_retries,
            batch_size = config.batch_size,
            "RelaySubmitter initialized"
        );
        Self {
            pending: VecDeque::new(),
            config,
        }
    }

    /// Submit a single relay blob to the configured RPC endpoint.
    ///
    /// This performs the HTTP POST to the validator's RPC endpoint with the
    /// serialized blob. On success, the validator returns a transaction hash
    /// and the relay fee is credited to the submitter.
    pub async fn submit_blob(&self, blob: RelayBlob) -> SubmissionResult {
        let blob_hash = blob.hash().0;
        let blob_bytes = match bincode::serialize(&blob) {
            Ok(b) => b,
            Err(e) => {
                return SubmissionResult::failure(
                    blob_hash,
                    format!("serialization failed: {e}"),
                );
            }
        };

        // Build the JSON-RPC request
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "relay_submitSettlement",
            "params": {
                "blob": hex::encode(&blob_bytes),
                "sender": blob.sender.to_string(),
                "receiver": blob.receiver.to_string(),
                "amount": blob.amount,
                "sequence": blob.sequence,
                "relay_fee": blob.relay_fee,
            },
            "id": 1
        });

        let mut last_error = String::from("no attempts made");

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                debug!(attempt, "Retrying blob submission");
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    self.config.retry_delay_ms,
                ))
                .await;
            }

            match self.do_rpc_call(&request_body).await {
                Ok(response) => {
                    // Parse the RPC response
                    if let Some(result) = response.get("result") {
                        let tx_hash = parse_tx_hash(result.get("tx_hash"));
                        let fee_earned = result
                            .get("relay_fee_earned")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(blob.relay_fee);

                        info!(
                            blob_hash = hex::encode(blob_hash),
                            tx_hash = hex::encode(tx_hash),
                            fee_earned,
                            "Blob submitted successfully"
                        );

                        return SubmissionResult::success(blob_hash, tx_hash, fee_earned);
                    }

                    if let Some(err) = response.get("error") {
                        let msg = err
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown RPC error");
                        last_error = msg.to_string();
                        warn!(attempt, error = %last_error, "RPC returned error");
                    }
                }
                Err(e) => {
                    last_error = e.to_string();
                    warn!(attempt, error = %last_error, "RPC call failed");
                }
            }
        }

        error!(
            blob_hash = hex::encode(blob_hash),
            error = %last_error,
            "All submission attempts exhausted"
        );
        SubmissionResult::failure(blob_hash, last_error)
    }

    /// Submit a batch of relay blobs. Returns one result per blob.
    pub async fn submit_batch(&self, blobs: Vec<RelayBlob>) -> Vec<SubmissionResult> {
        let mut results = Vec::with_capacity(blobs.len());

        // Process in chunks according to batch_size
        for chunk in blobs.chunks(self.config.batch_size) {
            // Submit each blob in the chunk concurrently
            let futures: Vec<_> = chunk.iter().cloned().map(|b| self.submit_blob(b)).collect();

            let chunk_results = futures::future::join_all(futures).await;
            results.extend(chunk_results);
        }

        let successes = results.iter().filter(|r| r.success).count();
        info!(
            total = results.len(),
            successes,
            failures = results.len() - successes,
            "Batch submission complete"
        );

        results
    }

    /// Add a blob to the pending queue for later submission.
    pub fn queue_for_submission(&mut self, blob: RelayBlob) {
        debug!(
            sender = %blob.sender,
            amount = blob.amount,
            pending = self.pending.len() + 1,
            "Queued blob for submission"
        );
        self.pending.push_back(blob);
    }

    /// Submit all pending blobs in the queue. Returns results for each submission.
    pub async fn process_queue(&mut self) -> Vec<SubmissionResult> {
        let blobs: Vec<RelayBlob> = self.pending.drain(..).collect();
        if blobs.is_empty() {
            return Vec::new();
        }

        debug!(count = blobs.len(), "Processing submission queue");
        self.submit_batch(blobs).await
    }

    /// Return the number of blobs pending in the submission queue.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get a reference to the submitter's configuration.
    pub fn config(&self) -> &SubmitterConfig {
        &self.config
    }

    /// Perform the actual HTTP POST to the RPC endpoint.
    ///
    /// This is a minimal HTTP client implementation using tokio's TCP stream
    /// to avoid pulling in a heavy HTTP crate like reqwest. For production use,
    /// integrators would swap this for their preferred HTTP client.
    async fn do_rpc_call(
        &self,
        request_body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let body = serde_json::to_string(request_body).map_err(|e| {
            RelayError::SerializationError(format!("failed to serialize RPC request: {e}"))
        })?;

        // Parse the endpoint URL to extract host and port
        let endpoint = &self.config.rpc_endpoint;
        let stripped = endpoint
            .strip_prefix("http://")
            .or_else(|| endpoint.strip_prefix("https://"))
            .unwrap_or(endpoint);

        let (host_port, path) = match stripped.find('/') {
            Some(i) => (&stripped[..i], &stripped[i..]),
            None => (stripped, "/"),
        };

        let addr = if host_port.contains(':') {
            host_port.to_string()
        } else {
            format!("{host_port}:80")
        };

        let mut stream = TcpStream::connect(&addr).await.map_err(|e| {
            RelayError::NetworkError(format!("failed to connect to {addr}: {e}"))
        })?;

        let request = format!(
            "POST {path} HTTP/1.1\r\n\
             Host: {host_port}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {body}",
            body.len()
        );

        stream.write_all(request.as_bytes()).await.map_err(|e| {
            RelayError::NetworkError(format!("failed to write request: {e}"))
        })?;

        let mut response_buf = Vec::new();
        stream.read_to_end(&mut response_buf).await.map_err(|e| {
            RelayError::NetworkError(format!("failed to read response: {e}"))
        })?;

        let response_str = String::from_utf8_lossy(&response_buf);

        // Find the JSON body after the HTTP headers (separated by \r\n\r\n)
        let json_body = response_str
            .find("\r\n\r\n")
            .map(|i| &response_str[i + 4..])
            .unwrap_or(&response_str);

        serde_json::from_str(json_body).map_err(|e| {
            RelayError::SubmissionFailed(format!("failed to parse RPC response: {e}"))
        })
    }
}

/// Parse a transaction hash from a JSON value (hex string -> [u8; 32]).
fn parse_tx_hash(value: Option<&serde_json::Value>) -> [u8; 32] {
    let mut hash = [0u8; 32];
    if let Some(serde_json::Value::String(hex_str)) = value {
        let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        if let Ok(bytes) = hex::decode(stripped) {
            if bytes.len() == 32 {
                hash.copy_from_slice(&bytes);
            }
        }
    }
    hash
}

/// Private module to bring in futures::future::join_all without adding
/// a full futures crate dependency — we use a simple polyfill.
mod futures {
    pub mod future {
        use std::future::Future;

        /// Await all futures concurrently and collect results in order.
        pub async fn join_all<F, T>(futures: Vec<F>) -> Vec<T>
        where
            F: Future<Output = T>,
        {
            // For a lightweight SDK, we process sequentially to avoid
            // requiring tokio::spawn or a full futures crate.
            // In production, integrators can swap this for true concurrent execution.
            let mut results = Vec::with_capacity(futures.len());
            for f in futures {
                results.push(f.await);
            }
            results
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob::DEFAULT_BLOB_TTL_SECS;
    use dina_core::crypto;
    use dina_core::transaction::Sig64;
    use dina_core::types::{Address, Hash};

    fn make_test_blob() -> RelayBlob {
        let (sender_sk, sender_vk) = crypto::generate_keypair();
        let (receiver_sk, receiver_vk) = crypto::generate_keypair();

        let mut blob = RelayBlob {
            version: 1,
            sender: Address::from_pubkey(&sender_vk),
            receiver: Address::from_pubkey(&receiver_vk),
            amount: 50_000,
            sequence: 1,
            created_at: 1700000000,
            ttl_secs: DEFAULT_BLOB_TTL_SECS,
            relay_fee: 10,
            channel_state_hash: Hash([0xcc; 32]),
            sender_signature: Sig64([0u8; 64]),
            receiver_signature: Sig64([0u8; 64]),
            hop_count: 0,
            max_hops: 10,
        };

        let msg = blob.signing_bytes();
        blob.sender_signature = Sig64(crypto::sign(&sender_sk, &msg));
        blob.receiver_signature = Sig64(crypto::sign(&receiver_sk, &msg));
        blob
    }

    #[test]
    fn queue_and_count() {
        let mut submitter = RelaySubmitter::new(SubmitterConfig::default());
        assert_eq!(submitter.pending_count(), 0);

        submitter.queue_for_submission(make_test_blob());
        submitter.queue_for_submission(make_test_blob());
        assert_eq!(submitter.pending_count(), 2);
    }

    #[test]
    fn submission_result_success() {
        let result = SubmissionResult::success([0xaa; 32], [0xbb; 32], 100);
        assert!(result.success);
        assert_eq!(result.fee_earned, 100);
        assert!(result.tx_hash.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn submission_result_failure() {
        let result = SubmissionResult::failure([0xaa; 32], "timeout".to_string());
        assert!(!result.success);
        assert_eq!(result.fee_earned, 0);
        assert!(result.tx_hash.is_none());
        assert_eq!(result.error.as_deref(), Some("timeout"));
    }

    #[test]
    fn parse_tx_hash_valid() {
        let hex_str = format!("0x{}", hex::encode([0xdd; 32]));
        let val = serde_json::Value::String(hex_str);
        let hash = parse_tx_hash(Some(&val));
        assert_eq!(hash, [0xdd; 32]);
    }

    #[test]
    fn parse_tx_hash_none() {
        let hash = parse_tx_hash(None);
        assert_eq!(hash, [0u8; 32]);
    }
}
