use drc38_bounty::{dispatch, BountyState, BountyStatus};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_bounty(admin: [u8; 32]) -> Option<BountyState> {
    let mut state: Option<BountyState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

fn create_test_bounty(state: &mut Option<BountyState>, poster: [u8; 32]) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "title": "Fix memory leak",
        "description": "Find and fix the OOM in agent service",
        "reward": 1000u64,
        "deadline": 9999u64
    }))
    .unwrap();
    let result = dispatch(state, "create_bounty", &args, poster);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn create_and_get_bounty() {
    let poster = addr(1);
    let mut state = init_bounty(poster);
    let id = create_test_bounty(&mut state, poster);

    let s = state.as_ref().unwrap();
    let bounty = s.get_bounty(id).unwrap();
    assert_eq!(bounty.title, "Fix memory leak");
    assert_eq!(bounty.reward, 1000);
    assert_eq!(bounty.status, BountyStatus::Active);
    assert!(bounty.submissions.is_empty());
}

#[test]
fn submit_and_select_winner() {
    let poster = addr(1);
    let hunter = addr(2);
    let mut state = init_bounty(poster);
    let id = create_test_bounty(&mut state, poster);

    // Submit
    let sub_args = serde_json::to_vec(&serde_json::json!({
        "bounty_id": id,
        "proof_hash": "fix_commit_hash",
        "description": "Patched the leak in agent pool",
        "submitted_at": 5000u64
    }))
    .unwrap();
    dispatch(&mut state, "submit", &sub_args, hunter);

    // Select winner
    let win_args = serde_json::to_vec(&serde_json::json!({
        "bounty_id": id,
        "winner": hunter
    }))
    .unwrap();
    dispatch(&mut state, "select_winner", &win_args, poster);

    let s = state.as_ref().unwrap();
    let bounty = s.get_bounty(id).unwrap();
    assert_eq!(bounty.status, BountyStatus::Completed);
    assert_eq!(bounty.winner, Some(hunter));
}

#[test]
fn cancel_bounty() {
    let poster = addr(1);
    let mut state = init_bounty(poster);
    let id = create_test_bounty(&mut state, poster);

    let args = serde_json::to_vec(&serde_json::json!({ "bounty_id": id })).unwrap();
    dispatch(&mut state, "cancel", &args, poster);

    let s = state.as_ref().unwrap();
    assert_eq!(s.get_bounty(id).unwrap().status, BountyStatus::Cancelled);
    assert_eq!(s.active_bounties().len(), 0);
}

#[test]
fn extend_deadline() {
    let poster = addr(1);
    let mut state = init_bounty(poster);
    let id = create_test_bounty(&mut state, poster);

    let args = serde_json::to_vec(&serde_json::json!({
        "bounty_id": id,
        "new_deadline": 19999u64
    }))
    .unwrap();
    dispatch(&mut state, "extend_deadline", &args, poster);

    let s = state.as_ref().unwrap();
    assert_eq!(s.get_bounty(id).unwrap().deadline, 19999);
}

#[test]
#[should_panic(expected = "poster cannot submit")]
fn poster_cannot_submit_own_bounty() {
    let poster = addr(1);
    let mut state = init_bounty(poster);
    let id = create_test_bounty(&mut state, poster);

    let args = serde_json::to_vec(&serde_json::json!({
        "bounty_id": id,
        "proof_hash": "self_fix",
        "description": "I fixed it myself",
        "submitted_at": 5000u64
    }))
    .unwrap();
    dispatch(&mut state, "submit", &args, poster);
}

#[test]
#[should_panic(expected = "already submitted")]
fn cannot_submit_twice() {
    let poster = addr(1);
    let hunter = addr(2);
    let mut state = init_bounty(poster);
    let id = create_test_bounty(&mut state, poster);

    let args = serde_json::to_vec(&serde_json::json!({
        "bounty_id": id,
        "proof_hash": "hash1",
        "description": "First attempt",
        "submitted_at": 5000u64
    }))
    .unwrap();
    dispatch(&mut state, "submit", &args, hunter);
    dispatch(&mut state, "submit", &args, hunter);
}
