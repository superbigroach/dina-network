use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{ChannelError, Result};
use crate::state::{self, SignedState};

/// A compact blob containing a signed channel state plus relay metadata.
///
/// When a device comes back online, it can relay this blob to the network
/// for on-chain settlement. The relay fee incentivizes mesh nodes to forward
/// settlement transactions on behalf of offline devices.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayBlob {
    /// The doubly-signed channel state to be settled.
    pub signed_state: SignedState,
    /// Fee (in USDC micro-units) paid to the relay node for submitting the settlement.
    pub relay_fee: u64,
    /// Optional recipient of the relay tip. If None, the submitting node keeps the fee.
    pub relay_tip_recipient: Option<[u8; 32]>,
}

/// Create a relay blob from a signed state and a relay fee.
pub fn create_relay_blob(signed_state: SignedState, relay_fee: u64) -> RelayBlob {
    debug!(
        channel_id = hex::encode(signed_state.state.channel_id),
        relay_fee,
        "creating relay blob"
    );

    RelayBlob {
        signed_state,
        relay_fee,
        relay_tip_recipient: None,
    }
}

/// Validate that a relay blob's signatures are authentic and the blob is well-formed.
pub fn validate_relay_blob(blob: &RelayBlob, party_a: &[u8; 32], party_b: &[u8; 32]) -> bool {
    // Verify both signatures on the enclosed state
    if !state::is_valid(&blob.signed_state, party_a, party_b) {
        return false;
    }

    // Verify that the relay fee does not exceed the total channel balance
    let total = blob.signed_state.state.balance_a + blob.signed_state.state.balance_b;
    if blob.relay_fee > total {
        return false;
    }

    true
}

/// Serialize a relay blob into compact binary suitable for QR codes.
///
/// Target size is under 250 bytes to fit in a standard QR code.
/// Layout:
///   - state bytes (64)
///   - signature_a (64)
///   - signature_b (64)
///   - relay_fee (8)
///   - has_recipient (1)
///   - recipient (32, only if has_recipient == 1)
///     Total: 201 bytes without recipient, 233 bytes with recipient.
pub fn blob_to_qr_bytes(blob: &RelayBlob) -> Vec<u8> {
    let state_bytes = state::to_bytes(&blob.signed_state.state);
    let mut buf = Vec::with_capacity(233);

    buf.extend_from_slice(&state_bytes); // 64 bytes
    buf.extend_from_slice(&blob.signed_state.signature_a); // 64 bytes
    buf.extend_from_slice(&blob.signed_state.signature_b); // 64 bytes
    buf.extend_from_slice(&blob.relay_fee.to_le_bytes()); // 8 bytes

    match &blob.relay_tip_recipient {
        Some(recipient) => {
            buf.push(1); // 1 byte
            buf.extend_from_slice(recipient); // 32 bytes
        }
        None => {
            buf.push(0); // 1 byte
        }
    }

    debug!(size = buf.len(), "relay blob serialized for QR");
    buf
}

/// Deserialize a relay blob from QR code bytes.
pub fn blob_from_qr_bytes(bytes: &[u8]) -> Result<RelayBlob> {
    // Minimum size: 64 (state) + 64 (sig_a) + 64 (sig_b) + 8 (fee) + 1 (flag) = 201
    if bytes.len() < 201 {
        return Err(ChannelError::InvalidRelayBlob);
    }

    let state_update = state::from_bytes(&bytes[0..64])?;

    let mut signature_a = [0u8; 64];
    signature_a.copy_from_slice(&bytes[64..128]);

    let mut signature_b = [0u8; 64];
    signature_b.copy_from_slice(&bytes[128..192]);

    let relay_fee = u64::from_le_bytes(
        bytes[192..200]
            .try_into()
            .map_err(|e| ChannelError::SerializationError(format!("{e}")))?,
    );

    let has_recipient = bytes[200];
    let relay_tip_recipient = if has_recipient == 1 {
        if bytes.len() < 233 {
            return Err(ChannelError::InvalidRelayBlob);
        }
        let mut recipient = [0u8; 32];
        recipient.copy_from_slice(&bytes[201..233]);
        Some(recipient)
    } else {
        None
    };

    Ok(RelayBlob {
        signed_state: SignedState {
            state: state_update,
            signature_a,
            signature_b,
        },
        relay_fee,
        relay_tip_recipient,
    })
}

/// Serialize a relay blob for BLE advertisement broadcast.
///
/// BLE advertisements have a maximum payload of ~200 bytes.
/// This format omits the optional recipient to stay under limit.
/// Layout:
///   - state bytes (64)
///   - signature_a (64)
///   - signature_b (64)
///   - relay_fee (8)
///     Total: 200 bytes exactly.
pub fn blob_to_ble_advertisement(blob: &RelayBlob) -> Vec<u8> {
    let state_bytes = state::to_bytes(&blob.signed_state.state);
    let mut buf = Vec::with_capacity(200);

    buf.extend_from_slice(&state_bytes); // 64 bytes
    buf.extend_from_slice(&blob.signed_state.signature_a); // 64 bytes
    buf.extend_from_slice(&blob.signed_state.signature_b); // 64 bytes
    buf.extend_from_slice(&blob.relay_fee.to_le_bytes()); // 8 bytes

    debug!(size = buf.len(), "relay blob serialized for BLE");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{self, StateUpdate};
    use ed25519_dalek::SigningKey;

    fn make_signed_state() -> (SignedState, [u8; 32], [u8; 32]) {
        let key_a = SigningKey::from_bytes(&[1u8; 32]);
        let key_b = SigningKey::from_bytes(&[2u8; 32]);
        let pub_a = key_a.verifying_key().to_bytes();
        let pub_b = key_b.verifying_key().to_bytes();

        let state_update = StateUpdate {
            channel_id: [0xAA; 32],
            balance_a: 800_000,
            balance_b: 1_200_000,
            sequence: 5,
            timestamp: 1700000000,
        };

        let sig_a = state::sign(&state_update, &key_a);
        let sig_b = state::sign(&state_update, &key_b);

        let signed = SignedState {
            state: state_update,
            signature_a: sig_a,
            signature_b: sig_b,
        };

        (signed, pub_a, pub_b)
    }

    #[test]
    fn create_and_validate_relay_blob() {
        let (signed, pub_a, pub_b) = make_signed_state();
        let blob = create_relay_blob(signed, 1_000);
        assert!(validate_relay_blob(&blob, &pub_a, &pub_b));
    }

    #[test]
    fn validate_fails_with_wrong_keys() {
        let (signed, _, _) = make_signed_state();
        let blob = create_relay_blob(signed, 1_000);
        let fake_pub = [0xFF; 32];
        // With invalid pubkeys the signature check should fail
        assert!(!validate_relay_blob(&blob, &fake_pub, &fake_pub));
    }

    #[test]
    fn qr_bytes_roundtrip_no_recipient() {
        let (signed, pub_a, pub_b) = make_signed_state();
        let blob = create_relay_blob(signed, 500);

        let qr_bytes = blob_to_qr_bytes(&blob);
        assert_eq!(qr_bytes.len(), 201); // No recipient

        let recovered = blob_from_qr_bytes(&qr_bytes).unwrap();
        assert_eq!(recovered.signed_state.state, blob.signed_state.state);
        assert_eq!(recovered.relay_fee, 500);
        assert!(recovered.relay_tip_recipient.is_none());
        assert!(validate_relay_blob(&recovered, &pub_a, &pub_b));
    }

    #[test]
    fn qr_bytes_roundtrip_with_recipient() {
        let (signed, pub_a, pub_b) = make_signed_state();
        let mut blob = create_relay_blob(signed, 1_000);
        blob.relay_tip_recipient = Some([0xBB; 32]);

        let qr_bytes = blob_to_qr_bytes(&blob);
        assert_eq!(qr_bytes.len(), 233); // With recipient

        let recovered = blob_from_qr_bytes(&qr_bytes).unwrap();
        assert_eq!(recovered.relay_tip_recipient, Some([0xBB; 32]));
        assert!(validate_relay_blob(&recovered, &pub_a, &pub_b));
    }

    #[test]
    fn ble_advertisement_size() {
        let (signed, _, _) = make_signed_state();
        let blob = create_relay_blob(signed, 100);
        let ble_bytes = blob_to_ble_advertisement(&blob);
        assert_eq!(ble_bytes.len(), 200);
    }

    #[test]
    fn qr_bytes_too_short() {
        let err = blob_from_qr_bytes(&[0u8; 100]);
        assert!(err.is_err());
    }
}
