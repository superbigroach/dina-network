use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-35  Robot Fleet Management
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];
pub type DeviceId = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AssignmentStatus {
    Active,
    Completed,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Assignment {
    pub id: u64,
    pub robot_id: DeviceId,
    pub task_description: String,
    pub start_time: u64,
    pub end_time: u64,
    pub status: AssignmentStatus,
    pub location: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Fleet {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub robots: Vec<DeviceId>,
    pub assignments: BTreeMap<u64, Assignment>,
    pub next_assignment_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FleetManagerState {
    pub admin: Address,
    pub fleets: BTreeMap<u64, Fleet>,
    pub next_fleet_id: u64,
}

impl FleetManagerState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            fleets: BTreeMap::new(),
            next_fleet_id: 1,
        }
    }

    pub fn create_fleet(&mut self, caller: Address, name: String) -> u64 {
        assert!(!name.is_empty(), "DRC35: name cannot be empty");
        let id = self.next_fleet_id;
        self.next_fleet_id += 1;
        let fleet = Fleet {
            id,
            owner: caller,
            name,
            robots: Vec::new(),
            assignments: BTreeMap::new(),
            next_assignment_id: 1,
        };
        self.fleets.insert(id, fleet);
        id
    }

    pub fn add_robot(&mut self, caller: Address, fleet_id: u64, robot_id: DeviceId) {
        let fleet = self
            .fleets
            .get_mut(&fleet_id)
            .expect("DRC35: fleet not found");
        assert!(fleet.owner == caller, "DRC35: only owner can add robots");
        assert!(
            !fleet.robots.contains(&robot_id),
            "DRC35: robot already in fleet"
        );
        fleet.robots.push(robot_id);
    }

    pub fn remove_robot(&mut self, caller: Address, fleet_id: u64, robot_id: DeviceId) {
        let fleet = self
            .fleets
            .get_mut(&fleet_id)
            .expect("DRC35: fleet not found");
        assert!(fleet.owner == caller, "DRC35: only owner can remove robots");
        let pos = fleet
            .robots
            .iter()
            .position(|r| *r == robot_id)
            .expect("DRC35: robot not in fleet");
        fleet.robots.remove(pos);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn assign_task(
        &mut self,
        caller: Address,
        fleet_id: u64,
        robot_id: DeviceId,
        task_description: String,
        start_time: u64,
        end_time: u64,
        location: String,
    ) -> u64 {
        let fleet = self
            .fleets
            .get_mut(&fleet_id)
            .expect("DRC35: fleet not found");
        assert!(
            fleet.owner == caller,
            "DRC35: only owner can assign tasks"
        );
        assert!(
            fleet.robots.contains(&robot_id),
            "DRC35: robot not in fleet"
        );
        assert!(end_time > start_time, "DRC35: end must be after start");
        let assignment_id = fleet.next_assignment_id;
        fleet.next_assignment_id += 1;
        let assignment = Assignment {
            id: assignment_id,
            robot_id,
            task_description,
            start_time,
            end_time,
            status: AssignmentStatus::Active,
            location,
        };
        fleet.assignments.insert(assignment_id, assignment);
        assignment_id
    }

    pub fn complete_assignment(&mut self, caller: Address, fleet_id: u64, assignment_id: u64) {
        let fleet = self
            .fleets
            .get_mut(&fleet_id)
            .expect("DRC35: fleet not found");
        assert!(
            fleet.owner == caller,
            "DRC35: only owner can complete assignments"
        );
        let assignment = fleet
            .assignments
            .get_mut(&assignment_id)
            .expect("DRC35: assignment not found");
        assert!(
            assignment.status == AssignmentStatus::Active,
            "DRC35: assignment not active"
        );
        assignment.status = AssignmentStatus::Completed;
    }

    pub fn fleet_status(&self, fleet_id: u64) -> Option<&Fleet> {
        self.fleets.get(&fleet_id)
    }

    pub fn robot_schedule(&self, fleet_id: u64, robot_id: &DeviceId) -> Vec<&Assignment> {
        self.fleets.get(&fleet_id).map_or(vec![], |f| {
            f.assignments
                .values()
                .filter(|a| a.robot_id == *robot_id && a.status == AssignmentStatus::Active)
                .collect()
        })
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateFleetArgs {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddRobotArgs {
    fleet_id: u64,
    robot_id: DeviceId,
}

#[derive(Serialize, Deserialize, Debug)]
struct RemoveRobotArgs {
    fleet_id: u64,
    robot_id: DeviceId,
}

#[derive(Serialize, Deserialize, Debug)]
struct AssignTaskArgs {
    fleet_id: u64,
    robot_id: DeviceId,
    task_description: String,
    start_time: u64,
    end_time: u64,
    location: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompleteAssignmentArgs {
    fleet_id: u64,
    assignment_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct FleetStatusArgs {
    fleet_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RobotScheduleArgs {
    fleet_id: u64,
    robot_id: DeviceId,
}

pub fn dispatch(
    state: &mut Option<FleetManagerState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC35: already initialised");
            *state = Some(FleetManagerState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_fleet" => {
            let s = state.as_mut().expect("DRC35: not initialised");
            let a: CreateFleetArgs =
                serde_json::from_slice(args).expect("DRC35: bad create_fleet args");
            let id = s.create_fleet(caller, a.name);
            serde_json::to_vec(&id).unwrap()
        }
        "add_robot" => {
            let s = state.as_mut().expect("DRC35: not initialised");
            let a: AddRobotArgs =
                serde_json::from_slice(args).expect("DRC35: bad add_robot args");
            s.add_robot(caller, a.fleet_id, a.robot_id);
            serde_json::to_vec("ok").unwrap()
        }
        "remove_robot" => {
            let s = state.as_mut().expect("DRC35: not initialised");
            let a: RemoveRobotArgs =
                serde_json::from_slice(args).expect("DRC35: bad remove_robot args");
            s.remove_robot(caller, a.fleet_id, a.robot_id);
            serde_json::to_vec("ok").unwrap()
        }
        "assign_task" => {
            let s = state.as_mut().expect("DRC35: not initialised");
            let a: AssignTaskArgs =
                serde_json::from_slice(args).expect("DRC35: bad assign_task args");
            let id = s.assign_task(
                caller,
                a.fleet_id,
                a.robot_id,
                a.task_description,
                a.start_time,
                a.end_time,
                a.location,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "complete_assignment" => {
            let s = state.as_mut().expect("DRC35: not initialised");
            let a: CompleteAssignmentArgs =
                serde_json::from_slice(args).expect("DRC35: bad complete_assignment args");
            s.complete_assignment(caller, a.fleet_id, a.assignment_id);
            serde_json::to_vec("ok").unwrap()
        }
        "fleet_status" => {
            let s = state.as_ref().expect("DRC35: not initialised");
            let a: FleetStatusArgs =
                serde_json::from_slice(args).expect("DRC35: bad fleet_status args");
            serde_json::to_vec(&s.fleet_status(a.fleet_id)).unwrap()
        }
        "robot_schedule" => {
            let s = state.as_ref().expect("DRC35: not initialised");
            let a: RobotScheduleArgs =
                serde_json::from_slice(args).expect("DRC35: bad robot_schedule args");
            serde_json::to_vec(&s.robot_schedule(a.fleet_id, &a.robot_id)).unwrap()
        }
        _ => panic!("DRC35: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [1u8; 32];
    const OTHER: Address = [2u8; 32];
    const ROBOT_A: DeviceId = [10u8; 32];
    const ROBOT_B: DeviceId = [11u8; 32];

    fn init_state() -> Option<FleetManagerState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", OWNER);
        state
    }

    fn create_fleet_with_robot(state: &mut Option<FleetManagerState>) -> u64 {
        let create_args = serde_json::to_vec(&serde_json::json!({
            "name": "Warehouse-Alpha"
        }))
        .unwrap();
        let result = dispatch(state, "create_fleet", &create_args, OWNER);
        let fleet_id: u64 = serde_json::from_slice(&result).unwrap();

        let add_args = serde_json::to_vec(&serde_json::json!({
            "fleet_id": fleet_id,
            "robot_id": ROBOT_A
        }))
        .unwrap();
        dispatch(state, "add_robot", &add_args, OWNER);
        fleet_id
    }

    #[test]
    fn test_create_fleet_and_add_robots() {
        let mut state = init_state();
        let fleet_id = create_fleet_with_robot(&mut state);
        assert_eq!(fleet_id, 1);

        let add_b = serde_json::to_vec(&serde_json::json!({
            "fleet_id": fleet_id,
            "robot_id": ROBOT_B
        }))
        .unwrap();
        dispatch(&mut state, "add_robot", &add_b, OWNER);

        let status_args = serde_json::to_vec(&serde_json::json!({"fleet_id": fleet_id})).unwrap();
        let result = dispatch(&mut state, "fleet_status", &status_args, OWNER);
        let fleet: Fleet = serde_json::from_slice(&result).unwrap();
        assert_eq!(fleet.name, "Warehouse-Alpha");
        assert_eq!(fleet.robots.len(), 2);
    }

    #[test]
    fn test_assign_task_and_complete() {
        let mut state = init_state();
        let fleet_id = create_fleet_with_robot(&mut state);

        let assign_args = serde_json::to_vec(&serde_json::json!({
            "fleet_id": fleet_id,
            "robot_id": ROBOT_A,
            "task_description": "Pick items from shelf B3",
            "start_time": 1000u64,
            "end_time": 2000u64,
            "location": "Aisle-B"
        }))
        .unwrap();
        let result = dispatch(&mut state, "assign_task", &assign_args, OWNER);
        let assignment_id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(assignment_id, 1);

        let complete_args = serde_json::to_vec(&serde_json::json!({
            "fleet_id": fleet_id,
            "assignment_id": assignment_id
        }))
        .unwrap();
        dispatch(&mut state, "complete_assignment", &complete_args, OWNER);

        let s = state.as_ref().unwrap();
        let fleet = s.fleet_status(fleet_id).unwrap();
        let assignment = fleet.assignments.get(&assignment_id).unwrap();
        assert_eq!(assignment.status, AssignmentStatus::Completed);
    }

    #[test]
    fn test_robot_schedule_filters_active_only() {
        let mut state = init_state();
        let fleet_id = create_fleet_with_robot(&mut state);

        // Assign two tasks
        for i in 0..2 {
            let assign_args = serde_json::to_vec(&serde_json::json!({
                "fleet_id": fleet_id,
                "robot_id": ROBOT_A,
                "task_description": format!("Task {}", i),
                "start_time": 1000u64 + i * 1000,
                "end_time": 2000u64 + i * 1000,
                "location": "Zone-A"
            }))
            .unwrap();
            dispatch(&mut state, "assign_task", &assign_args, OWNER);
        }

        // Complete first task
        let complete_args = serde_json::to_vec(&serde_json::json!({
            "fleet_id": fleet_id,
            "assignment_id": 1
        }))
        .unwrap();
        dispatch(&mut state, "complete_assignment", &complete_args, OWNER);

        let s = state.as_ref().unwrap();
        let schedule = s.robot_schedule(fleet_id, &ROBOT_A);
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule[0].id, 2);
    }

    #[test]
    #[should_panic(expected = "DRC35: only owner can add robots")]
    fn test_non_owner_cannot_add_robot() {
        let mut state = init_state();
        let create_args = serde_json::to_vec(&serde_json::json!({"name": "Fleet-1"})).unwrap();
        dispatch(&mut state, "create_fleet", &create_args, OWNER);

        let add_args = serde_json::to_vec(&serde_json::json!({
            "fleet_id": 1,
            "robot_id": ROBOT_A
        }))
        .unwrap();
        dispatch(&mut state, "add_robot", &add_args, OTHER);
    }

    #[test]
    #[should_panic(expected = "DRC35: robot already in fleet")]
    fn test_cannot_add_duplicate_robot() {
        let mut state = init_state();
        create_fleet_with_robot(&mut state);

        let add_args = serde_json::to_vec(&serde_json::json!({
            "fleet_id": 1,
            "robot_id": ROBOT_A
        }))
        .unwrap();
        dispatch(&mut state, "add_robot", &add_args, OWNER);
    }

    #[test]
    #[should_panic(expected = "DRC35: robot not in fleet")]
    fn test_cannot_assign_task_to_unknown_robot() {
        let mut state = init_state();
        let create_args = serde_json::to_vec(&serde_json::json!({"name": "Fleet-1"})).unwrap();
        dispatch(&mut state, "create_fleet", &create_args, OWNER);

        let assign_args = serde_json::to_vec(&serde_json::json!({
            "fleet_id": 1,
            "robot_id": ROBOT_B,
            "task_description": "Go fetch",
            "start_time": 100u64,
            "end_time": 200u64,
            "location": "Dock"
        }))
        .unwrap();
        dispatch(&mut state, "assign_task", &assign_args, OWNER);
    }

    #[test]
    fn test_remove_robot() {
        let mut state = init_state();
        create_fleet_with_robot(&mut state);

        let remove_args = serde_json::to_vec(&serde_json::json!({
            "fleet_id": 1,
            "robot_id": ROBOT_A
        }))
        .unwrap();
        dispatch(&mut state, "remove_robot", &remove_args, OWNER);

        let s = state.as_ref().unwrap();
        let fleet = s.fleet_status(1).unwrap();
        assert!(fleet.robots.is_empty());
    }
}
