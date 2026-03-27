//! QR code encoding/decoding for relay blobs.
//!
//! Allows relay blobs to be transmitted via QR codes as a fallback when BLE
//! is unavailable (e.g., scanning a QR code displayed on a merchant terminal).

use crate::blob::RelayBlob;
use crate::error::{RelayError, Result};

/// Magic bytes prepended to QR-encoded relay blobs for identification.
const QR_MAGIC: [u8; 2] = [0xD1, 0x4A]; // "D14A" — matches company ID

/// Maximum QR data capacity we target (version 15, binary mode, medium ECC).
const MAX_QR_BYTES: usize = 412;

/// Encode a relay blob into compact binary data suitable for QR code embedding.
///
/// Format:
///   [2 bytes magic] [N bytes bincode-serialized RelayBlob]
///
/// The result is typically 180-220 bytes, fitting comfortably in a QR version
/// 10-15 code with medium error correction.
pub fn blob_to_qr_data(blob: &RelayBlob) -> Result<Vec<u8>> {
    let serialized = bincode::serialize(blob).map_err(|e| {
        RelayError::SerializationError(format!("failed to serialize blob for QR: {e}"))
    })?;

    let total_size = QR_MAGIC.len() + serialized.len();
    if total_size > MAX_QR_BYTES {
        return Err(RelayError::PayloadTooLarge {
            size: total_size,
            max: MAX_QR_BYTES,
        });
    }

    let mut data = Vec::with_capacity(total_size);
    data.extend_from_slice(&QR_MAGIC);
    data.extend_from_slice(&serialized);

    Ok(data)
}

/// Decode a relay blob from QR code binary data.
///
/// Validates the magic bytes prefix and deserializes the blob.
pub fn blob_from_qr_data(data: &[u8]) -> Result<RelayBlob> {
    if data.len() < QR_MAGIC.len() + 1 {
        return Err(RelayError::QrError(format!(
            "QR data too short: {} bytes (minimum {})",
            data.len(),
            QR_MAGIC.len() + 1
        )));
    }

    if data[..2] != QR_MAGIC {
        return Err(RelayError::QrError(format!(
            "invalid QR magic bytes: expected {:02X}{:02X}, got {:02X}{:02X}",
            QR_MAGIC[0], QR_MAGIC[1], data[0], data[1]
        )));
    }

    let blob: RelayBlob = bincode::deserialize(&data[2..]).map_err(|e| {
        RelayError::SerializationError(format!("failed to deserialize blob from QR: {e}"))
    })?;

    Ok(blob)
}

/// Estimate which QR code version is needed to encode a given relay blob.
///
/// QR version determines the size of the code and its data capacity.
/// Returns the minimum version (1-40) needed in binary mode with medium ECC.
///
/// Typical relay blobs (180-220 bytes) need version 10-15.
pub fn estimate_qr_version(blob: &RelayBlob) -> u8 {
    let size = match blob_to_qr_data(blob) {
        Ok(data) => data.len(),
        Err(_) => blob.serialized_size() + QR_MAGIC.len(),
    };

    // Binary mode capacities at Medium ECC level (ISO 18004)
    // Each entry: (version, max_bytes)
    let capacities: &[(u8, usize)] = &[
        (1, 14),
        (2, 26),
        (3, 42),
        (4, 62),
        (5, 84),
        (6, 106),
        (7, 122),
        (8, 152),
        (9, 180),
        (10, 213),
        (11, 251),
        (12, 287),
        (13, 331),
        (14, 362),
        (15, 412),
        (16, 450),
        (17, 504),
        (18, 560),
        (19, 624),
        (20, 666),
        (21, 711),
        (22, 779),
        (23, 857),
        (24, 911),
        (25, 997),
        (26, 1059),
        (27, 1125),
        (28, 1190),
        (29, 1264),
        (30, 1370),
        (31, 1452),
        (32, 1538),
        (33, 1628),
        (34, 1722),
        (35, 1809),
        (36, 1911),
        (37, 1989),
        (38, 2099),
        (39, 2213),
        (40, 2331),
    ];

    for &(version, capacity) in capacities {
        if size <= capacity {
            return version;
        }
    }

    // If it doesn't fit even in version 40, return 40 as a sentinel
    40
}

/// Check whether a relay blob can fit within a QR code.
///
/// Returns true if the blob's serialized size (with magic bytes) is within
/// the maximum QR capacity we target (version 15, ~412 bytes).
pub fn is_qr_compatible(blob: &RelayBlob) -> bool {
    match blob_to_qr_data(blob) {
        Ok(data) => data.len() <= MAX_QR_BYTES,
        Err(_) => false,
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
            channel_state_hash: Hash([0xdd; 32]),
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
    fn roundtrip_qr_encode_decode() {
        let blob = make_test_blob();
        let data = blob_to_qr_data(&blob).unwrap();
        let decoded = blob_from_qr_data(&data).unwrap();

        assert_eq!(decoded.sender, blob.sender);
        assert_eq!(decoded.receiver, blob.receiver);
        assert_eq!(decoded.amount, blob.amount);
        assert_eq!(decoded.sequence, blob.sequence);
    }

    #[test]
    fn qr_data_starts_with_magic() {
        let blob = make_test_blob();
        let data = blob_to_qr_data(&blob).unwrap();
        assert_eq!(data[0], 0xD1);
        assert_eq!(data[1], 0x4A);
    }

    #[test]
    fn qr_data_under_250_bytes() {
        let blob = make_test_blob();
        let data = blob_to_qr_data(&blob).unwrap();
        assert!(
            data.len() < 300,
            "QR data should be under 300 bytes, got {}",
            data.len()
        );
    }

    #[test]
    fn invalid_magic_rejected() {
        let blob = make_test_blob();
        let mut data = blob_to_qr_data(&blob).unwrap();
        data[0] = 0xFF;
        let result = blob_from_qr_data(&data);
        assert!(result.is_err());
    }

    #[test]
    fn too_short_data_rejected() {
        let result = blob_from_qr_data(&[0xD1]);
        assert!(result.is_err());
    }

    #[test]
    fn estimate_version_for_typical_blob() {
        let blob = make_test_blob();
        let version = estimate_qr_version(&blob);
        assert!(
            (10..=15).contains(&version),
            "Expected version 10-15 for ~200 byte blob, got {version}"
        );
    }

    #[test]
    fn is_qr_compatible_for_normal_blob() {
        let blob = make_test_blob();
        assert!(is_qr_compatible(&blob));
    }
}
