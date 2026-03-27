use drc15_meta_tx::{dispatch, MetaTxRegistry};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<MetaTxRegistry> {
    let mut state: Option<MetaTxRegistry> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn register_forwarder(state: &mut Option<MetaTxRegistry>, forwarder: [u8; 32]) {
    let args = serde_json::to_vec(&serde_json::json!({
        "addr": forwarder, "name": "TestForwarder", "timestamp": 1000u64
    })).unwrap();
    dispatch(state, "register_forwarder", &args, addr(1));
}

fn make_sig() -> Vec<u8> {
    vec![0u8; 64]
}

#[test]
fn register_forwarder_makes_trusted() {
    let mut state = init();
    register_forwarder(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({"addr": addr(2)})).unwrap();
    let result = dispatch(&mut state, "is_trusted_forwarder", &args, addr(1));
    let trusted: bool = serde_json::from_slice(&result).unwrap();
    assert!(trusted);
}

#[test]
fn unregistered_forwarder_is_not_trusted() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"addr": addr(99)})).unwrap();
    let result = dispatch(&mut state, "is_trusted_forwarder", &args, addr(1));
    let trusted: bool = serde_json::from_slice(&result).unwrap();
    assert!(!trusted);
}

#[test]
fn verify_and_execute_succeeds_with_trusted_forwarder() {
    let mut state = init();
    register_forwarder(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "request": {
            "from": addr(3),
            "to": addr(4),
            "value": 100u64,
            "nonce": 0u64,
            "data": [],
            "signature": make_sig()
        }
    })).unwrap();
    let result = dispatch(&mut state, "verify_and_execute", &args, addr(2));
    let success: bool = serde_json::from_slice(&result).unwrap();
    assert!(success);
}

#[test]
fn verify_and_execute_increments_nonce() {
    let mut state = init();
    register_forwarder(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "request": {
            "from": addr(3), "to": addr(4), "value": 100u64,
            "nonce": 0u64, "data": [], "signature": make_sig()
        }
    })).unwrap();
    dispatch(&mut state, "verify_and_execute", &args, addr(2));

    let nonce_args = serde_json::to_vec(&serde_json::json!({"addr": addr(3)})).unwrap();
    let result = dispatch(&mut state, "get_nonce", &nonce_args, addr(1));
    let nonce: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(nonce, 1);
}

#[test]
#[should_panic(expected = "not a trusted forwarder")]
fn verify_and_execute_by_untrusted_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "request": {
            "from": addr(3), "to": addr(4), "value": 100u64,
            "nonce": 0u64, "data": [], "signature": make_sig()
        }
    })).unwrap();
    dispatch(&mut state, "verify_and_execute", &args, addr(99));
}

#[test]
#[should_panic(expected = "invalid nonce")]
fn verify_and_execute_with_wrong_nonce_fails() {
    let mut state = init();
    register_forwarder(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "request": {
            "from": addr(3), "to": addr(4), "value": 100u64,
            "nonce": 5u64, "data": [], "signature": make_sig()
        }
    })).unwrap();
    dispatch(&mut state, "verify_and_execute", &args, addr(2));
}

#[test]
#[should_panic(expected = "signature must be at least 64 bytes")]
fn verify_and_execute_with_short_signature_fails() {
    let mut state = init();
    register_forwarder(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "request": {
            "from": addr(3), "to": addr(4), "value": 100u64,
            "nonce": 0u64, "data": [], "signature": vec![0u8; 10]
        }
    })).unwrap();
    dispatch(&mut state, "verify_and_execute", &args, addr(2));
}

#[test]
fn remove_forwarder_revokes_trust() {
    let mut state = init();
    register_forwarder(&mut state, addr(2));
    let rm_args = serde_json::to_vec(&serde_json::json!({"addr": addr(2)})).unwrap();
    dispatch(&mut state, "remove_forwarder", &rm_args, addr(1));

    let args = serde_json::to_vec(&serde_json::json!({"addr": addr(2)})).unwrap();
    let result = dispatch(&mut state, "is_trusted_forwarder", &args, addr(1));
    let trusted: bool = serde_json::from_slice(&result).unwrap();
    assert!(!trusted);
}
