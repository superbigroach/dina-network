use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-32  Decentralized Task Queue
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Open,
    Claimed,
    Completed,
    Expired,
    Disputed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    pub id: u64,
    pub poster: Address,
    pub description: String,
    pub reward: u64,
    pub deadline: u64,
    pub required_capabilities: Vec<String>,
    pub assigned_to: Option<Address>,
    pub status: TaskStatus,
    pub proof: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskQueueState {
    pub admin: Address,
    pub tasks: BTreeMap<u64, Task>,
    pub next_id: u64,
    pub escrow: BTreeMap<Address, u64>,
}

impl TaskQueueState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            tasks: BTreeMap::new(),
            next_id: 1,
            escrow: BTreeMap::new(),
        }
    }

    pub fn post_task(
        &mut self,
        caller: Address,
        description: String,
        reward: u64,
        deadline: u64,
        required_capabilities: Vec<String>,
    ) -> u64 {
        assert!(reward > 0, "DRC32: reward must be positive");
        assert!(deadline > 0, "DRC32: deadline must be positive");
        let id = self.next_id;
        self.next_id += 1;
        let task = Task {
            id,
            poster: caller,
            description,
            reward,
            deadline,
            required_capabilities,
            assigned_to: None,
            status: TaskStatus::Open,
            proof: None,
        };
        self.tasks.insert(id, task);
        // Track escrowed reward
        let bal = self.escrow.entry(caller).or_insert(0);
        *bal += reward;
        id
    }

    pub fn claim_task(&mut self, caller: Address, task_id: u64) {
        let task = self.tasks.get_mut(&task_id).expect("DRC32: task not found");
        assert!(task.status == TaskStatus::Open, "DRC32: task not open");
        assert!(task.poster != caller, "DRC32: cannot claim own task");
        task.assigned_to = Some(caller);
        task.status = TaskStatus::Claimed;
    }

    pub fn complete_task(&mut self, caller: Address, task_id: u64, proof: String) {
        let task = self.tasks.get_mut(&task_id).expect("DRC32: task not found");
        assert!(
            task.status == TaskStatus::Claimed,
            "DRC32: task not claimed"
        );
        assert!(
            task.assigned_to == Some(caller),
            "DRC32: not assigned to caller"
        );
        task.proof = Some(proof);
        // Stays Claimed until verified
    }

    pub fn verify_completion(&mut self, caller: Address, task_id: u64, approved: bool) {
        let task = self.tasks.get_mut(&task_id).expect("DRC32: task not found");
        assert!(
            task.poster == caller || caller == self.admin,
            "DRC32: only poster or admin can verify"
        );
        assert!(
            task.status == TaskStatus::Claimed && task.proof.is_some(),
            "DRC32: no proof submitted"
        );
        if approved {
            task.status = TaskStatus::Completed;
            // Release escrow
            if let Some(bal) = self.escrow.get_mut(&task.poster) {
                *bal = bal.saturating_sub(task.reward);
            }
        } else {
            task.status = TaskStatus::Disputed;
        }
    }

    pub fn cancel_task(&mut self, caller: Address, task_id: u64) {
        let task = self.tasks.get_mut(&task_id).expect("DRC32: task not found");
        assert!(task.poster == caller, "DRC32: only poster can cancel");
        assert!(
            task.status == TaskStatus::Open,
            "DRC32: can only cancel open tasks"
        );
        task.status = TaskStatus::Expired;
        if let Some(bal) = self.escrow.get_mut(&task.poster) {
            *bal = bal.saturating_sub(task.reward);
        }
    }

    pub fn available_tasks(&self) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::Open)
            .collect()
    }

    pub fn my_tasks(&self, address: &Address) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.poster == *address || t.assigned_to.as_ref() == Some(address))
            .collect()
    }

    pub fn get_task(&self, id: u64) -> Option<&Task> {
        self.tasks.get(&id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct PostTaskArgs {
    description: String,
    reward: u64,
    deadline: u64,
    required_capabilities: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClaimTaskArgs {
    task_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompleteTaskArgs {
    task_id: u64,
    proof: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifyCompletionArgs {
    task_id: u64,
    approved: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct CancelTaskArgs {
    task_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct MyTasksArgs {
    address: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetTaskArgs {
    task_id: u64,
}

pub fn dispatch(
    state: &mut Option<TaskQueueState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC32: already initialised");
            *state = Some(TaskQueueState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "post_task" => {
            let s = state.as_mut().expect("DRC32: not initialised");
            let a: PostTaskArgs = serde_json::from_slice(args).expect("DRC32: bad post_task args");
            let id = s.post_task(
                caller,
                a.description,
                a.reward,
                a.deadline,
                a.required_capabilities,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "claim_task" => {
            let s = state.as_mut().expect("DRC32: not initialised");
            let a: ClaimTaskArgs =
                serde_json::from_slice(args).expect("DRC32: bad claim_task args");
            s.claim_task(caller, a.task_id);
            serde_json::to_vec("ok").unwrap()
        }
        "complete_task" => {
            let s = state.as_mut().expect("DRC32: not initialised");
            let a: CompleteTaskArgs =
                serde_json::from_slice(args).expect("DRC32: bad complete_task args");
            s.complete_task(caller, a.task_id, a.proof);
            serde_json::to_vec("ok").unwrap()
        }
        "verify_completion" => {
            let s = state.as_mut().expect("DRC32: not initialised");
            let a: VerifyCompletionArgs =
                serde_json::from_slice(args).expect("DRC32: bad verify_completion args");
            s.verify_completion(caller, a.task_id, a.approved);
            serde_json::to_vec("ok").unwrap()
        }
        "cancel_task" => {
            let s = state.as_mut().expect("DRC32: not initialised");
            let a: CancelTaskArgs =
                serde_json::from_slice(args).expect("DRC32: bad cancel_task args");
            s.cancel_task(caller, a.task_id);
            serde_json::to_vec("ok").unwrap()
        }
        "available_tasks" => {
            let s = state.as_ref().expect("DRC32: not initialised");
            serde_json::to_vec(&s.available_tasks()).unwrap()
        }
        "my_tasks" => {
            let s = state.as_ref().expect("DRC32: not initialised");
            let a: MyTasksArgs = serde_json::from_slice(args).expect("DRC32: bad my_tasks args");
            serde_json::to_vec(&s.my_tasks(&a.address)).unwrap()
        }
        "get_task" => {
            let s = state.as_ref().expect("DRC32: not initialised");
            let a: GetTaskArgs = serde_json::from_slice(args).expect("DRC32: bad get_task args");
            serde_json::to_vec(&s.get_task(a.task_id)).unwrap()
        }
        _ => panic!("DRC32: unknown method '{method}'"),
    }
}
