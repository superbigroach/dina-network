use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-38  Bug Bounty / Task Bounty
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum BountyStatus {
    Active,
    Completed,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Submission {
    pub submitter: Address,
    pub proof_hash: String,
    pub description: String,
    pub submitted_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Bounty {
    pub id: u64,
    pub poster: Address,
    pub title: String,
    pub description: String,
    pub reward: u64,
    pub deadline: u64,
    pub submissions: Vec<Submission>,
    pub winner: Option<Address>,
    pub status: BountyStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BountyState {
    pub admin: Address,
    pub bounties: BTreeMap<u64, Bounty>,
    pub next_id: u64,
}

impl BountyState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            bounties: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn create_bounty(
        &mut self,
        caller: Address,
        title: String,
        description: String,
        reward: u64,
        deadline: u64,
    ) -> u64 {
        assert!(reward > 0, "DRC38: reward must be positive");
        assert!(!title.is_empty(), "DRC38: title cannot be empty");
        let id = self.next_id;
        self.next_id += 1;
        let bounty = Bounty {
            id,
            poster: caller,
            title,
            description,
            reward,
            deadline,
            submissions: Vec::new(),
            winner: None,
            status: BountyStatus::Active,
        };
        self.bounties.insert(id, bounty);
        id
    }

    pub fn submit(
        &mut self,
        caller: Address,
        bounty_id: u64,
        proof_hash: String,
        description: String,
        submitted_at: u64,
    ) {
        let bounty = self
            .bounties
            .get_mut(&bounty_id)
            .expect("DRC38: bounty not found");
        assert!(
            bounty.status == BountyStatus::Active,
            "DRC38: bounty not active"
        );
        assert!(
            bounty.poster != caller,
            "DRC38: poster cannot submit to own bounty"
        );
        assert!(
            !bounty.submissions.iter().any(|s| s.submitter == caller),
            "DRC38: already submitted"
        );
        bounty.submissions.push(Submission {
            submitter: caller,
            proof_hash,
            description,
            submitted_at,
        });
    }

    pub fn select_winner(&mut self, caller: Address, bounty_id: u64, winner: Address) {
        let bounty = self
            .bounties
            .get_mut(&bounty_id)
            .expect("DRC38: bounty not found");
        assert!(
            bounty.poster == caller,
            "DRC38: only poster can select winner"
        );
        assert!(
            bounty.status == BountyStatus::Active,
            "DRC38: bounty not active"
        );
        assert!(
            bounty.submissions.iter().any(|s| s.submitter == winner),
            "DRC38: winner has no submission"
        );
        bounty.winner = Some(winner);
        bounty.status = BountyStatus::Completed;
    }

    pub fn cancel(&mut self, caller: Address, bounty_id: u64) {
        let bounty = self
            .bounties
            .get_mut(&bounty_id)
            .expect("DRC38: bounty not found");
        assert!(
            bounty.poster == caller || caller == self.admin,
            "DRC38: not authorized"
        );
        assert!(
            bounty.status == BountyStatus::Active,
            "DRC38: bounty not active"
        );
        bounty.status = BountyStatus::Cancelled;
    }

    pub fn extend_deadline(&mut self, caller: Address, bounty_id: u64, new_deadline: u64) {
        let bounty = self
            .bounties
            .get_mut(&bounty_id)
            .expect("DRC38: bounty not found");
        assert!(
            bounty.poster == caller,
            "DRC38: only poster can extend deadline"
        );
        assert!(
            new_deadline > bounty.deadline,
            "DRC38: new deadline must be later"
        );
        bounty.deadline = new_deadline;
    }

    pub fn active_bounties(&self) -> Vec<&Bounty> {
        self.bounties
            .values()
            .filter(|b| b.status == BountyStatus::Active)
            .collect()
    }

    pub fn get_bounty(&self, id: u64) -> Option<&Bounty> {
        self.bounties.get(&id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateBountyArgs {
    title: String,
    description: String,
    reward: u64,
    deadline: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SubmitArgs {
    bounty_id: u64,
    proof_hash: String,
    description: String,
    submitted_at: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SelectWinnerArgs {
    bounty_id: u64,
    winner: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct CancelArgs {
    bounty_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExtendDeadlineArgs {
    bounty_id: u64,
    new_deadline: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetBountyArgs {
    id: u64,
}

pub fn dispatch(
    state: &mut Option<BountyState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC38: already initialised");
            *state = Some(BountyState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_bounty" => {
            let s = state.as_mut().expect("DRC38: not initialised");
            let a: CreateBountyArgs =
                serde_json::from_slice(args).expect("DRC38: bad create_bounty args");
            let id = s.create_bounty(caller, a.title, a.description, a.reward, a.deadline);
            serde_json::to_vec(&id).unwrap()
        }
        "submit" => {
            let s = state.as_mut().expect("DRC38: not initialised");
            let a: SubmitArgs =
                serde_json::from_slice(args).expect("DRC38: bad submit args");
            s.submit(caller, a.bounty_id, a.proof_hash, a.description, a.submitted_at);
            serde_json::to_vec("ok").unwrap()
        }
        "select_winner" => {
            let s = state.as_mut().expect("DRC38: not initialised");
            let a: SelectWinnerArgs =
                serde_json::from_slice(args).expect("DRC38: bad select_winner args");
            s.select_winner(caller, a.bounty_id, a.winner);
            serde_json::to_vec("ok").unwrap()
        }
        "cancel" => {
            let s = state.as_mut().expect("DRC38: not initialised");
            let a: CancelArgs =
                serde_json::from_slice(args).expect("DRC38: bad cancel args");
            s.cancel(caller, a.bounty_id);
            serde_json::to_vec("ok").unwrap()
        }
        "extend_deadline" => {
            let s = state.as_mut().expect("DRC38: not initialised");
            let a: ExtendDeadlineArgs =
                serde_json::from_slice(args).expect("DRC38: bad extend_deadline args");
            s.extend_deadline(caller, a.bounty_id, a.new_deadline);
            serde_json::to_vec("ok").unwrap()
        }
        "active_bounties" => {
            let s = state.as_ref().expect("DRC38: not initialised");
            serde_json::to_vec(&s.active_bounties()).unwrap()
        }
        "get_bounty" => {
            let s = state.as_ref().expect("DRC38: not initialised");
            let a: GetBountyArgs =
                serde_json::from_slice(args).expect("DRC38: bad get_bounty args");
            serde_json::to_vec(&s.get_bounty(a.id)).unwrap()
        }
        _ => panic!("DRC38: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: Address = [1u8; 32];
    const POSTER: Address = [2u8; 32];
    const HUNTER_A: Address = [3u8; 32];
    const HUNTER_B: Address = [4u8; 32];

    fn init_state() -> Option<BountyState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", ADMIN);
        state
    }

    fn create_bounty_via_dispatch(state: &mut Option<BountyState>) -> u64 {
        let args = serde_json::to_vec(&serde_json::json!({
            "title": "Fix memory leak in agent runtime",
            "description": "Agent process grows unbounded after 24h",
            "reward": 5000u64,
            "deadline": 1700000000u64
        }))
        .unwrap();
        let result = dispatch(state, "create_bounty", &args, POSTER);
        serde_json::from_slice(&result).unwrap()
    }

    #[test]
    fn test_create_bounty() {
        let mut state = init_state();
        let id = create_bounty_via_dispatch(&mut state);
        assert_eq!(id, 1);

        let get_args = serde_json::to_vec(&serde_json::json!({"id": 1})).unwrap();
        let result = dispatch(&mut state, "get_bounty", &get_args, ADMIN);
        let bounty: Bounty = serde_json::from_slice(&result).unwrap();
        assert_eq!(bounty.title, "Fix memory leak in agent runtime");
        assert_eq!(bounty.reward, 5000);
        assert_eq!(bounty.status, BountyStatus::Active);
        assert!(bounty.winner.is_none());
    }

    #[test]
    fn test_submit_and_select_winner() {
        let mut state = init_state();
        create_bounty_via_dispatch(&mut state);

        // Two hunters submit
        let sub_a = serde_json::to_vec(&serde_json::json!({
            "bounty_id": 1,
            "proof_hash": "proof_a_hash",
            "description": "Fixed by adding drop handler",
            "submitted_at": 1699900000u64
        }))
        .unwrap();
        dispatch(&mut state, "submit", &sub_a, HUNTER_A);

        let sub_b = serde_json::to_vec(&serde_json::json!({
            "bounty_id": 1,
            "proof_hash": "proof_b_hash",
            "description": "Root cause was circular ref",
            "submitted_at": 1699950000u64
        }))
        .unwrap();
        dispatch(&mut state, "submit", &sub_b, HUNTER_B);

        // Select winner
        let winner_args = serde_json::to_vec(&serde_json::json!({
            "bounty_id": 1,
            "winner": HUNTER_A
        }))
        .unwrap();
        dispatch(&mut state, "select_winner", &winner_args, POSTER);

        let s = state.as_ref().unwrap();
        let bounty = s.get_bounty(1).unwrap();
        assert_eq!(bounty.status, BountyStatus::Completed);
        assert_eq!(bounty.winner.unwrap(), HUNTER_A);
        assert_eq!(bounty.submissions.len(), 2);
    }

    #[test]
    fn test_cancel_bounty() {
        let mut state = init_state();
        create_bounty_via_dispatch(&mut state);

        let cancel_args = serde_json::to_vec(&serde_json::json!({"bounty_id": 1})).unwrap();
        dispatch(&mut state, "cancel", &cancel_args, POSTER);

        let s = state.as_ref().unwrap();
        let bounty = s.get_bounty(1).unwrap();
        assert_eq!(bounty.status, BountyStatus::Cancelled);
    }

    #[test]
    fn test_active_bounties_filter() {
        let mut state = init_state();
        create_bounty_via_dispatch(&mut state);

        // Create second bounty
        let args2 = serde_json::to_vec(&serde_json::json!({
            "title": "Improve consensus speed",
            "description": "Target 50% faster finality",
            "reward": 10000u64,
            "deadline": 1701000000u64
        }))
        .unwrap();
        dispatch(&mut state, "create_bounty", &args2, POSTER);

        // Cancel first
        let cancel_args = serde_json::to_vec(&serde_json::json!({"bounty_id": 1})).unwrap();
        dispatch(&mut state, "cancel", &cancel_args, POSTER);

        let result = dispatch(&mut state, "active_bounties", b"", ADMIN);
        let active: Vec<Bounty> = serde_json::from_slice(&result).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, 2);
    }

    #[test]
    fn test_extend_deadline() {
        let mut state = init_state();
        create_bounty_via_dispatch(&mut state);

        let args = serde_json::to_vec(&serde_json::json!({
            "bounty_id": 1,
            "new_deadline": 1800000000u64
        }))
        .unwrap();
        dispatch(&mut state, "extend_deadline", &args, POSTER);

        let s = state.as_ref().unwrap();
        let bounty = s.get_bounty(1).unwrap();
        assert_eq!(bounty.deadline, 1800000000);
    }

    #[test]
    #[should_panic(expected = "DRC38: poster cannot submit to own bounty")]
    fn test_poster_cannot_self_submit() {
        let mut state = init_state();
        create_bounty_via_dispatch(&mut state);

        let args = serde_json::to_vec(&serde_json::json!({
            "bounty_id": 1,
            "proof_hash": "self_hash",
            "description": "my own fix",
            "submitted_at": 999u64
        }))
        .unwrap();
        dispatch(&mut state, "submit", &args, POSTER);
    }

    #[test]
    #[should_panic(expected = "DRC38: already submitted")]
    fn test_cannot_submit_twice() {
        let mut state = init_state();
        create_bounty_via_dispatch(&mut state);

        let args = serde_json::to_vec(&serde_json::json!({
            "bounty_id": 1,
            "proof_hash": "hash1",
            "description": "attempt 1",
            "submitted_at": 999u64
        }))
        .unwrap();
        dispatch(&mut state, "submit", &args, HUNTER_A);
        dispatch(&mut state, "submit", &args, HUNTER_A);
    }

    #[test]
    fn test_admin_can_cancel() {
        let mut state = init_state();
        create_bounty_via_dispatch(&mut state);

        let cancel_args = serde_json::to_vec(&serde_json::json!({"bounty_id": 1})).unwrap();
        dispatch(&mut state, "cancel", &cancel_args, ADMIN);

        let s = state.as_ref().unwrap();
        assert_eq!(s.get_bounty(1).unwrap().status, BountyStatus::Cancelled);
    }
}
