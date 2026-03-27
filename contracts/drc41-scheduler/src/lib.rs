use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-41  On-Chain Task Scheduler
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScheduledTask {
    pub id: u64,
    pub creator: Address,
    pub target_contract: Address,
    pub method: String,
    pub args: String, // JSON-encoded args
    pub execute_at: u64,
    pub recurring_interval: Option<u64>, // None = one-shot, Some(secs) = recurring
    pub last_executed: Option<u64>,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchedulerState {
    pub admin: Address,
    pub tasks: BTreeMap<u64, ScheduledTask>,
    pub next_id: u64,
}

impl SchedulerState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            tasks: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn schedule_task(
        &mut self,
        caller: Address,
        target_contract: Address,
        method: String,
        args: String,
        execute_at: u64,
        recurring_interval: Option<u64>,
    ) -> u64 {
        assert!(!method.is_empty(), "DRC41: method cannot be empty");
        assert!(execute_at > 0, "DRC41: execute_at must be positive");
        if let Some(interval) = recurring_interval {
            assert!(interval > 0, "DRC41: interval must be positive");
        }
        let id = self.next_id;
        self.next_id += 1;
        let task = ScheduledTask {
            id,
            creator: caller,
            target_contract,
            method,
            args,
            execute_at,
            recurring_interval,
            last_executed: None,
            active: true,
        };
        self.tasks.insert(id, task);
        id
    }

    pub fn cancel_task(&mut self, caller: Address, task_id: u64) {
        let task = self
            .tasks
            .get_mut(&task_id)
            .expect("DRC41: task not found");
        assert!(
            task.creator == caller || caller == self.admin,
            "DRC41: not authorized"
        );
        task.active = false;
    }

    /// Returns IDs of tasks that were executed. In a real runtime, this would
    /// invoke the target contracts. Here we mark them executed and reschedule
    /// recurring tasks.
    pub fn execute_due_tasks(&mut self, current_time: u64) -> Vec<u64> {
        let due_ids: Vec<u64> = self
            .tasks
            .values()
            .filter(|t| t.active && t.execute_at <= current_time)
            .map(|t| t.id)
            .collect();

        for id in &due_ids {
            let task = self.tasks.get_mut(id).unwrap();
            task.last_executed = Some(current_time);
            if let Some(interval) = task.recurring_interval {
                task.execute_at = current_time + interval;
            } else {
                task.active = false;
            }
        }
        due_ids
    }

    pub fn my_scheduled_tasks(&self, creator: &Address) -> Vec<&ScheduledTask> {
        self.tasks
            .values()
            .filter(|t| t.creator == *creator && t.active)
            .collect()
    }

    pub fn next_execution(&self, task_id: u64) -> Option<u64> {
        self.tasks
            .get(&task_id)
            .filter(|t| t.active)
            .map(|t| t.execute_at)
    }

    pub fn get_task(&self, id: u64) -> Option<&ScheduledTask> {
        self.tasks.get(&id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ScheduleTaskArgs {
    target_contract: Address,
    method: String,
    args: String,
    execute_at: u64,
    recurring_interval: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CancelTaskArgs {
    task_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteDueArgs {
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct MyTasksArgs {
    creator: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct NextExecArgs {
    task_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetTaskArgs {
    id: u64,
}

pub fn dispatch(
    state: &mut Option<SchedulerState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC41: already initialised");
            *state = Some(SchedulerState::new(caller));
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
            let executed = s.execute_due_tasks(a.current_time);
            serde_json::to_vec(&executed).unwrap()
        }
        "my_scheduled_tasks" => {
            let s = state.as_ref().expect("DRC41: not initialised");
            let a: MyTasksArgs =
                serde_json::from_slice(args).expect("DRC41: bad my_scheduled_tasks args");
            serde_json::to_vec(&s.my_scheduled_tasks(&a.creator)).unwrap()
        }
        "next_execution" => {
            let s = state.as_ref().expect("DRC41: not initialised");
            let a: NextExecArgs =
                serde_json::from_slice(args).expect("DRC41: bad next_execution args");
            serde_json::to_vec(&s.next_execution(a.task_id)).unwrap()
        }
        "get_task" => {
            let s = state.as_ref().expect("DRC41: not initialised");
            let a: GetTaskArgs =
                serde_json::from_slice(args).expect("DRC41: bad get_task args");
            serde_json::to_vec(&s.get_task(a.id)).unwrap()
        }
        _ => panic!("DRC41: unknown method '{method}'"),
    }
}
