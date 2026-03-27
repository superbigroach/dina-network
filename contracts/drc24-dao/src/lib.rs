use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-24  DAO Treasury
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Action {
    Transfer { to: Address, amount: u64 },
    AddMember { member: Address, voting_power: u64 },
    RemoveMember { member: Address },
    SetThreshold { threshold: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub title: String,
    pub actions: Vec<Action>,
    pub votes_for: u64,
    pub votes_against: u64,
    pub status: ProposalStatus,
    pub deadline: u64,
    pub voters: BTreeMap<Address, bool>, // true = for, false = against
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DaoState {
    pub treasury_balance: u64,
    pub proposals: BTreeMap<u64, Proposal>,
    pub members: BTreeMap<Address, u64>, // address -> voting power
    pub proposal_threshold: u64,         // min voting power to propose
    pub next_proposal_id: u64,
    pub owner: Address,
}

impl DaoState {
    pub fn new(owner: Address, initial_voting_power: u64, proposal_threshold: u64) -> Self {
        let mut members = BTreeMap::new();
        members.insert(owner, initial_voting_power);
        Self {
            treasury_balance: 0,
            proposals: BTreeMap::new(),
            members,
            proposal_threshold,
            next_proposal_id: 1,
            owner,
        }
    }

    pub fn deposit(&mut self, amount: u64) {
        assert!(amount > 0, "DRC24: deposit amount must be positive");
        self.treasury_balance += amount;
    }

    pub fn propose(
        &mut self,
        caller: Address,
        title: String,
        actions: Vec<Action>,
        deadline: u64,
    ) -> u64 {
        let power = self.members.get(&caller).copied().unwrap_or(0);
        assert!(
            power >= self.proposal_threshold,
            "DRC24: insufficient voting power to propose ({power} < {})",
            self.proposal_threshold
        );
        assert!(!title.is_empty(), "DRC24: title cannot be empty");
        assert!(!actions.is_empty(), "DRC24: actions cannot be empty");

        let id = self.next_proposal_id;
        self.next_proposal_id += 1;

        self.proposals.insert(
            id,
            Proposal {
                id,
                proposer: caller,
                title,
                actions,
                votes_for: 0,
                votes_against: 0,
                status: ProposalStatus::Active,
                deadline,
                voters: BTreeMap::new(),
            },
        );
        id
    }

    pub fn vote(&mut self, caller: Address, proposal_id: u64, support: bool, current_time: u64) {
        let power = self.members.get(&caller).copied().unwrap_or(0);
        assert!(power > 0, "DRC24: caller is not a member");

        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .expect("DRC24: proposal not found");
        assert!(
            proposal.status == ProposalStatus::Active,
            "DRC24: proposal is not active"
        );
        assert!(
            current_time <= proposal.deadline,
            "DRC24: voting period has ended"
        );
        assert!(
            !proposal.voters.contains_key(&caller),
            "DRC24: already voted"
        );

        proposal.voters.insert(caller, support);
        if support {
            proposal.votes_for += power;
        } else {
            proposal.votes_against += power;
        }
    }

    pub fn execute(&mut self, caller: Address, proposal_id: u64, current_time: u64) {
        // Finalize status first
        {
            let proposal = self
                .proposals
                .get_mut(&proposal_id)
                .expect("DRC24: proposal not found");
            assert!(
                proposal.status == ProposalStatus::Active,
                "DRC24: proposal is not active"
            );
            assert!(
                current_time > proposal.deadline,
                "DRC24: voting period has not ended"
            );

            if proposal.votes_for > proposal.votes_against {
                proposal.status = ProposalStatus::Passed;
            } else {
                proposal.status = ProposalStatus::Rejected;
                return;
            }
        }

        // Execute actions
        let actions = self.proposals[&proposal_id].actions.clone();
        for action in &actions {
            match action {
                Action::Transfer { to, amount } => {
                    assert!(
                        self.treasury_balance >= *amount,
                        "DRC24: insufficient treasury balance"
                    );
                    self.treasury_balance -= *amount;
                    // In a real chain this would credit `to`; here we just debit treasury
                    let _ = to;
                }
                Action::AddMember {
                    member,
                    voting_power,
                } => {
                    self.members.insert(*member, *voting_power);
                }
                Action::RemoveMember { member } => {
                    assert!(*member != self.owner, "DRC24: cannot remove owner");
                    self.members.remove(member);
                }
                Action::SetThreshold { threshold } => {
                    self.proposal_threshold = *threshold;
                }
            }
        }

        self.proposals.get_mut(&proposal_id).unwrap().status = ProposalStatus::Executed;
        let _ = caller;
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    initial_voting_power: u64,
    proposal_threshold: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProposeArgs {
    title: String,
    actions: Vec<Action>,
    deadline: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct VoteArgs {
    proposal_id: u64,
    support: bool,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteArgs {
    proposal_id: u64,
    current_time: u64,
}

/// Contract-level dispatch.
pub fn dispatch(
    state: &mut Option<DaoState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC24: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC24: bad init args");
            *state = Some(DaoState::new(caller, a.initial_voting_power, a.proposal_threshold));
            serde_json::to_vec("ok").unwrap()
        }

        "deposit" => {
            let s = state.as_mut().expect("DRC24: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC24: bad deposit args");
            s.deposit(a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        "propose" => {
            let s = state.as_mut().expect("DRC24: not initialised");
            let a: ProposeArgs = serde_json::from_slice(args).expect("DRC24: bad propose args");
            let id = s.propose(caller, a.title, a.actions, a.deadline);
            serde_json::to_vec(&id).unwrap()
        }

        "vote" => {
            let s = state.as_mut().expect("DRC24: not initialised");
            let a: VoteArgs = serde_json::from_slice(args).expect("DRC24: bad vote args");
            s.vote(caller, a.proposal_id, a.support, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }

        "execute" => {
            let s = state.as_mut().expect("DRC24: not initialised");
            let a: ExecuteArgs = serde_json::from_slice(args).expect("DRC24: bad execute args");
            s.execute(caller, a.proposal_id, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }

        "member_count" => {
            let s = state.as_ref().expect("DRC24: not initialised");
            serde_json::to_vec(&s.member_count()).unwrap()
        }

        _ => panic!("DRC24: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [1u8; 32];
    const MEMBER_A: Address = [2u8; 32];
    const MEMBER_B: Address = [3u8; 32];
    const NOBODY: Address = [4u8; 32];

    fn init_dao() -> Option<DaoState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            initial_voting_power: 100,
            proposal_threshold: 10,
        })
        .unwrap();
        dispatch(&mut state, "init", &args, OWNER);
        state
    }

    #[test]
    fn test_deposit_and_transfer_proposal() {
        let mut state = init_dao();

        // Deposit funds
        let dep = serde_json::to_vec(&DepositArgs { amount: 1000 }).unwrap();
        dispatch(&mut state, "deposit", &dep, OWNER);
        assert_eq!(state.as_ref().unwrap().treasury_balance, 1000);

        // Propose transfer
        let prop = serde_json::to_vec(&ProposeArgs {
            title: "Fund developer".into(),
            actions: vec![Action::Transfer {
                to: MEMBER_A,
                amount: 500,
            }],
            deadline: 100,
        })
        .unwrap();
        let result = dispatch(&mut state, "propose", &prop, OWNER);
        let proposal_id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(proposal_id, 1);

        // Vote for
        let vote = serde_json::to_vec(&VoteArgs {
            proposal_id: 1,
            support: true,
            current_time: 50,
        })
        .unwrap();
        dispatch(&mut state, "vote", &vote, OWNER);

        // Execute after deadline
        let exec = serde_json::to_vec(&ExecuteArgs {
            proposal_id: 1,
            current_time: 101,
        })
        .unwrap();
        dispatch(&mut state, "execute", &exec, OWNER);

        let s = state.as_ref().unwrap();
        assert_eq!(s.treasury_balance, 500);
        assert_eq!(s.proposals[&1].status, ProposalStatus::Executed);
    }

    #[test]
    fn test_add_member_via_proposal() {
        let mut state = init_dao();

        let prop = serde_json::to_vec(&ProposeArgs {
            title: "Add member A".into(),
            actions: vec![Action::AddMember {
                member: MEMBER_A,
                voting_power: 50,
            }],
            deadline: 100,
        })
        .unwrap();
        dispatch(&mut state, "propose", &prop, OWNER);

        let vote = serde_json::to_vec(&VoteArgs {
            proposal_id: 1,
            support: true,
            current_time: 50,
        })
        .unwrap();
        dispatch(&mut state, "vote", &vote, OWNER);

        let exec = serde_json::to_vec(&ExecuteArgs {
            proposal_id: 1,
            current_time: 101,
        })
        .unwrap();
        dispatch(&mut state, "execute", &exec, OWNER);

        assert_eq!(state.as_ref().unwrap().member_count(), 2);
        assert_eq!(state.as_ref().unwrap().members[&MEMBER_A], 50);
    }

    #[test]
    fn test_proposal_rejected_when_more_against() {
        let mut state = init_dao();

        // Manually add member_a and member_b with more combined power
        let s = state.as_mut().unwrap();
        s.members.insert(MEMBER_A, 60);
        s.members.insert(MEMBER_B, 60);

        // Owner proposes
        let prop = serde_json::to_vec(&ProposeArgs {
            title: "Bad idea".into(),
            actions: vec![Action::SetThreshold { threshold: 1 }],
            deadline: 100,
        })
        .unwrap();
        dispatch(&mut state, "propose", &prop, OWNER);

        // Owner votes for (100 power)
        let vote_for = serde_json::to_vec(&VoteArgs {
            proposal_id: 1,
            support: true,
            current_time: 50,
        })
        .unwrap();
        dispatch(&mut state, "vote", &vote_for, OWNER);

        // A and B vote against (120 power)
        let vote_a = serde_json::to_vec(&VoteArgs {
            proposal_id: 1,
            support: false,
            current_time: 60,
        })
        .unwrap();
        dispatch(&mut state, "vote", &vote_a, MEMBER_A);

        let vote_b = serde_json::to_vec(&VoteArgs {
            proposal_id: 1,
            support: false,
            current_time: 70,
        })
        .unwrap();
        dispatch(&mut state, "vote", &vote_b, MEMBER_B);

        // Execute — should be rejected
        let exec = serde_json::to_vec(&ExecuteArgs {
            proposal_id: 1,
            current_time: 101,
        })
        .unwrap();
        dispatch(&mut state, "execute", &exec, OWNER);

        assert_eq!(
            state.as_ref().unwrap().proposals[&1].status,
            ProposalStatus::Rejected
        );
    }

    #[test]
    #[should_panic(expected = "insufficient voting power")]
    fn test_non_member_cannot_propose() {
        let mut state = init_dao();
        let prop = serde_json::to_vec(&ProposeArgs {
            title: "Sneaky".into(),
            actions: vec![Action::SetThreshold { threshold: 0 }],
            deadline: 100,
        })
        .unwrap();
        dispatch(&mut state, "propose", &prop, NOBODY);
    }

    #[test]
    #[should_panic(expected = "already voted")]
    fn test_double_vote_rejected() {
        let mut state = init_dao();
        let prop = serde_json::to_vec(&ProposeArgs {
            title: "Test".into(),
            actions: vec![Action::SetThreshold { threshold: 5 }],
            deadline: 100,
        })
        .unwrap();
        dispatch(&mut state, "propose", &prop, OWNER);

        let vote = serde_json::to_vec(&VoteArgs {
            proposal_id: 1,
            support: true,
            current_time: 50,
        })
        .unwrap();
        dispatch(&mut state, "vote", &vote, OWNER);
        dispatch(&mut state, "vote", &vote, OWNER); // double vote
    }

    #[test]
    fn test_member_count() {
        let mut state = init_dao();
        let result = dispatch(&mut state, "member_count", b"", OWNER);
        let count: usize = serde_json::from_slice(&result).unwrap();
        assert_eq!(count, 1);
    }
}
