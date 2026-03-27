use drc5_soulbound::{dispatch, CredentialRegistry, CredentialStatus};
use std::collections::BTreeMap;

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<CredentialRegistry> {
    let mut state: Option<CredentialRegistry> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn authorize_and_issue(state: &mut Option<CredentialRegistry>, issuer: [u8; 32], holder: [u8; 32], cred_type: &str, expires_at: Option<u64>) -> u64 {
    // Owner authorizes issuer
    let auth_args = serde_json::to_vec(&serde_json::json!({
        "credential_type": cred_type,
        "issuer": issuer
    })).unwrap();
    dispatch(state, "add_authorized_issuer", &auth_args, addr(1));

    // Issuer issues credential
    let issue_args = serde_json::to_vec(&serde_json::json!({
        "holder": holder,
        "credential_type": cred_type,
        "data": {},
        "issued_at": 1000u64,
        "expires_at": expires_at
    })).unwrap();
    let result = dispatch(state, "issue", &issue_args, issuer);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn issue_credential_succeeds() {
    let mut state = init();
    let id = authorize_and_issue(&mut state, addr(2), addr(3), "KYC", None);
    assert_eq!(id, 1);
}

#[test]
fn has_credential_returns_true_for_valid_credential() {
    let mut state = init();
    authorize_and_issue(&mut state, addr(2), addr(3), "KYC", None);
    let args = serde_json::to_vec(&serde_json::json!({
        "holder": addr(3),
        "credential_type": "KYC"
    })).unwrap();
    let result = dispatch(&mut state, "has_credential", &args, addr(1));
    let has: bool = serde_json::from_slice(&result).unwrap();
    assert!(has);
}

#[test]
fn has_credential_returns_false_for_unknown() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "holder": addr(99),
        "credential_type": "KYC"
    })).unwrap();
    let result = dispatch(&mut state, "has_credential", &args, addr(1));
    let has: bool = serde_json::from_slice(&result).unwrap();
    assert!(!has);
}

#[test]
fn verify_returns_valid_for_active_credential() {
    let mut state = init();
    let id = authorize_and_issue(&mut state, addr(2), addr(3), "KYC", None);
    let args = serde_json::to_vec(&serde_json::json!({
        "credential_id": id,
        "current_time": 2000u64
    })).unwrap();
    let result = dispatch(&mut state, "verify", &args, addr(1));
    let status: CredentialStatus = serde_json::from_slice(&result).unwrap();
    assert_eq!(status, CredentialStatus::Valid);
}

#[test]
fn verify_returns_expired_for_expired_credential() {
    let mut state = init();
    let id = authorize_and_issue(&mut state, addr(2), addr(3), "KYC", Some(1500));
    let args = serde_json::to_vec(&serde_json::json!({
        "credential_id": id,
        "current_time": 2000u64
    })).unwrap();
    let result = dispatch(&mut state, "verify", &args, addr(1));
    let status: CredentialStatus = serde_json::from_slice(&result).unwrap();
    assert_eq!(status, CredentialStatus::Expired);
}

#[test]
fn revoke_makes_credential_revoked() {
    let mut state = init();
    let id = authorize_and_issue(&mut state, addr(2), addr(3), "KYC", None);
    let args = serde_json::to_vec(&serde_json::json!({"credential_id": id})).unwrap();
    dispatch(&mut state, "revoke", &args, addr(2));

    let verify_args = serde_json::to_vec(&serde_json::json!({
        "credential_id": id,
        "current_time": 2000u64
    })).unwrap();
    let result = dispatch(&mut state, "verify", &verify_args, addr(1));
    let status: CredentialStatus = serde_json::from_slice(&result).unwrap();
    assert_eq!(status, CredentialStatus::Revoked);
}

#[test]
fn has_credential_returns_false_after_revoke() {
    let mut state = init();
    authorize_and_issue(&mut state, addr(2), addr(3), "KYC", None);
    let revoke_args = serde_json::to_vec(&serde_json::json!({"credential_id": 1u64})).unwrap();
    dispatch(&mut state, "revoke", &revoke_args, addr(2));

    let args = serde_json::to_vec(&serde_json::json!({
        "holder": addr(3),
        "credential_type": "KYC"
    })).unwrap();
    let result = dispatch(&mut state, "has_credential", &args, addr(1));
    let has: bool = serde_json::from_slice(&result).unwrap();
    assert!(!has);
}

#[test]
fn verify_returns_not_found_for_nonexistent() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "credential_id": 999u64,
        "current_time": 2000u64
    })).unwrap();
    let result = dispatch(&mut state, "verify", &args, addr(1));
    let status: CredentialStatus = serde_json::from_slice(&result).unwrap();
    assert_eq!(status, CredentialStatus::NotFound);
}

#[test]
#[should_panic(expected = "not an authorized issuer")]
fn issue_by_unauthorized_issuer_fails() {
    let mut state = init();
    // Authorize addr(2) but try to issue as addr(3)
    let auth_args = serde_json::to_vec(&serde_json::json!({
        "credential_type": "KYC",
        "issuer": addr(2)
    })).unwrap();
    dispatch(&mut state, "add_authorized_issuer", &auth_args, addr(1));

    let issue_args = serde_json::to_vec(&serde_json::json!({
        "holder": addr(4),
        "credential_type": "KYC",
        "data": {},
        "issued_at": 1000u64,
        "expires_at": null
    })).unwrap();
    dispatch(&mut state, "issue", &issue_args, addr(3));
}
