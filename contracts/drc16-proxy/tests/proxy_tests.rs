use drc16_proxy::dispatch;

const ADMIN: &str = "admin_address";
const ALICE: &str = "alice_address";
const CODE_V1: &[u8] = b"wasm_v1";
const CODE_V2: &[u8] = b"wasm_v2";
const DELAY: u64 = 3600;

fn deploy() -> Option<drc16_proxy::ProxyState> {
    let mut state = None;
    let args = serde_json::to_vec(&serde_json::json!({
        "admin": ADMIN,
        "implementation_code": CODE_V1.to_vec(),
        "upgrade_delay": DELAY,
    }))
    .unwrap();
    dispatch(&mut state, "deploy_proxy", &args, ADMIN);
    state
}

#[test]
fn deploy_sets_admin_and_version() {
    let state = deploy();
    let s = state.as_ref().unwrap();
    assert_eq!(s.admin, ADMIN);
    assert_eq!(s.version, 1);
    assert!(!s.paused);
}

#[test]
fn proxy_call_forwards() {
    let mut state = deploy();
    let args = serde_json::to_vec(&serde_json::json!({
        "method": "transfer",
        "args": [1, 2, 3],
    }))
    .unwrap();
    let result = dispatch(&mut state, "proxy_call", &args, ALICE);
    let s: String = serde_json::from_slice(&result).unwrap();
    assert_eq!(s, "forwarded:transfer");
}

#[test]
fn upgrade_lifecycle() {
    let mut state = deploy();
    // Propose
    let args = serde_json::to_vec(&serde_json::json!({
        "new_code": CODE_V2.to_vec(),
        "current_time": 1000u64,
    }))
    .unwrap();
    dispatch(&mut state, "propose_upgrade", &args, ADMIN);
    assert!(state.as_ref().unwrap().pending_upgrade.is_some());

    // Execute
    let args = serde_json::to_vec(&serde_json::json!({
        "current_time": 1000u64 + DELAY,
    }))
    .unwrap();
    dispatch(&mut state, "execute_upgrade", &args, ADMIN);
    assert_eq!(state.as_ref().unwrap().version, 2);
}

#[test]
#[should_panic(expected = "only admin can propose upgrade")]
fn non_admin_cannot_propose() {
    let mut state = deploy();
    let args = serde_json::to_vec(&serde_json::json!({
        "new_code": CODE_V2.to_vec(),
        "current_time": 1000u64,
    }))
    .unwrap();
    dispatch(&mut state, "propose_upgrade", &args, ALICE);
}

#[test]
fn transfer_admin() {
    let mut state = deploy();
    let args = serde_json::to_vec(&serde_json::json!({
        "new_admin": ALICE,
    }))
    .unwrap();
    dispatch(&mut state, "transfer_admin", &args, ADMIN);
    assert_eq!(state.as_ref().unwrap().admin, ALICE);
}

#[test]
fn get_admin_query() {
    let mut state = deploy();
    let result = dispatch(&mut state, "get_admin", b"", ADMIN);
    let admin: String = serde_json::from_slice(&result).unwrap();
    assert_eq!(admin, ADMIN);
}
