use drc39_loyalty::{dispatch, LoyaltyState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_loyalty(admin: [u8; 32]) -> Option<LoyaltyState> {
    let mut state: Option<LoyaltyState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

fn create_test_program(state: &mut Option<LoyaltyState>, business: [u8; 32]) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "name": "CoffeeRewards",
        "points_per_usdc": 10u64,
        "redemption_rate": 100u64
    }))
    .unwrap();
    let result = dispatch(state, "create_program", &args, business);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn create_program_and_get_info() {
    let business = addr(1);
    let mut state = init_loyalty(business);
    let id = create_test_program(&mut state, business);

    let s = state.as_ref().unwrap();
    let program = s.program_info(id).unwrap();
    assert_eq!(program.name, "CoffeeRewards");
    assert_eq!(program.points_per_usdc, 10);
    assert_eq!(program.redemption_rate, 100);
}

#[test]
fn earn_and_check_balance() {
    let business = addr(1);
    let customer = addr(2);
    let mut state = init_loyalty(business);
    let pid = create_test_program(&mut state, business);

    // Customer spends 50 USDC -> earns 500 points
    let args = serde_json::to_vec(&serde_json::json!({
        "program_id": pid,
        "customer": customer,
        "spend_amount": 50u64
    }))
    .unwrap();
    dispatch(&mut state, "earn_points", &args, business);

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance(pid, &customer), 500);
}

#[test]
fn redeem_points_returns_value() {
    let business = addr(1);
    let customer = addr(2);
    let mut state = init_loyalty(business);
    let pid = create_test_program(&mut state, business);

    // Earn 1000 points
    let earn_args = serde_json::to_vec(&serde_json::json!({
        "program_id": pid,
        "customer": customer,
        "spend_amount": 100u64
    }))
    .unwrap();
    dispatch(&mut state, "earn_points", &earn_args, business);

    // Redeem 500 points -> 5 USDC value (500 / 100)
    let redeem_args = serde_json::to_vec(&serde_json::json!({
        "program_id": pid,
        "points": 500u64
    }))
    .unwrap();
    let result = dispatch(&mut state, "redeem_points", &redeem_args, customer);
    let value: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(value, 5);

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance(pid, &customer), 500);
}

#[test]
fn transfer_points_between_customers() {
    let business = addr(1);
    let alice = addr(2);
    let bob = addr(3);
    let mut state = init_loyalty(business);
    let pid = create_test_program(&mut state, business);

    // Alice earns 200 points
    let earn_args = serde_json::to_vec(&serde_json::json!({
        "program_id": pid,
        "customer": alice,
        "spend_amount": 20u64
    }))
    .unwrap();
    dispatch(&mut state, "earn_points", &earn_args, business);

    // Alice transfers 100 to Bob
    let transfer_args = serde_json::to_vec(&serde_json::json!({
        "program_id": pid,
        "to": bob,
        "points": 100u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_points", &transfer_args, alice);

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance(pid, &alice), 100);
    assert_eq!(s.balance(pid, &bob), 100);
}

#[test]
#[should_panic(expected = "insufficient points")]
fn cannot_redeem_more_than_balance() {
    let business = addr(1);
    let customer = addr(2);
    let mut state = init_loyalty(business);
    let pid = create_test_program(&mut state, business);

    let earn_args = serde_json::to_vec(&serde_json::json!({
        "program_id": pid,
        "customer": customer,
        "spend_amount": 10u64
    }))
    .unwrap();
    dispatch(&mut state, "earn_points", &earn_args, business);

    let redeem_args = serde_json::to_vec(&serde_json::json!({
        "program_id": pid,
        "points": 9999u64
    }))
    .unwrap();
    dispatch(&mut state, "redeem_points", &redeem_args, customer);
}

#[test]
#[should_panic(expected = "only business can issue")]
fn non_business_cannot_earn_points() {
    let business = addr(1);
    let rando = addr(99);
    let customer = addr(2);
    let mut state = init_loyalty(business);
    let pid = create_test_program(&mut state, business);

    let args = serde_json::to_vec(&serde_json::json!({
        "program_id": pid,
        "customer": customer,
        "spend_amount": 100u64
    }))
    .unwrap();
    dispatch(&mut state, "earn_points", &args, rando);
}
