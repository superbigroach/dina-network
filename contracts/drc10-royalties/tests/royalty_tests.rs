use drc10_royalties::{dispatch, RoyaltyRegistry};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<RoyaltyRegistry> {
    let mut state: Option<RoyaltyRegistry> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn register_and_set(state: &mut Option<RoyaltyRegistry>, creator: [u8; 32], token_id: u64, recipient: [u8; 32], bp: u16) {
    let reg_args = serde_json::to_vec(&serde_json::json!({"token_id": token_id})).unwrap();
    dispatch(state, "register_creator", &reg_args, creator);
    let set_args = serde_json::to_vec(&serde_json::json!({
        "token_id": token_id, "recipient": recipient, "basis_points": bp
    })).unwrap();
    dispatch(state, "set_royalty", &set_args, creator);
}

#[test]
fn set_royalty_stores_info() {
    let mut state = init();
    register_and_set(&mut state, addr(2), 1, addr(3), 500);
    let args = serde_json::to_vec(&serde_json::json!({"token_id": 1u64})).unwrap();
    let result = dispatch(&mut state, "get_royalty", &args, addr(1));
    let info: Option<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert!(info.is_some());
}

#[test]
fn royalty_info_calculates_correctly() {
    let mut state = init();
    register_and_set(&mut state, addr(2), 1, addr(3), 1000); // 10%
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": 1u64, "sale_price": 10000u64
    })).unwrap();
    let result = dispatch(&mut state, "royalty_info", &args, addr(1));
    let (recipient, amount): ([u8; 32], u64) = serde_json::from_slice(&result).unwrap();
    assert_eq!(recipient, addr(3));
    assert_eq!(amount, 1000); // 10% of 10000
}

#[test]
fn royalty_info_with_250_basis_points() {
    let mut state = init();
    register_and_set(&mut state, addr(2), 1, addr(3), 250); // 2.5%
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": 1u64, "sale_price": 20000u64
    })).unwrap();
    let result = dispatch(&mut state, "royalty_info", &args, addr(1));
    let (_recipient, amount): ([u8; 32], u64) = serde_json::from_slice(&result).unwrap();
    assert_eq!(amount, 500); // 2.5% of 20000
}

#[test]
fn royalty_zero_sale_price_returns_zero() {
    let mut state = init();
    register_and_set(&mut state, addr(2), 1, addr(3), 1000);
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": 1u64, "sale_price": 0u64
    })).unwrap();
    let result = dispatch(&mut state, "royalty_info", &args, addr(1));
    let (_recipient, amount): ([u8; 32], u64) = serde_json::from_slice(&result).unwrap();
    assert_eq!(amount, 0);
}

#[test]
#[should_panic(expected = "only creator can set royalty")]
fn set_royalty_by_non_creator_fails() {
    let mut state = init();
    let reg_args = serde_json::to_vec(&serde_json::json!({"token_id": 1u64})).unwrap();
    dispatch(&mut state, "register_creator", &reg_args, addr(2));
    let set_args = serde_json::to_vec(&serde_json::json!({
        "token_id": 1u64, "recipient": addr(3), "basis_points": 500u16
    })).unwrap();
    dispatch(&mut state, "set_royalty", &set_args, addr(99));
}

#[test]
#[should_panic(expected = "basis_points must be <= 10000")]
fn set_royalty_over_100_percent_fails() {
    let mut state = init();
    let reg_args = serde_json::to_vec(&serde_json::json!({"token_id": 1u64})).unwrap();
    dispatch(&mut state, "register_creator", &reg_args, addr(2));
    let set_args = serde_json::to_vec(&serde_json::json!({
        "token_id": 1u64, "recipient": addr(3), "basis_points": 10001u16
    })).unwrap();
    dispatch(&mut state, "set_royalty", &set_args, addr(2));
}

#[test]
#[should_panic(expected = "creator already registered")]
fn register_creator_twice_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"token_id": 1u64})).unwrap();
    dispatch(&mut state, "register_creator", &args, addr(2));
    dispatch(&mut state, "register_creator", &args, addr(2));
}
