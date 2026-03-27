use drc113_relay::{dispatch, RelayRegistry};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn channel(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<RelayRegistry> {
    let mut state: Option<RelayRegistry> = None;
    let args = serde_json::to_vec(&serde_json::json!({"challenge_period": 3600u64})).unwrap();
    dispatch(&mut state, "init", &args, addr(1));
    state
}

fn make_blob(channel_id: [u8; 32], seq: u64, submitted_at: u64) -> serde_json::Value {
    serde_json::json!({
        "channel_id": channel_id,
        "balance_a": 500u64,
        "balance_b": 500u64,
        "sequence": seq,
        "party_a": addr(10),
        "party_b": addr(11),
        "signature_a": [1u8, 2u8, 3u8],
        "signature_b": [4u8, 5u8, 6u8],
        "relay_fee": 10u64,
        "submitted_by": addr(0),
        "submitted_at": submitted_at
    })
}

#[test]
fn submit_settlement_stores_pending() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "blob": make_blob(channel(1), 1, 1000)
    })).unwrap();
    dispatch(&mut state, "submit_settlement", &args, addr(5));
    assert!(state.as_ref().unwrap().pending_settlements.contains_key(&channel(1)));
}

#[test]
fn challenge_replaces_with_higher_sequence() {
    let mut state = init();
    let submit = serde_json::to_vec(&serde_json::json!({
        "blob": make_blob(channel(1), 1, 1000)
    })).unwrap();
    dispatch(&mut state, "submit_settlement", &submit, addr(5));

    let challenge = serde_json::to_vec(&serde_json::json!({
        "channel_id": channel(1),
        "newer_blob": make_blob(channel(1), 2, 1500)
    })).unwrap();
    dispatch(&mut state, "challenge_settlement", &challenge, addr(6));

    let pending = state.as_ref().unwrap().pending_for_channel(&channel(1)).unwrap();
    assert_eq!(pending.sequence, 2);
}

#[test]
#[should_panic(expected = "challenger sequence must be higher")]
fn challenge_with_lower_sequence_fails() {
    let mut state = init();
    let submit = serde_json::to_vec(&serde_json::json!({
        "blob": make_blob(channel(1), 5, 1000)
    })).unwrap();
    dispatch(&mut state, "submit_settlement", &submit, addr(5));

    let challenge = serde_json::to_vec(&serde_json::json!({
        "channel_id": channel(1),
        "newer_blob": make_blob(channel(1), 3, 1500)
    })).unwrap();
    dispatch(&mut state, "challenge_settlement", &challenge, addr(6));
}

#[test]
fn finalize_after_challenge_period() {
    let mut state = init();
    let submit = serde_json::to_vec(&serde_json::json!({
        "blob": make_blob(channel(1), 1, 1000)
    })).unwrap();
    dispatch(&mut state, "submit_settlement", &submit, addr(5));

    // Register relay and finalize after challenge period
    let reg = serde_json::to_vec(&serde_json::json!({"addr": addr(5), "timestamp": 500u64})).unwrap();
    dispatch(&mut state, "register_relay", &reg, addr(1));

    let finalize = serde_json::to_vec(&serde_json::json!({
        "channel_id": channel(1), "current_time": 5000u64
    })).unwrap();
    dispatch(&mut state, "finalize_settlement", &finalize, addr(1));

    assert!(!state.as_ref().unwrap().pending_settlements.contains_key(&channel(1)));
    assert_eq!(state.as_ref().unwrap().finalized_count, 1);
}

#[test]
#[should_panic(expected = "challenge period has not elapsed")]
fn finalize_before_challenge_period_fails() {
    let mut state = init();
    let submit = serde_json::to_vec(&serde_json::json!({
        "blob": make_blob(channel(1), 1, 1000)
    })).unwrap();
    dispatch(&mut state, "submit_settlement", &submit, addr(5));

    let finalize = serde_json::to_vec(&serde_json::json!({
        "channel_id": channel(1), "current_time": 2000u64
    })).unwrap();
    dispatch(&mut state, "finalize_settlement", &finalize, addr(1));
}

#[test]
#[should_panic(expected = "parties must be different")]
fn submit_with_same_parties_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "blob": {
            "channel_id": channel(1), "balance_a": 500u64, "balance_b": 500u64,
            "sequence": 1u64, "party_a": addr(10), "party_b": addr(10),
            "signature_a": [1u8], "signature_b": [2u8],
            "relay_fee": 10u64, "submitted_by": addr(0), "submitted_at": 1000u64
        }
    })).unwrap();
    dispatch(&mut state, "submit_settlement", &args, addr(5));
}

#[test]
fn relay_info_returns_none_for_unregistered() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"addr": addr(99)})).unwrap();
    let result = dispatch(&mut state, "relay_info", &args, addr(1));
    let info: Option<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert!(info.is_none());
}

#[test]
fn finalize_updates_relay_stats() {
    let mut state = init();
    let reg = serde_json::to_vec(&serde_json::json!({"addr": addr(5), "timestamp": 500u64})).unwrap();
    dispatch(&mut state, "register_relay", &reg, addr(1));

    let submit = serde_json::to_vec(&serde_json::json!({
        "blob": make_blob(channel(1), 1, 1000)
    })).unwrap();
    dispatch(&mut state, "submit_settlement", &submit, addr(5));

    let finalize = serde_json::to_vec(&serde_json::json!({
        "channel_id": channel(1), "current_time": 5000u64
    })).unwrap();
    dispatch(&mut state, "finalize_settlement", &finalize, addr(1));

    let info = state.as_ref().unwrap().relay_info(&addr(5)).unwrap();
    assert_eq!(info.total_relays, 1);
    assert_eq!(info.total_fees_earned, 10);
}

#[test]
#[should_panic(expected = "relay already registered")]
fn double_register_relay_fails() {
    let mut state = init();
    let reg = serde_json::to_vec(&serde_json::json!({"addr": addr(5), "timestamp": 500u64})).unwrap();
    dispatch(&mut state, "register_relay", &reg, addr(1));
    dispatch(&mut state, "register_relay", &reg, addr(1));
}
