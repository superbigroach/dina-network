use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

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
    /// Pending withdrawals (Dina -> Base)
    pub pending_withdrawals: Vec<PendingWithdrawal>,
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
            pending_withdrawals: Vec::new(),
            processed_deposits: BTreeMap::new(),
            total_locked: 0,
            total_minted: 0,
            paused: false,
            min_amount: 1_000, // 0.001 USDC minimum
            max_amount: 1_000_000_000_000, // 1M USDC maximum
            fee_bps: 10, // 0.1% fee
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
    pub fn set_limits(
        &mut self,
        caller: [u8; 32],
        min_amount: u64,
        max_amount: u64,
    ) {
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
    /// The relayer submits the Base transaction proof. The proof is a hash
    /// of (base_tx_hash, amount, recipient, relayer_address) that the
    /// relayer signs. In production this would verify a Merkle proof
    /// against the Base block header.
    ///
    /// Only callable by the authorized relayer.
    pub fn claim(
        &mut self,
        caller: [u8; 32],
        base_tx_hash: [u8; 32],
        amount: u64,
        recipient: [u8; 32],
        proof: [u8; 32],
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

        // Verify proof: SHA-256(base_tx_hash || amount_bytes || recipient || relayer)
        let mut proof_input = Vec::new();
        proof_input.extend_from_slice(&base_tx_hash);
        proof_input.extend_from_slice(&amount.to_le_bytes());
        proof_input.extend_from_slice(&recipient);
        proof_input.extend_from_slice(&self.relayer);
        let expected = Sha256::digest(&proof_input);
        let mut expected_bytes = [0u8; 32];
        expected_bytes.copy_from_slice(&expected);
        assert!(proof == expected_bytes, "BaseBridge: invalid proof");

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

        self.pending_withdrawals.push(PendingWithdrawal {
            id,
            sender: caller,
            base_recipient,
            amount: release_amount,
            timestamp,
            processed: false,
        });

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
        for w in &mut self.pending_withdrawals {
            if w.id == withdrawal_id {
                assert!(!w.processed, "BaseBridge: already processed");
                w.processed = true;
                // M-7: Use saturating_sub to prevent underflow on total_locked.
                self.total_locked = self.total_locked.saturating_sub(w.amount);
                return;
            }
        }
        panic!("BaseBridge: withdrawal not found");
    }

    // -- Queries -------------------------------------------------------------

    /// Get a pending withdrawal by ID.
    pub fn get_withdrawal(&self, id: u64) -> Option<&PendingWithdrawal> {
        self.pending_withdrawals.iter().find(|w| w.id == id)
    }

    /// Get all unprocessed withdrawals.
    pub fn pending_withdrawal_count(&self) -> usize {
        self.pending_withdrawals
            .iter()
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
    proof: [u8; 32],
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
            let a: InitArgs =
                serde_json::from_slice(args).expect("BaseBridge: bad init args");
            *state = Some(BaseBridgeState::new(caller, a.relayer, a.usdc_token));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Admin -----------------------------------------------------------
        "set_relayer" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            let a: SetRelayerArgs =
                serde_json::from_slice(args).expect("BaseBridge: bad args");
            s.set_relayer(caller, a.new_relayer);
            serde_json::to_vec("ok").unwrap()
        }
        "set_fee" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            let a: SetFeeArgs =
                serde_json::from_slice(args).expect("BaseBridge: bad args");
            s.set_fee(caller, a.fee_bps);
            serde_json::to_vec("ok").unwrap()
        }
        "set_limits" => {
            let s = state.as_mut().expect("BaseBridge: not initialised");
            let a: SetLimitsArgs =
                serde_json::from_slice(args).expect("BaseBridge: bad args");
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
            let a: ClaimArgs =
                serde_json::from_slice(args).expect("BaseBridge: bad claim args");
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
            let a: MarkProcessedArgs =
                serde_json::from_slice(args).expect("BaseBridge: bad args");
            s.mark_withdrawal_processed(caller, a.withdrawal_id);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "get_withdrawal" => {
            let s = state.as_ref().expect("BaseBridge: not initialised");
            let a: GetWithdrawalArgs =
                serde_json::from_slice(args).expect("BaseBridge: bad args");
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

    fn owner() -> [u8; 32] {
        [1u8; 32]
    }
    fn relayer() -> [u8; 32] {
        [2u8; 32]
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

    fn setup() -> BaseBridgeState {
        BaseBridgeState::new(owner(), relayer(), usdc())
    }

    /// Compute a valid proof for a claim.
    fn compute_proof(
        base_tx_hash: &[u8; 32],
        amount: u64,
        recipient: &[u8; 32],
        relayer_addr: &[u8; 32],
    ) -> [u8; 32] {
        let mut input = Vec::new();
        input.extend_from_slice(base_tx_hash);
        input.extend_from_slice(&amount.to_le_bytes());
        input.extend_from_slice(recipient);
        input.extend_from_slice(relayer_addr);
        let digest = Sha256::digest(&input);
        let mut result = [0u8; 32];
        result.copy_from_slice(&digest);
        result
    }

    #[test]
    fn test_init() {
        let s = setup();
        assert_eq!(s.relayer, relayer());
        assert_eq!(s.usdc_token, usdc());
        assert_eq!(s.total_locked, 0);
        assert_eq!(s.fee_bps, 10);
    }

    #[test]
    fn test_claim_from_base() {
        let mut s = setup();
        let tx_hash = [42u8; 32];
        let amount = 1_000_000u64; // 1 USDC
        let proof = compute_proof(&tx_hash, amount, &alice(), &relayer());

        let minted = s.claim(relayer(), tx_hash, amount, alice(), proof);
        // Fee: 1_000_000 * 10 / 10_000 = 1_000
        assert_eq!(minted, 999_000);
        assert!(s.is_deposit_processed(&tx_hash));
        assert_eq!(s.total_locked, 1_000_000);
        assert_eq!(s.collected_fees, 1_000);
    }

    #[test]
    #[should_panic(expected = "deposit already processed")]
    fn test_double_claim_fails() {
        let mut s = setup();
        let tx_hash = [42u8; 32];
        let amount = 1_000_000u64;
        let proof = compute_proof(&tx_hash, amount, &alice(), &relayer());

        s.claim(relayer(), tx_hash, amount, alice(), proof);
        let proof2 = compute_proof(&tx_hash, amount, &alice(), &relayer());
        s.claim(relayer(), tx_hash, amount, alice(), proof2);
    }

    #[test]
    #[should_panic(expected = "only relayer")]
    fn test_claim_non_relayer_fails() {
        let mut s = setup();
        let tx_hash = [42u8; 32];
        let proof = compute_proof(&tx_hash, 1_000_000, &alice(), &relayer());
        s.claim(alice(), tx_hash, 1_000_000, alice(), proof);
    }

    #[test]
    fn test_withdraw_to_base() {
        let mut s = setup();
        // First deposit to have some minted supply
        let tx_hash = [42u8; 32];
        let amount = 10_000_000u64;
        let proof = compute_proof(&tx_hash, amount, &alice(), &relayer());
        s.claim(relayer(), tx_hash, amount, alice(), proof);

        // Withdraw
        let id = s.withdraw(alice(), 5_000_000, base_alice(), 1000);
        assert_eq!(id, 1);
        assert_eq!(s.pending_withdrawal_count(), 1);

        // Relayer marks as processed
        s.mark_withdrawal_processed(relayer(), 1);
        assert_eq!(s.pending_withdrawal_count(), 0);
    }

    #[test]
    #[should_panic(expected = "paused")]
    fn test_paused_blocks_claim() {
        let mut s = setup();
        s.pause(owner());
        let tx_hash = [42u8; 32];
        let proof = compute_proof(&tx_hash, 1_000_000, &alice(), &relayer());
        s.claim(relayer(), tx_hash, 1_000_000, alice(), proof);
    }

    #[test]
    fn test_set_fee() {
        let mut s = setup();
        s.set_fee(owner(), 50); // 0.5%
        assert_eq!(s.fee_bps, 50);
    }

    #[test]
    #[should_panic(expected = "fee too high")]
    fn test_fee_too_high() {
        let mut s = setup();
        s.set_fee(owner(), 600);
    }

    #[test]
    fn test_withdraw_fees() {
        let mut s = setup();
        let tx_hash = [42u8; 32];
        let amount = 1_000_000u64;
        let proof = compute_proof(&tx_hash, amount, &alice(), &relayer());
        s.claim(relayer(), tx_hash, amount, alice(), proof);

        let fees = s.withdraw_fees(owner());
        assert_eq!(fees, 1_000);
        assert_eq!(s.collected_fees, 0);
    }
}
