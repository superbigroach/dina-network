//! BLE Scanner abstraction for receiving and validating Dina relay blobs
//! from nearby devices.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{debug, trace, warn};

use crate::blob::RelayBlob;

/// Placeholder Bluetooth SIG company ID for Dina Network.
/// In production this would be a registered company ID from the Bluetooth SIG.
pub const DINA_COMPANY_ID: u16 = 0xD14A;

/// A raw BLE advertisement received from a nearby device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BleAdvertisement {
    /// Bluetooth SIG company ID from the manufacturer-specific data.
    pub company_id: u16,
    /// Raw payload bytes from the advertisement.
    pub payload: Vec<u8>,
    /// Received signal strength indicator (dBm).
    pub rssi: i8,
    /// Unix timestamp (seconds) when this advertisement was received.
    pub timestamp: u64,
}

/// Configuration for the relay scanner.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayScannerConfig {
    /// How often to scan for BLE advertisements (milliseconds).
    pub scan_interval_ms: u64,
    /// Maximum number of blobs to queue before dropping oldest.
    pub max_queue_size: usize,
    /// Whether to automatically submit blobs to the RPC endpoint.
    pub auto_submit: bool,
    /// RPC endpoint for auto-submission.
    pub submit_endpoint: String,
}

impl Default for RelayScannerConfig {
    fn default() -> Self {
        Self {
            scan_interval_ms: 1000,
            max_queue_size: 256,
            auto_submit: false,
            submit_endpoint: String::from("http://localhost:9944"),
        }
    }
}

/// Scans for Dina relay blobs from BLE advertisements, validates them,
/// and queues them for submission or re-broadcast.
#[derive(Clone, Debug)]
pub struct RelayScanner {
    relay_queue: VecDeque<RelayBlob>,
    config: RelayScannerConfig,
}

impl RelayScanner {
    /// Create a new relay scanner with the given configuration.
    pub fn new(config: RelayScannerConfig) -> Self {
        debug!(
            scan_interval_ms = config.scan_interval_ms,
            max_queue_size = config.max_queue_size,
            auto_submit = config.auto_submit,
            "RelayScanner initialized"
        );
        Self {
            relay_queue: VecDeque::new(),
            config,
        }
    }

    /// Process a raw BLE advertisement. If it contains a valid Dina relay blob,
    /// parse it, validate basic structure, and queue it. Returns the parsed blob
    /// on success.
    ///
    /// Validation performed:
    /// - Company ID matches DINA_COMPANY_ID
    /// - Payload deserializes to a valid RelayBlob
    /// - Blob version is supported (currently only version 1)
    /// - Blob has not exceeded its max hop count
    /// - Blob has not expired (based on advertisement timestamp)
    pub fn on_ble_advertisement(&mut self, adv: BleAdvertisement) -> Option<RelayBlob> {
        // Check company ID
        if adv.company_id != DINA_COMPANY_ID {
            trace!(
                company_id = adv.company_id,
                "Ignoring non-Dina advertisement"
            );
            return None;
        }

        // Deserialize the relay blob from the payload
        let blob: RelayBlob = match bincode::deserialize(&adv.payload) {
            Ok(b) => b,
            Err(e) => {
                warn!(error = %e, "Failed to deserialize relay blob from BLE payload");
                return None;
            }
        };

        // Validate version
        if blob.version != 1 {
            warn!(version = blob.version, "Unsupported relay blob version");
            return None;
        }

        // Check hop count
        if blob.is_max_hops_reached() {
            debug!(
                hop_count = blob.hop_count,
                max_hops = blob.max_hops,
                "Relay blob has reached max hops, dropping"
            );
            return None;
        }

        // Check expiry
        if blob.is_expired(adv.timestamp) {
            debug!(
                created_at = blob.created_at,
                ttl_secs = blob.ttl_secs,
                now = adv.timestamp,
                "Relay blob has expired, dropping"
            );
            return None;
        }

        // Queue the blob, evicting oldest if at capacity
        if self.relay_queue.len() >= self.config.max_queue_size {
            let evicted = self.relay_queue.pop_front();
            debug!(
                evicted_hash = ?evicted.map(|b| b.hash()),
                "Queue full, evicted oldest blob"
            );
        }

        debug!(
            sender = %blob.sender,
            receiver = %blob.receiver,
            amount = blob.amount,
            hop_count = blob.hop_count,
            rssi = adv.rssi,
            "Queued valid relay blob from BLE scan"
        );

        let queued = blob.clone();
        self.relay_queue.push_back(blob);
        Some(queued)
    }

    /// Return the number of blobs currently in the relay queue.
    pub fn queue_size(&self) -> usize {
        self.relay_queue.len()
    }

    /// Drain and return all queued relay blobs, clearing the queue.
    pub fn flush_queue(&mut self) -> Vec<RelayBlob> {
        let blobs: Vec<RelayBlob> = self.relay_queue.drain(..).collect();
        debug!(count = blobs.len(), "Flushed relay queue");
        blobs
    }

    /// Get a reference to the scanner's configuration.
    pub fn config(&self) -> &RelayScannerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob::{DEFAULT_BLOB_TTL_SECS, RelayBlob};
    use dina_core::crypto;
    use dina_core::transaction::Sig64;
    use dina_core::types::{Address, Hash};

    fn make_signed_blob(created_at: u64) -> RelayBlob {
        let (sender_sk, sender_vk) = crypto::generate_keypair();
        let (receiver_sk, receiver_vk) = crypto::generate_keypair();

        let mut blob = RelayBlob {
            version: 1,
            sender: Address::from_pubkey(&sender_vk),
            receiver: Address::from_pubkey(&receiver_vk),
            amount: 10_000,
            sequence: 1,
            created_at,
            ttl_secs: DEFAULT_BLOB_TTL_SECS,
            relay_fee: 5,
            channel_state_hash: Hash([0xbb; 32]),
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

    fn make_advertisement(blob: &RelayBlob, rssi: i8, timestamp: u64) -> BleAdvertisement {
        BleAdvertisement {
            company_id: DINA_COMPANY_ID,
            payload: bincode::serialize(blob).unwrap(),
            rssi,
            timestamp,
        }
    }

    #[test]
    fn accepts_valid_blob() {
        let mut scanner = RelayScanner::new(RelayScannerConfig::default());
        let blob = make_signed_blob(1700000000);
        let adv = make_advertisement(&blob, -60, 1700000100);

        let result = scanner.on_ble_advertisement(adv);
        assert!(result.is_some());
        assert_eq!(scanner.queue_size(), 1);
    }

    #[test]
    fn rejects_wrong_company_id() {
        let mut scanner = RelayScanner::new(RelayScannerConfig::default());
        let blob = make_signed_blob(1700000000);
        let mut adv = make_advertisement(&blob, -60, 1700000100);
        adv.company_id = 0xAAAA;

        let result = scanner.on_ble_advertisement(adv);
        assert!(result.is_none());
        assert_eq!(scanner.queue_size(), 0);
    }

    #[test]
    fn rejects_expired_blob() {
        let mut scanner = RelayScanner::new(RelayScannerConfig::default());
        let blob = make_signed_blob(1700000000);
        let adv = make_advertisement(&blob, -60, 1700000000 + 400);

        let result = scanner.on_ble_advertisement(adv);
        assert!(result.is_none());
    }

    #[test]
    fn rejects_max_hops() {
        let mut scanner = RelayScanner::new(RelayScannerConfig::default());
        let mut blob = make_signed_blob(1700000000);
        blob.hop_count = 10;
        blob.max_hops = 10;
        let adv = make_advertisement(&blob, -60, 1700000100);

        let result = scanner.on_ble_advertisement(adv);
        assert!(result.is_none());
    }

    #[test]
    fn flush_clears_queue() {
        let mut scanner = RelayScanner::new(RelayScannerConfig::default());
        let blob = make_signed_blob(1700000000);
        let adv = make_advertisement(&blob, -60, 1700000100);
        scanner.on_ble_advertisement(adv);

        let flushed = scanner.flush_queue();
        assert_eq!(flushed.len(), 1);
        assert_eq!(scanner.queue_size(), 0);
    }

    #[test]
    fn evicts_oldest_when_full() {
        let config = RelayScannerConfig {
            max_queue_size: 2,
            ..Default::default()
        };
        let mut scanner = RelayScanner::new(config);

        for i in 0..3 {
            let blob = make_signed_blob(1700000000 + i);
            let adv = make_advertisement(&blob, -60, 1700000100 + i);
            scanner.on_ble_advertisement(adv);
        }

        assert_eq!(scanner.queue_size(), 2);
    }
}
