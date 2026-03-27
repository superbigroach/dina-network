use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-41  On-Chain Task Scheduler
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];
pub type TaskId = u64;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScheduledTask {
    pub id: TaskId,
    pub creator: Address,
    pub target_contract: Address,
    pub method: String,
    pub args: Vec<u8>,
    pub execute_at: u64,
    pub recurring_interval: Option<u64>,
    pub last_executed: Option<u64>,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchedulerState {
    pub next_id: TaskId,
    pub tasks: BTreeMap<TaskId, ScheduledTask>,
}

/// Returned by execute_due_tasks so callers know which contracts/methods to invoke.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecutionResult {
    pub task_id: TaskId,
    pub target_contract: Address,
    pub method: String,
    pub args: Vec<u8>,
}

impl SchedulerState {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            tasks: BTreeMap::new(),
        }
    }

    pub fn schedule_task(
        &mut self,
        caller: Address,
        target_contract: Address,
        method: String,
        args: Vec<u8>,
        execute_at: u64,
        recurring_interval: Option<u64>,
    ) -> TaskId {
        if let Some(interval) = recurring_interval {
            assert!(interval > 0, "DRC41: recurring interval must be positive");
        }
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.insert(
            id,
            ScheduledTask {
                id,
                creator: caller,
                target_contract,
                method,
                args,
                execute_at,
                recurring_interval,
                last_executed: None,
                active: true,
            },
        );
        id
    }

    pub fn cancel_task(&mut self, caller: Address, task_id: TaskId) {
        let task = self.tasks.get(&task_id).expect("DRC41: task not found");
        assert!(task.creator == caller, "DRC41: only creator can cancel");
        assert!(task.active, "DRC41: task already inactive");

        let task = self.tasks.get_mut(&task_id).unwrap();
        task.active = false;
    }

    pub fn execute_due_tasks(&mut self, current_time: u64) -> Vec<ExecutionResult> {
        let mut results = Vec::new();
        let task_ids: Vec<TaskId> = self.tasks.keys().copied().collect();

        for id in task_ids {
            let task = self.tasks.get(&id).unwrap();
            if !task.active || task.execute_at > current_time {
                continue;
            }

            results.push(ExecutionResult {
                task_id: task.id,
                target_contract: task.target_contract,
                method: task.method.clone(),
                args: task.args.clone(),
            });

            let task = self.tasks.get_mut(&id).unwrap();
            task.last_executed = Some(current_time);

            match task.recurring_interval {
                Some(interval) => {
                    task.execute_at = current_time + interval;
                }
                None => {
                    task.active = false;
                }
            }
        }

        results
    }

    pub fn my_tasks(&self, caller: Address) -> Vec<&ScheduledTask> {
        self.tasks
            .values()
            .filter(|t| t.creator == caller && t.active)
            .collect()
    }

    pub fn next_execution(&self, task_id: TaskId) -> Option<u64> {
        let task = self.tasks.get(&task_id).expect("DRC41: task not found");
        if task.active {
            Some(task.execute_at)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ScheduleTaskArgs {
    target_contract: Address,
    method: String,
    args: Vec<u8>,
    execute_at: u64,
    recurring_interval: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CancelTaskArgs {
    task_id: TaskId,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteDueArgs {
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct NextExecutionArgs {
    task_id: TaskId,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<SchedulerState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC41: already initialised");
            *state = Some(SchedulerState::new());
            serde_json::to_vec("ok").unwrap()
        }

        "schedule_task" => {
            let s = state.as_mut().expect("DRC41: not initialised");
            let a: ScheduleTaskArgs =
                serde_json::from_slice(args).expect("DRC41: bad schedule_task args");
            let id = s.schedule_task(
                caller,
                a.target_contract,
                a.method,
                a.args,
                a.execute_at,
                a.recurring_interval,
            );
            serde_json::to_vec(&id).unwrap()
        }

        "cancel_task" => {
            let s = state.as_mut().expect("DRC41: not initialised");
            let a: CancelTaskArgs =
                serde_json::from_slice(args).expect("DRC41: bad cancel_task args");
            s.cancel_task(caller, a.task_id);
            serde_json::to_vec("ok").unwrap()
        }

        "execute_due_tasks" => {
            let s = state.as_mut().expect("DRC41: not initialised");
            let a: ExecuteDueArgs =
                serde_json::from_slice(args).expect("DRC41: bad execute_due_tasks args");
            let results = s.execute_due_tasks(a.current_time);
            serde_json::to_vec(&results).unwrap()
        }

        "my_tasks" => {
            let s = state.as_ref().expect("DRC41: not initialised");
            let tasks = s.my_tasks(caller);
            serde_json::to_vec(&tasks).unwrap()
        }

        "next_execution" => {
            let s = state.as_ref().expect("DRC41: not initialised");
            let a: NextExecutionArgs =
                serde_json::from_slice(args).expect("DRC41: bad next_execution args");
            let next = s.next_execution(a.task_id);
            serde_json::to_vec(&next).unwrap()
        }

        _ => panic!("DRC41: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ALICE: Address = [1u8; 32];
    const BOB: Address = [2u8; 32];
    const CONTRACT_A: Address = [10u8; 32];
    const CONTRACT_B: Address = [11u8; 32];

    fn init() -> Option<SchedulerState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", ALICE);
        state
    }

    fn schedule(
        state: &mut Option<SchedulerState>,
        caller: Address,
        target: Address,
        method: &str,
        execute_at: u64,
        recurring: Option<u64>,
    ) -> TaskId {
        let args = serde_json::to_vec(&ScheduleTaskArgs {
            target_contract: target,
            method: method.to_string(),
            args: vec![1, 2, 3],
            execute_at,
            recurring_interval: recurring,
        })
        .unwrap();
        let result = dispatch(state, "schedule_task", &args, caller);
        serde_json::from_slice(&result).unwrap()
    }

    #[test]
    fn test_schedule_and_execute_one_shot() {
        let mut state = init();
        let id = schedule(&mut state, ALICE, CONTRACT_A, "do_thing", 1000, None);
        assert_eq!(id, 1);

        // Not due yet at t=999
        let exec_args = serde_json::to_vec(&ExecuteDueArgs { current_time: 999 }).unwrap();
        let result = dispatch(&mut state, "execute_due_tasks", &exec_args, ALICE);
        let results: Vec<ExecutionResult> = serde_json::from_slice(&result).unwrap();
        assert!(results.is_empty());

        // Due at t=1000
        let exec_args = serde_json::to_vec(&ExecuteDueArgs { current_time: 1000 }).unwrap();
        let result = dispatch(&mut state, "execute_due_tasks", &exec_args, ALICE);
        let results: Vec<ExecutionResult> = serde_json::from_slice(&result).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].method, "do_thing");

        // Task should now be inactive (one-shot)
        let task = state.as_ref().unwrap().tasks.get(&id).unwrap();
        assert!(!task.active);
    }

    #[test]
    fn test_recurring_task() {
        let mut state = init();
        let id = schedule(&mut state, ALICE, CONTRACT_A, "heartbeat", 100, Some(50));

        // Execute at t=100
        let exec_args = serde_json::to_vec(&ExecuteDueArgs { current_time: 100 }).unwrap();
        let result = dispatch(&mut state, "execute_due_tasks", &exec_args, ALICE);
        let results: Vec<ExecutionResult> = serde_json::from_slice(&result).unwrap();
        assert_eq!(results.len(), 1);

        // Task should still be active, next at 150
        let task = state.as_ref().unwrap().tasks.get(&id).unwrap();
        assert!(task.active);
        assert_eq!(task.execute_at, 150);

        // Execute again at t=150
        let exec_args = serde_json::to_vec(&ExecuteDueArgs { current_time: 150 }).unwrap();
        let result = dispatch(&mut state, "execute_due_tasks", &exec_args, ALICE);
        let results: Vec<ExecutionResult> = serde_json::from_slice(&result).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(state.as_ref().unwrap().tasks.get(&id).unwrap().execute_at, 200);
    }

    #[test]
    fn test_cancel_task() {
        let mut state = init();
        let id = schedule(&mut state, ALICE, CONTRACT_A, "job", 500, None);

        let cancel_args = serde_json::to_vec(&CancelTaskArgs { task_id: id }).unwrap();
        dispatch(&mut state, "cancel_task", &cancel_args, ALICE);

        // Should not execute
        let exec_args = serde_json::to_vec(&ExecuteDueArgs { current_time: 600 }).unwrap();
        let result = dispatch(&mut state, "execute_due_tasks", &exec_args, ALICE);
        let results: Vec<ExecutionResult> = serde_json::from_slice(&result).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_my_tasks() {
        let mut state = init();
        schedule(&mut state, ALICE, CONTRACT_A, "a1", 100, None);
        schedule(&mut state, BOB, CONTRACT_B, "b1", 200, None);
        schedule(&mut state, ALICE, CONTRACT_B, "a2", 300, None);

        let result = dispatch(&mut state, "my_tasks", b"", ALICE);
        let tasks: Vec<ScheduledTask> = serde_json::from_slice(&result).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_next_execution() {
        let mut state = init();
        let id = schedule(&mut state, ALICE, CONTRACT_A, "ping", 750, Some(100));

        let next_args = serde_json::to_vec(&NextExecutionArgs { task_id: id }).unwrap();
        let result = dispatch(&mut state, "next_execution", &next_args, ALICE);
        let next: Option<u64> = serde_json::from_slice(&result).unwrap();
        assert_eq!(next, Some(750));

        // Execute, then check next
        let exec_args = serde_json::to_vec(&ExecuteDueArgs { current_time: 750 }).unwrap();
        dispatch(&mut state, "execute_due_tasks", &exec_args, ALICE);

        let result = dispatch(&mut state, "next_execution", &next_args, ALICE);
        let next: Option<u64> = serde_json::from_slice(&result).unwrap();
        assert_eq!(next, Some(850));
    }

    #[test]
    #[should_panic(expected = "only creator can cancel")]
    fn test_cancel_by_non_creator() {
        let mut state = init();
        let id = schedule(&mut state, ALICE, CONTRACT_A, "job", 500, None);

        let cancel_args = serde_json::to_vec(&CancelTaskArgs { task_id: id }).unwrap();
        dispatch(&mut state, "cancel_task", &cancel_args, BOB);
    }
}
