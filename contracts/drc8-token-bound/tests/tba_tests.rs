use drc8_token_bound::{dispatch, TbaRegistry};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<TbaRegistry> {
    let mut state: Option<TbaRegistry> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn create_account(
    state: &mut Option<TbaRegistry>,
    caller: [u8; 32],
    nft_contract: [u8; 32],
    token_id: u64,
) -> [u8; 32] {
    let args = serde_json::to_vec(&serde_json::json!({
        "nft_contract": nft_contract,
        "token_id": token_id
    }))
    .unwrap();
    let result = dispatch(state, "create_account", &args, caller);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn create_account_returns_deterministic_address() {
    let mut state = init();
    let acct = create_account(&mut state, addr(2), addr(10), 1);
    assert_ne!(acct, [0u8; 32]);
}

#[test]
fn account_of_returns_created_account() {
    let mut state = init();
    let acct = create_account(&mut state, addr(2), addr(10), 1);
    let args = serde_json::to_vec(&serde_json::json!({
        "nft_contract": addr(10), "token_id": 1u64
    }))
    .unwrap();
    let result = dispatch(&mut state, "account_of", &args, addr(1));
    let found: Option<[u8; 32]> = serde_json::from_slice(&result).unwrap();
    assert_eq!(found, Some(acct));
}

#[test]
fn account_of_returns_none_for_unknown() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "nft_contract": addr(99), "token_id": 1u64
    }))
    .unwrap();
    let result = dispatch(&mut state, "account_of", &args, addr(1));
    let found: Option<[u8; 32]> = serde_json::from_slice(&result).unwrap();
    assert_eq!(found, None);
}

#[test]
#[should_panic(expected = "account already exists")]
fn create_duplicate_account_fails() {
    let mut state = init();
    create_account(&mut state, addr(2), addr(10), 1);
    create_account(&mut state, addr(2), addr(10), 1);
}

#[test]
fn deposit_and_execute_transfer() {
    let mut state = init();
    create_account(&mut state, addr(2), addr(10), 1);

    // Deposit
    let dep_args = serde_json::to_vec(&serde_json::json!({
        "nft_contract": addr(10), "token_id": 1u64,
        "asset_key": "USDC", "amount": 500u64
    }))
    .unwrap();
    dispatch(&mut state, "deposit", &dep_args, addr(5));

    // Check balance
    let bal_args = serde_json::to_vec(&serde_json::json!({
        "nft_contract": addr(10), "token_id": 1u64, "asset_key": "USDC"
    }))
    .unwrap();
    let result = dispatch(&mut state, "account_balance", &bal_args, addr(1));
    let bal: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(bal, 500);

    // Execute transfer
    let exec_args = serde_json::to_vec(&serde_json::json!({
        "nft_contract": addr(10), "token_id": 1u64,
        "target": addr(20), "asset_key": "USDC", "amount": 200u64
    }))
    .unwrap();
    dispatch(&mut state, "execute", &exec_args, addr(2));

    let result2 = dispatch(&mut state, "account_balance", &bal_args, addr(1));
    let bal2: u64 = serde_json::from_slice(&result2).unwrap();
    assert_eq!(bal2, 300);
}

#[test]
#[should_panic(expected = "only NFT owner can execute")]
fn execute_by_non_owner_fails() {
    let mut state = init();
    create_account(&mut state, addr(2), addr(10), 1);
    let dep_args = serde_json::to_vec(&serde_json::json!({
        "nft_contract": addr(10), "token_id": 1u64,
        "asset_key": "USDC", "amount": 500u64
    }))
    .unwrap();
    dispatch(&mut state, "deposit", &dep_args, addr(5));

    let exec_args = serde_json::to_vec(&serde_json::json!({
        "nft_contract": addr(10), "token_id": 1u64,
        "target": addr(20), "asset_key": "USDC", "amount": 100u64
    }))
    .unwrap();
    dispatch(&mut state, "execute", &exec_args, addr(99));
}

#[test]
#[should_panic(expected = "insufficient TBA balance")]
fn execute_with_insufficient_balance_fails() {
    let mut state = init();
    create_account(&mut state, addr(2), addr(10), 1);
    let exec_args = serde_json::to_vec(&serde_json::json!({
        "nft_contract": addr(10), "token_id": 1u64,
        "target": addr(20), "asset_key": "USDC", "amount": 100u64
    }))
    .unwrap();
    dispatch(&mut state, "execute", &exec_args, addr(2));
}

#[test]
fn is_token_bound_returns_true_for_created_account() {
    let mut state = init();
    let acct = create_account(&mut state, addr(2), addr(10), 1);
    let args = serde_json::to_vec(&serde_json::json!({"account": acct})).unwrap();
    let result = dispatch(&mut state, "is_token_bound", &args, addr(1));
    let bound: bool = serde_json::from_slice(&result).unwrap();
    assert!(bound);
}
