use drc108_resource::{dispatch, ResourceRegistry};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<ResourceRegistry> {
    let mut state: Option<ResourceRegistry> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn allocate(state: &mut Option<ResourceRegistry>, device: [u8; 32], resource: &str, amount: u64) {
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": device, "resource_type": resource,
        "amount": amount, "expires_at": null
    })).unwrap();
    dispatch(state, "allocate", &args, addr(1));
}

#[test]
fn allocate_creates_balance() {
    let mut state = init();
    allocate(&mut state, addr(10), "compute", 1000);
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": addr(10), "resource_type": "compute"
    })).unwrap();
    let result = dispatch(&mut state, "balance", &args, addr(1));
    let bal: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(bal, 1000);
}

#[test]
fn transfer_resource_moves_allocation() {
    let mut state = init();
    allocate(&mut state, addr(10), "compute", 1000);
    let args = serde_json::to_vec(&serde_json::json!({
        "from_device": addr(10), "to_device": addr(11),
        "resource_type": "compute", "amount": 300u64
    })).unwrap();
    dispatch(&mut state, "transfer_resource", &args, addr(1));

    let s = state.as_ref().unwrap();
    assert_eq!(s.balance(&addr(10), "compute"), 700);
    assert_eq!(s.balance(&addr(11), "compute"), 300);
}

#[test]
fn purchase_resource_credits_buyer() {
    let mut state = init();
    let price_args = serde_json::to_vec(&serde_json::json!({
        "resource_type": "compute", "price_per_unit": 10u64
    })).unwrap();
    dispatch(&mut state, "set_price", &price_args, addr(1));

    let args = serde_json::to_vec(&serde_json::json!({
        "resource_type": "compute", "amount": 50u64
    })).unwrap();
    dispatch(&mut state, "purchase_resource", &args, addr(5));

    assert_eq!(state.as_ref().unwrap().balance(&addr(5), "compute"), 50);
    assert_eq!(state.as_ref().unwrap().revenue, 500);
}

#[test]
fn report_usage_tracks_consumption() {
    let mut state = init();
    allocate(&mut state, addr(10), "compute", 1000);
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": addr(10), "resource_type": "compute", "used": 400u64
    })).unwrap();
    dispatch(&mut state, "report_usage", &args, addr(1));

    // Balance should reflect used
    let s = state.as_ref().unwrap();
    assert_eq!(s.balance(&addr(10), "compute"), 600);
}

#[test]
#[should_panic(expected = "usage exceeds allocation")]
fn report_usage_exceeding_allocation_fails() {
    let mut state = init();
    allocate(&mut state, addr(10), "compute", 100);
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": addr(10), "resource_type": "compute", "used": 200u64
    })).unwrap();
    dispatch(&mut state, "report_usage", &args, addr(1));
}

#[test]
#[should_panic(expected = "only admin can allocate")]
fn allocate_by_non_admin_fails() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": addr(10), "resource_type": "compute",
        "amount": 100u64, "expires_at": null
    })).unwrap();
    dispatch(&mut state, "allocate", &args, addr(99));
}

#[test]
#[should_panic(expected = "insufficient resource balance")]
fn transfer_more_than_available_fails() {
    let mut state = init();
    allocate(&mut state, addr(10), "compute", 100);
    let args = serde_json::to_vec(&serde_json::json!({
        "from_device": addr(10), "to_device": addr(11),
        "resource_type": "compute", "amount": 200u64
    })).unwrap();
    dispatch(&mut state, "transfer_resource", &args, addr(1));
}

#[test]
fn balance_returns_zero_for_unknown() {
    let mut state = init();
    let args = serde_json::to_vec(&serde_json::json!({
        "device_id": addr(99), "resource_type": "compute"
    })).unwrap();
    let result = dispatch(&mut state, "balance", &args, addr(1));
    let bal: u64 = serde_json::from_slice(&result).unwrap();
    assert_eq!(bal, 0);
}
