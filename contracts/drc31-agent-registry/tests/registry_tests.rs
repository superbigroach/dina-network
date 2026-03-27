use drc31_agent_registry::{dispatch, AgentType, RegistryState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_registry(admin: [u8; 32]) -> Option<RegistryState> {
    let mut state: Option<RegistryState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

fn register_ai_agent(state: &mut Option<RegistryState>, owner: [u8; 32], agent: [u8; 32]) {
    let args = serde_json::to_vec(&serde_json::json!({
        "agent_address": agent,
        "name": "TestAgent",
        "agent_type": "AI",
        "capabilities": ["nlp", "vision"],
        "created_at": 1000u64,
        "location": [10, 20]
    }))
    .unwrap();
    dispatch(state, "register_agent", &args, owner);
}

#[test]
fn register_and_retrieve_agent() {
    let admin = addr(1);
    let agent_addr = addr(2);
    let mut state = init_registry(admin);
    register_ai_agent(&mut state, admin, agent_addr);

    let s = state.as_ref().unwrap();
    let profile = s.get_agent(&agent_addr).unwrap();
    assert_eq!(profile.name, "TestAgent");
    assert_eq!(profile.agent_type, AgentType::AI);
    assert!(profile.active);
    assert_eq!(profile.reputation_score, 0);
}

#[test]
#[should_panic(expected = "agent already registered")]
fn duplicate_registration_fails() {
    let admin = addr(1);
    let agent_addr = addr(2);
    let mut state = init_registry(admin);
    register_ai_agent(&mut state, admin, agent_addr);
    register_ai_agent(&mut state, admin, agent_addr);
}

#[test]
fn deactivate_agent() {
    let admin = addr(1);
    let agent_addr = addr(2);
    let mut state = init_registry(admin);
    register_ai_agent(&mut state, admin, agent_addr);

    let args = serde_json::to_vec(&serde_json::json!({ "agent_address": agent_addr })).unwrap();
    dispatch(&mut state, "deactivate", &args, admin);

    let s = state.as_ref().unwrap();
    assert!(!s.get_agent(&agent_addr).unwrap().active);
    assert_eq!(s.total_agents(), 0);
}

#[test]
fn search_by_type_returns_matching() {
    let admin = addr(1);
    let mut state = init_registry(admin);
    register_ai_agent(&mut state, admin, addr(2));

    let args = serde_json::to_vec(&serde_json::json!({
        "agent_address": addr(3),
        "name": "RobotOne",
        "agent_type": "Robot",
        "capabilities": ["locomotion"],
        "created_at": 2000u64,
        "location": null
    }))
    .unwrap();
    dispatch(&mut state, "register_agent", &args, admin);

    let s = state.as_ref().unwrap();
    assert_eq!(s.search_by_type(&AgentType::AI).len(), 1);
    assert_eq!(s.search_by_type(&AgentType::Robot).len(), 1);
    assert_eq!(s.search_by_type(&AgentType::IoT).len(), 0);
}

#[test]
fn search_by_capability() {
    let admin = addr(1);
    let mut state = init_registry(admin);
    register_ai_agent(&mut state, admin, addr(2));

    let s = state.as_ref().unwrap();
    assert_eq!(s.search_by_capability("nlp").len(), 1);
    assert_eq!(s.search_by_capability("unknown").len(), 0);
}

#[test]
fn agents_near_location() {
    let admin = addr(1);
    let mut state = init_registry(admin);
    register_ai_agent(&mut state, admin, addr(2)); // location (10, 20)

    let s = state.as_ref().unwrap();
    assert_eq!(s.agents_near_location(10, 20, 5).len(), 1);
    assert_eq!(s.agents_near_location(100, 200, 5).len(), 0);
}

#[test]
fn update_profile() {
    let admin = addr(1);
    let agent_addr = addr(2);
    let mut state = init_registry(admin);
    register_ai_agent(&mut state, admin, agent_addr);

    let args = serde_json::to_vec(&serde_json::json!({
        "agent_address": agent_addr,
        "name": "UpdatedAgent",
        "capabilities": ["nlp", "vision", "audio"],
        "location": null
    }))
    .unwrap();
    dispatch(&mut state, "update_profile", &args, admin);

    let s = state.as_ref().unwrap();
    let profile = s.get_agent(&agent_addr).unwrap();
    assert_eq!(profile.name, "UpdatedAgent");
    assert_eq!(profile.capabilities.len(), 3);
}
