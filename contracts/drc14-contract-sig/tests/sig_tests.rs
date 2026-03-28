use drc14_contract_sig::{dispatch, SignatureVerifier, INVALID, MAGIC_VALUE};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn zero_hash() -> [u8; 32] {
    [0u8; 32]
}

fn init() -> Option<SignatureVerifier> {
    let mut state: Option<SignatureVerifier> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

#[test]
fn is_valid_signature_returns_magic_for_authorized() {
    let mut state = init();
    let add_args = serde_json::to_vec(&serde_json::json!({
        "contract_addr": addr(10), "signer": addr(2)
    }))
    .unwrap();
    dispatch(&mut state, "add_signer", &add_args, addr(1));

    let check_args = serde_json::to_vec(&serde_json::json!({
        "hash": zero_hash(), "signer": addr(2)
    }))
    .unwrap();
    let result = dispatch(&mut state, "is_valid_signature", &check_args, addr(1));
    let val: u32 = serde_json::from_slice(&result).unwrap();
    assert_eq!(val, MAGIC_VALUE);
}

#[test]
fn is_valid_signature_returns_invalid_for_unknown() {
    let mut state = init();
    let check_args = serde_json::to_vec(&serde_json::json!({
        "hash": zero_hash(), "signer": addr(99)
    }))
    .unwrap();
    let result = dispatch(&mut state, "is_valid_signature", &check_args, addr(1));
    let val: u32 = serde_json::from_slice(&result).unwrap();
    assert_eq!(val, INVALID);
}

#[test]
fn add_and_remove_signer() {
    let mut state = init();
    let add_args = serde_json::to_vec(&serde_json::json!({
        "contract_addr": addr(10), "signer": addr(2)
    }))
    .unwrap();
    dispatch(&mut state, "add_signer", &add_args, addr(1));

    // Verify signer is authorized
    let s = state.as_ref().unwrap();
    assert_eq!(s.is_valid_signature([0u8; 32], addr(2)), MAGIC_VALUE);

    // Remove signer
    let rm_args = serde_json::to_vec(&serde_json::json!({
        "contract_addr": addr(10), "signer": addr(2)
    }))
    .unwrap();
    dispatch(&mut state, "remove_signer", &rm_args, addr(1));

    let s = state.as_ref().unwrap();
    assert_eq!(s.is_valid_signature([0u8; 32], addr(2)), INVALID);
}

#[test]
#[should_panic(expected = "only owner can add signers")]
fn add_signer_by_non_owner_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "contract_addr": addr(10), "signer": addr(2)
    }))
    .unwrap();
    dispatch(&mut state, "add_signer", &args, addr(99));
}

#[test]
#[should_panic(expected = "only owner can remove signers")]
fn remove_signer_by_non_owner_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "contract_addr": addr(10), "signer": addr(2)
    }))
    .unwrap();
    dispatch(&mut state, "remove_signer", &args, addr(99));
}

#[test]
fn signers_of_returns_list() {
    let mut state = init();
    let add1 = serde_json::to_vec(&serde_json::json!({
        "contract_addr": addr(10), "signer": addr(2)
    }))
    .unwrap();
    let add2 = serde_json::to_vec(&serde_json::json!({
        "contract_addr": addr(10), "signer": addr(3)
    }))
    .unwrap();
    dispatch(&mut state, "add_signer", &add1, addr(1));
    dispatch(&mut state, "add_signer", &add2, addr(1));

    let query = serde_json::to_vec(&serde_json::json!({"contract_addr": addr(10)})).unwrap();
    let result = dispatch(&mut state, "signers_of", &query, addr(1));
    let signers: Vec<[u8; 32]> = serde_json::from_slice(&result).unwrap();
    assert_eq!(signers.len(), 2);
}

#[test]
fn is_valid_signature_for_checks_specific_contract() {
    let mut state = init();
    let add_args = serde_json::to_vec(&serde_json::json!({
        "contract_addr": addr(10), "signer": addr(2)
    }))
    .unwrap();
    dispatch(&mut state, "add_signer", &add_args, addr(1));

    let check_args = serde_json::to_vec(&serde_json::json!({
        "hash": zero_hash(), "contract_addr": addr(10), "signer": addr(2)
    }))
    .unwrap();
    let result = dispatch(&mut state, "is_valid_signature_for", &check_args, addr(1));
    let val: u32 = serde_json::from_slice(&result).unwrap();
    assert_eq!(val, MAGIC_VALUE);

    // Different contract should return INVALID
    let check_args2 = serde_json::to_vec(&serde_json::json!({
        "hash": zero_hash(), "contract_addr": addr(20), "signer": addr(2)
    }))
    .unwrap();
    let result2 = dispatch(&mut state, "is_valid_signature_for", &check_args2, addr(1));
    let val2: u32 = serde_json::from_slice(&result2).unwrap();
    assert_eq!(val2, INVALID);
}
