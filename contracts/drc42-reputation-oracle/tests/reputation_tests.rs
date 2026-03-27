use drc42_reputation_oracle::{dispatch, ReputationOracleState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_oracle(admin: [u8; 32]) -> Option<ReputationOracleState> {
    let mut state: Option<ReputationOracleState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

#[test]
fn update_score_and_get_reputation() {
    let admin = addr(1);
    let agent = addr(10);
    let mut state = init_oracle(admin);

    let args = serde_json::to_vec(&serde_json::json!({
        "source": "task_completion",
        "address": agent,
        "score": 8500u64,
        "timestamp": 1000u64
    }))
    .unwrap();
    dispatch(&mut state, "update_score", &args, admin);

    let s = state.as_ref().unwrap();
    let rep = s.get_reputation(&agent).unwrap();
    assert_eq!(rep.aggregate_score, 8500);
    assert_eq!(rep.scores.get("task_completion"), Some(&8500));
}

#[test]
fn multi_source_aggregation() {
    let admin = addr(1);
    let agent = addr(10);
    let mut state = init_oracle(admin);

    // Source 1: 8000
    let args1 = serde_json::to_vec(&serde_json::json!({
        "source": "tasks",
        "address": agent,
        "score": 8000u64,
        "timestamp": 1000u64
    }))
    .unwrap();
    dispatch(&mut state, "update_score", &args1, admin);

    // Source 2: 6000
    let args2 = serde_json::to_vec(&serde_json::json!({
        "source": "reviews",
        "address": agent,
        "score": 6000u64,
        "timestamp": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "update_score", &args2, admin);

    let s = state.as_ref().unwrap();
    let rep = s.get_reputation(&agent).unwrap();
    assert_eq!(rep.aggregate_score, 7000); // (8000 + 6000) / 2
    assert_eq!(rep.last_updated, 2000);
}

#[test]
fn get_score_from_specific_source() {
    let admin = addr(1);
    let agent = addr(10);
    let mut state = init_oracle(admin);

    let args = serde_json::to_vec(&serde_json::json!({
        "source": "on_chain",
        "address": agent,
        "score": 9200u64,
        "timestamp": 1000u64
    }))
    .unwrap();
    dispatch(&mut state, "update_score", &args, admin);

    let s = state.as_ref().unwrap();
    assert_eq!(s.get_score_from_source(&agent, "on_chain"), Some(9200));
    assert_eq!(s.get_score_from_source(&agent, "off_chain"), None);
}

#[test]
fn top_rated_returns_sorted() {
    let admin = addr(1);
    let mut state = init_oracle(admin);

    for i in 1..=5u8 {
        let args = serde_json::to_vec(&serde_json::json!({
            "source": "general",
            "address": addr(i + 10),
            "score": (i as u64) * 1000,
            "timestamp": 1000u64
        }))
        .unwrap();
        dispatch(&mut state, "update_score", &args, admin);
    }

    let s = state.as_ref().unwrap();
    let top = s.top_rated(3);
    assert_eq!(top.len(), 3);
    assert_eq!(top[0].aggregate_score, 5000);
    assert_eq!(top[1].aggregate_score, 4000);
    assert_eq!(top[2].aggregate_score, 3000);
}

#[test]
#[should_panic(expected = "caller not an authorized source")]
fn unauthorized_source_cannot_update() {
    let admin = addr(1);
    let rando = addr(99);
    let agent = addr(10);
    let mut state = init_oracle(admin);

    let args = serde_json::to_vec(&serde_json::json!({
        "source": "fake",
        "address": agent,
        "score": 9999u64,
        "timestamp": 1000u64
    }))
    .unwrap();
    dispatch(&mut state, "update_score", &args, rando);
}

#[test]
fn authorize_new_source() {
    let admin = addr(1);
    let oracle = addr(5);
    let agent = addr(10);
    let mut state = init_oracle(admin);

    // Authorize oracle
    let auth_args =
        serde_json::to_vec(&serde_json::json!({ "source": oracle })).unwrap();
    dispatch(&mut state, "authorize_source", &auth_args, admin);

    // Oracle can now update
    let args = serde_json::to_vec(&serde_json::json!({
        "source": "external",
        "address": agent,
        "score": 7500u64,
        "timestamp": 1000u64
    }))
    .unwrap();
    dispatch(&mut state, "update_score", &args, oracle);

    let s = state.as_ref().unwrap();
    assert_eq!(s.get_reputation(&agent).unwrap().aggregate_score, 7500);
}

#[test]
#[should_panic(expected = "score must be 0-10000")]
fn score_out_of_range_fails() {
    let admin = addr(1);
    let mut state = init_oracle(admin);

    let args = serde_json::to_vec(&serde_json::json!({
        "source": "test",
        "address": addr(10),
        "score": 99999u64,
        "timestamp": 1000u64
    }))
    .unwrap();
    dispatch(&mut state, "update_score", &args, admin);
}
