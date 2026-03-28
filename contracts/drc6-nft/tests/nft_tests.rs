use drc6_nft::{dispatch, NftState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<NftState> {
    let mut state: Option<NftState> = None;
    let args = serde_json::to_vec(&serde_json::json!({
        "name": "TestNFT",
        "symbol": "TNFT"
    }))
    .unwrap();
    dispatch(&mut state, "init", &args, addr(1));
    state
}

fn mint(state: &mut Option<NftState>, to: [u8; 32]) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "to": to,
        "metadata": {"name": "Token", "description": "A test token", "attributes": {}}
    }))
    .unwrap();
    let result = dispatch(state, "mint", &args, addr(1));
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn mint_assigns_sequential_ids() {
    let mut state = init();
    let id1 = mint(&mut state, addr(2));
    let id2 = mint(&mut state, addr(2));
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn owner_of_returns_correct_owner() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({"token_id": id})).unwrap();
    let result = dispatch(&mut state, "owner_of", &args, addr(1));
    let owner: [u8; 32] = serde_json::from_slice(&result).unwrap();
    assert_eq!(owner, addr(2));
}

#[test]
fn transfer_from_changes_ownership() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "from": addr(2), "to": addr(3), "token_id": id
    }))
    .unwrap();
    dispatch(&mut state, "transfer_from", &args, addr(2));
    assert_eq!(state.as_ref().unwrap().owner_of(id), addr(3));
}

#[test]
fn approve_and_transfer_by_approved() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let approve_args = serde_json::to_vec(&serde_json::json!({
        "to": addr(3), "token_id": id
    }))
    .unwrap();
    dispatch(&mut state, "approve", &approve_args, addr(2));

    let transfer_args = serde_json::to_vec(&serde_json::json!({
        "from": addr(2), "to": addr(4), "token_id": id
    }))
    .unwrap();
    dispatch(&mut state, "transfer_from", &transfer_args, addr(3));
    assert_eq!(state.as_ref().unwrap().owner_of(id), addr(4));
}

#[test]
#[should_panic(expected = "caller is not owner nor approved")]
fn transfer_by_unauthorized_fails() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "from": addr(2), "to": addr(3), "token_id": id
    }))
    .unwrap();
    dispatch(&mut state, "transfer_from", &args, addr(5));
}

#[test]
fn burn_removes_token() {
    let mut state = init();
    let id = mint(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({"token_id": id})).unwrap();
    dispatch(&mut state, "burn", &args, addr(2));

    let supply_result = dispatch(&mut state, "total_supply", b"", addr(1));
    let supply: u64 = serde_json::from_slice(&supply_result).unwrap();
    assert_eq!(supply, 0);
}

#[test]
fn total_supply_increases_on_mint() {
    let mut state = init();
    mint(&mut state, addr(2));
    mint(&mut state, addr(3));
    let result = dispatch(&mut state, "total_supply", b"", addr(1));
    let supply: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(supply, 2);
}

#[test]
fn balance_of_tracks_correctly() {
    let mut state = init();
    mint(&mut state, addr(2));
    mint(&mut state, addr(2));
    mint(&mut state, addr(3));
    let args = serde_json::to_vec(&serde_json::json!({"owner": addr(2)})).unwrap();
    let result = dispatch(&mut state, "balance_of", &args, addr(1));
    let bal: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(bal, 2);
}

#[test]
#[should_panic(expected = "only minter can mint")]
fn mint_by_non_minter_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "to": addr(2),
        "metadata": {"name": "T", "description": "D", "attributes": {}}
    }))
    .unwrap();
    dispatch(&mut state, "mint", &args, addr(5));
}
