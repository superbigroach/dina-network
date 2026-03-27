use drc9_rental::{dispatch, RentalState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<RentalState> {
    let mut state: Option<RentalState> = None;
    let args = serde_json::to_vec(&serde_json::json!({"name": "Rental", "symbol": "RENT"})).unwrap();
    dispatch(&mut state, "init", &args, addr(1));
    state
}

fn mint(state: &mut Option<RentalState>, to: [u8; 32]) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "to": to,
        "metadata": {"name": "T", "description": "D", "attributes": {}}
    })).unwrap();
    let result = dispatch(state, "mint", &args, addr(1));
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn set_user_assigns_renter() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "user": addr(3), "expiry": 5000u64
    })).unwrap();
    dispatch(&mut state, "set_user", &args, addr(2));

    let user_args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "current_time": 3000u64
    })).unwrap();
    let result = dispatch(&mut state, "user_of", &user_args, addr(1));
    let user: Option<[u8; 32]> = serde_json::from_slice(&result).unwrap();
    assert_eq!(user, Some(addr(3)));
}

#[test]
fn user_of_returns_none_when_expired() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "user": addr(3), "expiry": 2000u64
    })).unwrap();
    dispatch(&mut state, "set_user", &args, addr(2));

    let user_args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "current_time": 3000u64
    })).unwrap();
    let result = dispatch(&mut state, "user_of", &user_args, addr(1));
    let user: Option<[u8; 32]> = serde_json::from_slice(&result).unwrap();
    assert_eq!(user, None);
}

#[test]
fn is_user_active_returns_true_before_expiry() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "user": addr(3), "expiry": 5000u64
    })).unwrap();
    dispatch(&mut state, "set_user", &args, addr(2));

    let active_args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "current_time": 3000u64
    })).unwrap();
    let result = dispatch(&mut state, "is_user_active", &active_args, addr(1));
    let active: bool = serde_json::from_slice(&result).unwrap();
    assert!(active);
}

#[test]
fn is_user_active_returns_false_after_expiry() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "user": addr(3), "expiry": 2000u64
    })).unwrap();
    dispatch(&mut state, "set_user", &args, addr(2));

    let active_args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "current_time": 3000u64
    })).unwrap();
    let result = dispatch(&mut state, "is_user_active", &active_args, addr(1));
    let active: bool = serde_json::from_slice(&result).unwrap();
    assert!(!active);
}

#[test]
fn transfer_clears_rental() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let set_args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "user": addr(3), "expiry": 5000u64
    })).unwrap();
    dispatch(&mut state, "set_user", &set_args, addr(2));

    let transfer_args = serde_json::to_vec(&serde_json::json!({
        "from": addr(2), "to": addr(4), "token_id": id
    })).unwrap();
    dispatch(&mut state, "transfer_from", &transfer_args, addr(2));

    let user_args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "current_time": 3000u64
    })).unwrap();
    let result = dispatch(&mut state, "user_of", &user_args, addr(1));
    let user: Option<[u8; 32]> = serde_json::from_slice(&result).unwrap();
    assert_eq!(user, None);
}

#[test]
#[should_panic(expected = "caller is not owner nor approved")]
fn set_user_by_non_owner_fails() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "user": addr(3), "expiry": 5000u64
    })).unwrap();
    dispatch(&mut state, "set_user", &args, addr(99));
}

#[test]
fn user_expires_returns_expiry_timestamp() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "user": addr(3), "expiry": 5000u64
    })).unwrap();
    dispatch(&mut state, "set_user", &args, addr(2));

    let exp_args = serde_json::to_vec(&serde_json::json!({"token_id": id})).unwrap();
    let result = dispatch(&mut state, "user_expires", &exp_args, addr(1));
    let exp: Option<u64> = serde_json::from_slice(&result).unwrap();
    assert_eq!(exp, Some(5000));
}
