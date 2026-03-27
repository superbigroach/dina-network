use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Wormhole Integration — Cross-Chain Token Transfers via Wormhole
// ---------------------------------------------------------------------------
//
// Implements a Wormhole-compatible bridge for Dina Network. Wormhole uses
// a guardian network (19 guardians) that observe and attest to cross-chain
// messages via Verified Action Approvals (VAAs).
//
// Flow (send from Dina):
//   1. User calls send_tokens() — burns tokens and creates a VAA payload
//   2. Wormhole guardians observe the on-chain event
//   3. Guardians sign the message, creating a VAA
//   4. Anyone submits the VAA to the target chain
//   5. Target chain verifies guardian signatures and mints tokens
//
// Flow (receive on Dina):
//   1. User burns tokens on source chain
//   2. Guardians create a VAA
//   3. User or relayer calls complete_transfer() on Dina with the VAA
//   4. Contract verifies guardian signatures and mints tokens
//
// Wormhole Chain IDs:
//   Solana = 1, Ethereum = 2, Arbitrum = 23, Base = 30, Dina = 99
// ---------------------------------------------------------------------------

/// Wormhole chain ID constants.
pub const CHAIN_SOLANA: u16 = 1;
pub const CHAIN_ETHEREUM: u16 = 2;
pub const CHAIN_ARBITRUM: u16 = 23;
pub const CHAIN_BASE: u16 = 30;
pub const CHAIN_DINA: u16 = 99;

/// A Wormhole guardian — holds a public key for signature verification.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Guardian {
    /// Guardian index in the guardian set
    pub index: u8,
    /// Guardian's public key (32 bytes)
    pub pubkey: [u8; 32],
}

/// The guardian set — a versioned collection of guardian public keys.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuardianSet {
    /// Guardian set index (increments on each guardian set update)
    pub index: u32,
    /// List of guardians in this set
    pub guardians: Vec<Guardian>,
    /// Number of signatures required (2/3 + 1 of guardians)
    pub quorum: u32,
}

/// A Verified Action Approval (VAA) — Wormhole's cross-chain message format.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vaa {
    /// VAA version (currently 1)
    pub version: u8,
    /// Guardian set index that signed this VAA
    pub guardian_set_index: u32,
    /// Timestamp of the observation
    pub timestamp: u32,
    /// Unique nonce
    pub nonce: u32,
    /// Source chain ID
    pub emitter_chain: u16,
    /// Emitter contract address on the source chain
    pub emitter_address: [u8; 32],
    /// Sequence number from the emitter
    pub sequence: u64,
    /// Consistency level (finality requirements)
    pub consistency_level: u8,
    /// The actual message payload
    pub payload: VaaPayload,
    /// Guardian signatures (simplified — just the signing guardian indices
    /// and a hash-based signature for verification)
    pub signatures: Vec<VaaSignature>,
}

/// A guardian signature on a VAA (simplified).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaaSignature {
    /// Index of the guardian in the guardian set
    pub guardian_index: u8,
    /// Simplified signature: SHA-256(payload_hash || guardian_pubkey)
    pub signature: [u8; 32],
}

/// The payload of a token transfer VAA.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaaPayload {
    /// Payload type (1 = transfer, 3 = transfer with payload)
    pub payload_type: u8,
    /// Amount to transfer (in token's smallest unit)
    pub amount: u64,
    /// Token address on the source chain
    pub token_address: [u8; 32],
    /// Token's chain ID
    pub token_chain: u16,
    /// Recipient address on the target chain
    pub recipient: [u8; 32],
    /// Target chain ID
    pub recipient_chain: u16,
    /// Optional sender address (for transfer-with-payload)
    pub sender: [u8; 32],
}

impl Vaa {
    /// Compute the hash of this VAA's body (everything except signatures).
    pub fn body_hash(&self) -> [u8; 32] {
        let body = serde_json::to_vec(&VaaBody {
            timestamp: self.timestamp,
            nonce: self.nonce,
            emitter_chain: self.emitter_chain,
            emitter_address: self.emitter_address,
            sequence: self.sequence,
            consistency_level: self.consistency_level,
            payload: self.payload.clone(),
        })
        .expect("Wormhole: VAA body serialization failed");
        let digest = Sha256::digest(&body);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&digest);
        hash
    }
}

/// VAA body (for hashing — excludes version, guardian set index, and signatures).
#[derive(Serialize, Deserialize)]
struct VaaBody {
    timestamp: u32,
    nonce: u32,
    emitter_chain: u16,
    emitter_address: [u8; 32],
    sequence: u64,
    consistency_level: u8,
    payload: VaaPayload,
}

/// Full on-chain state for the Wormhole bridge contract on Dina.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WormholeState {
    /// Contract owner
    pub owner: [u8; 32],
    /// This chain's Wormhole chain ID (99 for Dina)
    pub wormhole_chain_id: u16,
    /// Current guardian set
    pub guardian_set: GuardianSet,
    /// Required finality (number of confirmations)
    pub finality: u32,
    /// Consumed VAAs (prevents replay)
    pub consumed_vaas: BTreeMap<[u8; 32], bool>,
    /// Next sequence number for outgoing messages
    pub next_sequence: u64,
    /// The bridged USDC token address on Dina
    pub usdc_token: [u8; 32],
    /// Emitted transfer log (for guardian observation)
    pub transfer_log: Vec<VaaPayload>,
    /// Whether the bridge is paused
    pub paused: bool,
}

impl WormholeState {
    /// Initialize a new Wormhole bridge contract.
    pub fn new(
        owner: [u8; 32],
        guardians: Vec<Guardian>,
        usdc_token: [u8; 32],
    ) -> Self {
        let quorum = ((guardians.len() as u32) * 2 / 3) + 1;
        Self {
            owner,
            wormhole_chain_id: CHAIN_DINA,
            guardian_set: GuardianSet {
                index: 0,
                guardians,
                quorum,
            },
            finality: 1,
            consumed_vaas: BTreeMap::new(),
            next_sequence: 1,
            usdc_token,
            transfer_log: Vec::new(),
            paused: false,
        }
    }

    // -- Admin ---------------------------------------------------------------

    /// Update the guardian set. Only callable by owner.
    pub fn update_guardian_set(
        &mut self,
        caller: [u8; 32],
        new_guardians: Vec<Guardian>,
    ) {
        assert!(caller == self.owner, "Wormhole: only owner");
        let quorum = ((new_guardians.len() as u32) * 2 / 3) + 1;
        self.guardian_set = GuardianSet {
            index: self.guardian_set.index + 1,
            guardians: new_guardians,
            quorum,
        };
    }

    /// Set the finality requirement. Only callable by owner.
    pub fn set_finality(&mut self, caller: [u8; 32], finality: u32) {
        assert!(caller == self.owner, "Wormhole: only owner");
        self.finality = finality;
    }

    /// Pause the bridge. Only callable by owner.
    pub fn pause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "Wormhole: only owner");
        self.paused = true;
    }

    /// Unpause. Only callable by owner.
    pub fn unpause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "Wormhole: only owner");
        self.paused = false;
    }

    // -- Core Wormhole -------------------------------------------------------

    /// Send tokens from Dina to another chain via Wormhole.
    ///
    /// Burns the tokens on Dina and creates a transfer payload that
    /// Wormhole guardians will observe and attest to.
    ///
    /// Returns the sequence number for tracking.
    pub fn send_tokens(
        &mut self,
        caller: [u8; 32],
        amount: u64,
        target_chain: u16,
        target_address: [u8; 32],
    ) -> u64 {
        assert!(!self.paused, "Wormhole: paused");
        assert!(amount > 0, "Wormhole: amount must be positive");
        assert!(
            target_chain != self.wormhole_chain_id,
            "Wormhole: cannot send to self"
        );

        let payload = VaaPayload {
            payload_type: 1, // transfer
            amount,
            token_address: self.usdc_token,
            token_chain: CHAIN_DINA,
            recipient: target_address,
            recipient_chain: target_chain,
            sender: caller,
        };

        let sequence = self.next_sequence;
        self.next_sequence += 1;
        self.transfer_log.push(payload);

        // In production, this would burn tokens from the caller's balance
        // via the USDC.e token contract

        sequence
    }

    /// Complete a token transfer from another chain to Dina.
    ///
    /// Verifies the VAA's guardian signatures and mints tokens to the
    /// recipient on Dina.
    ///
    /// Returns the amount minted.
    pub fn complete_transfer(&mut self, vaa: Vaa) -> u64 {
        assert!(!self.paused, "Wormhole: paused");
        assert!(vaa.version == 1, "Wormhole: unsupported VAA version");
        assert!(
            vaa.guardian_set_index == self.guardian_set.index,
            "Wormhole: wrong guardian set"
        );
        assert!(
            vaa.payload.recipient_chain == self.wormhole_chain_id,
            "Wormhole: VAA not for this chain"
        );

        // Check VAA hasn't been consumed (replay protection)
        let vaa_hash = vaa.body_hash();
        assert!(
            !self.consumed_vaas.contains_key(&vaa_hash),
            "Wormhole: VAA already consumed"
        );

        // Verify guardian signatures
        self.verify_signatures(&vaa, &vaa_hash);

        // Mark as consumed
        self.consumed_vaas.insert(vaa_hash, true);

        // In production, this would mint tokens via the USDC.e token contract
        // to vaa.payload.recipient

        vaa.payload.amount
    }

    /// Verify that a VAA has sufficient valid guardian signatures.
    fn verify_signatures(&self, vaa: &Vaa, body_hash: &[u8; 32]) {
        assert!(
            vaa.signatures.len() as u32 >= self.guardian_set.quorum,
            "Wormhole: insufficient signatures ({} < {})",
            vaa.signatures.len(),
            self.guardian_set.quorum
        );

        let mut seen_indices = BTreeMap::new();
        for sig in &vaa.signatures {
            // Each guardian can only sign once
            assert!(
                !seen_indices.contains_key(&sig.guardian_index),
                "Wormhole: duplicate guardian signature"
            );
            seen_indices.insert(sig.guardian_index, true);

            // Find the guardian's pubkey
            let guardian = self
                .guardian_set
                .guardians
                .iter()
                .find(|g| g.index == sig.guardian_index)
                .expect("Wormhole: unknown guardian index");

            // Verify signature: SHA-256(body_hash || guardian_pubkey) == signature
            let mut input = Vec::new();
            input.extend_from_slice(body_hash);
            input.extend_from_slice(&guardian.pubkey);
            let expected = Sha256::digest(&input);
            let mut expected_bytes = [0u8; 32];
            expected_bytes.copy_from_slice(&expected);
            assert!(
                sig.signature == expected_bytes,
                "Wormhole: invalid guardian signature"
            );
        }
    }

    /// Check if a VAA hash has been consumed.
    pub fn is_vaa_consumed(&self, vaa_hash: &[u8; 32]) -> bool {
        self.consumed_vaas
            .get(vaa_hash)
            .copied()
            .unwrap_or(false)
    }

    /// Get the number of transfers sent from this chain.
    pub fn transfer_count(&self) -> usize {
        self.transfer_log.len()
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    guardians: Vec<Guardian>,
    usdc_token: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SendTokensArgs {
    amount: u64,
    target_chain: u16,
    target_address: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct CompleteTransferArgs {
    vaa: Vaa,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateGuardianSetArgs {
    new_guardians: Vec<Guardian>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetFinalityArgs {
    finality: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsVaaConsumedArgs {
    vaa_hash: [u8; 32],
}

// ---------------------------------------------------------------------------
// Contract dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<WormholeState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Wormhole: already initialised");
            let a: InitArgs =
                serde_json::from_slice(args).expect("Wormhole: bad init args");
            *state = Some(WormholeState::new(caller, a.guardians, a.usdc_token));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Admin -----------------------------------------------------------
        "update_guardian_set" => {
            let s = state.as_mut().expect("Wormhole: not initialised");
            let a: UpdateGuardianSetArgs =
                serde_json::from_slice(args).expect("Wormhole: bad args");
            s.update_guardian_set(caller, a.new_guardians);
            serde_json::to_vec("ok").unwrap()
        }
        "set_finality" => {
            let s = state.as_mut().expect("Wormhole: not initialised");
            let a: SetFinalityArgs =
                serde_json::from_slice(args).expect("Wormhole: bad args");
            s.set_finality(caller, a.finality);
            serde_json::to_vec("ok").unwrap()
        }
        "pause" => {
            let s = state.as_mut().expect("Wormhole: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("Wormhole: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Core Wormhole ---------------------------------------------------
        "send_tokens" => {
            let s = state.as_mut().expect("Wormhole: not initialised");
            let a: SendTokensArgs =
                serde_json::from_slice(args).expect("Wormhole: bad send_tokens args");
            let seq = s.send_tokens(caller, a.amount, a.target_chain, a.target_address);
            serde_json::to_vec(&seq).unwrap()
        }
        "complete_transfer" => {
            let s = state.as_mut().expect("Wormhole: not initialised");
            let a: CompleteTransferArgs =
                serde_json::from_slice(args).expect("Wormhole: bad complete_transfer args");
            let amount = s.complete_transfer(a.vaa);
            serde_json::to_vec(&amount).unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "is_vaa_consumed" => {
            let s = state.as_ref().expect("Wormhole: not initialised");
            let a: IsVaaConsumedArgs =
                serde_json::from_slice(args).expect("Wormhole: bad args");
            serde_json::to_vec(&s.is_vaa_consumed(&a.vaa_hash)).unwrap()
        }
        "wormhole_chain_id" => {
            let s = state.as_ref().expect("Wormhole: not initialised");
            serde_json::to_vec(&s.wormhole_chain_id).unwrap()
        }
        "transfer_count" => {
            let s = state.as_ref().expect("Wormhole: not initialised");
            serde_json::to_vec(&s.transfer_count()).unwrap()
        }
        "guardian_set_index" => {
            let s = state.as_ref().expect("Wormhole: not initialised");
            serde_json::to_vec(&s.guardian_set.index).unwrap()
        }

        _ => panic!("Wormhole: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> [u8; 32] {
        [1u8; 32]
    }
    fn usdc() -> [u8; 32] {
        [20u8; 32]
    }
    fn alice() -> [u8; 32] {
        [3u8; 32]
    }

    fn make_guardians(n: usize) -> Vec<Guardian> {
        (0..n)
            .map(|i| {
                let mut pubkey = [0u8; 32];
                pubkey[0] = 100 + i as u8;
                Guardian {
                    index: i as u8,
                    pubkey,
                }
            })
            .collect()
    }

    fn setup() -> WormholeState {
        WormholeState::new(owner(), make_guardians(3), usdc())
    }

    /// Create valid signatures for a VAA body hash using the given guardians.
    fn sign_vaa(body_hash: &[u8; 32], guardians: &[Guardian], count: usize) -> Vec<VaaSignature> {
        guardians
            .iter()
            .take(count)
            .map(|g| {
                let mut input = Vec::new();
                input.extend_from_slice(body_hash);
                input.extend_from_slice(&g.pubkey);
                let digest = Sha256::digest(&input);
                let mut sig = [0u8; 32];
                sig.copy_from_slice(&digest);
                VaaSignature {
                    guardian_index: g.index,
                    signature: sig,
                }
            })
            .collect()
    }

    #[test]
    fn test_init() {
        let s = setup();
        assert_eq!(s.wormhole_chain_id, CHAIN_DINA);
        assert_eq!(s.guardian_set.guardians.len(), 3);
        assert_eq!(s.guardian_set.quorum, 3); // 3*2/3+1 = 3
        assert_eq!(s.next_sequence, 1);
    }

    #[test]
    fn test_send_tokens() {
        let mut s = setup();
        let seq = s.send_tokens(alice(), 1_000_000, CHAIN_BASE, alice());
        assert_eq!(seq, 1);
        assert_eq!(s.transfer_count(), 1);
        assert_eq!(s.next_sequence, 2);
    }

    #[test]
    #[should_panic(expected = "cannot send to self")]
    fn test_send_to_self_fails() {
        let mut s = setup();
        s.send_tokens(alice(), 1_000_000, CHAIN_DINA, alice());
    }

    #[test]
    fn test_complete_transfer() {
        let mut s = setup();
        let payload = VaaPayload {
            payload_type: 1,
            amount: 500_000,
            token_address: usdc(),
            token_chain: CHAIN_BASE,
            recipient: alice(),
            recipient_chain: CHAIN_DINA,
            sender: alice(),
        };

        let mut vaa = Vaa {
            version: 1,
            guardian_set_index: 0,
            timestamp: 1000,
            nonce: 1,
            emitter_chain: CHAIN_BASE,
            emitter_address: [50u8; 32],
            sequence: 1,
            consistency_level: 1,
            payload,
            signatures: vec![],
        };

        let body_hash = vaa.body_hash();
        vaa.signatures = sign_vaa(&body_hash, &s.guardian_set.guardians, 3);

        let amount = s.complete_transfer(vaa);
        assert_eq!(amount, 500_000);
    }

    #[test]
    #[should_panic(expected = "VAA already consumed")]
    fn test_replay_prevention() {
        let mut s = setup();
        let payload = VaaPayload {
            payload_type: 1,
            amount: 500_000,
            token_address: usdc(),
            token_chain: CHAIN_BASE,
            recipient: alice(),
            recipient_chain: CHAIN_DINA,
            sender: alice(),
        };

        let mut vaa = Vaa {
            version: 1,
            guardian_set_index: 0,
            timestamp: 1000,
            nonce: 1,
            emitter_chain: CHAIN_BASE,
            emitter_address: [50u8; 32],
            sequence: 1,
            consistency_level: 1,
            payload,
            signatures: vec![],
        };

        let body_hash = vaa.body_hash();
        vaa.signatures = sign_vaa(&body_hash, &s.guardian_set.guardians, 3);

        s.complete_transfer(vaa.clone());
        s.complete_transfer(vaa);
    }

    #[test]
    #[should_panic(expected = "insufficient signatures")]
    fn test_insufficient_signatures_fails() {
        let mut s = setup();
        let payload = VaaPayload {
            payload_type: 1,
            amount: 500_000,
            token_address: usdc(),
            token_chain: CHAIN_BASE,
            recipient: alice(),
            recipient_chain: CHAIN_DINA,
            sender: alice(),
        };

        let mut vaa = Vaa {
            version: 1,
            guardian_set_index: 0,
            timestamp: 1000,
            nonce: 1,
            emitter_chain: CHAIN_BASE,
            emitter_address: [50u8; 32],
            sequence: 1,
            consistency_level: 1,
            payload,
            signatures: vec![],
        };

        let body_hash = vaa.body_hash();
        // Only 1 signature, need 3
        vaa.signatures = sign_vaa(&body_hash, &s.guardian_set.guardians, 1);

        s.complete_transfer(vaa);
    }

    #[test]
    fn test_chain_id_constants() {
        assert_eq!(CHAIN_SOLANA, 1);
        assert_eq!(CHAIN_ETHEREUM, 2);
        assert_eq!(CHAIN_ARBITRUM, 23);
        assert_eq!(CHAIN_BASE, 30);
        assert_eq!(CHAIN_DINA, 99);
    }
}
