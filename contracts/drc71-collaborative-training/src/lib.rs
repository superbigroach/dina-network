use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-71  Federated Learning Coordination
// ---------------------------------------------------------------------------
// Coordinate federated ML training across multiple Cognitum Seeds.
// Participants submit gradient updates per round, the coordinator
// aggregates, and rewards are distributed proportionally.

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TrainingStatus {
    Recruiting,
    InProgress,
    Aggregating,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AggregationMethod {
    FedAvg,
    FedProx,
    Custom(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParticipantUpdate {
    pub address: Address,
    pub gradient_hash: Vec<u8>,
    pub round: u64,
    pub metrics: BTreeMap<String, u64>, // e.g., "loss" -> 2500 (fixed-point x10000)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrainingSession {
    pub id: u64,
    pub coordinator: Address,
    pub model_hash: Vec<u8>,
    pub participants: Vec<Address>,
    pub round: u64,
    pub max_rounds: u64,
    pub min_participants: u64,
    pub aggregation_method: AggregationMethod,
    pub status: TrainingStatus,
    pub reward_per_round: u64,
    pub updates: Vec<ParticipantUpdate>,
    pub aggregated_hashes: Vec<Vec<u8>>, // one per completed round
    pub total_rewards_distributed: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrainingState {
    pub owner: Address,
    pub sessions: BTreeMap<u64, TrainingSession>,
    pub next_session_id: u64,
}

impl TrainingState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            sessions: BTreeMap::new(),
            next_session_id: 1,
        }
    }

    pub fn create_session(
        &mut self,
        caller: Address,
        model_hash: Vec<u8>,
        max_rounds: u64,
        min_participants: u64,
        aggregation_method: AggregationMethod,
        reward_per_round: u64,
    ) -> u64 {
        assert!(max_rounds > 0, "DRC71: max_rounds must be > 0");
        assert!(min_participants >= 2, "DRC71: need >= 2 participants");
        assert!(!model_hash.is_empty(), "DRC71: model hash required");

        let id = self.next_session_id;
        self.next_session_id += 1;
        self.sessions.insert(id, TrainingSession {
            id,
            coordinator: caller,
            model_hash,
            participants: Vec::new(),
            round: 0,
            max_rounds,
            min_participants,
            aggregation_method,
            status: TrainingStatus::Recruiting,
            reward_per_round,
            updates: Vec::new(),
            aggregated_hashes: Vec::new(),
            total_rewards_distributed: 0,
        });
        id
    }

    pub fn join_training(&mut self, caller: Address, session_id: u64) {
        let session = self.sessions.get_mut(&session_id).expect("DRC71: session not found");
        assert!(session.status == TrainingStatus::Recruiting,
            "DRC71: not recruiting");
        assert!(!session.participants.contains(&caller), "DRC71: already joined");
        session.participants.push(caller);

        if session.participants.len() as u64 >= session.min_participants {
            session.status = TrainingStatus::InProgress;
            session.round = 1;
        }
    }

    pub fn submit_update(
        &mut self,
        caller: Address,
        session_id: u64,
        gradient_hash: Vec<u8>,
        metrics: BTreeMap<String, u64>,
    ) {
        let session = self.sessions.get_mut(&session_id).expect("DRC71: session not found");
        assert!(session.status == TrainingStatus::InProgress, "DRC71: not in progress");
        assert!(session.participants.contains(&caller), "DRC71: not a participant");

        // Check no duplicate submission for current round
        let already = session.updates.iter().any(|u|
            u.address == caller && u.round == session.round);
        assert!(!already, "DRC71: already submitted for this round");

        session.updates.push(ParticipantUpdate {
            address: caller,
            gradient_hash,
            round: session.round,
            metrics,
        });

        // If all participants submitted, move to aggregating
        let round_submissions = session.updates.iter()
            .filter(|u| u.round == session.round)
            .count();
        if round_submissions == session.participants.len() {
            session.status = TrainingStatus::Aggregating;
        }
    }

    pub fn aggregate_round(
        &mut self,
        caller: Address,
        session_id: u64,
        aggregated_hash: Vec<u8>,
    ) {
        let session = self.sessions.get_mut(&session_id).expect("DRC71: session not found");
        assert!(caller == session.coordinator, "DRC71: only coordinator");
        assert!(session.status == TrainingStatus::Aggregating, "DRC71: not aggregating");

        session.aggregated_hashes.push(aggregated_hash);
        session.model_hash = session.aggregated_hashes.last().unwrap().clone();

        // Distribute rewards for this round
        let reward_each = session.reward_per_round / session.participants.len() as u64;
        session.total_rewards_distributed += reward_each * session.participants.len() as u64;

        if session.round >= session.max_rounds {
            session.status = TrainingStatus::Completed;
        } else {
            session.round += 1;
            session.status = TrainingStatus::InProgress;
        }
    }

    pub fn complete_training(&mut self, caller: Address, session_id: u64) {
        let session = self.sessions.get_mut(&session_id).expect("DRC71: session not found");
        assert!(caller == session.coordinator, "DRC71: only coordinator");
        assert!(session.status != TrainingStatus::Completed, "DRC71: already completed");
        session.status = TrainingStatus::Completed;
    }

    pub fn session_metrics(&self, session_id: u64, round: u64) -> Vec<&ParticipantUpdate> {
        let session = self.sessions.get(&session_id).expect("DRC71: session not found");
        session.updates.iter()
            .filter(|u| u.round == round)
            .collect()
    }

    pub fn distribute_training_rewards(&self, session_id: u64) -> Vec<(Address, u64)> {
        let session = self.sessions.get(&session_id).expect("DRC71: session not found");
        let completed_rounds = session.aggregated_hashes.len() as u64;
        let reward_each = session.reward_per_round * completed_rounds / session.participants.len() as u64;
        session.participants.iter().map(|p| (*p, reward_each)).collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateSessionArgs { model_hash: Vec<u8>, max_rounds: u64, min_participants: u64, aggregation_method: AggregationMethod, reward_per_round: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct JoinArgs { session_id: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct SubmitUpdateArgs { session_id: u64, gradient_hash: Vec<u8>, metrics: BTreeMap<String, u64> }
#[derive(Serialize, Deserialize, Debug)]
struct AggregateArgs { session_id: u64, aggregated_hash: Vec<u8> }
#[derive(Serialize, Deserialize, Debug)]
struct SessionIdArgs { session_id: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct SessionRoundArgs { session_id: u64, round: u64 }

pub fn dispatch(
    state: &mut Option<TrainingState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC71: already initialised");
            *state = Some(TrainingState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_session" => {
            let s = state.as_mut().expect("DRC71: not initialised");
            let a: CreateSessionArgs = serde_json::from_slice(args).expect("DRC71: bad args");
            let id = s.create_session(caller, a.model_hash, a.max_rounds, a.min_participants, a.aggregation_method, a.reward_per_round);
            serde_json::to_vec(&id).unwrap()
        }
        "join_training" => {
            let s = state.as_mut().expect("DRC71: not initialised");
            let a: JoinArgs = serde_json::from_slice(args).expect("DRC71: bad args");
            s.join_training(caller, a.session_id);
            serde_json::to_vec("ok").unwrap()
        }
        "submit_update" => {
            let s = state.as_mut().expect("DRC71: not initialised");
            let a: SubmitUpdateArgs = serde_json::from_slice(args).expect("DRC71: bad args");
            s.submit_update(caller, a.session_id, a.gradient_hash, a.metrics);
            serde_json::to_vec("ok").unwrap()
        }
        "aggregate_round" => {
            let s = state.as_mut().expect("DRC71: not initialised");
            let a: AggregateArgs = serde_json::from_slice(args).expect("DRC71: bad args");
            s.aggregate_round(caller, a.session_id, a.aggregated_hash);
            serde_json::to_vec("ok").unwrap()
        }
        "complete_training" => {
            let s = state.as_mut().expect("DRC71: not initialised");
            let a: SessionIdArgs = serde_json::from_slice(args).expect("DRC71: bad args");
            s.complete_training(caller, a.session_id);
            serde_json::to_vec("ok").unwrap()
        }
        "session_metrics" => {
            let s = state.as_ref().expect("DRC71: not initialised");
            let a: SessionRoundArgs = serde_json::from_slice(args).expect("DRC71: bad args");
            serde_json::to_vec(&s.session_metrics(a.session_id, a.round)).unwrap()
        }
        "distribute_training_rewards" => {
            let s = state.as_ref().expect("DRC71: not initialised");
            let a: SessionIdArgs = serde_json::from_slice(args).expect("DRC71: bad args");
            serde_json::to_vec(&s.distribute_training_rewards(a.session_id)).unwrap()
        }
        _ => panic!("DRC71: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const COORDINATOR: Address = [1u8; 32];
    const PARTICIPANT_A: Address = [2u8; 32];
    const PARTICIPANT_B: Address = [3u8; 32];

    fn setup_session() -> (TrainingState, u64) {
        let mut s = TrainingState::new(COORDINATOR);
        let sid = s.create_session(
            COORDINATOR, vec![0xAA, 0xBB], 3, 2,
            AggregationMethod::FedAvg, 1000,
        );
        (s, sid)
    }

    fn setup_active_session() -> (TrainingState, u64) {
        let (mut s, sid) = setup_session();
        s.join_training(PARTICIPANT_A, sid);
        s.join_training(PARTICIPANT_B, sid);
        (s, sid)
    }

    #[test]
    fn test_create_and_join() {
        let (s, sid) = setup_active_session();
        let session = s.sessions.get(&sid).unwrap();
        assert_eq!(session.participants.len(), 2);
        assert_eq!(session.status, TrainingStatus::InProgress);
        assert_eq!(session.round, 1);
    }

    #[test]
    fn test_submit_updates_triggers_aggregating() {
        let (mut s, sid) = setup_active_session();
        let mut m = BTreeMap::new();
        m.insert("loss".into(), 2500);

        s.submit_update(PARTICIPANT_A, sid, vec![0x11], m.clone());
        assert_eq!(s.sessions.get(&sid).unwrap().status, TrainingStatus::InProgress);

        s.submit_update(PARTICIPANT_B, sid, vec![0x22], m);
        assert_eq!(s.sessions.get(&sid).unwrap().status, TrainingStatus::Aggregating);
    }

    #[test]
    fn test_aggregate_advances_round() {
        let (mut s, sid) = setup_active_session();
        let m = BTreeMap::new();
        s.submit_update(PARTICIPANT_A, sid, vec![1], m.clone());
        s.submit_update(PARTICIPANT_B, sid, vec![2], m);

        s.aggregate_round(COORDINATOR, sid, vec![0xFF]);
        let session = s.sessions.get(&sid).unwrap();
        assert_eq!(session.round, 2);
        assert_eq!(session.status, TrainingStatus::InProgress);
        assert_eq!(session.aggregated_hashes.len(), 1);
        assert_eq!(session.model_hash, vec![0xFF]);
    }

    #[test]
    fn test_full_training_flow() {
        let (mut s, sid) = setup_active_session();
        let m = BTreeMap::new();

        for round_expected in 1..=3 {
            let session = s.sessions.get(&sid).unwrap();
            assert_eq!(session.round, round_expected);

            s.submit_update(PARTICIPANT_A, sid, vec![round_expected as u8], m.clone());
            s.submit_update(PARTICIPANT_B, sid, vec![round_expected as u8], m.clone());
            s.aggregate_round(COORDINATOR, sid, vec![round_expected as u8, 0xFF]);
        }

        let session = s.sessions.get(&sid).unwrap();
        assert_eq!(session.status, TrainingStatus::Completed);
        assert_eq!(session.aggregated_hashes.len(), 3);
    }

    #[test]
    fn test_distribute_rewards() {
        let (mut s, sid) = setup_active_session();
        let m = BTreeMap::new();
        // Complete 2 rounds
        for _ in 0..2 {
            s.submit_update(PARTICIPANT_A, sid, vec![1], m.clone());
            s.submit_update(PARTICIPANT_B, sid, vec![1], m.clone());
            s.aggregate_round(COORDINATOR, sid, vec![0xAA]);
        }

        let rewards = s.distribute_training_rewards(sid);
        assert_eq!(rewards.len(), 2);
        // 1000 reward_per_round * 2 rounds / 2 participants = 1000 each
        assert_eq!(rewards[0].1, 1000);
        assert_eq!(rewards[1].1, 1000);
    }

    #[test]
    fn test_session_metrics() {
        let (mut s, sid) = setup_active_session();
        let mut m = BTreeMap::new();
        m.insert("loss".into(), 3000);
        s.submit_update(PARTICIPANT_A, sid, vec![1], m.clone());
        m.insert("loss".into(), 2800);
        s.submit_update(PARTICIPANT_B, sid, vec![2], m);

        let metrics = s.session_metrics(sid, 1);
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].metrics["loss"], 3000);
    }

    #[test]
    #[should_panic(expected = "already submitted")]
    fn test_duplicate_update_rejected() {
        let (mut s, sid) = setup_active_session();
        let m = BTreeMap::new();
        s.submit_update(PARTICIPANT_A, sid, vec![1], m.clone());
        s.submit_update(PARTICIPANT_A, sid, vec![2], m);
    }
}
