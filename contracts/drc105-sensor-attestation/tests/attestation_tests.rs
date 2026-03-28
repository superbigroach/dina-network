use drc105_sensor_attestation::{dispatch, SensorAttestationState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<SensorAttestationState> {
    let mut state: Option<SensorAttestationState> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn make_reading(device_id: [u8; 32], timestamp: u64) -> serde_json::Value {
    let zero: [u8; 32] = [0u8; 32];
    serde_json::json!({
        "device_id": device_id,
        "sensor_type": "temperature",
        "value": {"Float": 25.5},
        "timestamp": timestamp,
        "witness_hash": zero,
        "device_signature": [1, 2, 3]
    })
}

fn attest(
    state: &mut Option<SensorAttestationState>,
    caller: [u8; 32],
    device_id: [u8; 32],
    timestamp: u64,
) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "reading": make_reading(device_id, timestamp),
        "timestamp": timestamp
    }))
    .unwrap();
    let result = dispatch(state, "attest", &args, caller);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn attest_returns_id() {
    let mut state = init();
    let id = attest(&mut state, addr(2), addr(10), 1000);
    assert_eq!(id, 1);
}

#[test]
fn verify_attestation_marks_as_verified() {
    let mut state = init();
    let id = attest(&mut state, addr(2), addr(10), 1000);
    let args = serde_json::to_vec(&serde_json::json!({
        "attestation_id": id, "timestamp": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "verify", &args, addr(3));

    let att = state.as_ref().unwrap().get_attestation(id).unwrap();
    assert!(att.verified);
    assert_eq!(att.verifier, Some(addr(3)));
}

#[test]
#[should_panic(expected = "attester cannot self-verify")]
fn self_verify_fails() {
    let mut state = init();
    let id = attest(&mut state, addr(2), addr(10), 1000);
    let args = serde_json::to_vec(&serde_json::json!({
        "attestation_id": id, "timestamp": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "verify", &args, addr(2));
}

#[test]
#[should_panic(expected = "attestation already verified")]
fn double_verify_fails() {
    let mut state = init();
    let id = attest(&mut state, addr(2), addr(10), 1000);
    let args = serde_json::to_vec(&serde_json::json!({
        "attestation_id": id, "timestamp": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "verify", &args, addr(3));
    dispatch(&mut state, "verify", &args, addr(4));
}

#[test]
fn attestations_of_returns_device_attestations() {
    let mut state = init();
    attest(&mut state, addr(2), addr(10), 1000);
    attest(&mut state, addr(2), addr(10), 2000);
    attest(&mut state, addr(2), addr(11), 3000);

    let args = serde_json::to_vec(&serde_json::json!({"device_id": addr(10)})).unwrap();
    let result = dispatch(&mut state, "attestations_of", &args, addr(1));
    let atts: Vec<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert_eq!(atts.len(), 2);
}

#[test]
fn get_attestation_returns_none_for_unknown() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"attestation_id": 999u64})).unwrap();
    let result = dispatch(&mut state, "get_attestation", &args, addr(1));
    let att: Option<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert!(att.is_none());
}

#[test]
#[should_panic(expected = "reading timestamp cannot be in the future")]
fn attest_with_future_reading_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "reading": make_reading(addr(10), 5000),
        "timestamp": 1000u64
    }))
    .unwrap();
    dispatch(&mut state, "attest", &args, addr(2));
}
