use drc103_service_agreement::{dispatch, ServiceAgreementState, AgreementStatus};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<ServiceAgreementState> {
    let mut state: Option<ServiceAgreementState> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn propose(state: &mut Option<ServiceAgreementState>, client: [u8; 32], provider: [u8; 32], amount: u64) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "terms": {
            "client": client,
            "provider": provider,
            "description": "Test service",
            "amount": amount,
            "deliverables": ["item1"],
            "deadline": 5000u64,
            "auto_confirm_after": 3600u64
        },
        "timestamp": 1000u64,
        "client_balance": 10000u64
    })).unwrap();
    let result = dispatch(state, "propose", &args, client);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn propose_creates_agreement() {
    let mut state = init();
    let id = propose(&mut state, addr(2), addr(3), 500);
    assert_eq!(id, 1);
    let ag = state.as_ref().unwrap().get_agreement(id).unwrap();
    assert_eq!(ag.status, AgreementStatus::Proposed);
}

#[test]
fn accept_transitions_to_active() {
    let mut state = init();
    let id = propose(&mut state, addr(2), addr(3), 500);
    let args = serde_json::to_vec(&serde_json::json!({"agreement_id": id})).unwrap();
    dispatch(&mut state, "accept", &args, addr(3));
    let ag = state.as_ref().unwrap().get_agreement(id).unwrap();
    assert_eq!(ag.status, AgreementStatus::Active);
}

#[test]
fn deliver_transitions_to_delivered() {
    let mut state = init();
    let id = propose(&mut state, addr(2), addr(3), 500);
    let accept = serde_json::to_vec(&serde_json::json!({"agreement_id": id})).unwrap();
    dispatch(&mut state, "accept", &accept, addr(3));
    let deliver = serde_json::to_vec(&serde_json::json!({
        "agreement_id": id, "proof": "done", "timestamp": 2000u64
    })).unwrap();
    dispatch(&mut state, "deliver", &deliver, addr(3));
    let ag = state.as_ref().unwrap().get_agreement(id).unwrap();
    assert_eq!(ag.status, AgreementStatus::Delivered);
}

#[test]
fn confirm_completes_and_returns_payout() {
    let mut state = init();
    let id = propose(&mut state, addr(2), addr(3), 500);
    let accept = serde_json::to_vec(&serde_json::json!({"agreement_id": id})).unwrap();
    dispatch(&mut state, "accept", &accept, addr(3));
    let deliver = serde_json::to_vec(&serde_json::json!({
        "agreement_id": id, "proof": "done", "timestamp": 2000u64
    })).unwrap();
    dispatch(&mut state, "deliver", &deliver, addr(3));
    let confirm = serde_json::to_vec(&serde_json::json!({
        "agreement_id": id, "timestamp": 3000u64
    })).unwrap();
    let result = dispatch(&mut state, "confirm", &confirm, addr(2));
    let res: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(res["payout"], 500);
}

#[test]
fn full_lifecycle_propose_accept_deliver_confirm() {
    let mut state = init();
    let id = propose(&mut state, addr(2), addr(3), 1000);

    dispatch(&mut state, "accept", &serde_json::to_vec(&serde_json::json!({"agreement_id": id})).unwrap(), addr(3));
    dispatch(&mut state, "deliver", &serde_json::to_vec(&serde_json::json!({"agreement_id": id, "proof": "proof", "timestamp": 2000u64})).unwrap(), addr(3));
    dispatch(&mut state, "confirm", &serde_json::to_vec(&serde_json::json!({"agreement_id": id, "timestamp": 3000u64})).unwrap(), addr(2));

    let ag = state.as_ref().unwrap().get_agreement(id).unwrap();
    assert_eq!(ag.status, AgreementStatus::Completed);
}

#[test]
#[should_panic(expected = "only the provider can accept")]
fn accept_by_non_provider_fails() {
    let mut state = init();
    let id = propose(&mut state, addr(2), addr(3), 500);
    let args = serde_json::to_vec(&serde_json::json!({"agreement_id": id})).unwrap();
    dispatch(&mut state, "accept", &args, addr(99));
}

#[test]
fn cancel_returns_escrow() {
    let mut state = init();
    let id = propose(&mut state, addr(2), addr(3), 500);
    let args = serde_json::to_vec(&serde_json::json!({"agreement_id": id})).unwrap();
    let result = dispatch(&mut state, "cancel", &args, addr(2));
    let refund: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(refund, 500);
}

#[test]
fn dispute_transitions_to_disputed() {
    let mut state = init();
    let id = propose(&mut state, addr(2), addr(3), 500);
    dispatch(&mut state, "accept", &serde_json::to_vec(&serde_json::json!({"agreement_id": id})).unwrap(), addr(3));
    let args = serde_json::to_vec(&serde_json::json!({
        "agreement_id": id, "reason": "bad quality"
    })).unwrap();
    dispatch(&mut state, "dispute", &args, addr(2));
    let ag = state.as_ref().unwrap().get_agreement(id).unwrap();
    assert_eq!(ag.status, AgreementStatus::Disputed);
}

#[test]
#[should_panic(expected = "only the client can propose")]
fn propose_by_non_client_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "terms": {
            "client": addr(2), "provider": addr(3),
            "description": "T", "amount": 100u64,
            "deliverables": ["x"], "deadline": 5000u64,
            "auto_confirm_after": 3600u64
        },
        "timestamp": 1000u64, "client_balance": 10000u64
    })).unwrap();
    dispatch(&mut state, "propose", &args, addr(99));
}
