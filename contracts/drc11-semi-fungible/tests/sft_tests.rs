use drc11_semi_fungible::{dispatch, SftState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<SftState> {
    let mut state: Option<SftState> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn mint(state: &mut Option<SftState>, to: [u8; 32], slot: u64, value: u64) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "to": to, "slot": slot, "value": value, "metadata": "test"
    }))
    .unwrap();
    let result = dispatch(state, "mint", &args, addr(1));
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn mint_assigns_sequential_ids() {
    let mut state = init();
    let id1 = mint(&mut state, addr(2), 1, 100);
    let id2 = mint(&mut state, addr(2), 1, 200);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn value_of_returns_correct_value() {
    let mut state = init();
    let id = mint(&mut state, addr(2), 1, 500);
    let args = serde_json::to_vec(&serde_json::json!({"token_id": id})).unwrap();
    let result = dispatch(&mut state, "value_of", &args, addr(1));
    let val: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(val, 500);
}

#[test]
fn transfer_value_same_slot_works() {
    let mut state = init();
    let id1 = mint(&mut state, addr(2), 1, 100);
    let id2 = mint(&mut state, addr(3), 1, 50);
    let args = serde_json::to_vec(&serde_json::json!({
        "from_token": id1, "to_token": id2, "amount": 30u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_value", &args, addr(2));

    let s = state.as_ref().unwrap();
    assert_eq!(s.value_of(id1), 70);
    assert_eq!(s.value_of(id2), 80);
}

#[test]
#[should_panic(expected = "tokens must be in the same slot")]
fn transfer_value_different_slot_fails() {
    let mut state = init();
    let id1 = mint(&mut state, addr(2), 1, 100);
    let id2 = mint(&mut state, addr(3), 2, 50);
    let args = serde_json::to_vec(&serde_json::json!({
        "from_token": id1, "to_token": id2, "amount": 10u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_value", &args, addr(2));
}

#[test]
fn transfer_value_to_creates_new_token() {
    let mut state = init();
    let id1 = mint(&mut state, addr(2), 1, 100);
    let args = serde_json::to_vec(&serde_json::json!({
        "from_token": id1, "to_address": addr(3), "amount": 40u64
    }))
    .unwrap();
    let result = dispatch(&mut state, "transfer_value_to", &args, addr(2));
    let new_id: u64 = serde_json::from_slice(&result).unwrap();

    let s = state.as_ref().unwrap();
    assert_eq!(s.value_of(id1), 60);
    assert_eq!(s.value_of(new_id), 40);
    assert_eq!(s.slot_of(new_id), 1);
    assert_eq!(s.owner_of(new_id), addr(3));
}

#[test]
#[should_panic(expected = "insufficient value")]
fn transfer_value_exceeding_balance_fails() {
    let mut state = init();
    let id1 = mint(&mut state, addr(2), 1, 10);
    let id2 = mint(&mut state, addr(3), 1, 50);
    let args = serde_json::to_vec(&serde_json::json!({
        "from_token": id1, "to_token": id2, "amount": 20u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_value", &args, addr(2));
}

#[test]
#[should_panic(expected = "caller is not owner")]
fn transfer_value_by_non_owner_fails() {
    let mut state = init();
    let id1 = mint(&mut state, addr(2), 1, 100);
    let id2 = mint(&mut state, addr(3), 1, 50);
    let args = serde_json::to_vec(&serde_json::json!({
        "from_token": id1, "to_token": id2, "amount": 10u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_value", &args, addr(99));
}

#[test]
fn burn_removes_token() {
    let mut state = init();
    let id = mint(&mut state, addr(2), 1, 100);
    let args = serde_json::to_vec(&serde_json::json!({"token_id": id})).unwrap();
    dispatch(&mut state, "burn", &args, addr(2));
    assert!(!state.as_ref().unwrap().tokens.contains_key(&id));
}
