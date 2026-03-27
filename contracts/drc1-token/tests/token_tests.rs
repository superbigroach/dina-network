use drc1_token::{dispatch, TokenState};

// ============================================================
// Helpers
// ============================================================

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_token(owner: [u8; 32]) -> Option<TokenState> {
    let mut state: Option<TokenState> = None;
    let args = serde_json::to_vec(&serde_json::json!({
        "name": "DinaToken",
        "symbol": "DINA",
        "decimals": 18
    }))
    .unwrap();
    dispatch(&mut state, "init", &args, owner);
    state
}

fn init_and_mint(owner: [u8; 32], amount: u64) -> Option<TokenState> {
    let mut state = init_token(owner);
    let mint_args = serde_json::to_vec(&serde_json::json!({
        "to": owner,
        "amount": amount
    }))
    .unwrap();
    dispatch(&mut state, "mint", &mint_args, owner);
    state
}

// ============================================================
// Initialize token
// ============================================================

#[test]
fn initialize_token_with_name_symbol_decimals_supply() {
    let owner = addr(1);
    let state = init_token(owner);
    let s = state.as_ref().unwrap();

    assert_eq!(s.name(), "DinaToken");
    assert_eq!(s.symbol(), "DINA");
    assert_eq!(s.decimals(), 18);
    assert_eq!(s.total_supply(), 0);
    assert_eq!(s.balance_of(&owner), 0);
}

// ============================================================
// balance_of
// ============================================================

#[test]
fn balance_of_returns_correct_balance_for_creator() {
    let owner = addr(1);
    let mut state = init_and_mint(owner, 1_000_000);
    let s = state.as_ref().unwrap();

    assert_eq!(s.balance_of(&owner), 1_000_000);
    assert_eq!(s.total_supply(), 1_000_000);
}

#[test]
fn balance_of_returns_zero_for_unknown_account() {
    let owner = addr(1);
    let state = init_token(owner);
    let s = state.as_ref().unwrap();

    let unknown = addr(99);
    assert_eq!(s.balance_of(&unknown), 0);
}

// ============================================================
// transfer
// ============================================================

#[test]
fn transfer_succeeds_with_sufficient_balance() {
    let owner = addr(1);
    let recipient = addr(2);
    let mut state = init_and_mint(owner, 1000);

    let args = serde_json::to_vec(&serde_json::json!({
        "to": recipient,
        "amount": 400u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer", &args, owner);

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance_of(&owner), 600);
    assert_eq!(s.balance_of(&recipient), 400);
    // Total supply unchanged
    assert_eq!(s.total_supply(), 1000);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn transfer_fails_with_insufficient_balance() {
    let owner = addr(1);
    let recipient = addr(2);
    let mut state = init_and_mint(owner, 100);

    let args = serde_json::to_vec(&serde_json::json!({
        "to": recipient,
        "amount": 200u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer", &args, owner);
}

// ============================================================
// approve / allowance
// ============================================================

#[test]
fn approve_and_allowance_roundtrip() {
    let owner = addr(1);
    let spender = addr(2);
    let mut state = init_token(owner);

    let args = serde_json::to_vec(&serde_json::json!({
        "spender": spender,
        "amount": 500u64
    }))
    .unwrap();
    dispatch(&mut state, "approve", &args, owner);

    // Query allowance via dispatch
    let q_args = serde_json::to_vec(&serde_json::json!({
        "owner": owner,
        "spender": spender
    }))
    .unwrap();
    let result = dispatch(&mut state, "allowance", &q_args, owner);
    let allowance: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(allowance, 500);
}

// ============================================================
// transfer_from
// ============================================================

#[test]
fn transfer_from_with_approval_works() {
    let owner = addr(1);
    let spender = addr(2);
    let recipient = addr(3);
    let mut state = init_and_mint(owner, 1000);

    // Approve spender for 500
    let approve_args = serde_json::to_vec(&serde_json::json!({
        "spender": spender,
        "amount": 500u64
    }))
    .unwrap();
    dispatch(&mut state, "approve", &approve_args, owner);

    // Spender calls transfer_from
    let tf_args = serde_json::to_vec(&serde_json::json!({
        "from": owner,
        "to": recipient,
        "amount": 300u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_from", &tf_args, spender);

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance_of(&owner), 700);
    assert_eq!(s.balance_of(&recipient), 300);
    assert_eq!(s.allowance(&owner, &spender), 200); // 500 - 300
}

#[test]
#[should_panic(expected = "allowance exceeded")]
fn transfer_from_without_approval_fails() {
    let owner = addr(1);
    let spender = addr(2);
    let recipient = addr(3);
    let mut state = init_and_mint(owner, 1000);

    // No approval given, try transfer_from
    let tf_args = serde_json::to_vec(&serde_json::json!({
        "from": owner,
        "to": recipient,
        "amount": 100u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_from", &tf_args, spender);
}

// ============================================================
// mint
// ============================================================

#[test]
fn mint_increases_supply_owner_only() {
    let owner = addr(1);
    let recipient = addr(2);
    let mut state = init_token(owner);

    let args = serde_json::to_vec(&serde_json::json!({
        "to": recipient,
        "amount": 5000u64
    }))
    .unwrap();
    dispatch(&mut state, "mint", &args, owner);

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance_of(&recipient), 5000);
    assert_eq!(s.total_supply(), 5000);
}

#[test]
#[should_panic(expected = "only owner can mint")]
fn mint_fails_for_non_owner() {
    let owner = addr(1);
    let non_owner = addr(2);
    let mut state = init_token(owner);

    let args = serde_json::to_vec(&serde_json::json!({
        "to": non_owner,
        "amount": 100u64
    }))
    .unwrap();
    dispatch(&mut state, "mint", &args, non_owner);
}
