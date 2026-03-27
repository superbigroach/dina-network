use drc40_warranty::{dispatch, ClaimStatus, WarrantyState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_warranty(admin: [u8; 32]) -> Option<WarrantyState> {
    let mut state: Option<WarrantyState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

fn create_test_warranty(
    state: &mut Option<WarrantyState>,
    manufacturer: [u8; 32],
    owner: [u8; 32],
) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": addr(10),
        "owner": owner,
        "purchase_date": 1000u64,
        "expiry_date": 5000u64,
        "coverage_type": "full",
        "max_claims": 3u64
    }))
    .unwrap();
    let result = dispatch(state, "create_warranty", &args, manufacturer);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn create_and_check_warranty() {
    let mfr = addr(1);
    let owner = addr(2);
    let mut state = init_warranty(mfr);
    let id = create_test_warranty(&mut state, mfr, owner);

    let s = state.as_ref().unwrap();
    let warranty = s.check_warranty(id).unwrap();
    assert_eq!(warranty.coverage_type, "full");
    assert_eq!(warranty.max_claims, 3);
    assert!(warranty.active);
}

#[test]
fn file_and_approve_claim() {
    let mfr = addr(1);
    let owner = addr(2);
    let mut state = init_warranty(mfr);
    let wid = create_test_warranty(&mut state, mfr, owner);

    let file_args = serde_json::to_vec(&serde_json::json!({
        "warranty_id": wid,
        "description": "Screen cracked",
        "filed_at": 2000u64
    }))
    .unwrap();
    let result = dispatch(&mut state, "file_claim", &file_args, owner);
    let claim_id: u64 = serde_json::from_slice(&result).unwrap();

    let approve_args = serde_json::to_vec(&serde_json::json!({
        "warranty_id": wid,
        "claim_id": claim_id,
        "resolution": "Replaced screen"
    }))
    .unwrap();
    dispatch(&mut state, "approve_claim", &approve_args, mfr);

    let s = state.as_ref().unwrap();
    let warranty = s.check_warranty(wid).unwrap();
    assert_eq!(warranty.claims[0].status, ClaimStatus::Approved);
    assert_eq!(warranty.claims[0].resolution, "Replaced screen");
}

#[test]
fn reject_claim() {
    let mfr = addr(1);
    let owner = addr(2);
    let mut state = init_warranty(mfr);
    let wid = create_test_warranty(&mut state, mfr, owner);

    let file_args = serde_json::to_vec(&serde_json::json!({
        "warranty_id": wid,
        "description": "Water damage",
        "filed_at": 2000u64
    }))
    .unwrap();
    let result = dispatch(&mut state, "file_claim", &file_args, owner);
    let claim_id: u64 = serde_json::from_slice(&result).unwrap();

    let reject_args = serde_json::to_vec(&serde_json::json!({
        "warranty_id": wid,
        "claim_id": claim_id,
        "reason": "Not covered - user negligence"
    }))
    .unwrap();
    dispatch(&mut state, "reject_claim", &reject_args, mfr);

    let s = state.as_ref().unwrap();
    assert_eq!(s.check_warranty(wid).unwrap().claims[0].status, ClaimStatus::Rejected);
}

#[test]
fn transfer_warranty_to_new_owner() {
    let mfr = addr(1);
    let owner = addr(2);
    let new_owner = addr(3);
    let mut state = init_warranty(mfr);
    let wid = create_test_warranty(&mut state, mfr, owner);

    let args = serde_json::to_vec(&serde_json::json!({
        "warranty_id": wid,
        "new_owner": new_owner
    }))
    .unwrap();
    dispatch(&mut state, "transfer_warranty", &args, owner);

    let s = state.as_ref().unwrap();
    assert_eq!(s.check_warranty(wid).unwrap().owner, new_owner);
}

#[test]
fn extend_warranty() {
    let mfr = addr(1);
    let owner = addr(2);
    let mut state = init_warranty(mfr);
    let wid = create_test_warranty(&mut state, mfr, owner);

    let args = serde_json::to_vec(&serde_json::json!({
        "warranty_id": wid,
        "new_expiry": 10000u64
    }))
    .unwrap();
    dispatch(&mut state, "extend_warranty", &args, mfr);

    let s = state.as_ref().unwrap();
    assert_eq!(s.check_warranty(wid).unwrap().expiry_date, 10000);
}

#[test]
#[should_panic(expected = "warranty expired")]
fn cannot_file_claim_after_expiry() {
    let mfr = addr(1);
    let owner = addr(2);
    let mut state = init_warranty(mfr);
    let wid = create_test_warranty(&mut state, mfr, owner);

    let args = serde_json::to_vec(&serde_json::json!({
        "warranty_id": wid,
        "description": "Late claim",
        "filed_at": 99999u64
    }))
    .unwrap();
    dispatch(&mut state, "file_claim", &args, owner);
}
