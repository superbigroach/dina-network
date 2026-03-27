use drc104_swarm::{dispatch, SwarmRegistry, SwarmConfig, SpendingLimits};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<SwarmRegistry> {
    let mut state: Option<SwarmRegistry> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn default_config(admin: [u8; 32]) -> SwarmConfig {
    SwarmConfig {
        name: "TestSwarm".to_string(),
        admin,
        quorum: 51,
        max_members: 10,
        spending_limits: SpendingLimits {
            max_per_tx: 1000,
            max_per_day: 5000,
        },
    }
}

fn create_swarm(state: &mut Option<SwarmRegistry>, admin: [u8; 32]) -> [u8; 32] {
    let args = serde_json::to_vec(&serde_json::json!({
        "config": default_config(admin)
    })).unwrap();
    let result = dispatch(state, "create_swarm", &args, admin);
    serde_json::from_slice(&result).unwrap()
}

fn add_member(state: &mut Option<SwarmRegistry>, admin: [u8; 32], swarm_id: [u8; 32], device: [u8; 32]) {
    let args = serde_json::to_vec(&serde_json::json!({
        "swarm_id": swarm_id, "device_id": device
    })).unwrap();
    dispatch(state, "add_member", &args, admin);
}

#[test]
fn create_swarm_returns_id() {
    let mut state = init();
    let id = create_swarm(&mut state, addr(2));
    assert_ne!(id, [0u8; 32]);
}

#[test]
fn add_member_makes_device_a_member() {
    let mut state = init();
    let id = create_swarm(&mut state, addr(2));
    add_member(&mut state, addr(2), id, addr(10));

    let args = serde_json::to_vec(&serde_json::json!({
        "swarm_id": id, "device_id": addr(10)
    })).unwrap();
    let result = dispatch(&mut state, "is_member", &args, addr(1));
    let is_member: bool = serde_json::from_slice(&result).unwrap();
    assert!(is_member);
}

#[test]
#[should_panic(expected = "only swarm admin can add members")]
fn add_member_by_non_admin_fails() {
    let mut state = init();
    let id = create_swarm(&mut state, addr(2));
    add_member(&mut state, addr(99), id, addr(10));
}

#[test]
#[should_panic(expected = "device already a member")]
fn add_duplicate_member_fails() {
    let mut state = init();
    let id = create_swarm(&mut state, addr(2));
    add_member(&mut state, addr(2), id, addr(10));
    add_member(&mut state, addr(2), id, addr(10));
}

#[test]
fn swarm_execute_with_quorum_succeeds() {
    let mut state = init();
    let id = create_swarm(&mut state, addr(2));
    add_member(&mut state, addr(2), id, addr(10));
    add_member(&mut state, addr(2), id, addr(11));

    // Fund the swarm wallet directly
    state.as_mut().unwrap().swarm_wallets.insert(id, 5000);

    let args = serde_json::to_vec(&serde_json::json!({
        "swarm_id": id,
        "action": {"Transfer": {"to": addr(20), "amount": 100u64}},
        "signatures": [
            {"member": addr(10), "signature": [1u8]},
            {"member": addr(11), "signature": [2u8]}
        ]
    })).unwrap();
    dispatch(&mut state, "swarm_execute", &args, addr(1));

    assert_eq!(state.as_ref().unwrap().swarm_wallet(&id), 4900);
}

#[test]
#[should_panic(expected = "quorum not met")]
fn swarm_execute_without_quorum_fails() {
    let mut state = init();
    let id = create_swarm(&mut state, addr(2));
    add_member(&mut state, addr(2), id, addr(10));
    add_member(&mut state, addr(2), id, addr(11));
    add_member(&mut state, addr(2), id, addr(12));
    add_member(&mut state, addr(2), id, addr(13));

    state.as_mut().unwrap().swarm_wallets.insert(id, 5000);

    // Only 1 of 4 signatures, need 51% = 3
    let args = serde_json::to_vec(&serde_json::json!({
        "swarm_id": id,
        "action": {"Transfer": {"to": addr(20), "amount": 100u64}},
        "signatures": [{"member": addr(10), "signature": [1u8]}]
    })).unwrap();
    dispatch(&mut state, "swarm_execute", &args, addr(1));
}

#[test]
fn remove_member_works() {
    let mut state = init();
    let id = create_swarm(&mut state, addr(2));
    add_member(&mut state, addr(2), id, addr(10));
    let rm_args = serde_json::to_vec(&serde_json::json!({
        "swarm_id": id, "device_id": addr(10)
    })).unwrap();
    dispatch(&mut state, "remove_member", &rm_args, addr(2));
    assert!(!state.as_ref().unwrap().is_member(&id, &addr(10)));
}

#[test]
fn members_returns_list() {
    let mut state = init();
    let id = create_swarm(&mut state, addr(2));
    add_member(&mut state, addr(2), id, addr(10));
    add_member(&mut state, addr(2), id, addr(11));
    let args = serde_json::to_vec(&serde_json::json!({"swarm_id": id})).unwrap();
    let result = dispatch(&mut state, "members", &args, addr(1));
    let members: Vec<[u8; 32]> = serde_json::from_slice(&result).unwrap();
    assert_eq!(members.len(), 2);
}
