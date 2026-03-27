use drc110_firmware::{dispatch, FirmwareRegistry, FirmwareStatus};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<FirmwareRegistry> {
    let mut state: Option<FirmwareRegistry> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn register_manufacturer(state: &mut Option<FirmwareRegistry>, mfg: [u8; 32]) {
    let args = serde_json::to_vec(&serde_json::json!({
        "addr": mfg, "name": "TestMfg"
    })).unwrap();
    dispatch(state, "register_manufacturer", &args, addr(1));
}

fn register_trusted(state: &mut Option<FirmwareRegistry>, mfg: [u8; 32], hash: [u8; 32], version: &str, ts: u64) {
    let args = serde_json::to_vec(&serde_json::json!({
        "hash": hash, "version": version, "timestamp": ts
    })).unwrap();
    dispatch(state, "register_trusted_firmware", &args, mfg);
}

fn attest(state: &mut Option<FirmwareRegistry>, device_id: [u8; 32], fw_hash: [u8; 32]) {
    let zero: [u8; 32] = [0u8; 32];
    let args = serde_json::to_vec(&serde_json::json!({
        "record": {
            "device_id": device_id,
            "firmware_hash": fw_hash,
            "boot_hash": zero,
            "version": "1.0",
            "timestamp": 1000u64,
            "signature": [1, 2]
        }
    })).unwrap();
    dispatch(state, "attest_firmware", &args, addr(5));
}

#[test]
fn attest_firmware_stores_record() {
    let mut state = init();
    attest(&mut state, addr(10), [42u8; 32]);
    let args = serde_json::to_vec(&serde_json::json!({"device_id": addr(10)})).unwrap();
    let result = dispatch(&mut state, "firmware_history", &args, addr(1));
    let history: Vec<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert_eq!(history.len(), 1);
}

#[test]
fn verify_firmware_returns_unknown_for_unregistered() {
    let mut state = init();
    attest(&mut state, addr(10), [42u8; 32]);
    let args = serde_json::to_vec(&serde_json::json!({"device_id": addr(10)})).unwrap();
    let result = dispatch(&mut state, "verify_firmware", &args, addr(1));
    let status: FirmwareStatus = serde_json::from_slice(&result).unwrap();
    match status {
        FirmwareStatus::Unknown { .. } => {}
        _ => panic!("expected Unknown status"),
    }
}

#[test]
fn verify_firmware_returns_trusted_for_latest() {
    let mut state = init();
    register_manufacturer(&mut state, addr(5));
    let fw_hash = [42u8; 32];
    register_trusted(&mut state, addr(5), fw_hash, "1.0", 1000);
    attest(&mut state, addr(10), fw_hash);

    let args = serde_json::to_vec(&serde_json::json!({"device_id": addr(10)})).unwrap();
    let result = dispatch(&mut state, "verify_firmware", &args, addr(1));
    let status: FirmwareStatus = serde_json::from_slice(&result).unwrap();
    match status {
        FirmwareStatus::Trusted { version, .. } => assert_eq!(version, "1.0"),
        _ => panic!("expected Trusted status"),
    }
}

#[test]
fn register_trusted_firmware_makes_it_trusted() {
    let mut state = init();
    register_manufacturer(&mut state, addr(5));
    let hash = [99u8; 32];
    register_trusted(&mut state, addr(5), hash, "2.0", 2000);

    let args = serde_json::to_vec(&serde_json::json!({"hash": hash})).unwrap();
    let result = dispatch(&mut state, "is_trusted_firmware", &args, addr(1));
    let trusted: bool = serde_json::from_slice(&result).unwrap();
    assert!(trusted);
}

#[test]
#[should_panic(expected = "only registered manufacturers")]
fn register_trusted_by_non_manufacturer_fails() {
    let mut state = init();
    let hash = [42u8; 32];
    let args = serde_json::to_vec(&serde_json::json!({
        "hash": hash, "version": "1.0", "timestamp": 1000u64
    })).unwrap();
    dispatch(&mut state, "register_trusted_firmware", &args, addr(99));
}

#[test]
#[should_panic(expected = "only admin can register manufacturers")]
fn register_manufacturer_by_non_admin_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "addr": addr(5), "name": "X"
    })).unwrap();
    dispatch(&mut state, "register_manufacturer", &args, addr(99));
}

#[test]
fn verify_returns_outdated_for_old_firmware() {
    let mut state = init();
    register_manufacturer(&mut state, addr(5));
    let old_hash = [42u8; 32];
    let new_hash = [43u8; 32];
    register_trusted(&mut state, addr(5), old_hash, "1.0", 1000);
    register_trusted(&mut state, addr(5), new_hash, "2.0", 2000);
    attest(&mut state, addr(10), old_hash);

    let args = serde_json::to_vec(&serde_json::json!({"device_id": addr(10)})).unwrap();
    let result = dispatch(&mut state, "verify_firmware", &args, addr(1));
    let status: FirmwareStatus = serde_json::from_slice(&result).unwrap();
    match status {
        FirmwareStatus::Outdated { current, latest } => {
            assert_eq!(current, "1.0");
            assert_eq!(latest, "2.0");
        }
        _ => panic!("expected Outdated status"),
    }
}
