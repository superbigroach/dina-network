use drc13_compliant::{dispatch, CompliantTokenState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<CompliantTokenState> {
    let mut state: Option<CompliantTokenState> = None;
    let args = serde_json::to_vec(&serde_json::json!({
        "name": "CompToken", "symbol": "CT", "decimals": 18u8
    }))
    .unwrap();
    dispatch(&mut state, "init", &args, addr(1));
    state
}

fn verify_addr(state: &mut Option<CompliantTokenState>, target: [u8; 32], country: &str) {
    let args = serde_json::to_vec(&serde_json::json!({
        "addr": target,
        "info": {"country": country, "credentials": ["KYC"], "verified_at": 1000u64}
    }))
    .unwrap();
    dispatch(state, "verify_address", &args, addr(1));
}

fn mint_to(state: &mut Option<CompliantTokenState>, to: [u8; 32], amount: u64) {
    let args = serde_json::to_vec(&serde_json::json!({"to": to, "amount": amount})).unwrap();
    dispatch(state, "mint", &args, addr(1));
}

#[test]
fn compliant_transfer_succeeds_with_verified_parties() {
    let mut state = init();
    verify_addr(&mut state, addr(2), "US");
    verify_addr(&mut state, addr(3), "US");
    mint_to(&mut state, addr(2), 1000);

    let args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 400u64, "current_time": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "compliant_transfer", &args, addr(2));

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance_of(&addr(2)), 600);
    assert_eq!(s.balance_of(&addr(3)), 400);
}

#[test]
#[should_panic(expected = "sender is frozen")]
fn transfer_by_frozen_sender_fails() {
    let mut state = init();
    verify_addr(&mut state, addr(2), "US");
    verify_addr(&mut state, addr(3), "US");
    mint_to(&mut state, addr(2), 1000);

    let freeze_args = serde_json::to_vec(&serde_json::json!({"addr": addr(2)})).unwrap();
    dispatch(&mut state, "freeze", &freeze_args, addr(1));

    let args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 100u64, "current_time": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "compliant_transfer", &args, addr(2));
}

#[test]
fn freeze_and_unfreeze_allows_transfer() {
    let mut state = init();
    verify_addr(&mut state, addr(2), "US");
    verify_addr(&mut state, addr(3), "US");
    mint_to(&mut state, addr(2), 1000);

    let freeze_args = serde_json::to_vec(&serde_json::json!({"addr": addr(2)})).unwrap();
    dispatch(&mut state, "freeze", &freeze_args, addr(1));
    let unfreeze_args = serde_json::to_vec(&serde_json::json!({"addr": addr(2)})).unwrap();
    dispatch(&mut state, "unfreeze", &unfreeze_args, addr(1));

    let args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 100u64, "current_time": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "compliant_transfer", &args, addr(2));
    assert_eq!(state.as_ref().unwrap().balance_of(&addr(3)), 100);
}

#[test]
#[should_panic(expected = "sender is not verified")]
fn transfer_by_unverified_sender_fails() {
    let mut state = init();
    verify_addr(&mut state, addr(3), "US");

    let args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 100u64, "current_time": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "compliant_transfer", &args, addr(2));
}

#[test]
#[should_panic(expected = "recipient is not verified")]
fn transfer_to_unverified_recipient_fails() {
    let mut state = init();
    verify_addr(&mut state, addr(2), "US");
    mint_to(&mut state, addr(2), 1000);

    let args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 100u64, "current_time": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "compliant_transfer", &args, addr(2));
}

#[test]
fn compliance_rule_require_credential() {
    let mut state = init();
    let rule_args = serde_json::to_vec(&serde_json::json!({
        "rule": {"RequireCredential": "KYC"}
    }))
    .unwrap();
    dispatch(&mut state, "add_compliance", &rule_args, addr(1));

    verify_addr(&mut state, addr(2), "US");
    verify_addr(&mut state, addr(3), "US");
    mint_to(&mut state, addr(2), 1000);

    // Transfer should work since addr(3) has KYC credential
    let args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 100u64, "current_time": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "compliant_transfer", &args, addr(2));
    assert_eq!(state.as_ref().unwrap().balance_of(&addr(3)), 100);
}

#[test]
fn is_verified_returns_correct_status() {
    let mut state = init();
    verify_addr(&mut state, addr(2), "US");

    let args = serde_json::to_vec(&serde_json::json!({"addr": addr(2)})).unwrap();
    let result = dispatch(&mut state, "is_verified", &args, addr(1));
    let verified: bool = serde_json::from_slice(&result).unwrap();
    assert!(verified);

    let args2 = serde_json::to_vec(&serde_json::json!({"addr": addr(99)})).unwrap();
    let result2 = dispatch(&mut state, "is_verified", &args2, addr(1));
    let verified2: bool = serde_json::from_slice(&result2).unwrap();
    assert!(!verified2);
}
