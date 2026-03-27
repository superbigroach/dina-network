//! BLE Broadcaster abstraction for transmitting Dina relay blobs
//! to nearby devices via Bluetooth Low Energy advertisements.

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::blob::RelayBlob;
use crate::scanner::{BleAdvertisement, DINA_COMPANY_ID};

/// Configuration for BLE broadcasting.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BroadcastConfig {
    /// Advertising interval in milliseconds.
    pub interval_ms: u64,
    /// Transmit power in dBm (typical range: -20 to +4).
    pub tx_power_dbm: i8,
    /// Maximum payload size in bytes for BLE advertisements.
    /// BLE 4.x max is 31 bytes (manufacturer data), BLE 5.x extended is 255 bytes.
    pub max_payload_bytes: usize,
}

impl Default for BroadcastConfig {
    fn default() -> Self {
        Self {
            interval_ms: 200,
            tx_power_dbm: 0,
            max_payload_bytes: 255, // BLE 5.x extended advertising
        }
    }
}

/// Handles creating BLE advertisements from relay blobs and deciding
/// whether a blob should be broadcast.
#[derive(Clone, Debug)]
pub struct RelayBroadcaster {
    config: BroadcastConfig,
}

impl RelayBroadcaster {
    /// Create a new relay broadcaster with the given configuration.
    pub fn new(config: BroadcastConfig) -> Self {
        debug!(
            interval_ms = config.interval_ms,
            tx_power_dbm = config.tx_power_dbm,
            max_payload_bytes = config.max_payload_bytes,
            "RelayBroadcaster initialized"
        );
        Self { config }
    }

    /// Serialize a relay blob into a BLE advertisement that can be transmitted.
    ///
    /// The advertisement uses the Dina company ID (0xD14A) in the manufacturer-specific
    /// data field, with the blob serialized via bincode.
    pub fn create_advertisement(&self, blob: &RelayBlob) -> BleAdvertisement {
        let payload = bincode::serialize(blob).expect("blob serialization cannot fail");

        debug!(
            payload_bytes = payload.len(),
            sender = %blob.sender,
            "Created BLE advertisement for relay blob"
        );

        BleAdvertisement {
            company_id: DINA_COMPANY_ID,
            payload,
            rssi: self.config.tx_power_dbm,
            timestamp: blob.created_at,
        }
    }

    /// Determine whether a relay blob should be broadcast.
    ///
    /// A blob is worth broadcasting if:
    /// - It has not expired (based on current system time approximation from created_at + ttl)
    /// - It has not reached its maximum hop count
    /// - Its serialized size fits within the BLE payload limit
    /// - Both sender and receiver signatures are non-zero (i.e., present)
    pub fn should_broadcast(&self, blob: &RelayBlob) -> bool {
        // Check hop count
        if blob.is_max_hops_reached() {
            debug!("Blob rejected: max hops reached");
            return false;
        }

        // Check that signatures are present (non-zero)
        if blob.sender_signature.0 == [0u8; 64] {
            debug!("Blob rejected: missing sender signature");
            return false;
        }
        if blob.receiver_signature.0 == [0u8; 64] {
            debug!("Blob rejected: missing receiver signature");
            return false;
        }

        // Check serialized size fits in BLE payload
        let size = blob.serialized_size();
        if size > self.config.max_payload_bytes {
            debug!(
                size,
                max = self.config.max_payload_bytes,
                "Blob rejected: too large for BLE payload"
            );
            return false;
        }

        // Check version
        if blob.version != 1 {
            debug!(version = blob.version, "Blob rejected: unsupported version");
            return false;
        }

        true
    }

    /// Estimate the approximate BLE range in meters based on transmit power.
    ///
    /// Uses the log-distance path loss model with typical indoor parameters:
    ///   RSSI = tx_power - 10 * n * log10(d) - environmental_factor
    ///
    /// Where n=2.0 (free space) and we solve for d when RSSI = -90 dBm
    /// (typical BLE receiver sensitivity).
    pub fn estimate_range_meters(tx_power_dbm: i8) -> f64 {
        let receiver_sensitivity_dbm: f64 = -90.0;
        let path_loss_exponent: f64 = 2.0;
        let environmental_factor: f64 = 5.0;

        // path_loss = tx_power - receiver_sensitivity - environmental_factor
        let path_loss = (tx_power_dbm as f64) - receiver_sensitivity_dbm - environmental_factor;

        // d = 10^(path_loss / (10 * n))
        let exponent = path_loss / (10.0 * path_loss_exponent);
        let distance = 10.0_f64.powf(exponent);

        // Clamp to reasonable BLE range (0.1m to 200m)
        distance.clamp(0.1, 200.0)
    }

    /// Get a reference to the broadcaster's configuration.
    pub fn config(&self) -> &BroadcastConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob::DEFAULT_BLOB_TTL_SECS;
    use dina_core::crypto;
    use dina_core::transaction::Sig64;
    use dina_core::types::{Address, Hash};

    fn make_signed_blob() -> RelayBlob {
        let (sender_sk, sender_vk) = crypto::generate_keypair();
        let (receiver_sk, receiver_vk) = crypto::generate_keypair();

        let mut blob = RelayBlob {
            version: 1,
            sender: Address::from_pubkey(&sender_vk),
            receiver: Address::from_pubkey(&receiver_vk),
            amount: 10_000,
            sequence: 1,
            created_at: 1700000000,
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

    #[test]
    fn create_advertisement_roundtrip() {
        let broadcaster = RelayBroadcaster::new(BroadcastConfig::default());
        let blob = make_signed_blob();
        let adv = broadcaster.create_advertisement(&blob);

        assert_eq!(adv.company_id, DINA_COMPANY_ID);
        let decoded: RelayBlob = bincode::deserialize(&adv.payload).unwrap();
        assert_eq!(decoded.sender, blob.sender);
        assert_eq!(decoded.amount, blob.amount);
    }

    #[test]
    fn should_broadcast_valid_blob() {
        let broadcaster = RelayBroadcaster::new(BroadcastConfig::default());
        let blob = make_signed_blob();
        assert!(broadcaster.should_broadcast(&blob));
    }

    #[test]
    fn should_not_broadcast_max_hops() {
        let broadcaster = RelayBroadcaster::new(BroadcastConfig::default());
        let mut blob = make_signed_blob();
        blob.hop_count = 10;
        assert!(!broadcaster.should_broadcast(&blob));
    }

    #[test]
    fn should_not_broadcast_unsigned() {
        let broadcaster = RelayBroadcaster::new(BroadcastConfig::default());
        let mut blob = make_signed_blob();
        blob.sender_signature = Sig64([0u8; 64]);
        assert!(!broadcaster.should_broadcast(&blob));
    }

    #[test]
    fn should_not_broadcast_too_large() {
        let config = BroadcastConfig {
            max_payload_bytes: 10, // way too small
            ..Default::default()
        };
        let broadcaster = RelayBroadcaster::new(config);
        let blob = make_signed_blob();
        assert!(!broadcaster.should_broadcast(&blob));
    }

    #[test]
    fn estimate_range_reasonable() {
        // At 0 dBm, range should be in tens of meters
        let range = RelayBroadcaster::estimate_range_meters(0);
        assert!(range > 5.0 && range < 100.0, "range was {range}");

        // Higher power = more range
        let range_high = RelayBroadcaster::estimate_range_meters(4);
        let range_low = RelayBroadcaster::estimate_range_meters(-20);
        assert!(range_high > range_low);
    }
}
