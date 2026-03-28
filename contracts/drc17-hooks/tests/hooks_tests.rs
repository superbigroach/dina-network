use drc17_hooks::{dispatch, HookCall, TokenWithHooks};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<TokenWithHooks> {
    let mut state: Option<TokenWithHooks> = None;
    let args = serde_json::to_vec(&serde_json::json!({
        "name": "HookToken", "symbol": "HT", "decimals": 18u8
    }))
    .unwrap();
    dispatch(&mut state, "init", &args, addr(1));
    state
}

fn mint(state: &mut Option<TokenWithHooks>, to: [u8; 32], amount: u64) {
    let args = serde_json::to_vec(&serde_json::json!({"to": to, "amount": amount})).unwrap();
    dispatch(state, "mint", &args, addr(1));
}

#[test]
fn send_transfers_tokens() {
    let mut state = init();
    mint(&mut state, addr(2), 1000);
    let args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 300u64, "data": []
    }))
    .unwrap();
    dispatch(&mut state, "send", &args, addr(2));

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance_of(&addr(2)), 700);
    assert_eq!(s.balance_of(&addr(3)), 300);
}

#[test]
fn send_with_receive_hook_queues_hook_call() {
    let mut state = init();
    mint(&mut state, addr(2), 1000);

    // Register receive hook for addr(3)
    let reg_args = serde_json::to_vec(&serde_json::json!({"hook_contract": addr(50)})).unwrap();
    dispatch(&mut state, "register_receive_hook", &reg_args, addr(3));

    let send_args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 100u64, "data": []
    }))
    .unwrap();
    let result = dispatch(&mut state, "send", &send_args, addr(2));
    let hooks: Vec<HookCall> = serde_json::from_slice(&result).unwrap();
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].to, addr(50));
}

#[test]
fn send_with_send_hook_queues_hook_call() {
    let mut state = init();
    mint(&mut state, addr(2), 1000);

    // Register send hook for addr(2)
    let reg_args = serde_json::to_vec(&serde_json::json!({"hook_contract": addr(60)})).unwrap();
    dispatch(&mut state, "register_send_hook", &reg_args, addr(2));

    let send_args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 100u64, "data": []
    }))
    .unwrap();
    let result = dispatch(&mut state, "send", &send_args, addr(2));
    let hooks: Vec<HookCall> = serde_json::from_slice(&result).unwrap();
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].to, addr(60));
}

#[test]
fn send_without_hooks_returns_empty() {
    let mut state = init();
    mint(&mut state, addr(2), 1000);
    let send_args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 100u64, "data": []
    }))
    .unwrap();
    let result = dispatch(&mut state, "send", &send_args, addr(2));
    let hooks: Vec<HookCall> = serde_json::from_slice(&result).unwrap();
    assert_eq!(hooks.len(), 0);
}

#[test]
fn register_both_hooks_queues_two_calls() {
    let mut state = init();
    mint(&mut state, addr(2), 1000);

    let send_hook = serde_json::to_vec(&serde_json::json!({"hook_contract": addr(60)})).unwrap();
    dispatch(&mut state, "register_send_hook", &send_hook, addr(2));
    let recv_hook = serde_json::to_vec(&serde_json::json!({"hook_contract": addr(70)})).unwrap();
    dispatch(&mut state, "register_receive_hook", &recv_hook, addr(3));

    let send_args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 100u64, "data": []
    }))
    .unwrap();
    let result = dispatch(&mut state, "send", &send_args, addr(2));
    let hooks: Vec<HookCall> = serde_json::from_slice(&result).unwrap();
    assert_eq!(hooks.len(), 2);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn send_with_insufficient_balance_fails() {
    let mut state = init();
    mint(&mut state, addr(2), 100);
    let args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "amount": 200u64, "data": []
    }))
    .unwrap();
    dispatch(&mut state, "send", &args, addr(2));
}

#[test]
#[should_panic(expected = "only owner can mint")]
fn mint_by_non_owner_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"to": addr(2), "amount": 100u64})).unwrap();
    dispatch(&mut state, "mint", &args, addr(99));
}
