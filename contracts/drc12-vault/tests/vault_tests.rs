use drc12_vault::{dispatch, VaultState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<VaultState> {
    let mut state: Option<VaultState> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn deposit(state: &mut Option<VaultState>, amount: u64, receiver: [u8; 32]) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "amount": amount, "receiver": receiver, "timestamp": 1000u64
    })).unwrap();
    let result = dispatch(state, "deposit", &args, addr(1));
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn deposit_mints_shares_one_to_one_initially() {
    let mut state = init();
    let shares = deposit(&mut state, 1000, addr(2));
    assert_eq!(shares, 1000);
}

#[test]
fn deposit_tracks_balance() {
    let mut state = init();
    deposit(&mut state, 1000, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({"owner": addr(2)})).unwrap();
    let result = dispatch(&mut state, "balance_of", &args, addr(1));
    let bal: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(bal, 1000);
}

#[test]
fn withdraw_burns_shares() {
    let mut state = init();
    deposit(&mut state, 1000, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "amount": 400u64, "receiver": addr(2), "owner": addr(2)
    })).unwrap();
    dispatch(&mut state, "withdraw", &args, addr(2));

    let bal_args = serde_json::to_vec(&serde_json::json!({"owner": addr(2)})).unwrap();
    let result = dispatch(&mut state, "balance_of", &bal_args, addr(1));
    let bal: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(bal, 600);
}

#[test]
#[should_panic(expected = "only owner can withdraw")]
fn withdraw_by_non_owner_fails() {
    let mut state = init();
    deposit(&mut state, 1000, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "amount": 100u64, "receiver": addr(2), "owner": addr(2)
    })).unwrap();
    dispatch(&mut state, "withdraw", &args, addr(99));
}

#[test]
fn preview_deposit_matches_actual() {
    let mut state = init();
    deposit(&mut state, 1000, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({"amount": 500u64})).unwrap();
    let result = dispatch(&mut state, "preview_deposit", &args, addr(1));
    let preview: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(preview, 500);
}

#[test]
fn add_yield_increases_share_value() {
    let mut state = init();
    deposit(&mut state, 1000, addr(2));
    // Add 500 yield
    let yield_args = serde_json::to_vec(&serde_json::json!({"amount": 500u64})).unwrap();
    dispatch(&mut state, "add_yield", &yield_args, addr(1));

    // Total assets now 1500, shares still 1000
    let total_args = b"";
    let result = dispatch(&mut state, "total_assets", total_args, addr(1));
    let total: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(total, 1500);

    // preview_redeem for 1000 shares should return 1500
    let redeem_args = serde_json::to_vec(&serde_json::json!({"shares": 1000u64})).unwrap();
    let result = dispatch(&mut state, "preview_redeem", &redeem_args, addr(1));
    let assets: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(assets, 1500);
}

#[test]
#[should_panic(expected = "only admin can add yield")]
fn add_yield_by_non_admin_fails() {
    let mut state = init();
    deposit(&mut state, 1000, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({"amount": 100u64})).unwrap();
    dispatch(&mut state, "add_yield", &args, addr(99));
}

#[test]
#[should_panic(expected = "insufficient shares")]
fn withdraw_more_than_available_fails() {
    let mut state = init();
    deposit(&mut state, 100, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "amount": 200u64, "receiver": addr(2), "owner": addr(2)
    })).unwrap();
    dispatch(&mut state, "withdraw", &args, addr(2));
}
