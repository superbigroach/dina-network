use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-65  Agent Swarm Coordination
// ---------------------------------------------------------------------------
// Coordinate swarms of AI agents working together on a task with consensus
// mechanisms, budgets, and reward distribution.

type Address = [u8; 32];
type SwarmId = u64;
type TaskId = u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SwarmStatus {
    Forming,
    Active,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConsensusMethod {
    Majority,
    Unanimous,
    WeightedVote,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentSwarm {
    pub id: SwarmId,
    pub coordinator: Address,
    pub agents: Vec<Address>,
    pub objective: String,
    pub status: SwarmStatus,
    pub consensus_method: ConsensusMethod,
    pub created_at: u64,
    pub budget: u64,
    pub spent: u64,
    pub max_agents: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmTask {
    pub id: TaskId,
    pub swarm_id: SwarmId,
    pub description: String,
    pub assigned_agents: Vec<Address>,
    pub results: BTreeMap<String, Vec<u8>>, // hex(address) -> result
    pub consensus_result: Option<Vec<u8>>,
    pub deadline: u64,
    pub reward_per_agent: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmState {
    pub owner: Address,
    pub swarms: BTreeMap<SwarmId, AgentSwarm>,
    pub tasks: BTreeMap<TaskId, SwarmTask>,
    pub next_swarm_id: u64,
    pub next_task_id: u64,
}

fn addr_key(a: &Address) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

impl SwarmState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            swarms: BTreeMap::new(),
            tasks: BTreeMap::new(),
            next_swarm_id: 1,
            next_task_id: 1,
        }
    }

    pub fn create_swarm(
        &mut self,
        caller: Address,
        objective: String,
        consensus_method: ConsensusMethod,
        budget: u64,
        max_agents: u64,
        timestamp: u64,
    ) -> SwarmId {
        assert!(!objective.is_empty(), "DRC65: objective required");
        assert!(max_agents >= 2, "DRC65: need at least 2 agents");
        let id = self.next_swarm_id;
        self.next_swarm_id += 1;
        self.swarms.insert(id, AgentSwarm {
            id,
            coordinator: caller,
            agents: vec![caller],
            objective,
            status: SwarmStatus::Forming,
            consensus_method,
            created_at: timestamp,
            budget,
            spent: 0,
            max_agents,
        });
        id
    }

    pub fn join_swarm(&mut self, caller: Address, swarm_id: SwarmId) {
        let swarm = self.swarms.get_mut(&swarm_id).expect("DRC65: swarm not found");
        assert!(swarm.status == SwarmStatus::Forming || swarm.status == SwarmStatus::Active,
            "DRC65: swarm not accepting members");
        assert!((swarm.agents.len() as u64) < swarm.max_agents, "DRC65: swarm full");
        assert!(!swarm.agents.contains(&caller), "DRC65: already a member");
        swarm.agents.push(caller);
        if swarm.status == SwarmStatus::Forming && swarm.agents.len() >= 2 {
            swarm.status = SwarmStatus::Active;
        }
    }

    pub fn assign_task(
        &mut self,
        caller: Address,
        swarm_id: SwarmId,
        description: String,
        assigned_agents: Vec<Address>,
        deadline: u64,
        reward_per_agent: u64,
    ) -> TaskId {
        let swarm = self.swarms.get(&swarm_id).expect("DRC65: swarm not found");
        assert!(caller == swarm.coordinator, "DRC65: only coordinator");
        assert!(swarm.status == SwarmStatus::Active, "DRC65: swarm not active");
        // Verify all assigned agents are members
        for agent in &assigned_agents {
            assert!(swarm.agents.contains(agent), "DRC65: agent not in swarm");
        }
        let total_reward = reward_per_agent * assigned_agents.len() as u64;
        assert!(swarm.budget - swarm.spent >= total_reward, "DRC65: insufficient budget");

        let id = self.next_task_id;
        self.next_task_id += 1;
        self.tasks.insert(id, SwarmTask {
            id,
            swarm_id,
            description,
            assigned_agents,
            results: BTreeMap::new(),
            consensus_result: None,
            deadline,
            reward_per_agent,
        });
        id
    }

    pub fn submit_result(
        &mut self,
        caller: Address,
        task_id: TaskId,
        result: Vec<u8>,
    ) {
        let task = self.tasks.get_mut(&task_id).expect("DRC65: task not found");
        assert!(task.assigned_agents.contains(&caller), "DRC65: not assigned");
        let key = addr_key(&caller);
        assert!(!task.results.contains_key(&key), "DRC65: already submitted");
        task.results.insert(key, result);
    }

    pub fn reach_consensus(&mut self, caller: Address, task_id: TaskId) -> Option<Vec<u8>> {
        let task = self.tasks.get(&task_id).expect("DRC65: task not found");
        let swarm = self.swarms.get(&task.swarm_id).expect("DRC65: swarm not found");
        assert!(caller == swarm.coordinator, "DRC65: only coordinator");
        assert!(task.consensus_result.is_none(), "DRC65: consensus already reached");

        let total_assigned = task.assigned_agents.len();
        let submitted = task.results.len();

        let consensus = match swarm.consensus_method {
            ConsensusMethod::Unanimous => {
                if submitted < total_assigned { return None; }
                // All must match
                let values: Vec<&Vec<u8>> = task.results.values().collect();
                if values.windows(2).all(|w| w[0] == w[1]) {
                    Some(values[0].clone())
                } else {
                    None
                }
            }
            ConsensusMethod::Majority => {
                if submitted * 2 <= total_assigned { return None; }
                // Find most common result
                let mut counts: BTreeMap<&Vec<u8>, usize> = BTreeMap::new();
                for v in task.results.values() {
                    *counts.entry(v).or_insert(0) += 1;
                }
                counts.into_iter()
                    .max_by_key(|(_, c)| *c)
                    .map(|(v, _)| v.clone())
            }
            ConsensusMethod::WeightedVote => {
                // Equal weights, pick most common
                if submitted == 0 { return None; }
                let mut counts: BTreeMap<&Vec<u8>, usize> = BTreeMap::new();
                for v in task.results.values() {
                    *counts.entry(v).or_insert(0) += 1;
                }
                counts.into_iter()
                    .max_by_key(|(_, c)| *c)
                    .map(|(v, _)| v.clone())
            }
        };

        if let Some(ref result) = consensus {
            let task = self.tasks.get_mut(&task_id).unwrap();
            task.consensus_result = Some(result.clone());
            // Deduct from budget
            let reward_total = task.reward_per_agent * task.assigned_agents.len() as u64;
            let swarm = self.swarms.get_mut(&task.swarm_id).unwrap();
            swarm.spent += reward_total;
        }

        consensus
    }

    pub fn complete_swarm(&mut self, caller: Address, swarm_id: SwarmId) {
        let swarm = self.swarms.get_mut(&swarm_id).expect("DRC65: swarm not found");
        assert!(caller == swarm.coordinator, "DRC65: only coordinator");
        assert!(swarm.status == SwarmStatus::Active, "DRC65: not active");
        swarm.status = SwarmStatus::Completed;
    }

    pub fn distribute_rewards(&self, task_id: TaskId) -> Vec<(Address, u64)> {
        let task = self.tasks.get(&task_id).expect("DRC65: task not found");
        assert!(task.consensus_result.is_some(), "DRC65: no consensus reached");
        task.assigned_agents.iter()
            .filter(|a| task.results.contains_key(&addr_key(a)))
            .map(|a| (*a, task.reward_per_agent))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateSwarmArgs { objective: String, consensus_method: ConsensusMethod, budget: u64, max_agents: u64, timestamp: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct JoinSwarmArgs { swarm_id: SwarmId }
#[derive(Serialize, Deserialize, Debug)]
struct AssignTaskArgs { swarm_id: SwarmId, description: String, assigned_agents: Vec<Address>, deadline: u64, reward_per_agent: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct SubmitResultArgs { task_id: TaskId, result: Vec<u8> }
#[derive(Serialize, Deserialize, Debug)]
struct TaskIdArgs { task_id: TaskId }
#[derive(Serialize, Deserialize, Debug)]
struct SwarmIdArgs { swarm_id: SwarmId }

pub fn dispatch(
    state: &mut Option<SwarmState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC65: already initialised");
            *state = Some(SwarmState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_swarm" => {
            let s = state.as_mut().expect("DRC65: not initialised");
            let a: CreateSwarmArgs = serde_json::from_slice(args).expect("DRC65: bad args");
            let id = s.create_swarm(caller, a.objective, a.consensus_method, a.budget, a.max_agents, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        "join_swarm" => {
            let s = state.as_mut().expect("DRC65: not initialised");
            let a: JoinSwarmArgs = serde_json::from_slice(args).expect("DRC65: bad args");
            s.join_swarm(caller, a.swarm_id);
            serde_json::to_vec("ok").unwrap()
        }
        "assign_task" => {
            let s = state.as_mut().expect("DRC65: not initialised");
            let a: AssignTaskArgs = serde_json::from_slice(args).expect("DRC65: bad args");
            let id = s.assign_task(caller, a.swarm_id, a.description, a.assigned_agents, a.deadline, a.reward_per_agent);
            serde_json::to_vec(&id).unwrap()
        }
        "submit_result" => {
            let s = state.as_mut().expect("DRC65: not initialised");
            let a: SubmitResultArgs = serde_json::from_slice(args).expect("DRC65: bad args");
            s.submit_result(caller, a.task_id, a.result);
            serde_json::to_vec("ok").unwrap()
        }
        "reach_consensus" => {
            let s = state.as_mut().expect("DRC65: not initialised");
            let a: TaskIdArgs = serde_json::from_slice(args).expect("DRC65: bad args");
            let result = s.reach_consensus(caller, a.task_id);
            serde_json::to_vec(&result).unwrap()
        }
        "complete_swarm" => {
            let s = state.as_mut().expect("DRC65: not initialised");
            let a: SwarmIdArgs = serde_json::from_slice(args).expect("DRC65: bad args");
            s.complete_swarm(caller, a.swarm_id);
            serde_json::to_vec("ok").unwrap()
        }
        "distribute_rewards" => {
            let s = state.as_ref().expect("DRC65: not initialised");
            let a: TaskIdArgs = serde_json::from_slice(args).expect("DRC65: bad args");
            let rewards = s.distribute_rewards(a.task_id);
            serde_json::to_vec(&rewards).unwrap()
        }
        _ => panic!("DRC65: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const COORDINATOR: Address = [1u8; 32];
    const AGENT_A: Address = [2u8; 32];
    const AGENT_B: Address = [3u8; 32];
    const AGENT_C: Address = [4u8; 32];

    fn setup_active_swarm() -> (SwarmState, SwarmId) {
        let mut s = SwarmState::new(COORDINATOR);
        let sid = s.create_swarm(COORDINATOR, "classify images".into(),
            ConsensusMethod::Majority, 10_000, 5, 1000);
        s.join_swarm(AGENT_A, sid);
        s.join_swarm(AGENT_B, sid);
        (s, sid)
    }

    #[test]
    fn test_create_and_join_swarm() {
        let (s, sid) = setup_active_swarm();
        let swarm = s.swarms.get(&sid).unwrap();
        assert_eq!(swarm.agents.len(), 3); // coordinator + 2
        assert_eq!(swarm.status, SwarmStatus::Active);
        assert_eq!(swarm.objective, "classify images");
    }

    #[test]
    fn test_assign_task_and_submit() {
        let (mut s, sid) = setup_active_swarm();
        let tid = s.assign_task(COORDINATOR, sid,
            "classify batch-42".into(),
            vec![AGENT_A, AGENT_B], 5000, 100);
        assert_eq!(tid, 1);

        s.submit_result(AGENT_A, tid, vec![1, 0]); // "cat"
        s.submit_result(AGENT_B, tid, vec![1, 0]); // "cat"

        let task = s.tasks.get(&tid).unwrap();
        assert_eq!(task.results.len(), 2);
    }

    #[test]
    fn test_majority_consensus() {
        let (mut s, sid) = setup_active_swarm();
        s.join_swarm(AGENT_C, sid);
        let tid = s.assign_task(COORDINATOR, sid,
            "sentiment".into(),
            vec![AGENT_A, AGENT_B, AGENT_C], 5000, 50);

        s.submit_result(AGENT_A, tid, vec![1]); // positive
        s.submit_result(AGENT_B, tid, vec![0]); // negative
        s.submit_result(AGENT_C, tid, vec![1]); // positive

        let result = s.reach_consensus(COORDINATOR, tid);
        assert_eq!(result, Some(vec![1])); // majority says positive
    }

    #[test]
    fn test_unanimous_consensus_fails_on_disagreement() {
        let mut s = SwarmState::new(COORDINATOR);
        let sid = s.create_swarm(COORDINATOR, "critical task".into(),
            ConsensusMethod::Unanimous, 10_000, 5, 1000);
        s.join_swarm(AGENT_A, sid);

        let tid = s.assign_task(COORDINATOR, sid,
            "verify".into(), vec![COORDINATOR, AGENT_A], 5000, 100);

        s.submit_result(COORDINATOR, tid, vec![1]);
        s.submit_result(AGENT_A, tid, vec![0]);

        let result = s.reach_consensus(COORDINATOR, tid);
        assert_eq!(result, None); // disagreement
    }

    #[test]
    fn test_distribute_rewards() {
        let (mut s, sid) = setup_active_swarm();
        let tid = s.assign_task(COORDINATOR, sid,
            "task".into(), vec![AGENT_A, AGENT_B], 5000, 200);

        s.submit_result(AGENT_A, tid, vec![42]);
        s.submit_result(AGENT_B, tid, vec![42]);
        s.reach_consensus(COORDINATOR, tid);

        let rewards = s.distribute_rewards(tid);
        assert_eq!(rewards.len(), 2);
        assert_eq!(rewards[0].1, 200);
        assert_eq!(rewards[1].1, 200);

        let swarm = s.swarms.get(&sid).unwrap();
        assert_eq!(swarm.spent, 400);
    }

    #[test]
    fn test_complete_swarm() {
        let (mut s, sid) = setup_active_swarm();
        s.complete_swarm(COORDINATOR, sid);
        assert_eq!(s.swarms.get(&sid).unwrap().status, SwarmStatus::Completed);
    }

    #[test]
    #[should_panic(expected = "already a member")]
    fn test_double_join_rejected() {
        let (mut s, sid) = setup_active_swarm();
        s.join_swarm(AGENT_A, sid);
    }
}
