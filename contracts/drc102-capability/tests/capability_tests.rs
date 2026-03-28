use drc102_capability::{
    dispatch, Capability, CapabilityRegistryState, CapabilityStatus, PricingModel,
};
use std::collections::BTreeMap;

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<CapabilityRegistryState> {
    let mut state: Option<CapabilityRegistryState> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn make_cap(cap_type: &str) -> Capability {
    Capability {
        capability_type: cap_type.to_string(),
        version: "1.0".to_string(),
        status: CapabilityStatus::Online,
        pricing: PricingModel::Free,
        metadata: BTreeMap::new(),
        registered_at: 1000,
        last_updated: 1000,
    }
}

fn register_cap(state: &mut Option<CapabilityRegistryState>, device: [u8; 32], cap_type: &str) {
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": device,
        "capabilities": [make_cap(cap_type)]
    }))
    .unwrap();
    dispatch(state, "register_capabilities", &args, addr(2));
}

#[test]
fn register_capabilities_succeeds() {
    let mut state = init();
    register_cap(&mut state, addr(10), "compute");
    let s = state.as_ref().unwrap();
    assert!(s.has_capability(&addr(10), "compute"));
}

#[test]
fn find_by_capability_returns_matching_devices() {
    let mut state = init();
    register_cap(&mut state, addr(10), "compute");
    register_cap(&mut state, addr(11), "compute");
    register_cap(&mut state, addr(12), "storage");

    let args = serde_json::to_vec(&serde_json::json!({"capability_type": "compute"})).unwrap();
    let result = dispatch(&mut state, "find_by_capability", &args, addr(1));
    let found: Vec<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert_eq!(found.len(), 2);
}

#[test]
fn find_by_capability_returns_empty_for_unknown() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"capability_type": "nonexistent"})).unwrap();
    let result = dispatch(&mut state, "find_by_capability", &args, addr(1));
    let found: Vec<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert_eq!(found.len(), 0);
}

#[test]
fn update_status_changes_capability_status() {
    let mut state = init();
    register_cap(&mut state, addr(10), "compute");
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": addr(10),
        "capability_type": "compute",
        "new_status": "Offline",
        "timestamp": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "update_status", &args, addr(2));

    let caps = state.as_ref().unwrap().capabilities_of(&addr(10));
    assert_eq!(caps[0].status, CapabilityStatus::Offline);
}

#[test]
fn has_capability_returns_false_for_unknown_device() {
    let state = init();
    assert!(!state.as_ref().unwrap().has_capability(&addr(99), "compute"));
}

#[test]
#[should_panic(expected = "must register at least one capability")]
fn register_empty_capabilities_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": addr(10),
        "capabilities": []
    }))
    .unwrap();
    dispatch(&mut state, "register_capabilities", &args, addr(2));
}

#[test]
fn capabilities_of_returns_all_for_device() {
    let mut state = init();
    register_cap(&mut state, addr(10), "compute");
    register_cap(&mut state, addr(10), "storage");

    let args = serde_json::to_vec(&serde_json::json!({"device_id": addr(10)})).unwrap();
    let result = dispatch(&mut state, "capabilities_of", &args, addr(1));
    let caps: Vec<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert_eq!(caps.len(), 2);
}
