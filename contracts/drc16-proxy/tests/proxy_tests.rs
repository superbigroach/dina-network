use drc16_proxy::{dispatch, ProxyState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<ProxyState> {
    let mut state: Option<ProxyState> = None;
    let args = serde_json::to_vec(&serde_json::json!({"implementation": addr(10)})).unwrap();
    dispatch(&mut state, "init", &args, addr(1));
    state
}

#[test]
fn init_sets_implementation_and_admin() {
    let state = init();
    let s = state.as_ref().unwrap();
    assert_eq!(s.implementation, addr(10));
    assert_eq!(s.admin, addr(1));
}

#[test]
fn upgrade_to_changes_implementation() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"new_impl": addr(20)})).unwrap();
    dispatch(&mut state, "upgrade_to", &args, addr(1));

    let result = dispatch(&mut state, "implementation", b"", addr(1));
    let impl_addr: [u8; 32] = serde_json::from_slice(&result).unwrap();
    assert_eq!(impl_addr, addr(20));
}

#[test]
#[should_panic(expected = "only admin can upgrade")]
fn upgrade_by_non_admin_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"new_impl": addr(20)})).unwrap();
    dispatch(&mut state, "upgrade_to", &args, addr(99));
}

#[test]
#[should_panic(expected = "implementation cannot be zero address")]
fn upgrade_to_zero_address_fails() {
    let mut state = init();
    let zero: [u8; 32] = [0u8; 32];
    let args = serde_json::to_vec(&serde_json::json!({"new_impl": zero})).unwrap();
    dispatch(&mut state, "upgrade_to", &args, addr(1));
}

#[test]
fn change_admin_sets_pending() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"new_admin": addr(2)})).unwrap();
    dispatch(&mut state, "change_admin", &args, addr(1));
    assert_eq!(state.as_ref().unwrap().pending_admin, Some(addr(2)));
    // Admin hasn't changed yet
    assert_eq!(state.as_ref().unwrap().admin, addr(1));
}

#[test]
fn accept_admin_completes_transfer() {
    let mut state = init();
    let change_args = serde_json::to_vec(&serde_json::json!({"new_admin": addr(2)})).unwrap();
    dispatch(&mut state, "change_admin", &change_args, addr(1));
    dispatch(&mut state, "accept_admin", b"", addr(2));

    let s = state.as_ref().unwrap();
    assert_eq!(s.admin, addr(2));
    assert_eq!(s.pending_admin, None);
}

#[test]
#[should_panic(expected = "only pending admin can accept")]
fn accept_admin_by_wrong_address_fails() {
    let mut state = init();
    let change_args = serde_json::to_vec(&serde_json::json!({"new_admin": addr(2)})).unwrap();
    dispatch(&mut state, "change_admin", &change_args, addr(1));
    dispatch(&mut state, "accept_admin", b"", addr(99));
}

#[test]
#[should_panic(expected = "no pending admin transfer")]
fn accept_admin_without_pending_fails() {
    let mut state = init();
    dispatch(&mut state, "accept_admin", b"", addr(1));
}

#[test]
fn proxy_admin_query_returns_admin() {
    let mut state = init();
    let result = dispatch(&mut state, "proxy_admin", b"", addr(1));
    let admin: [u8; 32] = serde_json::from_slice(&result).unwrap();
    assert_eq!(admin, addr(1));
}
