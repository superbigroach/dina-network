use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-67  Multi-Agent Escrow
// ---------------------------------------------------------------------------
// Escrow that handles N agents collaborating on a task, with proportional
// payout based on contribution shares and milestone completion.

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum EscrowStatus {
    Active,
    Completed,
    Disputed,
    Refunded,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Milestone {
    pub description: String,
    pub weight_bps: u16, // basis points of total, sum should = 10000
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentShare {
    pub address: Address,
    pub contribution_share_bps: u16, // basis points
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MultiAgentEscrow {
    pub id: u64,
    pub client: Address,
    pub agents: Vec<AgentShare>,
    pub total_amount: u64,
    pub milestones: Vec<Milestone>,
    pub agent_completions: BTreeMap<String, Vec<bool>>, // hex(addr) -> per-milestone completion
    pub client_verifications: Vec<bool>,
    pub status: EscrowStatus,
    pub released_amount: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EscrowState {
    pub owner: Address,
    pub escrows: BTreeMap<u64, MultiAgentEscrow>,
    pub next_escrow_id: u64,
}

fn addr_key(a: &Address) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

impl EscrowState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            escrows: BTreeMap::new(),
            next_escrow_id: 1,
        }
    }

    pub fn create_escrow(
        &mut self,
        caller: Address,
        agents: Vec<AgentShare>,
        total_amount: u64,
        milestones: Vec<Milestone>,
    ) -> u64 {
        assert!(!agents.is_empty(), "DRC67: need at least 1 agent");
        assert!(!milestones.is_empty(), "DRC67: need at least 1 milestone");
        assert!(total_amount > 0, "DRC67: amount must be > 0");

        // Validate shares sum to 10000
        let share_sum: u16 = agents.iter().map(|a| a.contribution_share_bps).sum();
        assert_eq!(
            share_sum, 10000,
            "DRC67: agent shares must sum to 10000 bps"
        );

        // Validate milestone weights sum to 10000
        let weight_sum: u16 = milestones.iter().map(|m| m.weight_bps).sum();
        assert_eq!(
            weight_sum, 10000,
            "DRC67: milestone weights must sum to 10000 bps"
        );

        let num_milestones = milestones.len();
        let mut agent_completions = BTreeMap::new();
        for agent in &agents {
            agent_completions.insert(addr_key(&agent.address), vec![false; num_milestones]);
        }

        let id = self.next_escrow_id;
        self.next_escrow_id += 1;
        self.escrows.insert(
            id,
            MultiAgentEscrow {
                id,
                client: caller,
                agents,
                total_amount,
                milestones,
                agent_completions,
                client_verifications: vec![false; num_milestones],
                status: EscrowStatus::Active,
                released_amount: 0,
            },
        );
        id
    }

    pub fn agent_complete_milestone(
        &mut self,
        caller: Address,
        escrow_id: u64,
        milestone_idx: usize,
    ) {
        let esc = self
            .escrows
            .get_mut(&escrow_id)
            .expect("DRC67: escrow not found");
        assert!(
            esc.status == EscrowStatus::Active,
            "DRC67: escrow not active"
        );
        let key = addr_key(&caller);
        let completions = esc
            .agent_completions
            .get_mut(&key)
            .expect("DRC67: not an agent");
        assert!(
            milestone_idx < completions.len(),
            "DRC67: invalid milestone index"
        );
        assert!(
            !completions[milestone_idx],
            "DRC67: milestone already completed"
        );
        completions[milestone_idx] = true;
    }

    pub fn client_verify(&mut self, caller: Address, escrow_id: u64, milestone_idx: usize) {
        let esc = self
            .escrows
            .get_mut(&escrow_id)
            .expect("DRC67: escrow not found");
        assert!(caller == esc.client, "DRC67: only client can verify");
        assert!(
            esc.status == EscrowStatus::Active,
            "DRC67: escrow not active"
        );
        assert!(
            milestone_idx < esc.milestones.len(),
            "DRC67: invalid milestone index"
        );
        assert!(
            !esc.client_verifications[milestone_idx],
            "DRC67: already verified"
        );

        // All agents must have completed this milestone
        for agent in &esc.agents {
            let key = addr_key(&agent.address);
            let completions = esc.agent_completions.get(&key).unwrap();
            assert!(
                completions[milestone_idx],
                "DRC67: not all agents completed milestone"
            );
        }

        esc.client_verifications[milestone_idx] = true;
    }

    pub fn release_proportional(&mut self, caller: Address, escrow_id: u64) -> Vec<(Address, u64)> {
        let esc = self
            .escrows
            .get_mut(&escrow_id)
            .expect("DRC67: escrow not found");
        assert!(caller == esc.client, "DRC67: only client can release");
        assert!(
            esc.status == EscrowStatus::Active,
            "DRC67: escrow not active"
        );

        // Calculate amount to release based on verified milestones
        let mut verified_weight: u64 = 0;
        for (i, milestone) in esc.milestones.iter().enumerate() {
            if esc.client_verifications[i] {
                verified_weight += milestone.weight_bps as u64;
            }
        }

        let releasable_total = (esc.total_amount * verified_weight) / 10000;
        let new_release = releasable_total.saturating_sub(esc.released_amount);
        assert!(new_release > 0, "DRC67: nothing new to release");

        let mut payouts = Vec::new();
        for agent in &esc.agents {
            let agent_amount = (new_release * agent.contribution_share_bps as u64) / 10000;
            if agent_amount > 0 {
                payouts.push((agent.address, agent_amount));
            }
        }

        esc.released_amount += new_release;

        // Check if all milestones verified
        if esc.client_verifications.iter().all(|v| *v) {
            esc.status = EscrowStatus::Completed;
        }

        payouts
    }

    pub fn dispute(&mut self, caller: Address, escrow_id: u64) {
        let esc = self
            .escrows
            .get_mut(&escrow_id)
            .expect("DRC67: escrow not found");
        assert!(
            esc.status == EscrowStatus::Active,
            "DRC67: escrow not active"
        );
        // Client or any agent can dispute
        let is_agent = esc.agents.iter().any(|a| a.address == caller);
        assert!(
            caller == esc.client || is_agent,
            "DRC67: not a party to escrow"
        );
        esc.status = EscrowStatus::Disputed;
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateEscrowArgs {
    agents: Vec<AgentShare>,
    total_amount: u64,
    milestones: Vec<Milestone>,
}
#[derive(Serialize, Deserialize, Debug)]
struct MilestoneArgs {
    escrow_id: u64,
    milestone_idx: usize,
}
#[derive(Serialize, Deserialize, Debug)]
struct EscrowIdArgs {
    escrow_id: u64,
}

pub fn dispatch(
    state: &mut Option<EscrowState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC67: already initialised");
            *state = Some(EscrowState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_escrow" => {
            let s = state.as_mut().expect("DRC67: not initialised");
            let a: CreateEscrowArgs = serde_json::from_slice(args).expect("DRC67: bad args");
            let id = s.create_escrow(caller, a.agents, a.total_amount, a.milestones);
            serde_json::to_vec(&id).unwrap()
        }
        "agent_complete_milestone" => {
            let s = state.as_mut().expect("DRC67: not initialised");
            let a: MilestoneArgs = serde_json::from_slice(args).expect("DRC67: bad args");
            s.agent_complete_milestone(caller, a.escrow_id, a.milestone_idx);
            serde_json::to_vec("ok").unwrap()
        }
        "client_verify" => {
            let s = state.as_mut().expect("DRC67: not initialised");
            let a: MilestoneArgs = serde_json::from_slice(args).expect("DRC67: bad args");
            s.client_verify(caller, a.escrow_id, a.milestone_idx);
            serde_json::to_vec("ok").unwrap()
        }
        "release_proportional" => {
            let s = state.as_mut().expect("DRC67: not initialised");
            let a: EscrowIdArgs = serde_json::from_slice(args).expect("DRC67: bad args");
            let payouts = s.release_proportional(caller, a.escrow_id);
            serde_json::to_vec(&payouts).unwrap()
        }
        "dispute" => {
            let s = state.as_mut().expect("DRC67: not initialised");
            let a: EscrowIdArgs = serde_json::from_slice(args).expect("DRC67: bad args");
            s.dispute(caller, a.escrow_id);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC67: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const CLIENT: Address = [1u8; 32];
    const AGENT_A: Address = [2u8; 32];
    const AGENT_B: Address = [3u8; 32];

    fn two_agent_escrow() -> (EscrowState, u64) {
        let mut s = EscrowState::new(CLIENT);
        let agents = vec![
            AgentShare {
                address: AGENT_A,
                contribution_share_bps: 6000,
            },
            AgentShare {
                address: AGENT_B,
                contribution_share_bps: 4000,
            },
        ];
        let milestones = vec![
            Milestone {
                description: "Design".into(),
                weight_bps: 3000,
            },
            Milestone {
                description: "Implementation".into(),
                weight_bps: 5000,
            },
            Milestone {
                description: "Testing".into(),
                weight_bps: 2000,
            },
        ];
        let id = s.create_escrow(CLIENT, agents, 10_000, milestones);
        (s, id)
    }

    #[test]
    fn test_create_escrow() {
        let (s, id) = two_agent_escrow();
        let esc = s.escrows.get(&id).unwrap();
        assert_eq!(esc.agents.len(), 2);
        assert_eq!(esc.milestones.len(), 3);
        assert_eq!(esc.total_amount, 10_000);
        assert_eq!(esc.status, EscrowStatus::Active);
    }

    #[test]
    fn test_milestone_completion_and_verify() {
        let (mut s, id) = two_agent_escrow();
        // Both agents complete milestone 0
        s.agent_complete_milestone(AGENT_A, id, 0);
        s.agent_complete_milestone(AGENT_B, id, 0);
        // Client verifies
        s.client_verify(CLIENT, id, 0);
        let esc = s.escrows.get(&id).unwrap();
        assert!(esc.client_verifications[0]);
    }

    #[test]
    fn test_proportional_release() {
        let (mut s, id) = two_agent_escrow();
        // Complete and verify first milestone (30%)
        s.agent_complete_milestone(AGENT_A, id, 0);
        s.agent_complete_milestone(AGENT_B, id, 0);
        s.client_verify(CLIENT, id, 0);

        let payouts = s.release_proportional(CLIENT, id);
        // 10000 * 3000/10000 = 3000 total
        // Agent A: 3000 * 6000/10000 = 1800
        // Agent B: 3000 * 4000/10000 = 1200
        assert_eq!(payouts.len(), 2);
        assert_eq!(payouts[0], (AGENT_A, 1800));
        assert_eq!(payouts[1], (AGENT_B, 1200));
    }

    #[test]
    fn test_full_completion() {
        let (mut s, id) = two_agent_escrow();
        for mi in 0..3 {
            s.agent_complete_milestone(AGENT_A, id, mi);
            s.agent_complete_milestone(AGENT_B, id, mi);
            s.client_verify(CLIENT, id, mi);
        }
        let payouts = s.release_proportional(CLIENT, id);
        let total_paid: u64 = payouts.iter().map(|(_, a)| a).sum();
        assert_eq!(total_paid, 10_000);
        assert_eq!(s.escrows.get(&id).unwrap().status, EscrowStatus::Completed);
    }

    #[test]
    fn test_dispute() {
        let (mut s, id) = two_agent_escrow();
        s.dispute(AGENT_A, id);
        assert_eq!(s.escrows.get(&id).unwrap().status, EscrowStatus::Disputed);
    }

    #[test]
    #[should_panic(expected = "agent shares must sum to 10000")]
    fn test_invalid_shares_rejected() {
        let mut s = EscrowState::new(CLIENT);
        let agents = vec![
            AgentShare {
                address: AGENT_A,
                contribution_share_bps: 5000,
            },
            AgentShare {
                address: AGENT_B,
                contribution_share_bps: 3000,
            },
        ];
        let milestones = vec![Milestone {
            description: "x".into(),
            weight_bps: 10000,
        }];
        s.create_escrow(CLIENT, agents, 1000, milestones);
    }

    #[test]
    #[should_panic(expected = "not all agents completed")]
    fn test_verify_before_completion_rejected() {
        let (mut s, id) = two_agent_escrow();
        s.agent_complete_milestone(AGENT_A, id, 0);
        // AGENT_B hasn't completed yet
        s.client_verify(CLIENT, id, 0);
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state = None;
        dispatch(&mut state, "init", b"{}", CLIENT);
        let args = serde_json::to_vec(&CreateEscrowArgs {
            agents: vec![AgentShare {
                address: AGENT_A,
                contribution_share_bps: 10000,
            }],
            total_amount: 5000,
            milestones: vec![Milestone {
                description: "all".into(),
                weight_bps: 10000,
            }],
        })
        .unwrap();
        let result = dispatch(&mut state, "create_escrow", &args, CLIENT);
        let id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(id, 1);
    }
}
