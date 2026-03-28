use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Serde helper for `[u8; 64]` arrays (Ed25519 signatures).
mod serde_sig64 {
    use serde::{self, Deserialize, Deserializer, Serializer};
    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Vec<u8> = Vec::deserialize(deserializer)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("expected 64 bytes for signature"))
    }
}

// ---------------------------------------------------------------------------
// Base <-> Dina Bridge — Lock/Mint Bridge for USDC between Base and Dina
// ---------------------------------------------------------------------------
//
// This contract implements a lock/mint bridge pattern:
//
//   Base -> Dina (deposit):
//     1. User locks USDC in the bridge contract on Base
//     2. Relayer observes the lock event and submits proof to Dina
//     3. Bridge contract on Dina verifies proof and mints bridged USDC
//
//   Dina -> Base (withdrawal):
//     1. User calls withdraw() which burns bridged USDC on Dina
//     2. Relayer observes the burn event on Dina
//     3. Relayer releases locked USDC to the user on Base
//
// The bridge uses a trusted relayer model. In production, this would be
// upgraded to use a decentralized relayer set with threshold signatures
// or a light client verification.
// ---------------------------------------------------------------------------

/// A pending withdrawal waiting to be processed by the relayer.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingWithdrawal {
    /// Unique ID for this withdrawal
    pub id: u64,
    /// Who initiated the withdrawal on Dina
    pub sender: [u8; 32],
    /// Recipient address on Base (20-byte Ethereum address, zero-padded)
    pub base_recipient: [u8; 32],
    /// Amount of USDC.e burned
    pub amount: u64,
    /// Timestamp when the withdrawal was initiated
    pub timestamp: u64,
    /// Whether the relayer has processed this withdrawal
    pub processed: bool,
}

/// Full on-chain state for the Base <-> Dina bridge contract.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BaseBridgeState {
    /// Contract owner
    pub owner: [u8; 32],
    /// The relayer address authorized to submit proofs and process withdrawals
    pub relayer: [u8; 32],
    /// The bridged USDC token contract address on Dina
    pub usdc_token: [u8; 32],
    /// Next withdrawal ID
    pub next_withdrawal_id: u64,
    /// Pending withdrawals (Dina -> Base), keyed by withdrawal ID
    pub pending_withdrawals: BTreeMap<u64, PendingWithdrawal>,
    /// Processed deposit IDs from Base (prevents replay)
    pub processed_deposits: BTreeMap<[u8; 32], bool>,
    /// Total USDC locked on Base side (tracked for accounting)
    pub total_locked: u64,
    /// Total USDC minted on Dina side
    pub total_minted: u64,
    /// Whether the bridge is paused
    pub paused: bool,
    /// Minimum bridge amount (to prevent dust attacks)
    pub min_amount: u64,
    /// Maximum bridge amount per transaction
    pub max_amount: u64,
    /// Bridge fee in basis points (100 = 1%)
    pub fee_bps: u64,
    /// Accumulated fees
    pub collected_fees: u64,
}

impl BaseBridgeState {
    /// Initialize a new Base <-> Dina bridge.
    pub fn new(owner: [u8; 32], relayer: [u8; 32], usdc_token: [u8; 32]) -> Self {
        Self {
            owner,
            relayer,
            usdc_token,
            next_withdrawal_id: 1,
            pending_withdrawals: BTreeMap::new(),
            processed_deposits: BTreeMap::new(),
            total_locked: 0,
            total_minted: 0,
            paused: false,
            min_amount: 1_000,             // 0.001 USDC minimum
            max_amount: 1_000_000_000_000, // 1M USDC maximum
            fee_bps: 10,                   // 0.1% fee
            collected_fees: 0,
        }
    }

    // -- Admin functions -----------------------------------------------------

    /// Update the relayer address. Only callable by owner.
    pub fn set_relayer(&mut self, caller: [u8; 32], new_relayer: [u8; 32]) {
        assert!(caller == self.owner, "BaseBridge: only owner");
        self.relayer = new_relayer;
    }

    /// Update bridge fee. Only callable by owner.
    pub fn set_fee(&mut self, caller: [u8; 32], fee_bps: u64) {
        assert!(caller == self.owner, "BaseBridge: only owner");
        assert!(fee_bps <= 500, "BaseBridge: fee too high (max 5%)");
        self.fee_bps = fee_bps;
    }

    /// Set min/max bridge amounts. Only callable by owner.
    pub fn set_limits(&mut self, caller: [u8; 32], min_amount: u64, max_amount: u64) {
        assert!(caller == self.owner, "BaseBridge: only owner");
        assert!(min_amount < max_amount, "BaseBridge: invalid limits");
        self.min_amount = min_amount;
        self.max_amount = max_amount;
    }

    /// Pause the bridge. Only callable by owner.
    pub fn pause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "BaseBridge: only owner");
        self.paused = true;
    }

    /// Unpause the bridge. Only callable by owner.
    pub fn unpause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "BaseBridge: only owner");
        self.paused = false;
    }

    /// Withdraw collected fees. Only callable by owner.
    pub fn withdraw_fees(&mut self, caller: [u8; 32]) -> u64 {
        assert!(caller == self.owner, "BaseBridge: only owner");
        let fees = self.collected_fees;
        self.collected_fees = 0;
        fees
    }

    // -- Bridge: Base -> Dina (claim) ----------------------------------------

    /// Claim bridged USDC on Dina after USDC was locked on Base.
    ///
    /// The relayer submits the Base transaction details along with an
    /// Ed25519 signature proving they authorized this claim. The proof is
    /// the relayer's signature over `SHA-256(base_tx_hash || amount ||
    /// recipient)`.
    ///
    /// Only callable by the authorized relayer.
    pub fn claim(
        &mut self,
        caller: [u8; 32],
        base_tx_hash: [u8; 32],
        amount: u64,
        recipient: [u8; 32],
        proof: [u8; 64],
    ) -> u64 {
        assert!(!self.paused, "BaseBridge: paused");
        assert!(caller == self.relayer, "BaseBridge: only relayer");
        assert!(amount >= self.min_amount, "BaseBridge: below minimum");
        assert!(amount <= self.max_amount, "BaseBridge: above maximum");

        // Verify this deposit hasn't been processed
        assert!(
            !self.processed_deposits.contains_key(&base_tx_hash),
            "BaseBridge: deposit already processed"
        );

        // Build the message: SHA-256(base_tx_hash || amount_bytes || recipient)
        let mut message_input = Vec::new();
        message_input.extend_from_slice(&base_tx_hash);
        message_input.extend_from_slice(&amount.to_le_bytes());
        message_input.extend_from_slice(&recipient);
        let message_hash = Sha256::digest(&message_input);

        // Verify Ed25519 signature from the relayer over the message
        let verifying_key = VerifyingKey::from_bytes(&self.relayer)
            .expect("BaseBridge: invalid relayer public key");
        let signature = Signature::from_bytes(&proof);
        verifying_key
            .verify(&message_hash, &signature)
            .expect("BaseBridge: invalid proof");

        // Calculate fee
        let fee = (amount * self.fee_bps) / 10_000;
        let mint_amount = amount - fee;

        // Mark as processed
        self.processed_deposits.insert(base_tx_hash, true);
        self.total_locked += amount;
        self.total_minted += mint_amount;
        self.collected_fees += fee;

        // In production, this would call mint() on the USDC.e token contract
        // to mint `mint_amount` to `recipient`

        mint_amount
    }

    // -- Bridge: Dina -> Base (withdraw) -------------------------------------

    /// Initiate a withdrawal from Dina to Base.
    ///
    /// The caller burns their bridged USDC on Dina. The relayer then
    /// observes this event and releases the locked USDC on Base.
    ///
    /// Returns the withdrawal ID for tracking.
    pub fn withdraw(
        &mut self,
        caller: [u8; 32],
        amount: u64,
        base_recipient: [u8; 32],
        timestamp: u64,
    ) -> u64 {
        assert!(!self.paused, "BaseBridge: paused");
        assert!(amount >= self.min_amount, "BaseBridge: below minimum");
        assert!(amount <= self.max_amount, "BaseBridge: above maximum");

        // Calculate fee
        let fee = (amount * self.fee_bps) / 10_000;
        let release_amount = amount - fee;

        let id = self.next_withdrawal_id;
        self.next_withdrawal_id += 1;

        self.pending_withdrawals.insert(
            id,
            PendingWithdrawal {
                id,
                sender: caller,
                base_recipient,
                amount: release_amount,
                timestamp,
                processed: false,
            },
        );

        // M-7: Use saturating_sub to prevent underflow wrapping on total_minted.
        self.total_minted = self.total_minted.saturating_sub(amount); // burned on Dina
        self.collected_fees += fee;

        // In production, this would call burn() on the USDC.e token contract
        // to burn `amount` from `caller`

        id
    }

    /// Mark a withdrawal as processed by the relayer. Only callable by relayer.
    pub fn mark_withdrawal_processed(&mut self, caller: [u8; 32], withdrawal_id: u64) {
        assert!(caller == self.relayer, "BaseBridge: only relayer");
        let w = self
            .pending_withdrawals
            .get_mut(&withdrawal_id)
            .expect("BaseBridge: withdrawal not found");
        assert!(!w.processed, "BaseBridge: already processed");
        w.processed = true;
        // M-7: Use saturating_sub to prevent underflow on total_locked.
        self.total_locked = self.total_locked.saturating_sub(w.amount);
    }

    // -- Queries -------------------------------------------------------------

    /// Get a pending withdrawal by ID.
    pub fn get_withdrawal(&self, id: u64) -> Option<&PendingWithdrawal> {
        self.pending_withdrawals.get(&id)
    }

    /// Get all unprocessed withdrawals.
    pub fn pending_withdrawal_count(&self) -> usize {
        self.pending_withdrawals
            .values()
            .filter(|w| !w.processed)
            .count()
    }

    /// Check if a Base deposit has been processed.
    pub fn is_deposit_processed(&self, base_tx_hash: &[u8; 32]) -> bool {
        self.processed_deposits
            .get(base_tx_hash)
            .copied()
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    relayer: [u8; 32],
    usdc_token: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct ClaimArgs {
    base_tx_hash: [u8; 32],
    amount: u64,
    recipient: [u8; 32],
    #[serde(with = "serde_sig64")]
    proof: [u8; 64],
}

#[derive(Serialize, Deserialize, Debug)]
struct WithdrawArgs {
    amount: u64,
    base_recipient: [u8; 32],
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetRelayerArgs {
    new_relayer: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SetFeeArgs {
    fee_bps: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetLimitsArgs {
    min_amount: u64,
    max_amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct MarkProcessedArgs {
    withdrawal_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetWithdrawalArgs {
    id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsDepositProcessedArgs {
    base_tx_hash: [u8; 32],
}

// ---------------------------------------------------------------------------
// Contract dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<BaseBridgeState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "BaseBridge: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("BaseBridge: bad init args");
            *state = Some(BaseBridgeState::new(caller, a.relayer, a.usdc_token));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Admin -----------------------------------------------------------
        "set_relayer" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            let a: SetRelayerArgs = serde_json::from_slice(args).expect("BaseBridge: bad args");
            s.set_relayer(caller, a.new_relayer);
            serde_json::to_vec("ok").unwrap()
        }
        "set_fee" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            let a: SetFeeArgs = serde_json::from_slice(args).expect("BaseBridge: bad args");
            s.set_fee(caller, a.fee_bps);
            serde_json::to_vec("ok").unwrap()
        }
        "set_limits" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            let a: SetLimitsArgs = serde_json::from_slice(args).expect("BaseBridge: bad args");
            s.set_limits(caller, a.min_amount, a.max_amount);
            serde_json::to_vec("ok").unwrap()
        }
        "pause" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "withdraw_fees" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            let fees = s.withdraw_fees(caller);
            serde_json::to_vec(&fees).unwrap()
        }

        // -- Bridge ----------------------------------------------------------
        "claim" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            let a: ClaimArgs = serde_json::from_slice(args).expect("BaseBridge: bad claim args");
            let minted = s.claim(caller, a.base_tx_hash, a.amount, a.recipient, a.proof);
            serde_json::to_vec(&minted).unwrap()
        }
        "withdraw" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            let a: WithdrawArgs =
                serde_json::from_slice(args).expect("BaseBridge: bad withdraw args");
            let id = s.withdraw(caller, a.amount, a.base_recipient, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        "mark_withdrawal_processed" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            let a: MarkProcessedArgs = serde_json::from_slice(args).expect("BaseBridge: bad args");
            s.mark_withdrawal_processed(caller, a.withdrawal_id);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "get_withdrawal" => {
            let s = state.as_ref().expect("BaseBridge: not initialised");
            let a: GetWithdrawalArgs = serde_json::from_slice(args).expect("BaseBridge: bad args");
            serde_json::to_vec(&s.get_withdrawal(a.id)).unwrap()
        }
        "pending_withdrawal_count" => {
            let s = state.as_ref().expect("BaseBridge: not initialised");
            serde_json::to_vec(&s.pending_withdrawal_count()).unwrap()
        }
        "is_deposit_processed" => {
            let s = state.as_ref().expect("BaseBridge: not initialised");
            let a: IsDepositProcessedArgs =
                serde_json::from_slice(args).expect("BaseBridge: bad args");
            serde_json::to_vec(&s.is_deposit_processed(&a.base_tx_hash)).unwrap()
        }
        "total_locked" => {
            let s = state.as_ref().expect("BaseBridge: not initialised");
            serde_json::to_vec(&s.total_locked).unwrap()
        }
        "total_minted" => {
            let s = state.as_ref().expect("BaseBridge: not initialised");
            serde_json::to_vec(&s.total_minted).unwrap()
        }

        _ => panic!("BaseBridge: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    fn owner() -> [u8; 32] {
        [1u8; 32]
    }
    fn usdc() -> [u8; 32] {
        [20u8; 32]
    }
    fn alice() -> [u8; 32] {
        [3u8; 32]
    }
    fn base_alice() -> [u8; 32] {
        [4u8; 32]
    }

    /// Generate a relayer keypair; the public key serves as the relayer address.
    fn make_relayer_key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    fn setup() -> (BaseBridgeState, SigningKey) {
        let relayer_key = make_relayer_key();
        let relayer_pubkey = relayer_key.verifying_key().to_bytes();
        let state = BaseBridgeState::new(owner(), relayer_pubkey, usdc());
        (state, relayer_key)
    }

    /// Compute a valid Ed25519 proof for a claim.
    fn compute_proof(
        signing_key: &SigningKey,
        base_tx_hash: &[u8; 32],
        amount: u64,
        recipient: &[u8; 32],
    ) -> [u8; 64] {
        let mut input = Vec::new();
        input.extend_from_slice(base_tx_hash);
        input.extend_from_slice(&amount.to_le_bytes());
        input.extend_from_slice(recipient);
        let message_hash = Sha256::digest(&input);
        let sig = signing_key.sign(&message_hash);
        sig.to_bytes()
    }

    #[test]
    fn test_init() {
        let (s, relayer_key) = setup();
        assert_eq!(s.relayer, relayer_key.verifying_key().to_bytes());
        assert_eq!(s.usdc_token, usdc());
        assert_eq!(s.total_locked, 0);
        assert_eq!(s.fee_bps, 10);
    }

    #[test]
    fn test_claim_from_base() {
        let (mut s, relayer_key) = setup();
        let relayer_pubkey = relayer_key.verifying_key().to_bytes();
        let tx_hash = [42u8; 32];
        let amount = 1_000_000u64; // 1 USDC
        let proof = compute_proof(&relayer_key, &tx_hash, amount, &alice());

        let minted = s.claim(relayer_pubkey, tx_hash, amount, alice(), proof);
        // Fee: 1_000_000 * 10 / 10_000 = 1_000
        assert_eq!(minted, 999_000);
        assert!(s.is_deposit_processed(&tx_hash));
        assert_eq!(s.total_locked, 1_000_000);
        assert_eq!(s.collected_fees, 1_000);
    }

    #[test]
    #[should_panic(expected = "deposit already processed")]
    fn test_double_claim_fails() {
        let (mut s, relayer_key) = setup();
        let relayer_pubkey = relayer_key.verifying_key().to_bytes();
        let tx_hash = [42u8; 32];
        let amount = 1_000_000u64;
        let proof = compute_proof(&relayer_key, &tx_hash, amount, &alice());

        s.claim(relayer_pubkey, tx_hash, amount, alice(), proof);
        let proof2 = compute_proof(&relayer_key, &tx_hash, amount, &alice());
        s.claim(relayer_pubkey, tx_hash, amount, alice(), proof2);
    }

    #[test]
    #[should_panic(expected = "only relayer")]
    fn test_claim_non_relayer_fails() {
        let (mut s, relayer_key) = setup();
        let tx_hash = [42u8; 32];
        let proof = compute_proof(&relayer_key, &tx_hash, 1_000_000, &alice());
        s.claim(alice(), tx_hash, 1_000_000, alice(), proof);
    }

    #[test]
    #[should_panic(expected = "invalid proof")]
    fn test_forged_proof_fails() {
        let (mut s, _relayer_key) = setup();
        let relayer_pubkey = _relayer_key.verifying_key().to_bytes();
        let tx_hash = [42u8; 32];
        let amount = 1_000_000u64;
        // Sign with a different key (attacker's key)
        let attacker_key = make_relayer_key();
        let fake_proof = compute_proof(&attacker_key, &tx_hash, amount, &alice());
        s.claim(relayer_pubkey, tx_hash, amount, alice(), fake_proof);
    }

    #[test]
    fn test_withdraw_to_base() {
        let (mut s, relayer_key) = setup();
        let relayer_pubkey = relayer_key.verifying_key().to_bytes();
        // First deposit to have some minted supply
        let tx_hash = [42u8; 32];
        let amount = 10_000_000u64;
        let proof = compute_proof(&relayer_key, &tx_hash, amount, &alice());
        s.claim(relayer_pubkey, tx_hash, amount, alice(), proof);

        // Withdraw
        let id = s.withdraw(alice(), 5_000_000, base_alice(), 1000);
        assert_eq!(id, 1);
        assert_eq!(s.pending_withdrawal_count(), 1);

        // Relayer marks as processed
        s.mark_withdrawal_processed(relayer_pubkey, 1);
        assert_eq!(s.pending_withdrawal_count(), 0);
    }

    #[test]
    #[should_panic(expected = "paused")]
    fn test_paused_blocks_claim() {
        let (mut s, relayer_key) = setup();
        let relayer_pubkey = relayer_key.verifying_key().to_bytes();
        s.pause(owner());
        let tx_hash = [42u8; 32];
        let proof = compute_proof(&relayer_key, &tx_hash, 1_000_000, &alice());
        s.claim(relayer_pubkey, tx_hash, 1_000_000, alice(), proof);
    }

    #[test]
    fn test_set_fee() {
        let (mut s, _relayer_key) = setup();
        s.set_fee(owner(), 50); // 0.5%
        assert_eq!(s.fee_bps, 50);
    }

    #[test]
    #[should_panic(expected = "fee too high")]
    fn test_fee_too_high() {
        let (mut s, _relayer_key) = setup();
        s.set_fee(owner(), 600);
    }

    #[test]
    fn test_withdraw_fees() {
        let (mut s, relayer_key) = setup();
        let relayer_pubkey = relayer_key.verifying_key().to_bytes();
        let tx_hash = [42u8; 32];
        let amount = 1_000_000u64;
        let proof = compute_proof(&relayer_key, &tx_hash, amount, &alice());
        s.claim(relayer_pubkey, tx_hash, amount, alice(), proof);

        let fees = s.withdraw_fees(owner());
        assert_eq!(fees, 1_000);
        assert_eq!(s.collected_fees, 0);
    }
}
