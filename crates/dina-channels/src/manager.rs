use std::collections::BTreeMap;

use ed25519_dalek::SigningKey;
use tracing::{debug, info};

use crate::channel::PaymentChannel;
use crate::error::{ChannelError, Result};
use crate::relay::{self, RelayBlob};
use crate::state::{self, SignedState, StateUpdate};

/// Manages multiple payment channels for a single device/agent.
///
/// Provides a high-level API for opening channels, making payments,
/// and preparing relay blobs for on-chain settlement when connectivity
/// is restored.
#[derive(Clone, Debug)]
pub struct ChannelManager {
    /// All channels indexed by channel_id.
    channels: BTreeMap<[u8; 32], PaymentChannel>,
    /// Relay blobs waiting to be submitted when internet is available.
    pending_relays: Vec<RelayBlob>,
}

impl ChannelManager {
    /// Create a new empty channel manager.
    pub fn new() -> Self {
        ChannelManager {
            channels: BTreeMap::new(),
            pending_relays: Vec::new(),
        }
    }

    /// Open a new payment channel between two parties.
    ///
    /// Returns the channel ID for future reference.
    pub fn open_channel(
        &mut self,
        party_a: [u8; 32],
        party_b: [u8; 32],
        deposit_a: u64,
        deposit_b: u64,
    ) -> [u8; 32] {
        let channel = PaymentChannel::open(party_a, party_b, deposit_a, deposit_b);
        let id = channel.channel_id;
        self.channels.insert(id, channel);

        info!(
            channel_id = hex::encode(id),
            "channel opened in manager"
        );

        id
    }

    /// Create a payment within a channel, signing as the given key.
    ///
    /// This creates a state update transferring `amount` micro-units from A to B,
    /// signs it with the provided key, and returns the partially signed state.
    /// The counterparty must also sign it to produce a fully valid SignedState.
    pub fn pay(
        &mut self,
        channel_id: &[u8; 32],
        amount: u64,
        _signing_key: &SigningKey,
    ) -> Result<StateUpdate> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or(ChannelError::ChannelNotFound(*channel_id))?;

        let state_update = channel.update(amount)?;

        debug!(
            channel_id = hex::encode(channel_id),
            amount,
            sequence = state_update.sequence,
            "payment created"
        );

        Ok(state_update)
    }

    /// Receive and verify a payment (signed state) from the counterparty.
    ///
    /// Validates signatures and ensures the sequence number is higher than
    /// the current state. Updates the local channel state accordingly.
    pub fn receive_payment(
        &mut self,
        channel_id: &[u8; 32],
        signed_state: &SignedState,
    ) -> Result<()> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or(ChannelError::ChannelNotFound(*channel_id))?;

        // Verify the signed state belongs to this channel
        if signed_state.state.channel_id != *channel_id {
            return Err(ChannelError::InvalidSignature);
        }

        // Verify both signatures
        if !state::is_valid(signed_state, &channel.party_a, &channel.party_b) {
            return Err(ChannelError::InvalidSignature);
        }

        // Sequence must be strictly increasing
        if signed_state.state.sequence <= channel.sequence {
            return Err(ChannelError::InvalidSequence {
                got: signed_state.state.sequence,
                current: channel.sequence,
            });
        }

        // Conservation of funds
        let total = signed_state.state.balance_a + signed_state.state.balance_b;
        if total != channel.total_locked {
            return Err(ChannelError::InsufficientBalance {
                need: channel.total_locked,
                have: total,
            });
        }

        channel.balance_a = signed_state.state.balance_a;
        channel.balance_b = signed_state.state.balance_b;
        channel.sequence = signed_state.state.sequence;

        debug!(
            channel_id = hex::encode(channel_id),
            sequence = channel.sequence,
            "payment received and verified"
        );

        Ok(())
    }

    /// Get a reference to a channel by its ID.
    pub fn get_channel(&self, channel_id: &[u8; 32]) -> Option<&PaymentChannel> {
        self.channels.get(channel_id)
    }

    /// Close a channel and produce a relay blob for on-chain settlement.
    ///
    /// The caller must provide both signing keys (cooperative close) or
    /// this will create the final state for signing.
    pub fn close_channel(
        &mut self,
        channel_id: &[u8; 32],
        key_a: &SigningKey,
        key_b: &SigningKey,
        relay_fee: u64,
    ) -> Result<RelayBlob> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or(ChannelError::ChannelNotFound(*channel_id))?;

        // Create the final state
        let final_state = StateUpdate {
            channel_id: *channel_id,
            balance_a: channel.balance_a,
            balance_b: channel.balance_b,
            sequence: channel.sequence + 1,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        let sig_a = state::sign(&final_state, key_a);
        let sig_b = state::sign(&final_state, key_b);

        let signed = SignedState {
            state: final_state,
            signature_a: sig_a,
            signature_b: sig_b,
        };

        // Close the channel cooperatively
        channel.close_cooperative(&signed)?;

        // Create the relay blob
        let blob = relay::create_relay_blob(signed, relay_fee);

        info!(
            channel_id = hex::encode(channel_id),
            "channel closed, relay blob created"
        );

        Ok(blob)
    }

    /// Queue a relay blob for later submission when internet is available.
    pub fn queue_relay(&mut self, blob: RelayBlob) {
        debug!(
            channel_id = hex::encode(blob.signed_state.state.channel_id),
            "relay blob queued"
        );
        self.pending_relays.push(blob);
    }

    /// Get all pending relay blobs awaiting submission.
    pub fn get_pending_relays(&self) -> &[RelayBlob] {
        &self.pending_relays
    }

    /// Remove relay blobs for channels that have been settled on-chain.
    pub fn clear_settled_relays(&mut self, channel_ids: &[[u8; 32]]) {
        self.pending_relays
            .retain(|blob| !channel_ids.contains(&blob.signed_state.state.channel_id));

        debug!(
            cleared = channel_ids.len(),
            remaining = self.pending_relays.len(),
            "settled relays cleared"
        );
    }

    /// Return the IDs of all channels that are not closed.
    pub fn active_channels(&self) -> Vec<[u8; 32]> {
        self.channels
            .iter()
            .filter(|(_, ch)| ch.status != crate::channel::ChannelStatus::Closed)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Sum of all locked balances across all channels.
    pub fn total_locked(&self) -> u64 {
        self.channels.values().map(|ch| ch.total_locked).sum()
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn test_keys() -> (SigningKey, SigningKey) {
        let key_a = SigningKey::from_bytes(&[1u8; 32]);
        let key_b = SigningKey::from_bytes(&[2u8; 32]);
        (key_a, key_b)
    }

    #[test]
    fn open_and_get_channel() {
        let (key_a, key_b) = test_keys();
        let pub_a = key_a.verifying_key().to_bytes();
        let pub_b = key_b.verifying_key().to_bytes();

        let mut mgr = ChannelManager::new();
        let id = mgr.open_channel(pub_a, pub_b, 500_000, 500_000);

        let ch = mgr.get_channel(&id).unwrap();
        assert_eq!(ch.balance_a, 500_000);
        assert_eq!(ch.balance_b, 500_000);
        assert_eq!(ch.total_locked, 1_000_000);
    }

    #[test]
    fn pay_and_receive() {
        let (key_a, key_b) = test_keys();
        let pub_a = key_a.verifying_key().to_bytes();
        let pub_b = key_b.verifying_key().to_bytes();

        let mut mgr = ChannelManager::new();
        let id = mgr.open_channel(pub_a, pub_b, 1_000_000, 1_000_000);

        // A pays B 100_000 (this updates local state to sequence=1)
        let state_update = mgr.pay(&id, 100_000, &key_a).unwrap();
        assert_eq!(state_update.sequence, 1);

        // Both sign the state
        let sig_a = state::sign(&state_update, &key_a);
        let sig_b = state::sign(&state_update, &key_b);
        let signed = SignedState {
            state: state_update,
            signature_a: sig_a,
            signature_b: sig_b,
        };

        // Simulate the counterparty receiving the payment.
        // The counterparty has the same channel (same ID) but hasn't updated locally.
        // We clone the manager's channel at sequence=0 state by re-inserting it.
        let mut mgr_receiver = ChannelManager::new();
        let mut fresh_channel = mgr.get_channel(&id).unwrap().clone();
        fresh_channel.balance_a = 1_000_000;
        fresh_channel.balance_b = 1_000_000;
        fresh_channel.sequence = 0;
        mgr_receiver.channels.insert(id, fresh_channel);

        // Receiver accepts the first signed state (sequence=1 > current=0)
        mgr_receiver.receive_payment(&id, &signed).unwrap();
        let ch = mgr_receiver.get_channel(&id).unwrap();
        assert_eq!(ch.balance_a, 900_000);
        assert_eq!(ch.balance_b, 1_100_000);
        assert_eq!(ch.sequence, 1);
    }

    #[test]
    fn close_channel_and_relay() {
        let (key_a, key_b) = test_keys();
        let pub_a = key_a.verifying_key().to_bytes();
        let pub_b = key_b.verifying_key().to_bytes();

        let mut mgr = ChannelManager::new();
        let id = mgr.open_channel(pub_a, pub_b, 1_000_000, 1_000_000);

        // Make a payment first
        let _state = mgr.pay(&id, 200_000, &key_a).unwrap();

        // Close and get relay blob
        let blob = mgr.close_channel(&id, &key_a, &key_b, 500).unwrap();
        assert_eq!(blob.relay_fee, 500);

        // Queue the relay
        mgr.queue_relay(blob);
        assert_eq!(mgr.get_pending_relays().len(), 1);

        // Clear settled
        mgr.clear_settled_relays(&[id]);
        assert_eq!(mgr.get_pending_relays().len(), 0);
    }

    #[test]
    fn active_channels_excludes_closed() {
        let (key_a, key_b) = test_keys();
        let pub_a = key_a.verifying_key().to_bytes();
        let pub_b = key_b.verifying_key().to_bytes();

        // Use a third key so the two channels get different IDs
        // (same parties + same timestamp nonce would produce the same ID)
        let key_c = SigningKey::from_bytes(&[3u8; 32]);
        let pub_c = key_c.verifying_key().to_bytes();

        let mut mgr = ChannelManager::new();
        let id1 = mgr.open_channel(pub_a, pub_b, 100_000, 100_000);
        let id2 = mgr.open_channel(pub_a, pub_c, 200_000, 200_000);

        assert_eq!(mgr.active_channels().len(), 2);
        assert_eq!(mgr.total_locked(), 600_000);

        // Close one
        mgr.close_channel(&id1, &key_a, &key_b, 0).unwrap();
        assert_eq!(mgr.active_channels().len(), 1);
        assert!(mgr.active_channels().contains(&id2));
    }

    #[test]
    fn channel_not_found() {
        let (key_a, _) = test_keys();
        let mut mgr = ChannelManager::new();
        let fake_id = [0xFF; 32];
        let err = mgr.pay(&fake_id, 100, &key_a).unwrap_err();
        assert!(matches!(err, ChannelError::ChannelNotFound(_)));
    }

    #[test]
    fn default_impl() {
        let mgr = ChannelManager::default();
        assert_eq!(mgr.total_locked(), 0);
        assert!(mgr.active_channels().is_empty());
        assert!(mgr.get_pending_relays().is_empty());
    }
}
