use drc35_fleet_manager::{dispatch, AssignmentStatus, FleetManagerState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_fleet_mgr(admin: [u8; 32]) -> Option<FleetManagerState> {
    let mut state: Option<FleetManagerState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

fn create_fleet_with_robot(
    state: &mut Option<FleetManagerState>,
    owner: [u8; 32],
    robot: [u8; 32],
) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({ "name": "AlphaFleet" })).unwrap();
    let result = dispatch(state, "create_fleet", &args, owner);
    let fleet_id: u64 = serde_json::from_slice(&result).unwrap();

    let add_args =
        serde_json::to_vec(&serde_json::json!({ "fleet_id": fleet_id, "robot_id": robot }))
            .unwrap();
    dispatch(state, "add_robot", &add_args, owner);
    fleet_id
}

#[test]
fn create_fleet_and_add_robot() {
    let owner = addr(1);
    let robot = addr(10);
    let mut state = init_fleet_mgr(owner);
    let fleet_id = create_fleet_with_robot(&mut state, owner, robot);

    let s = state.as_ref().unwrap();
    let fleet = s.fleet_status(fleet_id).unwrap();
    assert_eq!(fleet.name, "AlphaFleet");
    assert_eq!(fleet.robots.len(), 1);
    assert_eq!(fleet.robots[0], robot);
}

#[test]
fn remove_robot_from_fleet() {
    let owner = addr(1);
    let robot = addr(10);
    let mut state = init_fleet_mgr(owner);
    let fleet_id = create_fleet_with_robot(&mut state, owner, robot);

    let args =
        serde_json::to_vec(&serde_json::json!({ "fleet_id": fleet_id, "robot_id": robot }))
            .unwrap();
    dispatch(&mut state, "remove_robot", &args, owner);

    let s = state.as_ref().unwrap();
    assert_eq!(s.fleet_status(fleet_id).unwrap().robots.len(), 0);
}

#[test]
fn assign_and_complete_task() {
    let owner = addr(1);
    let robot = addr(10);
    let mut state = init_fleet_mgr(owner);
    let fleet_id = create_fleet_with_robot(&mut state, owner, robot);

    let assign_args = serde_json::to_vec(&serde_json::json!({
        "fleet_id": fleet_id,
        "robot_id": robot,
        "task_description": "Deliver package",
        "start_time": 1000u64,
        "end_time": 2000u64,
        "location": "Warehouse A"
    }))
    .unwrap();
    let result = dispatch(&mut state, "assign_task", &assign_args, owner);
    let assignment_id: u64 = serde_json::from_slice(&result).unwrap();

    // Complete
    let complete_args = serde_json::to_vec(&serde_json::json!({
        "fleet_id": fleet_id,
        "assignment_id": assignment_id
    }))
    .unwrap();
    dispatch(&mut state, "complete_assignment", &complete_args, owner);

    let s = state.as_ref().unwrap();
    let fleet = s.fleet_status(fleet_id).unwrap();
    assert_eq!(
        fleet.assignments.get(&assignment_id).unwrap().status,
        AssignmentStatus::Completed
    );
}

#[test]
fn robot_schedule_shows_active_only() {
    let owner = addr(1);
    let robot = addr(10);
    let mut state = init_fleet_mgr(owner);
    let fleet_id = create_fleet_with_robot(&mut state, owner, robot);

    // Assign two tasks
    for i in 0..2 {
        let args = serde_json::to_vec(&serde_json::json!({
            "fleet_id": fleet_id,
            "robot_id": robot,
            "task_description": format!("Task {i}"),
            "start_time": (1000 + i * 1000) as u64,
            "end_time": (2000 + i * 1000) as u64,
            "location": "Zone B"
        }))
        .unwrap();
        dispatch(&mut state, "assign_task", &args, owner);
    }

    // Complete first
    let complete_args = serde_json::to_vec(&serde_json::json!({
        "fleet_id": fleet_id,
        "assignment_id": 1u64
    }))
    .unwrap();
    dispatch(&mut state, "complete_assignment", &complete_args, owner);

    let s = state.as_ref().unwrap();
    assert_eq!(s.robot_schedule(fleet_id, &robot).len(), 1);
}

#[test]
#[should_panic(expected = "robot not in fleet")]
fn assign_task_to_unknown_robot_fails() {
    let owner = addr(1);
    let robot = addr(10);
    let unknown = addr(99);
    let mut state = init_fleet_mgr(owner);
    let fleet_id = create_fleet_with_robot(&mut state, owner, robot);

    let args = serde_json::to_vec(&serde_json::json!({
        "fleet_id": fleet_id,
        "robot_id": unknown,
        "task_description": "Bad task",
        "start_time": 1000u64,
        "end_time": 2000u64,
        "location": "Nowhere"
    }))
    .unwrap();
    dispatch(&mut state, "assign_task", &args, owner);
}

#[test]
#[should_panic(expected = "robot already in fleet")]
fn duplicate_robot_add_fails() {
    let owner = addr(1);
    let robot = addr(10);
    let mut state = init_fleet_mgr(owner);
    let fleet_id = create_fleet_with_robot(&mut state, owner, robot);

    let args =
        serde_json::to_vec(&serde_json::json!({ "fleet_id": fleet_id, "robot_id": robot }))
            .unwrap();
    dispatch(&mut state, "add_robot", &args, owner);
}
