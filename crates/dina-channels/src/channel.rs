use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::error::{ChannelError, Result};
use crate::state::{self, SignedState, StateUpdate};

/// The lifecycle status of a payment channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelStatus {
    /// Channel has been created but not yet confirmed on-chain.
    Opening,
    /// Channel is open and accepting state updates.
    Open,
    /// One party has initiated unilateral close; challenge period is active.
    Closing,
    /// Channel has been settled and funds distributed.
    Closed,
    /// A dispute has been raised during the challenge period.
    Disputed,
}

/// A bidirectional payment channel between two parties (Cognitum Seeds).
///
/// Funds are locked in the channel and can be redistributed off-chain
/// via signed state updates. Settlement happens on-chain when the
/// channel is closed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentChannel {
    /// Unique identifier derived from hash(party_a || party_b || nonce).
    pub channel_id: [u8; 32],
    /// Ed25519 public key of party A.
    pub party_a: [u8; 32],
    /// Ed25519 public key of party B.
    pub party_b: [u8; 32],
    /// Current balance of party A in USDC micro-units.
    pub balance_a: u64,
    /// Current balance of party B in USDC micro-units.
    pub balance_b: u64,
    /// Total locked funds (balance_a + balance_b, invariant).
    pub total_locked: u64,
    /// Monotonically increasing sequence number for state updates.
    pub sequence: u64,
    /// Current lifecycle status.
    pub status: ChannelStatus,
    /// Unix timestamp when the channel was created.
    pub created_at: u64,
    /// Number of blocks for the challenge period during unilateral close.
    pub timeout_blocks: u64,
    /// The latest signed state submitted during closing/dispute (if any).
    latest_closing_state: Option<SignedState>,
}

impl PaymentChannel {
    /// Create a new payment channel between two parties with initial deposits.
    ///
    /// The channel ID is derived deterministically from the two public keys
    /// and the current timestamp as a nonce.
    pub fn open(
        party_a: [u8; 32],
        party_b: [u8; 32],
        deposit_a: u64,
        deposit_b: u64,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        let channel_id = derive_channel_id(&party_a, &party_b, now);
        // Use saturating_add to prevent panic on overflow. In practice the
        // on-chain validation layer should reject deposits that would overflow
        // before reaching this point.
        let total = deposit_a.saturating_add(deposit_b);

        debug!(
            channel_id = hex::encode(channel_id),
            deposit_a,
            deposit_b,
            "opening payment channel"
        );

        PaymentChannel {
            channel_id,
            party_a,
            party_b,
            balance_a: deposit_a,
            balance_b: deposit_b,
            total_locked: total,
            sequence: 0,
            status: ChannelStatus::Open,
            created_at: now,
            timeout_blocks: 100, // Default challenge period
            latest_closing_state: None,
        }
    }

    /// Create a state update that transfers `amount_to_b` micro-units from A to B.
    ///
    /// Pass a value where the transfer goes from A to B. To transfer from B to A,
    /// pass a negative conceptual amount by having the caller adjust accordingly
    /// (i.e., call `update` with the amount that B wants to send, and swap roles).
    ///
    /// Returns the new StateUpdate with an incremented sequence number.
    pub fn update(&mut self, amount_to_b: u64) -> Result<StateUpdate> {
        if self.status != ChannelStatus::Open {
            return Err(ChannelError::ChannelClosed);
        }

        if amount_to_b > self.balance_a {
            return Err(ChannelError::InsufficientBalance {
                need: amount_to_b,
                have: self.balance_a,
            });
        }

        self.balance_a -= amount_to_b;
        self.balance_b += amount_to_b;
        self.sequence += 1;

        debug!(
            channel_id = hex::encode(self.channel_id),
            sequence = self.sequence,
            balance_a = self.balance_a,
            balance_b = self.balance_b,
            "channel state updated"
        );

        Ok(StateUpdate {
            channel_id: self.channel_id,
            balance_a: self.balance_a,
            balance_b: self.balance_b,
            sequence: self.sequence,
            timestamp: chrono::Utc::now().timestamp() as u64,
        })
    }

    /// Cooperatively close the channel. Both parties must have signed the final state.
    ///
    /// This is the happy path: both parties agree on the final balances and
    /// the channel can be settled immediately without a challenge period.
    pub fn close_cooperative(&mut self, final_state: &SignedState) -> Result<()> {
        if self.status == ChannelStatus::Closed {
            return Err(ChannelError::ChannelClosed);
        }

        // Verify both signatures
        if !state::is_valid(final_state, &self.party_a, &self.party_b) {
            return Err(ChannelError::InvalidSignature);
        }

        // Verify the state belongs to this channel
        if final_state.state.channel_id != self.channel_id {
            return Err(ChannelError::InvalidSignature);
        }

        // Verify conservation of funds (with overflow protection)
        let total = final_state.state.balance_a.checked_add(final_state.state.balance_b)
            .ok_or_else(|| ChannelError::InvalidSignature)?;
        if total != self.total_locked {
            return Err(ChannelError::InsufficientBalance {
                need: self.total_locked,
                have: total,
            });
        }

        self.balance_a = final_state.state.balance_a;
        self.balance_b = final_state.state.balance_b;
        self.sequence = final_state.state.sequence;
        self.status = ChannelStatus::Closed;

        debug!(
            channel_id = hex::encode(self.channel_id),
            "channel closed cooperatively"
        );

        Ok(())
    }

    /// Unilaterally close the channel by submitting a signed state.
    ///
    /// This starts the challenge period during which the other party can
    /// submit a newer state. The submitter must be one of the channel parties.
    pub fn close_unilateral(
        &mut self,
        submitter: &[u8; 32],
        signed_state: &SignedState,
    ) -> Result<()> {
        if self.status == ChannelStatus::Closed {
            return Err(ChannelError::ChannelClosed);
        }

        // Verify the submitter is a party to the channel
        if submitter != &self.party_a && submitter != &self.party_b {
            return Err(ChannelError::NotPartyToChannel);
        }

        // Verify both signatures on the state
        if !state::is_valid(signed_state, &self.party_a, &self.party_b) {
            return Err(ChannelError::InvalidSignature);
        }

        // Verify channel ID matches
        if signed_state.state.channel_id != self.channel_id {
            return Err(ChannelError::InvalidSignature);
        }

        // Verify conservation of funds (with overflow protection)
        let total = signed_state.state.balance_a.checked_add(signed_state.state.balance_b)
            .ok_or_else(|| ChannelError::InvalidSignature)?;
        if total != self.total_locked {
            return Err(ChannelError::InsufficientBalance {
                need: self.total_locked,
                have: total,
            });
        }

        self.balance_a = signed_state.state.balance_a;
        self.balance_b = signed_state.state.balance_b;
        self.sequence = signed_state.state.sequence;
        self.status = ChannelStatus::Closing;
        self.latest_closing_state = Some(signed_state.clone());

        warn!(
            channel_id = hex::encode(self.channel_id),
            submitter = hex::encode(submitter),
            sequence = self.sequence,
            "unilateral close initiated, challenge period started"
        );

        Ok(())
    }

    /// Challenge a unilateral close by submitting a state with a higher sequence number.
    ///
    /// During the challenge period, either party can submit a newer signed state
    /// to prove the submitted closing state was outdated.
    pub fn challenge(
        &mut self,
        challenger: &[u8; 32],
        signed_state: &SignedState,
    ) -> Result<()> {
        if self.status != ChannelStatus::Closing && self.status != ChannelStatus::Disputed {
            return Err(ChannelError::ChannelClosed);
        }

        // Verify the challenger is a party to the channel
        if challenger != &self.party_a && challenger != &self.party_b {
            return Err(ChannelError::NotPartyToChannel);
        }

        // Verify both signatures
        if !state::is_valid(signed_state, &self.party_a, &self.party_b) {
            return Err(ChannelError::InvalidSignature);
        }

        // Verify channel ID matches
        if signed_state.state.channel_id != self.channel_id {
            return Err(ChannelError::InvalidSignature);
        }

        // The challenged state must have a strictly higher sequence number
        if signed_state.state.sequence <= self.sequence {
            return Err(ChannelError::InvalidSequence {
                got: signed_state.state.sequence,
                current: self.sequence,
            });
        }

        // Verify conservation of funds (with overflow protection)
        let total = signed_state.state.balance_a.checked_add(signed_state.state.balance_b)
            .ok_or_else(|| ChannelError::InvalidSignature)?;
        if total != self.total_locked {
            return Err(ChannelError::InsufficientBalance {
                need: self.total_locked,
                have: total,
            });
        }

        self.balance_a = signed_state.state.balance_a;
        self.balance_b = signed_state.state.balance_b;
        self.sequence = signed_state.state.sequence;
        self.status = ChannelStatus::Disputed;
        self.latest_closing_state = Some(signed_state.clone());

        debug!(
            channel_id = hex::encode(self.channel_id),
            challenger = hex::encode(challenger),
            sequence = self.sequence,
            "challenge submitted with newer state"
        );

        Ok(())
    }

    /// Finalize the channel after the challenge period has expired.
    ///
    /// Applies the latest submitted state and moves the channel to Closed.
    pub fn finalize(&mut self) -> Result<()> {
        match self.status {
            ChannelStatus::Closing | ChannelStatus::Disputed => {
                self.status = ChannelStatus::Closed;
                debug!(
                    channel_id = hex::encode(self.channel_id),
                    balance_a = self.balance_a,
                    balance_b = self.balance_b,
                    "channel finalized"
                );
                Ok(())
            }
            ChannelStatus::Closed => Err(ChannelError::ChannelClosed),
            _ => Err(ChannelError::ChallengePeriodActive),
        }
    }
}

/// Derive a deterministic channel ID from the two party public keys and a nonce.
fn derive_channel_id(party_a: &[u8; 32], party_b: &[u8; 32], nonce: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(party_a);
    hasher.update(party_b);
    hasher.update(nonce.to_le_bytes());
    let result = hasher.finalize();
    let mut id = [0u8; 32];
    id.copy_from_slice(&result);
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state;
    use ed25519_dalek::SigningKey;

    fn setup() -> (SigningKey, SigningKey, PaymentChannel) {
        let key_a = SigningKey::from_bytes(&[1u8; 32]);
        let key_b = SigningKey::from_bytes(&[2u8; 32]);
        let pub_a = key_a.verifying_key().to_bytes();
        let pub_b = key_b.verifying_key().to_bytes();
        let channel = PaymentChannel::open(pub_a, pub_b, 1_000_000, 1_000_000);
        (key_a, key_b, channel)
    }

    #[test]
    fn open_channel_sets_correct_balances() {
        let (_, _, channel) = setup();
        assert_eq!(channel.balance_a, 1_000_000);
        assert_eq!(channel.balance_b, 1_000_000);
        assert_eq!(channel.total_locked, 2_000_000);
        assert_eq!(channel.sequence, 0);
        assert_eq!(channel.status, ChannelStatus::Open);
    }

    #[test]
    fn update_transfers_from_a_to_b() {
        let (_, _, mut channel) = setup();
        let update = channel.update(100_000).unwrap();
        assert_eq!(update.balance_a, 900_000);
        assert_eq!(update.balance_b, 1_100_000);
        assert_eq!(update.sequence, 1);
        assert_eq!(channel.balance_a, 900_000);
        assert_eq!(channel.balance_b, 1_100_000);
    }

    #[test]
    fn update_insufficient_balance() {
        let (_, _, mut channel) = setup();
        let err = channel.update(1_500_000).unwrap_err();
        assert!(matches!(err, ChannelError::InsufficientBalance { .. }));
    }

    #[test]
    fn cooperative_close() {
        let (key_a, key_b, mut channel) = setup();
        let state_update = channel.update(200_000).unwrap();
        let sig_a = state::sign(&state_update, &key_a);
        let sig_b = state::sign(&state_update, &key_b);

        let signed = SignedState {
            state: state_update,
            signature_a: sig_a,
            signature_b: sig_b,
        };

        channel.close_cooperative(&signed).unwrap();
        assert_eq!(channel.status, ChannelStatus::Closed);
        assert_eq!(channel.balance_a, 800_000);
        assert_eq!(channel.balance_b, 1_200_000);
    }

    #[test]
    fn unilateral_close_and_challenge() {
        let (key_a, key_b, mut channel) = setup();
        let pub_a = key_a.verifying_key().to_bytes();
        let pub_b = key_b.verifying_key().to_bytes();

        // Create state at sequence 1
        let state1 = channel.update(100_000).unwrap();
        let sig_a1 = state::sign(&state1, &key_a);
        let sig_b1 = state::sign(&state1, &key_b);
        let signed1 = SignedState {
            state: state1,
            signature_a: sig_a1,
            signature_b: sig_b1,
        };

        // Create state at sequence 2
        let state2 = channel.update(50_000).unwrap();
        let sig_a2 = state::sign(&state2, &key_a);
        let sig_b2 = state::sign(&state2, &key_b);
        let signed2 = SignedState {
            state: state2,
            signature_a: sig_a2,
            signature_b: sig_b2,
        };

        // Party A submits old state (sequence 1)
        // We need a fresh channel with same params to simulate on-chain
        let mut on_chain = channel.clone();
        on_chain.sequence = 0;
        on_chain.balance_a = 1_000_000;
        on_chain.balance_b = 1_000_000;
        on_chain.status = ChannelStatus::Open;

        on_chain.close_unilateral(&pub_a, &signed1).unwrap();
        assert_eq!(on_chain.status, ChannelStatus::Closing);

        // Party B challenges with newer state (sequence 2)
        on_chain.challenge(&pub_b, &signed2).unwrap();
        assert_eq!(on_chain.status, ChannelStatus::Disputed);
        assert_eq!(on_chain.balance_a, 850_000);
        assert_eq!(on_chain.balance_b, 1_150_000);

        // Finalize after challenge period
        on_chain.finalize().unwrap();
        assert_eq!(on_chain.status, ChannelStatus::Closed);
    }

    #[test]
    fn challenge_with_lower_sequence_fails() {
        let (key_a, key_b, mut channel) = setup();
        let pub_b = key_b.verifying_key().to_bytes();

        let state1 = channel.update(100_000).unwrap();
        let sig_a1 = state::sign(&state1, &key_a);
        let sig_b1 = state::sign(&state1, &key_b);

        let state2 = channel.update(50_000).unwrap();
        let sig_a2 = state::sign(&state2, &key_a);
        let sig_b2 = state::sign(&state2, &key_b);

        let signed1 = SignedState {
            state: state1,
            signature_a: sig_a1,
            signature_b: sig_b1,
        };
        let signed2 = SignedState {
            state: state2,
            signature_a: sig_a2,
            signature_b: sig_b2,
        };

        let mut on_chain = channel.clone();
        on_chain.sequence = 0;
        on_chain.balance_a = 1_000_000;
        on_chain.balance_b = 1_000_000;
        on_chain.status = ChannelStatus::Open;

        // Submit newer state first
        on_chain
            .close_unilateral(&key_a.verifying_key().to_bytes(), &signed2)
            .unwrap();

        // Challenge with older state should fail
        let err = on_chain.challenge(&pub_b, &signed1).unwrap_err();
        assert!(matches!(err, ChannelError::InvalidSequence { .. }));
    }

    #[test]
    fn non_party_cannot_close() {
        let (key_a, key_b, mut channel) = setup();
        let outsider_key = SigningKey::from_bytes(&[3u8; 32]);
        let outsider_pub = outsider_key.verifying_key().to_bytes();

        let state1 = channel.update(100_000).unwrap();
        let sig_a = state::sign(&state1, &key_a);
        let sig_b = state::sign(&state1, &key_b);
        let signed = SignedState {
            state: state1,
            signature_a: sig_a,
            signature_b: sig_b,
        };

        let err = channel.close_unilateral(&outsider_pub, &signed).unwrap_err();
        assert!(matches!(err, ChannelError::NotPartyToChannel));
    }
}
