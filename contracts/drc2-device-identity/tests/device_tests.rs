use drc2_device_identity::{dispatch, DeviceMetadata, DeviceRegistryState};
use std::collections::BTreeMap;

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<DeviceRegistryState> {
    let mut state: Option<DeviceRegistryState> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn default_metadata() -> DeviceMetadata {
    DeviceMetadata {
        manufacturer: "Acme".into(),
        model: "Sensor-X".into(),
        firmware_version: "1.0.0".into(),
        capabilities: vec!["temperature".into()],
        location: Some("warehouse-A".into()),
        custom: BTreeMap::new(),
    }
}

fn register(state: &mut Option<DeviceRegistryState>, owner: [u8; 32], device_id: [u8; 32], pubkey: [u8; 32]) {
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": device_id,
        "public_key": pubkey,
        "device_type": "Sensor",
        "metadata": {
            "manufacturer": "Acme",
            "model": "Sensor-X",
            "firmware_version": "1.0.0",
            "capabilities": ["temperature"],
            "location": "warehouse-A",
            "custom": {}
        },
        "timestamp": 1000u64
    })).unwrap();
    dispatch(state, "register_device", &args, owner);
}

// ============================================================
// Init
// ============================================================

#[test]
fn init_creates_empty_registry() {
    let state = init();
    let s = state.as_ref().unwrap();
    assert_eq!(s.total_devices, 0);
    assert_eq!(s.admin, addr(1));
}

// ============================================================
// Register device
// ============================================================

#[test]
fn register_device_succeeds() {
    let mut state = init();
    register(&mut state, addr(1), addr(10), addr(20));
    let s = state.as_ref().unwrap();
    assert_eq!(s.total_devices, 1);
    assert!(s.is_active(&addr(10)));
}

#[test]
#[should_panic(expected = "device already registered")]
fn register_duplicate_device_fails() {
    let mut state = init();
    register(&mut state, addr(1), addr(10), addr(20));
    register(&mut state, addr(1), addr(10), addr(21));
}

#[test]
#[should_panic(expected = "public key already bound")]
fn register_duplicate_pubkey_fails() {
    let mut state = init();
    register(&mut state, addr(1), addr(10), addr(20));
    register(&mut state, addr(1), addr(11), addr(20));
}

// ============================================================
// Resolve
// ============================================================

#[test]
fn resolve_returns_device_identity() {
    let mut state = init();
    register(&mut state, addr(1), addr(10), addr(20));
    let args = serde_json::to_vec(&serde_json::json!({"device_id": addr(10)})).unwrap();
    let result = dispatch(&mut state, "resolve", &args, addr(1));
    let identity: Option<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert!(identity.is_some());
}

// ============================================================
// Revoke
// ============================================================

#[test]
fn revoke_makes_device_inactive() {
    let mut state = init();
    register(&mut state, addr(1), addr(10), addr(20));
    let args = serde_json::to_vec(&serde_json::json!({"device_id": addr(10)})).unwrap();
    dispatch(&mut state, "revoke", &args, addr(1));
    assert!(!state.as_ref().unwrap().is_active(&addr(10)));
}

#[test]
#[should_panic(expected = "only owner or admin can revoke")]
fn revoke_by_non_owner_non_admin_fails() {
    let mut state = init();
    register(&mut state, addr(2), addr(10), addr(20));
    let args = serde_json::to_vec(&serde_json::json!({"device_id": addr(10)})).unwrap();
    dispatch(&mut state, "revoke", &args, addr(3));
}

// ============================================================
// is_active
// ============================================================

#[test]
fn is_active_returns_false_for_unknown_device() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"device_id": addr(99)})).unwrap();
    let result = dispatch(&mut state, "is_active", &args, addr(1));
    let active: bool = serde_json::from_slice(&result).unwrap();
    assert!(!active);
}

// ============================================================
// devices_of
// ============================================================

#[test]
fn devices_of_returns_all_owner_devices() {
    let mut state = init();
    register(&mut state, addr(2), addr(10), addr(20));
    register(&mut state, addr(2), addr(11), addr(21));
    let args = serde_json::to_vec(&serde_json::json!({"owner": addr(2)})).unwrap();
    let result = dispatch(&mut state, "devices_of", &args, addr(1));
    let devices: Vec<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert_eq!(devices.len(), 2);
}

#[test]
fn devices_of_returns_empty_for_unknown_owner() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({"owner": addr(99)})).unwrap();
    let result = dispatch(&mut state, "devices_of", &args, addr(1));
    let devices: Vec<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert_eq!(devices.len(), 0);
}
