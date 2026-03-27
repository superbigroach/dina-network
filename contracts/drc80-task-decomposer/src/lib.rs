use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-80  AI Task Decomposition
// ---------------------------------------------------------------------------

type Address = [u8; 32];
type TaskId = u64;
type SubtaskId = u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Created,
    Decomposed,
    InProgress,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SubtaskStatus {
    Pending,
    Assigned,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Subtask {
    pub id: SubtaskId,
    pub parent_task: TaskId,
    pub description: String,
    pub required_capability: String,
    pub assigned_to: Option<Address>,
    pub reward: u64,
    pub status: SubtaskStatus,
    pub result_hash: Option<[u8; 32]>,
    pub dependencies: Vec<SubtaskId>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplexTask {
    pub id: TaskId,
    pub creator: Address,
    pub description: String,
    pub budget: u64,
    pub subtasks: Vec<SubtaskId>,
    pub status: TaskStatus,
    pub final_result_hash: Option<[u8; 32]>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskProgress {
    pub total_subtasks: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
    pub assigned: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskDecomposerState {
    pub owner: Address,
    pub tasks: BTreeMap<TaskId, ComplexTask>,
    pub subtasks: BTreeMap<SubtaskId, Subtask>,
    pub next_task_id: TaskId,
    pub next_subtask_id: SubtaskId,
    pub balances: BTreeMap<Address, u64>,
}

impl TaskDecomposerState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            tasks: BTreeMap::new(),
            subtasks: BTreeMap::new(),
            next_task_id: 1,
            next_subtask_id: 1,
            balances: BTreeMap::new(),
        }
    }

    pub fn deposit(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC80: deposit must be positive");
        *self.balances.entry(caller).or_insert(0) += amount;
    }

    pub fn create_complex_task(
        &mut self,
        caller: Address,
        description: String,
        budget: u64,
    ) -> TaskId {
        assert!(!description.is_empty(), "DRC80: description required");
        assert!(budget > 0, "DRC80: budget must be positive");

        let bal = self.balances.get(&caller).copied().unwrap_or(0);
        assert!(bal >= budget, "DRC80: insufficient balance for budget");
        self.balances.insert(caller, bal - budget);

        let id = self.next_task_id;
        self.next_task_id += 1;
        self.tasks.insert(id, ComplexTask {
            id, creator: caller, description, budget,
            subtasks: Vec::new(),
            status: TaskStatus::Created,
            final_result_hash: None,
        });
        id
    }

    /// Decompose a task into subtasks. Only the creator can decompose.
    pub fn decompose(
        &mut self,
        caller: Address,
        task_id: TaskId,
        subtask_defs: Vec<(String, String, u64, Vec<SubtaskId>)>, // (description, capability, reward, deps)
    ) {
        let task = self.tasks.get(&task_id).expect("DRC80: task not found");
        assert!(task.creator == caller, "DRC80: only creator can decompose");
        assert!(task.status == TaskStatus::Created, "DRC80: task already decomposed");

        let total_reward: u64 = subtask_defs.iter().map(|(_, _, r, _)| r).sum();
        assert!(total_reward <= task.budget, "DRC80: subtask rewards exceed budget");

        let mut subtask_ids = Vec::new();
        for (desc, cap, reward, deps) in subtask_defs {
            let sid = self.next_subtask_id;
            self.next_subtask_id += 1;
            self.subtasks.insert(sid, Subtask {
                id: sid, parent_task: task_id, description: desc,
                required_capability: cap, assigned_to: None, reward,
                status: SubtaskStatus::Pending, result_hash: None,
                dependencies: deps,
            });
            subtask_ids.push(sid);
        }

        let task = self.tasks.get_mut(&task_id).unwrap();
        task.subtasks = subtask_ids;
        task.status = TaskStatus::Decomposed;
    }

    pub fn assign_subtask(&mut self, caller: Address, subtask_id: SubtaskId, agent: Address) {
        let subtask = self.subtasks.get(&subtask_id).expect("DRC80: subtask not found");
        let task = self.tasks.get(&subtask.parent_task).expect("DRC80: parent task not found");
        assert!(task.creator == caller, "DRC80: only task creator can assign");
        assert!(subtask.status == SubtaskStatus::Pending, "DRC80: subtask not pending");

        let subtask = self.subtasks.get_mut(&subtask_id).unwrap();
        subtask.assigned_to = Some(agent);
        subtask.status = SubtaskStatus::Assigned;

        // Update parent task status
        let task = self.tasks.get_mut(&subtask.parent_task).unwrap();
        if task.status == TaskStatus::Decomposed {
            task.status = TaskStatus::InProgress;
        }
    }

    /// Check if all dependencies of a subtask are completed.
    pub fn check_dependencies(&self, subtask_id: SubtaskId) -> bool {
        let subtask = self.subtasks.get(&subtask_id).expect("DRC80: subtask not found");
        subtask.dependencies.iter().all(|dep_id| {
            self.subtasks.get(dep_id)
                .map_or(false, |d| d.status == SubtaskStatus::Completed)
        })
    }

    pub fn complete_subtask(
        &mut self,
        caller: Address,
        subtask_id: SubtaskId,
        result_hash: [u8; 32],
    ) {
        let subtask = self.subtasks.get(&subtask_id).expect("DRC80: subtask not found");
        assert!(subtask.status == SubtaskStatus::Assigned, "DRC80: subtask not assigned");
        assert!(subtask.assigned_to == Some(caller), "DRC80: only assigned agent can complete");
        assert!(self.check_dependencies(subtask_id), "DRC80: dependencies not met");

        let reward = subtask.reward;
        let subtask = self.subtasks.get_mut(&subtask_id).unwrap();
        subtask.result_hash = Some(result_hash);
        subtask.status = SubtaskStatus::Completed;

        // Pay the agent
        *self.balances.entry(caller).or_insert(0) += reward;
    }

    /// Aggregate results — check if all subtasks are done, mark task complete.
    pub fn aggregate_results(&mut self, task_id: TaskId, final_hash: [u8; 32]) -> bool {
        let task = self.tasks.get(&task_id).expect("DRC80: task not found");
        let all_done = task.subtasks.iter().all(|sid| {
            self.subtasks.get(sid).map_or(false, |s| s.status == SubtaskStatus::Completed)
        });

        if all_done {
            let task = self.tasks.get_mut(&task_id).unwrap();
            task.status = TaskStatus::Completed;
            task.final_result_hash = Some(final_hash);

            // Refund unused budget
            let total_rewards: u64 = task.subtasks.iter()
                .filter_map(|sid| self.subtasks.get(sid))
                .map(|s| s.reward)
                .sum();
            let refund = task.budget.saturating_sub(total_rewards);
            if refund > 0 {
                let creator = task.creator;
                *self.balances.entry(creator).or_insert(0) += refund;
            }
        }
        all_done
    }

    pub fn task_progress(&self, task_id: TaskId) -> TaskProgress {
        let task = self.tasks.get(&task_id).expect("DRC80: task not found");
        let mut progress = TaskProgress {
            total_subtasks: task.subtasks.len(),
            completed: 0, failed: 0, pending: 0, assigned: 0,
        };
        for sid in &task.subtasks {
            if let Some(st) = self.subtasks.get(sid) {
                match st.status {
                    SubtaskStatus::Completed => progress.completed += 1,
                    SubtaskStatus::Failed => progress.failed += 1,
                    SubtaskStatus::Pending => progress.pending += 1,
                    SubtaskStatus::Assigned => progress.assigned += 1,
                }
            }
        }
        progress
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateTaskArgs { description: String, budget: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct DecomposeArgs { task_id: TaskId, subtask_defs: Vec<(String, String, u64, Vec<SubtaskId>)> }
#[derive(Serialize, Deserialize, Debug)]
struct AssignArgs { subtask_id: SubtaskId, agent: Address }
#[derive(Serialize, Deserialize, Debug)]
struct CompleteArgs { subtask_id: SubtaskId, result_hash: [u8; 32] }
#[derive(Serialize, Deserialize, Debug)]
struct AggregateArgs { task_id: TaskId, final_hash: [u8; 32] }
#[derive(Serialize, Deserialize, Debug)]
struct TaskIdArgs { task_id: TaskId }
#[derive(Serialize, Deserialize, Debug)]
struct SubtaskIdArgs { subtask_id: SubtaskId }
#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs { amount: u64 }

pub fn dispatch(
    state: &mut Option<TaskDecomposerState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC80: already initialised");
            *state = Some(TaskDecomposerState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "deposit" => {
            let s = state.as_mut().expect("DRC80: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC80: bad args");
            s.deposit(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "create_complex_task" => {
            let s = state.as_mut().expect("DRC80: not initialised");
            let a: CreateTaskArgs = serde_json::from_slice(args).expect("DRC80: bad args");
            let id = s.create_complex_task(caller, a.description, a.budget);
            serde_json::to_vec(&id).unwrap()
        }
        "decompose" => {
            let s = state.as_mut().expect("DRC80: not initialised");
            let a: DecomposeArgs = serde_json::from_slice(args).expect("DRC80: bad args");
            s.decompose(caller, a.task_id, a.subtask_defs);
            serde_json::to_vec("ok").unwrap()
        }
        "assign_subtask" => {
            let s = state.as_mut().expect("DRC80: not initialised");
            let a: AssignArgs = serde_json::from_slice(args).expect("DRC80: bad args");
            s.assign_subtask(caller, a.subtask_id, a.agent);
            serde_json::to_vec("ok").unwrap()
        }
        "complete_subtask" => {
            let s = state.as_mut().expect("DRC80: not initialised");
            let a: CompleteArgs = serde_json::from_slice(args).expect("DRC80: bad args");
            s.complete_subtask(caller, a.subtask_id, a.result_hash);
            serde_json::to_vec("ok").unwrap()
        }
        "check_dependencies" => {
            let s = state.as_ref().expect("DRC80: not initialised");
            let a: SubtaskIdArgs = serde_json::from_slice(args).expect("DRC80: bad args");
            serde_json::to_vec(&s.check_dependencies(a.subtask_id)).unwrap()
        }
        "aggregate_results" => {
            let s = state.as_mut().expect("DRC80: not initialised");
            let a: AggregateArgs = serde_json::from_slice(args).expect("DRC80: bad args");
            let done = s.aggregate_results(a.task_id, a.final_hash);
            serde_json::to_vec(&done).unwrap()
        }
        "task_progress" => {
            let s = state.as_ref().expect("DRC80: not initialised");
            let a: TaskIdArgs = serde_json::from_slice(args).expect("DRC80: bad args");
            serde_json::to_vec(&s.task_progress(a.task_id)).unwrap()
        }
        _ => panic!("DRC80: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const CREATOR: Address = [1u8; 32];
    const AGENT_A: Address = [2u8; 32];
    const AGENT_B: Address = [3u8; 32];

    fn setup() -> (TaskDecomposerState, TaskId) {
        let mut s = TaskDecomposerState::new(OWNER);
        s.deposit(CREATOR, 100_000);
        let tid = s.create_complex_task(
            CREATOR, "Analyze satellite imagery and generate report".into(), 5000,
        );
        s.decompose(CREATOR, tid, vec![
            ("Download imagery".into(), "data-access".into(), 1000, vec![]),
            ("Run ML model".into(), "ml-inference".into(), 2500, vec![1]), // depends on subtask 1
            ("Generate report".into(), "text-generation".into(), 1000, vec![2]), // depends on subtask 2
        ]);
        (s, tid)
    }

    #[test]
    fn test_create_and_decompose() {
        let (s, tid) = setup();
        let task = s.tasks.get(&tid).unwrap();
        assert_eq!(task.status, TaskStatus::Decomposed);
        assert_eq!(task.subtasks.len(), 3);
        assert_eq!(s.balances.get(&CREATOR).copied().unwrap(), 95_000);
    }

    #[test]
    fn test_assign_and_complete_subtask() {
        let (mut s, tid) = setup();
        s.assign_subtask(CREATOR, 1, AGENT_A);
        assert_eq!(s.subtasks.get(&1).unwrap().status, SubtaskStatus::Assigned);
        assert_eq!(s.tasks.get(&tid).unwrap().status, TaskStatus::InProgress);

        s.complete_subtask(AGENT_A, 1, [0xAA; 32]);
        assert_eq!(s.subtasks.get(&1).unwrap().status, SubtaskStatus::Completed);
        assert_eq!(s.balances.get(&AGENT_A).copied().unwrap(), 1000);
    }

    #[test]
    fn test_dependency_chain() {
        let (mut s, _tid) = setup();
        // Subtask 2 depends on subtask 1
        assert!(!s.check_dependencies(2));

        s.assign_subtask(CREATOR, 1, AGENT_A);
        s.complete_subtask(AGENT_A, 1, [0xAA; 32]);
        assert!(s.check_dependencies(2));
    }

    #[test]
    #[should_panic(expected = "dependencies not met")]
    fn test_cannot_complete_with_unmet_deps() {
        let (mut s, _tid) = setup();
        s.assign_subtask(CREATOR, 2, AGENT_B);
        s.complete_subtask(AGENT_B, 2, [0xBB; 32]); // subtask 1 not done
    }

    #[test]
    fn test_aggregate_results_and_refund() {
        let (mut s, tid) = setup();
        // Complete all subtasks in order
        s.assign_subtask(CREATOR, 1, AGENT_A);
        s.complete_subtask(AGENT_A, 1, [0x11; 32]);

        s.assign_subtask(CREATOR, 2, AGENT_B);
        s.complete_subtask(AGENT_B, 2, [0x22; 32]);

        s.assign_subtask(CREATOR, 3, AGENT_A);
        s.complete_subtask(AGENT_A, 3, [0x33; 32]);

        let done = s.aggregate_results(tid, [0xFF; 32]);
        assert!(done);
        assert_eq!(s.tasks.get(&tid).unwrap().status, TaskStatus::Completed);
        // Budget was 5000, total rewards = 1000+2500+1000 = 4500, refund = 500
        assert_eq!(s.balances.get(&CREATOR).copied().unwrap(), 95_500);
    }

    #[test]
    fn test_task_progress() {
        let (mut s, tid) = setup();
        let progress = s.task_progress(tid);
        assert_eq!(progress.total_subtasks, 3);
        assert_eq!(progress.pending, 3);

        s.assign_subtask(CREATOR, 1, AGENT_A);
        s.complete_subtask(AGENT_A, 1, [0x11; 32]);
        let progress = s.task_progress(tid);
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.pending, 2);
    }
}
