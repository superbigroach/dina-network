use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-54  Agent-to-Agent Escrow with Dispute Resolution
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum EscrowStatus {
    Active,
    Completed,
    Disputed,
    Refunded,
    Arbitrated,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Milestone {
    pub description: String,
    pub amount: u64,
    pub completed: bool,
    pub verified: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ArbitratorInfo {
    pub address: Address,
    pub name: String,
    pub cases_handled: u64,
    pub reputation_score: u64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Escrow {
    pub id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub total_amount: u64,
    pub deadline: u64,
    pub milestones: Vec<Milestone>,
    pub dispute_deadline: u64,
    pub arbitrator: Option<Address>,
    pub status: EscrowStatus,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EscrowState {
    pub owner: Address,
    pub escrows: BTreeMap<u64, Escrow>,
    pub arbitrators: BTreeMap<Address, ArbitratorInfo>,
    pub next_escrow_id: u64,
    pub released_funds: BTreeMap<Address, u64>,
}

impl EscrowState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            escrows: BTreeMap::new(),
            arbitrators: BTreeMap::new(),
            next_escrow_id: 1,
            released_funds: BTreeMap::new(),
        }
    }

    pub fn register_arbitrator(
        &mut self,
        caller: Address,
        arbitrator_address: Address,
        name: String,
    ) {
        assert!(
            caller == self.owner,
            "DRC54: only owner can register arbitrators"
        );
        let info = ArbitratorInfo {
            address: arbitrator_address,
            name,
            cases_handled: 0,
            reputation_score: 100,
            active: true,
        };
        self.arbitrators.insert(arbitrator_address, info);
    }

    pub fn create_escrow(
        &mut self,
        caller: Address,
        seller: Address,
        milestones: Vec<Milestone>,
        deadline: u64,
        dispute_deadline: u64,
        arbitrator: Option<Address>,
        created_at: u64,
    ) -> u64 {
        let total_amount: u64 = milestones.iter().map(|m| m.amount).sum();
        assert!(total_amount > 0, "DRC54: escrow amount must be positive");
        assert!(
            !milestones.is_empty(),
            "DRC54: must have at least one milestone"
        );
        if let Some(arb) = &arbitrator {
            assert!(
                self.arbitrators.contains_key(arb),
                "DRC54: arbitrator not registered"
            );
        }
        let id = self.next_escrow_id;
        self.next_escrow_id += 1;
        let escrow = Escrow {
            id,
            buyer: caller,
            seller,
            total_amount,
            deadline,
            milestones,
            dispute_deadline,
            arbitrator,
            status: EscrowStatus::Active,
            created_at,
        };
        self.escrows.insert(id, escrow);
        id
    }

    pub fn complete_milestone(&mut self, caller: Address, escrow_id: u64, milestone_index: usize) {
        let escrow = self
            .escrows
            .get_mut(&escrow_id)
            .expect("DRC54: escrow not found");
        assert!(
            escrow.status == EscrowStatus::Active,
            "DRC54: escrow not active"
        );
        assert!(
            caller == escrow.seller,
            "DRC54: only seller can complete milestones"
        );
        assert!(
            milestone_index < escrow.milestones.len(),
            "DRC54: invalid milestone index"
        );
        assert!(
            !escrow.milestones[milestone_index].completed,
            "DRC54: milestone already completed"
        );
        escrow.milestones[milestone_index].completed = true;
    }

    pub fn verify_milestone(&mut self, caller: Address, escrow_id: u64, milestone_index: usize) {
        let escrow = self
            .escrows
            .get_mut(&escrow_id)
            .expect("DRC54: escrow not found");
        assert!(
            escrow.status == EscrowStatus::Active,
            "DRC54: escrow not active"
        );
        assert!(
            caller == escrow.buyer,
            "DRC54: only buyer can verify milestones"
        );
        assert!(
            milestone_index < escrow.milestones.len(),
            "DRC54: invalid milestone index"
        );
        let milestone = &mut escrow.milestones[milestone_index];
        assert!(milestone.completed, "DRC54: milestone not completed yet");
        assert!(!milestone.verified, "DRC54: milestone already verified");
        milestone.verified = true;

        // Release funds for this milestone
        let amount = milestone.amount;
        let seller = escrow.seller;
        let balance = self.released_funds.entry(seller).or_insert(0);
        *balance += amount;
    }

    pub fn dispute(&mut self, caller: Address, escrow_id: u64) {
        let escrow = self
            .escrows
            .get_mut(&escrow_id)
            .expect("DRC54: escrow not found");
        assert!(
            escrow.status == EscrowStatus::Active,
            "DRC54: escrow not active"
        );
        assert!(
            caller == escrow.buyer || caller == escrow.seller,
            "DRC54: only buyer or seller can dispute"
        );
        assert!(escrow.arbitrator.is_some(), "DRC54: no arbitrator assigned");
        escrow.status = EscrowStatus::Disputed;
    }

    pub fn arbitrate(&mut self, caller: Address, escrow_id: u64, release_to_seller: bool) {
        let escrow = self
            .escrows
            .get_mut(&escrow_id)
            .expect("DRC54: escrow not found");
        assert!(
            escrow.status == EscrowStatus::Disputed,
            "DRC54: escrow not disputed"
        );
        let arb = escrow.arbitrator.expect("DRC54: no arbitrator");
        assert!(
            caller == arb,
            "DRC54: only assigned arbitrator can arbitrate"
        );

        // Calculate unreleased funds
        let released: u64 = escrow
            .milestones
            .iter()
            .filter(|m| m.verified)
            .map(|m| m.amount)
            .sum();
        let remaining = escrow.total_amount - released;

        if release_to_seller {
            let balance = self.released_funds.entry(escrow.seller).or_insert(0);
            *balance += remaining;
        } else {
            let balance = self.released_funds.entry(escrow.buyer).or_insert(0);
            *balance += remaining;
        }

        escrow.status = EscrowStatus::Arbitrated;

        // Update arbitrator stats
        if let Some(arb_info) = self.arbitrators.get_mut(&arb) {
            arb_info.cases_handled += 1;
        }
    }

    pub fn release_all(&mut self, caller: Address, escrow_id: u64) {
        let escrow = self
            .escrows
            .get_mut(&escrow_id)
            .expect("DRC54: escrow not found");
        assert!(
            escrow.status == EscrowStatus::Active,
            "DRC54: escrow not active"
        );
        assert!(caller == escrow.buyer, "DRC54: only buyer can release all");

        let unreleased: u64 = escrow
            .milestones
            .iter()
            .filter(|m| !m.verified)
            .map(|m| m.amount)
            .sum();
        let balance = self.released_funds.entry(escrow.seller).or_insert(0);
        *balance += unreleased;

        for m in escrow.milestones.iter_mut() {
            m.completed = true;
            m.verified = true;
        }
        escrow.status = EscrowStatus::Completed;
    }

    pub fn refund(&mut self, caller: Address, escrow_id: u64, current_time: u64) {
        let escrow = self
            .escrows
            .get_mut(&escrow_id)
            .expect("DRC54: escrow not found");
        assert!(
            escrow.status == EscrowStatus::Active,
            "DRC54: escrow not active"
        );
        assert!(
            caller == escrow.buyer,
            "DRC54: only buyer can request refund"
        );
        assert!(current_time > escrow.deadline, "DRC54: deadline not passed");

        let unreleased: u64 = escrow
            .milestones
            .iter()
            .filter(|m| !m.verified)
            .map(|m| m.amount)
            .sum();
        let balance = self.released_funds.entry(escrow.buyer).or_insert(0);
        *balance += unreleased;
        escrow.status = EscrowStatus::Refunded;
    }

    pub fn get_escrow(&self, escrow_id: u64) -> Option<&Escrow> {
        self.escrows.get(&escrow_id)
    }

    pub fn get_released(&self, addr: &Address) -> u64 {
        self.released_funds.get(addr).copied().unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterArbitratorArgs {
    arbitrator_address: Address,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateEscrowArgs {
    seller: Address,
    milestones: Vec<Milestone>,
    deadline: u64,
    dispute_deadline: u64,
    arbitrator: Option<Address>,
    created_at: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct MilestoneArgs {
    escrow_id: u64,
    milestone_index: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct DisputeArgs {
    escrow_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ArbitrateArgs {
    escrow_id: u64,
    release_to_seller: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReleaseAllArgs {
    escrow_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RefundArgs {
    escrow_id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetEscrowArgs {
    escrow_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetReleasedArgs {
    addr: Address,
}

pub fn dispatch(
    state: &mut Option<EscrowState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC54: already initialised");
            *state = Some(EscrowState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "register_arbitrator" => {
            let s = state.as_mut().expect("DRC54: not initialised");
            let a: RegisterArbitratorArgs = serde_json::from_slice(args).expect("DRC54: bad args");
            s.register_arbitrator(caller, a.arbitrator_address, a.name);
            serde_json::to_vec("ok").unwrap()
        }
        "create_escrow" => {
            let s = state.as_mut().expect("DRC54: not initialised");
            let a: CreateEscrowArgs = serde_json::from_slice(args).expect("DRC54: bad args");
            let id = s.create_escrow(
                caller,
                a.seller,
                a.milestones,
                a.deadline,
                a.dispute_deadline,
                a.arbitrator,
                a.created_at,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "complete_milestone" => {
            let s = state.as_mut().expect("DRC54: not initialised");
            let a: MilestoneArgs = serde_json::from_slice(args).expect("DRC54: bad args");
            s.complete_milestone(caller, a.escrow_id, a.milestone_index);
            serde_json::to_vec("ok").unwrap()
        }
        "verify_milestone" => {
            let s = state.as_mut().expect("DRC54: not initialised");
            let a: MilestoneArgs = serde_json::from_slice(args).expect("DRC54: bad args");
            s.verify_milestone(caller, a.escrow_id, a.milestone_index);
            serde_json::to_vec("ok").unwrap()
        }
        "dispute" => {
            let s = state.as_mut().expect("DRC54: not initialised");
            let a: DisputeArgs = serde_json::from_slice(args).expect("DRC54: bad args");
            s.dispute(caller, a.escrow_id);
            serde_json::to_vec("ok").unwrap()
        }
        "arbitrate" => {
            let s = state.as_mut().expect("DRC54: not initialised");
            let a: ArbitrateArgs = serde_json::from_slice(args).expect("DRC54: bad args");
            s.arbitrate(caller, a.escrow_id, a.release_to_seller);
            serde_json::to_vec("ok").unwrap()
        }
        "release_all" => {
            let s = state.as_mut().expect("DRC54: not initialised");
            let a: ReleaseAllArgs = serde_json::from_slice(args).expect("DRC54: bad args");
            s.release_all(caller, a.escrow_id);
            serde_json::to_vec("ok").unwrap()
        }
        "refund" => {
            let s = state.as_mut().expect("DRC54: not initialised");
            let a: RefundArgs = serde_json::from_slice(args).expect("DRC54: bad args");
            s.refund(caller, a.escrow_id, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "get_escrow" => {
            let s = state.as_ref().expect("DRC54: not initialised");
            let a: GetEscrowArgs = serde_json::from_slice(args).expect("DRC54: bad args");
            serde_json::to_vec(&s.get_escrow(a.escrow_id)).unwrap()
        }
        "get_released" => {
            let s = state.as_ref().expect("DRC54: not initialised");
            let a: GetReleasedArgs = serde_json::from_slice(args).expect("DRC54: bad args");
            serde_json::to_vec(&s.get_released(&a.addr)).unwrap()
        }
        _ => panic!("DRC54: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const BUYER: Address = [1u8; 32];
    const SELLER: Address = [2u8; 32];
    const ARBITRATOR: Address = [3u8; 32];

    fn init_state() -> Option<EscrowState> {
        let mut state = None;
        dispatch(&mut state, "init", b"{}", BUYER);
        state
    }

    fn make_milestones() -> Vec<Milestone> {
        vec![
            Milestone {
                description: "Design".into(),
                amount: 500,
                completed: false,
                verified: false,
            },
            Milestone {
                description: "Build".into(),
                amount: 1500,
                completed: false,
                verified: false,
            },
        ]
    }

    #[test]
    fn test_create_escrow_and_milestone_flow() {
        let mut state = init_state();
        let s = state.as_mut().unwrap();
        let id = s.create_escrow(BUYER, SELLER, make_milestones(), 1000, 1200, None, 100);
        assert_eq!(id, 1);

        s.complete_milestone(SELLER, id, 0);
        let escrow = s.get_escrow(id).unwrap();
        assert!(escrow.milestones[0].completed);
        assert!(!escrow.milestones[0].verified);

        s.verify_milestone(BUYER, id, 0);
        assert_eq!(s.get_released(&SELLER), 500);
    }

    #[test]
    fn test_release_all() {
        let mut state = init_state();
        let s = state.as_mut().unwrap();
        let id = s.create_escrow(BUYER, SELLER, make_milestones(), 1000, 1200, None, 100);
        s.release_all(BUYER, id);
        assert_eq!(s.get_released(&SELLER), 2000);
        let escrow = s.get_escrow(id).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Completed);
    }

    #[test]
    fn test_refund_after_deadline() {
        let mut state = init_state();
        let s = state.as_mut().unwrap();
        let id = s.create_escrow(BUYER, SELLER, make_milestones(), 1000, 1200, None, 100);
        s.complete_milestone(SELLER, id, 0);
        s.verify_milestone(BUYER, id, 0);
        // Milestone 0 = 500 released, milestone 1 = 1500 unreleased
        s.refund(BUYER, id, 1001);
        assert_eq!(s.get_released(&BUYER), 1500);
        assert_eq!(s.get_released(&SELLER), 500);
    }

    #[test]
    fn test_dispute_and_arbitrate_to_seller() {
        let mut state = init_state();
        let s = state.as_mut().unwrap();
        s.register_arbitrator(BUYER, ARBITRATOR, "Judge Bot".into());
        let id = s.create_escrow(
            BUYER,
            SELLER,
            make_milestones(),
            1000,
            1200,
            Some(ARBITRATOR),
            100,
        );
        s.dispute(BUYER, id);
        let escrow = s.get_escrow(id).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Disputed);

        s.arbitrate(ARBITRATOR, id, true);
        assert_eq!(s.get_released(&SELLER), 2000);
    }

    #[test]
    fn test_dispute_and_arbitrate_to_buyer() {
        let mut state = init_state();
        let s = state.as_mut().unwrap();
        s.register_arbitrator(BUYER, ARBITRATOR, "Judge Bot".into());
        let id = s.create_escrow(
            BUYER,
            SELLER,
            make_milestones(),
            1000,
            1200,
            Some(ARBITRATOR),
            100,
        );
        s.dispute(SELLER, id);
        s.arbitrate(ARBITRATOR, id, false);
        assert_eq!(s.get_released(&BUYER), 2000);
        let arb = s.arbitrators.get(&ARBITRATOR).unwrap();
        assert_eq!(arb.cases_handled, 1);
    }

    #[test]
    #[should_panic(expected = "only seller can complete")]
    fn test_buyer_cannot_complete_milestone() {
        let mut state = init_state();
        let s = state.as_mut().unwrap();
        let id = s.create_escrow(BUYER, SELLER, make_milestones(), 1000, 1200, None, 100);
        s.complete_milestone(BUYER, id, 0);
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state = None;
        dispatch(&mut state, "init", b"{}", BUYER);
        let args = serde_json::to_vec(&RegisterArbitratorArgs {
            arbitrator_address: ARBITRATOR,
            name: "Arb".into(),
        })
        .unwrap();
        dispatch(&mut state, "register_arbitrator", &args, BUYER);
        assert!(state
            .as_ref()
            .unwrap()
            .arbitrators
            .contains_key(&ARBITRATOR));
    }
}
