use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-103  Service Agreement  -- escrow-based agreements between agents
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum AgreementStatus {
    Proposed,
    Active,
    Delivered,
    Completed,
    Disputed,
    Cancelled,
    Expired,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgreementTerms {
    pub client: [u8; 32],
    pub provider: [u8; 32],
    pub description: String,
    pub amount: u64,
    pub deliverables: Vec<String>,
    pub deadline: u64,
    pub auto_confirm_after: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Agreement {
    pub id: u64,
    pub terms: AgreementTerms,
    pub status: AgreementStatus,
    pub created_at: u64,
    pub delivered_at: Option<u64>,
    pub delivery_proof: Option<String>,
    pub completed_at: Option<u64>,
    pub dispute_reason: Option<String>,
    pub escrow_locked: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServiceAgreementState {
    pub agreements: BTreeMap<u64, Agreement>,
    pub next_id: u64,
    /// Tracks escrow balances per client address
    pub escrow_balances: BTreeMap<[u8; 32], u64>,
}

impl Default for ServiceAgreementState {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceAgreementState {
    pub fn new() -> Self {
        Self {
            agreements: BTreeMap::new(),
            next_id: 1,
            escrow_balances: BTreeMap::new(),
        }
    }

    /// Propose a new agreement. The client's escrow is locked.
    pub fn propose(
        &mut self,
        caller: [u8; 32],
        terms: AgreementTerms,
        timestamp: u64,
        client_balance: u64,
    ) -> u64 {
        assert!(
            caller == terms.client,
            "DRC103: only the client can propose"
        );
        assert!(terms.amount > 0, "DRC103: amount must be positive");
        assert!(
            !terms.deliverables.is_empty(),
            "DRC103: must specify at least one deliverable"
        );
        assert!(
            terms.deadline > timestamp,
            "DRC103: deadline must be in the future"
        );
        assert!(
            client_balance >= terms.amount,
            "DRC103: insufficient balance to lock escrow"
        );

        let id = self.next_id;
        self.next_id += 1;

        let escrow_amount = terms.amount;

        let agreement = Agreement {
            id,
            terms,
            status: AgreementStatus::Proposed,
            created_at: timestamp,
            delivered_at: None,
            delivery_proof: None,
            completed_at: None,
            dispute_reason: None,
            escrow_locked: escrow_amount,
        };

        self.agreements.insert(id, agreement);

        // Lock escrow
        let entry = self.escrow_balances.entry(caller).or_insert(0);
        *entry += escrow_amount;

        id
    }

    /// Provider accepts the proposed agreement.
    pub fn accept(&mut self, caller: [u8; 32], agreement_id: u64) {
        let agreement = self
            .agreements
            .get_mut(&agreement_id)
            .expect("DRC103: agreement not found");
        assert!(
            caller == agreement.terms.provider,
            "DRC103: only the provider can accept"
        );
        assert!(
            agreement.status == AgreementStatus::Proposed,
            "DRC103: agreement is not in Proposed state"
        );
        agreement.status = AgreementStatus::Active;
    }

    /// Provider marks the agreement as delivered with proof.
    pub fn deliver(&mut self, caller: [u8; 32], agreement_id: u64, proof: String, timestamp: u64) {
        let agreement = self
            .agreements
            .get_mut(&agreement_id)
            .expect("DRC103: agreement not found");
        assert!(
            caller == agreement.terms.provider,
            "DRC103: only the provider can deliver"
        );
        assert!(
            agreement.status == AgreementStatus::Active,
            "DRC103: agreement is not Active"
        );
        agreement.status = AgreementStatus::Delivered;
        agreement.delivered_at = Some(timestamp);
        agreement.delivery_proof = Some(proof);
    }

    /// Client confirms delivery and releases escrow to the provider.
    pub fn confirm(
        &mut self,
        caller: [u8; 32],
        agreement_id: u64,
        timestamp: u64,
    ) -> (u64, [u8; 32]) {
        let agreement = self
            .agreements
            .get_mut(&agreement_id)
            .expect("DRC103: agreement not found");
        assert!(
            caller == agreement.terms.client,
            "DRC103: only the client can confirm"
        );
        assert!(
            agreement.status == AgreementStatus::Delivered,
            "DRC103: agreement is not in Delivered state"
        );

        agreement.status = AgreementStatus::Completed;
        agreement.completed_at = Some(timestamp);

        let payout = agreement.escrow_locked;
        let provider = agreement.terms.provider;

        // Release escrow
        let client_escrow = self
            .escrow_balances
            .get_mut(&agreement.terms.client)
            .expect("DRC103: escrow balance missing");
        *client_escrow -= payout;

        (payout, provider)
    }

    /// Either party can dispute a Delivered agreement.
    pub fn dispute(&mut self, caller: [u8; 32], agreement_id: u64, reason: String) {
        let agreement = self
            .agreements
            .get_mut(&agreement_id)
            .expect("DRC103: agreement not found");
        assert!(
            caller == agreement.terms.client || caller == agreement.terms.provider,
            "DRC103: only client or provider can dispute"
        );
        assert!(
            agreement.status == AgreementStatus::Active
                || agreement.status == AgreementStatus::Delivered,
            "DRC103: can only dispute Active or Delivered agreements"
        );
        agreement.status = AgreementStatus::Disputed;
        agreement.dispute_reason = Some(reason);
    }

    /// Client can cancel a Proposed agreement (before acceptance). Escrow is returned.
    pub fn cancel(&mut self, caller: [u8; 32], agreement_id: u64) -> u64 {
        let agreement = self
            .agreements
            .get_mut(&agreement_id)
            .expect("DRC103: agreement not found");
        assert!(
            caller == agreement.terms.client,
            "DRC103: only the client can cancel"
        );
        assert!(
            agreement.status == AgreementStatus::Proposed,
            "DRC103: can only cancel Proposed agreements"
        );

        agreement.status = AgreementStatus::Cancelled;
        let refund = agreement.escrow_locked;

        // Release escrow back to client
        let client_escrow = self
            .escrow_balances
            .get_mut(&caller)
            .expect("DRC103: escrow balance missing");
        *client_escrow -= refund;

        refund
    }

    /// Auto-confirm after the configured period has elapsed since delivery.
    pub fn auto_confirm(
        &mut self,
        agreement_id: u64,
        current_timestamp: u64,
    ) -> Option<(u64, [u8; 32])> {
        let agreement = self
            .agreements
            .get(&agreement_id)
            .expect("DRC103: agreement not found");

        if agreement.status != AgreementStatus::Delivered {
            return None;
        }

        let delivered_at = agreement
            .delivered_at
            .expect("DRC103: delivered_at missing");
        let auto_confirm_after = agreement.terms.auto_confirm_after;

        if current_timestamp < delivered_at + auto_confirm_after {
            return None;
        }

        // Auto-confirm: release escrow
        let agreement = self.agreements.get_mut(&agreement_id).unwrap();
        agreement.status = AgreementStatus::Completed;
        agreement.completed_at = Some(current_timestamp);

        let payout = agreement.escrow_locked;
        let provider = agreement.terms.provider;
        let client = agreement.terms.client;

        let client_escrow = self
            .escrow_balances
            .get_mut(&client)
            .expect("DRC103: escrow balance missing");
        *client_escrow -= payout;

        Some((payout, provider))
    }

    pub fn get_agreement(&self, agreement_id: u64) -> Option<&Agreement> {
        self.agreements.get(&agreement_id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch arg types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ProposeArgs {
    terms: AgreementTerms,
    timestamp: u64,
    client_balance: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AcceptArgs {
    agreement_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeliverArgs {
    agreement_id: u64,
    proof: String,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfirmArgs {
    agreement_id: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DisputeArgs {
    agreement_id: u64,
    reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CancelArgs {
    agreement_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AutoConfirmArgs {
    agreement_id: u64,
    current_timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetAgreementArgs {
    agreement_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfirmResult {
    payout: u64,
    provider: [u8; 32],
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<ServiceAgreementState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC103: already initialised");
            *state = Some(ServiceAgreementState::new());
            serde_json::to_vec("ok").unwrap()
        }

        "propose" => {
            let s = state.as_mut().expect("DRC103: not initialised");
            let a: ProposeArgs = serde_json::from_slice(args).expect("DRC103: bad propose args");
            let id = s.propose(caller, a.terms, a.timestamp, a.client_balance);
            serde_json::to_vec(&id).unwrap()
        }

        "accept" => {
            let s = state.as_mut().expect("DRC103: not initialised");
            let a: AcceptArgs = serde_json::from_slice(args).expect("DRC103: bad accept args");
            s.accept(caller, a.agreement_id);
            serde_json::to_vec("ok").unwrap()
        }

        "deliver" => {
            let s = state.as_mut().expect("DRC103: not initialised");
            let a: DeliverArgs = serde_json::from_slice(args).expect("DRC103: bad deliver args");
            s.deliver(caller, a.agreement_id, a.proof, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }

        "confirm" => {
            let s = state.as_mut().expect("DRC103: not initialised");
            let a: ConfirmArgs = serde_json::from_slice(args).expect("DRC103: bad confirm args");
            let (payout, provider) = s.confirm(caller, a.agreement_id, a.timestamp);
            serde_json::to_vec(&ConfirmResult { payout, provider }).unwrap()
        }

        "dispute" => {
            let s = state.as_mut().expect("DRC103: not initialised");
            let a: DisputeArgs = serde_json::from_slice(args).expect("DRC103: bad dispute args");
            s.dispute(caller, a.agreement_id, a.reason);
            serde_json::to_vec("ok").unwrap()
        }

        "cancel" => {
            let s = state.as_mut().expect("DRC103: not initialised");
            let a: CancelArgs = serde_json::from_slice(args).expect("DRC103: bad cancel args");
            let refund = s.cancel(caller, a.agreement_id);
            serde_json::to_vec(&refund).unwrap()
        }

        "auto_confirm" => {
            let s = state.as_mut().expect("DRC103: not initialised");
            let a: AutoConfirmArgs =
                serde_json::from_slice(args).expect("DRC103: bad auto_confirm args");
            let result = s.auto_confirm(a.agreement_id, a.current_timestamp);
            match result {
                Some((payout, provider)) => {
                    serde_json::to_vec(&ConfirmResult { payout, provider }).unwrap()
                }
                None => serde_json::to_vec(&Option::<ConfirmResult>::None).unwrap(),
            }
        }

        "get_agreement" => {
            let s = state.as_ref().expect("DRC103: not initialised");
            let a: GetAgreementArgs =
                serde_json::from_slice(args).expect("DRC103: bad get_agreement args");
            serde_json::to_vec(&s.get_agreement(a.agreement_id)).unwrap()
        }

        _ => panic!("DRC103: unknown method '{method}'"),
    }
}
