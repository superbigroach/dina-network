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
    }))
    .unwrap();
    let result = dispatch(state, "deposit", &args, addr(1));
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn deposit_mints_shares_one_to_one_initially() {
    let mut state = init();
    // First deposit: 10_000 minted, 1000 burned to dead address, receiver gets 9000
    let shares = deposit(&mut state, 10_000, addr(2));
    assert_eq!(shares, 9_000);
}

#[test]
fn deposit_tracks_balance() {
    let mut state = init();
    deposit(&mut state, 10_000, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({"owner": addr(2)})).unwrap();
    let result = dispatch(&mut state, "balance_of", &args, addr(1));
    let bal: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(bal, 9_000); // 10_000 - 1000 burned
}

#[test]
fn withdraw_burns_shares() {
    let mut state = init();
    deposit(&mut state, 10_000, addr(2)); // receiver gets 9000 shares
    let args = serde_json::to_vec(&serde_json::json!({
        "amount": 4000u64, "receiver": addr(2), "owner": addr(2)
    }))
    .unwrap();
    dispatch(&mut state, "withdraw", &args, addr(2));

    let bal_args = serde_json::to_vec(&serde_json::json!({"owner": addr(2)})).unwrap();
    let result = dispatch(&mut state, "balance_of", &bal_args, addr(1));
    let bal: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(bal, 5_000); // 9000 - 4000
}

#[test]
#[should_panic(expected = "only owner can withdraw")]
fn withdraw_by_non_owner_fails() {
    let mut state = init();
    deposit(&mut state, 10_000, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "amount": 100u64, "receiver": addr(2), "owner": addr(2)
    }))
    .unwrap();
    dispatch(&mut state, "withdraw", &args, addr(99));
}

#[test]
fn preview_deposit_matches_actual() {
    let mut state = init();
    deposit(&mut state, 10_000, addr(2)); // first deposit
                                          // Second deposit: preview should match actual (no burn on subsequent deposits)
    let args = serde_json::to_vec(&serde_json::json!({"amount": 5000u64})).unwrap();
    let result = dispatch(&mut state, "preview_deposit", &args, addr(1));
    let preview: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(preview, 5000);
}

#[test]
fn add_yield_increases_share_value() {
    let mut state = init();
    deposit(&mut state, 10_000, addr(2)); // 10_000 total shares, receiver gets 9000
                                          // Add 5000 yield
    let yield_args = serde_json::to_vec(&serde_json::json!({"amount": 5000u64})).unwrap();
    dispatch(&mut state, "add_yield", &yield_args, addr(1));

    // Total assets now 15_000, total shares 10_000
    let total_args = b"";
    let result = dispatch(&mut state, "total_assets", total_args, addr(1));
    let total: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(total, 15_000);

    // preview_redeem for 10_000 shares should return 15_000
    let redeem_args = serde_json::to_vec(&serde_json::json!({"shares": 10_000u64})).unwrap();
    let result = dispatch(&mut state, "preview_redeem", &redeem_args, addr(1));
    let assets: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(assets, 15_000);
}

#[test]
#[should_panic(expected = "only admin can add yield")]
fn add_yield_by_non_admin_fails() {
    let mut state = init();
    deposit(&mut state, 10_000, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({"amount": 100u64})).unwrap();
    dispatch(&mut state, "add_yield", &args, addr(99));
}

#[test]
#[should_panic(expected = "insufficient shares")]
fn withdraw_more_than_available_fails() {
    let mut state = init();
    deposit(&mut state, 10_000, addr(2)); // receiver gets 9000
    let args = serde_json::to_vec(&serde_json::json!({
        "amount": 10_000u64, "receiver": addr(2), "owner": addr(2)
    }))
    .unwrap();
    dispatch(&mut state, "withdraw", &args, addr(2));
}
