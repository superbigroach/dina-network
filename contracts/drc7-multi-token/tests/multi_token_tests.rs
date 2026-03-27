use drc7_multi_token::{dispatch, MultiTokenState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<MultiTokenState> {
    let mut state: Option<MultiTokenState> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn mint(state: &mut Option<MultiTokenState>, to: [u8; 32], token_id: u64, amount: u64) {
    let args = serde_json::to_vec(&serde_json::json!({
        "to": to, "token_id": token_id, "amount": amount, "uri": null
    })).unwrap();
    dispatch(state, "mint", &args, addr(1));
}

#[test]
fn mint_creates_balance() {
    let mut state = init();
    mint(&mut state, addr(2), 1, 100);
    let args = serde_json::to_vec(&serde_json::json!({"account": addr(2), "token_id": 1u64})).unwrap();
    let result = dispatch(&mut state, "balance_of", &args, addr(1));
    let bal: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(bal, 100);
}

#[test]
fn transfer_moves_tokens() {
    let mut state = init();
    mint(&mut state, addr(2), 1, 100);
    let args = serde_json::to_vec(&serde_json::json!({
        "from": addr(2), "to": addr(3), "token_id": 1u64, "amount": 40u64
    })).unwrap();
    dispatch(&mut state, "transfer", &args, addr(2));

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance_of(&addr(2), 1), 60);
    assert_eq!(s.balance_of(&addr(3), 1), 40);
}

#[test]
fn batch_transfer_moves_multiple_tokens() {
    let mut state = init();
    mint(&mut state, addr(2), 1, 100);
    mint(&mut state, addr(2), 2, 200);
    let args = serde_json::to_vec(&serde_json::json!({
        "from": addr(2), "to": addr(3),
        "token_ids": [1u64, 2u64],
        "amounts": [10u64, 20u64]
    })).unwrap();
    dispatch(&mut state, "batch_transfer", &args, addr(2));

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance_of(&addr(2), 1), 90);
    assert_eq!(s.balance_of(&addr(3), 2), 20);
}

#[test]
fn balance_of_batch_returns_correct_values() {
    let mut state = init();
    mint(&mut state, addr(2), 1, 100);
    mint(&mut state, addr(3), 2, 200);
    let args = serde_json::to_vec(&serde_json::json!({
        "accounts": [addr(2), addr(3)],
        "token_ids": [1u64, 2u64]
    })).unwrap();
    let result = dispatch(&mut state, "balance_of_batch", &args, addr(1));
    let bals: Vec<u64> = serde_json::from_slice(&result).unwrap();
    assert_eq!(bals, vec![100, 200]);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn transfer_with_insufficient_balance_fails() {
    let mut state = init();
    mint(&mut state, addr(2), 1, 10);
    let args = serde_json::to_vec(&serde_json::json!({
        "from": addr(2), "to": addr(3), "token_id": 1u64, "amount": 50u64
    })).unwrap();
    dispatch(&mut state, "transfer", &args, addr(2));
}

#[test]
fn set_approval_for_all_allows_transfer() {
    let mut state = init();
    mint(&mut state, addr(2), 1, 100);
    let approval_args = serde_json::to_vec(&serde_json::json!({
        "operator": addr(5), "approved": true
    })).unwrap();
    dispatch(&mut state, "set_approval_for_all", &approval_args, addr(2));

    let transfer_args = serde_json::to_vec(&serde_json::json!({
        "from": addr(2), "to": addr(3), "token_id": 1u64, "amount": 10u64
    })).unwrap();
    dispatch(&mut state, "transfer", &transfer_args, addr(5));
    assert_eq!(state.as_ref().unwrap().balance_of(&addr(3), 1), 10);
}

#[test]
fn mint_with_uri_sets_uri() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "to": addr(2), "token_id": 1u64, "amount": 10u64, "uri": "ipfs://abc"
    })).unwrap();
    dispatch(&mut state, "mint", &args, addr(1));

    let uri_args = serde_json::to_vec(&serde_json::json!({"token_id": 1u64})).unwrap();
    let result = dispatch(&mut state, "uri", &uri_args, addr(1));
    let uri: Option<String> = serde_json::from_slice(&result).unwrap();
    assert_eq!(uri, Some("ipfs://abc".to_string()));
}
