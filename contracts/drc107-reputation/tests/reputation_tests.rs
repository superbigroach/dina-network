use drc107_reputation::{dispatch, ReputationState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<ReputationState> {
    let mut state: Option<ReputationState> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn record(
    state: &mut Option<ReputationState>,
    subject: [u8; 32],
    counterparty: [u8; 32],
    outcome: &str,
    rating: u16,
) {
    let args = serde_json::to_vec(&serde_json::json!({
        "subject": subject,
        "counterparty": counterparty,
        "outcome": outcome,
        "rating": rating,
        "volume": 1000u64,
        "timestamp": 1000u64,
        "category": "service"
    }))
    .unwrap();
    dispatch(state, "record_interaction", &args, addr(1));
}

#[test]
fn reputation_of_defaults_to_5000() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"account": addr(99)})).unwrap();
    let result = dispatch(&mut state, "reputation_of", &args, addr(1));
    let score: u16 = serde_json::from_slice(&result).unwrap();
    assert_eq!(score, 5000);
}

#[test]
fn record_interaction_updates_reputation() {
    let mut state = init();
    record(&mut state, addr(2), addr(3), "Success", 8000);
    let args = serde_json::to_vec(&serde_json::json!({"account": addr(2)})).unwrap();
    let result = dispatch(&mut state, "reputation_of", &args, addr(1));
    let score: u16 = serde_json::from_slice(&result).unwrap();
    assert!(score > 5000);
}

#[test]
fn failed_interaction_lowers_reputation() {
    let mut state = init();
    record(&mut state, addr(2), addr(3), "Failure", 2000);
    let args = serde_json::to_vec(&serde_json::json!({"account": addr(2)})).unwrap();
    let result = dispatch(&mut state, "reputation_of", &args, addr(1));
    let score: u16 = serde_json::from_slice(&result).unwrap();
    assert!(score < 5000);
}

#[test]
fn meets_threshold_returns_correct_result() {
    let mut state = init();
    record(&mut state, addr(2), addr(3), "Success", 9000);

    let args_high = serde_json::to_vec(&serde_json::json!({
        "account": addr(2), "threshold": 7000u16
    }))
    .unwrap();
    let result = dispatch(&mut state, "meets_threshold", &args_high, addr(1));
    let meets: bool = serde_json::from_slice(&result).unwrap();
    assert!(meets);

    let args_low = serde_json::to_vec(&serde_json::json!({
        "account": addr(2), "threshold": 9500u16
    }))
    .unwrap();
    let result2 = dispatch(&mut state, "meets_threshold", &args_low, addr(1));
    let meets2: bool = serde_json::from_slice(&result2).unwrap();
    assert!(!meets2);
}

#[test]
#[should_panic(expected = "cannot rate yourself")]
fn self_rating_fails() {
    let mut state = init();
    record(&mut state, addr(2), addr(2), "Success", 9000);
}

#[test]
#[should_panic(expected = "rating must be 0-10000")]
fn rating_over_10000_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "subject": addr(2), "counterparty": addr(3),
        "outcome": "Success", "rating": 20000u16,
        "volume": 1000u64, "timestamp": 1000u64, "category": "service"
    }))
    .unwrap();
    dispatch(&mut state, "record_interaction", &args, addr(1));
}

#[test]
fn reputation_details_tracks_counts() {
    let mut state = init();
    record(&mut state, addr(2), addr(3), "Success", 8000);
    record(&mut state, addr(2), addr(4), "Failure", 3000);

    let args = serde_json::to_vec(&serde_json::json!({"account": addr(2)})).unwrap();
    let result = dispatch(&mut state, "reputation_details", &args, addr(1));
    let details: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(details["total_interactions"], 2);
    assert_eq!(details["successful"], 1);
    assert_eq!(details["failed"], 1);
}
