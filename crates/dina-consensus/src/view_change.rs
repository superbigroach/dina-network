use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// A view change message sent by a validator when the current round leader
/// is unresponsive. Once 2/3+ validators send view change messages for the
/// same (height, new_round), the round advances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChange {
    pub height: u64,
    pub old_round: u32,
    pub new_round: u32,
    pub voter: [u8; 32],
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}

impl ViewChange {
    /// Create a new signed view change message.
    pub fn new(
        height: u64,
        old_round: u32,
        new_round: u32,
        signing_key: &SigningKey,
    ) -> Self {
        let voter = signing_key.verifying_key().to_bytes();
        let sign_bytes = Self::sign_bytes(height, old_round, new_round);
        let signature = signing_key.sign(&sign_bytes);

        ViewChange {
            height,
            old_round,
            new_round,
            voter,
            signature: signature.to_bytes(),
        }
    }

    /// Compute the bytes to sign: SHA-256("VIEWCHANGE" || height || old_round || new_round).
    fn sign_bytes(height: u64, old_round: u32, new_round: u32) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"VIEWCHANGE");
        hasher.update(height.to_le_bytes());
        hasher.update(old_round.to_le_bytes());
        hasher.update(new_round.to_le_bytes());
        hasher.finalize().to_vec()
    }

    /// Verify the voter's ed25519 signature on this view change message.
    pub fn verify_signature(&self) -> bool {
        let verifying_key = match VerifyingKey::from_bytes(&self.voter) {
            Ok(k) => k,
            Err(_) => {
                warn!("Invalid view change voter public key");
                return false;
            }
        };
        let sign_bytes = Self::sign_bytes(self.height, self.old_round, self.new_round);
        let signature = Signature::from_bytes(&self.signature);
        verifying_key.verify(&sign_bytes, &signature).is_ok()
    }
}

/// Collects view change messages from validators and determines when enough
/// have been received to advance to a new round.
///
/// The collector is scoped to a single height. When the height advances,
/// create a new collector.
#[derive(Debug)]
pub struct ViewChangeCollector {
    height: u64,
    total_validators: usize,
    /// Maps new_round -> (voter_pubkey -> ViewChange).
    /// We track per-round so validators can propose different target rounds.
    messages: HashMap<u32, HashMap<[u8; 32], ViewChange>>,
    /// Set of valid validator public keys for authorization.
    validator_set: std::collections::HashSet<[u8; 32]>,
}

impl ViewChangeCollector {
    /// Create a new collector for the given height and validator set.
    pub fn new(height: u64, validators: &[[u8; 32]]) -> Self {
        let validator_set = validators.iter().cloned().collect();
        ViewChangeCollector {
            height,
            total_validators: validators.len(),
            messages: HashMap::new(),
            validator_set,
        }
    }

    /// The quorum needed for a view change: 2f + 1 = ceil(2n/3).
    pub fn quorum_size(&self) -> usize {
        (self.total_validators * 2 + 2) / 3
    }

    /// Add a view change message. Returns `Some(new_round)` if quorum has now
    /// been reached for that round, triggering the view change. Returns `None`
    /// if more messages are still needed.
    ///
    /// Validates:
    /// - Height matches
    /// - new_round > old_round
    /// - Voter is a known validator
    /// - Signature is valid
    /// - No duplicate from the same voter for the same new_round
    pub fn add_view_change(&mut self, vc: ViewChange) -> Option<u32> {
        // Validate height
        if vc.height != self.height {
            debug!(
                expected = self.height,
                got = vc.height,
                "Rejected view change: wrong height"
            );
            return None;
        }

        // Validate round progression
        if vc.new_round <= vc.old_round {
            warn!(
                old_round = vc.old_round,
                new_round = vc.new_round,
                "Rejected view change: new_round must be > old_round"
            );
            return None;
        }

        // Validate voter is a known validator
        if !self.validator_set.contains(&vc.voter) {
            warn!(
                voter = hex::encode(vc.voter),
                "Rejected view change: unknown validator"
            );
            return None;
        }

        // Verify signature
        if !vc.verify_signature() {
            warn!(
                voter = hex::encode(vc.voter),
                "Rejected view change: invalid signature"
            );
            return None;
        }

        let new_round = vc.new_round;
        let quorum = self.quorum_size();
        let round_messages = self.messages.entry(new_round).or_default();

        // Reject duplicates
        if round_messages.contains_key(&vc.voter) {
            debug!(
                voter = hex::encode(vc.voter),
                new_round,
                "Rejected duplicate view change"
            );
            return None;
        }

        info!(
            voter = hex::encode(vc.voter),
            height = self.height,
            old_round = vc.old_round,
            new_round,
            count = round_messages.len() + 1,
            quorum,
            "Received view change message"
        );

        round_messages.insert(vc.voter, vc);

        // Check if quorum reached for this new_round
        if round_messages.len() >= quorum {
            info!(
                height = self.height,
                new_round,
                "View change quorum reached — advancing to new round"
            );
            Some(new_round)
        } else {
            None
        }
    }

    /// Get the number of view change messages received for a specific target round.
    pub fn count_for_round(&self, new_round: u32) -> usize {
        self.messages
            .get(&new_round)
            .map_or(0, |m| m.len())
    }

    /// Get all view change messages for a specific target round.
    pub fn messages_for_round(&self, new_round: u32) -> Vec<ViewChange> {
        self.messages
            .get(&new_round)
            .map_or_else(Vec::new, |m| m.values().cloned().collect())
    }

    /// Reset the collector for a new height (called after height advances).
    pub fn reset(&mut self, new_height: u64) {
        self.height = new_height;
        self.messages.clear();
    }
}
