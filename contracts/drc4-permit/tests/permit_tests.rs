use drc4_permit::{dispatch, PermitRegistry};
use sha2::{Digest, Sha256};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<PermitRegistry> {
    let mut state: Option<PermitRegistry> = None;
    let args = serde_json::to_vec(&serde_json::json!({
        "token_contract": addr(100),
        "chain_id": 1u64
    })).unwrap();
    dispatch(&mut state, "init", &args, addr(1));
    state
}

fn make_signature(state: &PermitRegistry, owner: &[u8; 32], spender: &[u8; 32], amount: u64, nonce: u64, deadline: u64) -> Vec<u8> {
    let digest = state.build_permit_digest(owner, spender, amount, nonce, deadline);
    Sha256::digest(digest).to_vec()
}

fn do_permit(state: &mut Option<PermitRegistry>, owner: [u8; 32], spender: [u8; 32], amount: u64, deadline: u64, current_time: u64, sig: Vec<u8>) {
    let args = serde_json::to_vec(&serde_json::json!({
        "owner": owner,
        "spender": spender,
        "amount": amount,
        "deadline": deadline,
        "current_time": current_time,
        "signature_bytes": sig
    })).unwrap();
    dispatch(state, "permit", &args, addr(1));
}

// ============================================================

#[test]
fn permit_with_valid_signature_sets_allowance() {
    let mut state = init();
    let owner = addr(2);
    let spender = addr(3);
    let sig = make_signature(state.as_ref().unwrap(), &owner, &spender, 500, 0, 2000);
    do_permit(&mut state, owner, spender, 500, 2000, 1000, sig);

    let args = serde_json::to_vec(&serde_json::json!({"owner": owner, "spender": spender})).unwrap();
    let result = dispatch(&mut state, "allowance", &args, addr(1));
    let allowance: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(allowance, 500);
}

#[test]
fn permit_increments_nonce() {
    let mut state = init();
    let owner = addr(2);
    let spender = addr(3);
    let sig = make_signature(state.as_ref().unwrap(), &owner, &spender, 500, 0, 2000);
    do_permit(&mut state, owner, spender, 500, 2000, 1000, sig);

    let args = serde_json::to_vec(&serde_json::json!({"owner": owner})).unwrap();
    let result = dispatch(&mut state, "nonces", &args, addr(1));
    let nonce: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(nonce, 1);
}

#[test]
#[should_panic(expected = "permit expired")]
fn permit_with_expired_deadline_fails() {
    let mut state = init();
    let owner = addr(2);
    let spender = addr(3);
    let sig = make_signature(state.as_ref().unwrap(), &owner, &spender, 500, 0, 500);
    do_permit(&mut state, owner, spender, 500, 500, 1000, sig);
}

#[test]
#[should_panic(expected = "invalid permit signature")]
fn permit_with_wrong_signature_fails() {
    let mut state = init();
    let owner = addr(2);
    let spender = addr(3);
    let bad_sig = vec![0u8; 32];
    do_permit(&mut state, owner, spender, 500, 2000, 1000, bad_sig);
}

#[test]
#[should_panic(expected = "invalid permit signature")]
fn permit_replay_with_old_nonce_signature_fails() {
    let mut state = init();
    let owner = addr(2);
    let spender = addr(3);
    // First permit uses nonce 0
    let sig = make_signature(state.as_ref().unwrap(), &owner, &spender, 500, 0, 2000);
    do_permit(&mut state, owner, spender, 500, 2000, 1000, sig);

    // Nonce is now 1. Build sig with old nonce 0 -- the digest won't match
    // the expected nonce (1), so the signature check will fail.
    let old_sig = {
        let s = state.as_ref().unwrap();
        let digest = s.build_permit_digest(&owner, &spender, 500, 0, 2000);
        Sha256::digest(digest).to_vec()
    };
    do_permit(&mut state, owner, spender, 500, 2000, 1000, old_sig);
}

#[test]
fn domain_separator_is_deterministic() {
    let mut state = init();
    let args = b"";
    let r1 = dispatch(&mut state, "domain_separator", args, addr(1));
    let r2 = dispatch(&mut state, "domain_separator", args, addr(1));
    assert_eq!(r1, r2);
}

#[test]
fn nonce_starts_at_zero() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"owner": addr(99)})).unwrap();
    let result = dispatch(&mut state, "nonces", &args, addr(1));
    let nonce: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(nonce, 0);
}
