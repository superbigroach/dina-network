use drc109_emergency_stop::{dispatch, EmergencyStopState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<EmergencyStopState> {
    let mut state: Option<EmergencyStopState> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn freeze(state: &mut Option<EmergencyStopState>, caller: [u8; 32], target: [u8; 32], scope: &str) {
    let args = serde_json::to_vec(&serde_json::json!({
        "target": target, "reason": "security", "scope": scope, "timestamp": 1000u64
    }))
    .unwrap();
    dispatch(state, "freeze", &args, caller);
}

#[test]
fn freeze_marks_target_as_frozen() {
    let mut state = init();
    freeze(&mut state, addr(1), addr(10), "FullFreeze");
    let args = serde_json::to_vec(&serde_json::json!({"target": addr(10)})).unwrap();
    let result = dispatch(&mut state, "is_frozen", &args, addr(1));
    let frozen: bool = serde_json::from_slice(&result).unwrap();
    assert!(frozen);
}

#[test]
fn unfreeze_removes_freeze() {
    let mut state = init();
    freeze(&mut state, addr(1), addr(10), "FullFreeze");
    let args = serde_json::to_vec(&serde_json::json!({"target": addr(10)})).unwrap();
    dispatch(&mut state, "unfreeze", &args, addr(1));
    let result = dispatch(&mut state, "is_frozen", &args, addr(1));
    let frozen: bool = serde_json::from_slice(&result).unwrap();
    assert!(!frozen);
}

#[test]
fn is_frozen_returns_false_for_unknown() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"target": addr(99)})).unwrap();
    let result = dispatch(&mut state, "is_frozen", &args, addr(1));
    let frozen: bool = serde_json::from_slice(&result).unwrap();
    assert!(!frozen);
}

#[test]
#[should_panic(expected = "not an authorised responder")]
fn freeze_by_unauthorized_fails() {
    let mut state = init();
    freeze(&mut state, addr(99), addr(10), "FullFreeze");
}

#[test]
fn registered_responder_can_freeze() {
    let mut state = init();
    let reg_args = serde_json::to_vec(&serde_json::json!({
        "responder": addr(5), "label": "security-bot"
    }))
    .unwrap();
    dispatch(&mut state, "register_responder", &reg_args, addr(1));
    freeze(&mut state, addr(5), addr(10), "WalletOnly");
    assert!(state.as_ref().unwrap().is_frozen(&addr(10)));
}

#[test]
#[should_panic(expected = "only admin can lift a NetworkBan")]
fn network_ban_unfreeze_by_non_admin_fails() {
    let mut state = init();
    // Register a responder
    let reg_args = serde_json::to_vec(&serde_json::json!({
        "responder": addr(5), "label": "bot"
    }))
    .unwrap();
    dispatch(&mut state, "register_responder", &reg_args, addr(1));

    // Admin freezes with NetworkBan
    freeze(&mut state, addr(1), addr(10), "NetworkBan");

    // Non-admin responder tries to unfreeze
    let args = serde_json::to_vec(&serde_json::json!({"target": addr(10)})).unwrap();
    dispatch(&mut state, "unfreeze", &args, addr(5));
}

#[test]
#[should_panic(expected = "target is already frozen")]
fn double_freeze_fails() {
    let mut state = init();
    freeze(&mut state, addr(1), addr(10), "FullFreeze");
    freeze(&mut state, addr(1), addr(10), "WalletOnly");
}

#[test]
fn stats_tracks_counts() {
    let mut state = init();
    freeze(&mut state, addr(1), addr(10), "FullFreeze");
    freeze(&mut state, addr(1), addr(11), "WalletOnly");
    let unf = serde_json::to_vec(&serde_json::json!({"target": addr(10)})).unwrap();
    dispatch(&mut state, "unfreeze", &unf, addr(1));

    let result = dispatch(&mut state, "stats", b"", addr(1));
    let stats: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(stats["total_freezes"], 2);
    assert_eq!(stats["total_unfreezes"], 1);
    assert_eq!(stats["currently_frozen"], 1);
}

#[test]
#[should_panic(expected = "cannot remove admin as responder")]
fn remove_admin_responder_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"responder": addr(1)})).unwrap();
    dispatch(&mut state, "remove_responder", &args, addr(1));
}
