use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-77  Swarm Consensus Protocol
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vote {
    pub option_index: usize,
    pub confidence: u64,
    pub reasoning_hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmDecision {
    pub id: u64,
    pub proposer: Address,
    pub question: String,
    pub options: Vec<String>,
    pub votes: BTreeMap<Address, Vote>,
    pub weights: BTreeMap<Address, u64>,
    pub deadline: u64,
    pub consensus_threshold: u16, // basis points, e.g. 6000 = 60%
    pub result: Option<usize>,
    pub resolved: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmConsensusState {
    pub owner: Address,
    pub decisions: BTreeMap<u64, SwarmDecision>,
    pub next_id: u64,
}

impl SwarmConsensusState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            decisions: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn propose_decision(
        &mut self,
        caller: Address,
        question: String,
        options: Vec<String>,
        deadline: u64,
        consensus_threshold: u16,
    ) -> u64 {
        assert!(!question.is_empty(), "DRC77: question required");
        assert!(options.len() >= 2, "DRC77: at least 2 options required");
        assert!(consensus_threshold > 0 && consensus_threshold <= 10000, "DRC77: threshold must be 1-10000 bps");

        let id = self.next_id;
        self.next_id += 1;
        self.decisions.insert(id, SwarmDecision {
            id,
            proposer: caller,
            question,
            options,
            votes: BTreeMap::new(),
            weights: BTreeMap::new(),
            deadline,
            consensus_threshold,
            result: None,
            resolved: false,
        });
        id
    }

    pub fn register_voter(&mut self, caller: Address, decision_id: u64, voter: Address, weight: u64) {
        let decision = self.decisions.get_mut(&decision_id).expect("DRC77: decision not found");
        assert!(caller == decision.proposer, "DRC77: only proposer can register voters");
        assert!(!decision.resolved, "DRC77: already resolved");
        assert!(weight > 0, "DRC77: weight must be positive");
        decision.weights.insert(voter, weight);
    }

    pub fn vote(
        &mut self,
        caller: Address,
        decision_id: u64,
        option_index: usize,
        confidence: u64,
        reasoning_hash: [u8; 32],
        current_time: u64,
    ) {
        let decision = self.decisions.get_mut(&decision_id).expect("DRC77: decision not found");
        assert!(!decision.resolved, "DRC77: already resolved");
        assert!(current_time <= decision.deadline, "DRC77: voting period ended");
        assert!(option_index < decision.options.len(), "DRC77: invalid option index");
        assert!(decision.weights.contains_key(&caller), "DRC77: not a registered voter");
        assert!(confidence > 0 && confidence <= 100, "DRC77: confidence must be 1-100");

        decision.votes.insert(caller, Vote {
            option_index,
            confidence,
            reasoning_hash,
        });
    }

    /// Resolve the decision using weighted consensus.
    /// Returns the winning option index if consensus was reached.
    pub fn resolve(&mut self, decision_id: u64) -> Option<usize> {
        let decision = self.decisions.get(&decision_id).expect("DRC77: decision not found");
        assert!(!decision.resolved, "DRC77: already resolved");

        let num_options = decision.options.len();
        // Calculate weighted votes per option
        let mut option_scores: Vec<u64> = vec![0; num_options];
        let mut total_weight_voted: u64 = 0;

        for (voter, vote) in &decision.votes {
            let weight = decision.weights.get(voter).copied().unwrap_or(0);
            let weighted_score = weight * vote.confidence;
            option_scores[vote.option_index] += weighted_score;
            total_weight_voted += weighted_score;
        }

        if total_weight_voted == 0 {
            let decision = self.decisions.get_mut(&decision_id).unwrap();
            decision.resolved = true;
            return None;
        }

        // Find winner and check if it meets threshold
        let (winner_idx, winner_score) = option_scores.iter()
            .enumerate()
            .max_by_key(|(_, s)| *s)
            .unwrap();

        let winner_pct_bps = (winner_score * 10000) / total_weight_voted;

        let decision = self.decisions.get_mut(&decision_id).unwrap();
        decision.resolved = true;

        if winner_pct_bps >= decision.consensus_threshold as u64 {
            decision.result = Some(winner_idx);
            Some(winner_idx)
        } else {
            None // No consensus reached
        }
    }

    pub fn get_result(&self, decision_id: u64) -> Option<usize> {
        let decision = self.decisions.get(&decision_id).expect("DRC77: decision not found");
        decision.result
    }

    pub fn active_decisions(&self) -> Vec<&SwarmDecision> {
        self.decisions.values()
            .filter(|d| !d.resolved)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ProposeArgs { question: String, options: Vec<String>, deadline: u64, consensus_threshold: u16 }
#[derive(Serialize, Deserialize, Debug)]
struct RegisterVoterArgs { decision_id: u64, voter: Address, weight: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct VoteArgs { decision_id: u64, option_index: usize, confidence: u64, reasoning_hash: [u8; 32], current_time: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct IdArgs { decision_id: u64 }

pub fn dispatch(
    state: &mut Option<SwarmConsensusState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC77: already initialised");
            *state = Some(SwarmConsensusState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "propose_decision" => {
            let s = state.as_mut().expect("DRC77: not initialised");
            let a: ProposeArgs = serde_json::from_slice(args).expect("DRC77: bad args");
            let id = s.propose_decision(caller, a.question, a.options, a.deadline, a.consensus_threshold);
            serde_json::to_vec(&id).unwrap()
        }
        "register_voter" => {
            let s = state.as_mut().expect("DRC77: not initialised");
            let a: RegisterVoterArgs = serde_json::from_slice(args).expect("DRC77: bad args");
            s.register_voter(caller, a.decision_id, a.voter, a.weight);
            serde_json::to_vec("ok").unwrap()
        }
        "vote" => {
            let s = state.as_mut().expect("DRC77: not initialised");
            let a: VoteArgs = serde_json::from_slice(args).expect("DRC77: bad args");
            s.vote(caller, a.decision_id, a.option_index, a.confidence, a.reasoning_hash, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "resolve" => {
            let s = state.as_mut().expect("DRC77: not initialised");
            let a: IdArgs = serde_json::from_slice(args).expect("DRC77: bad args");
            let result = s.resolve(a.decision_id);
            serde_json::to_vec(&result).unwrap()
        }
        "get_result" => {
            let s = state.as_ref().expect("DRC77: not initialised");
            let a: IdArgs = serde_json::from_slice(args).expect("DRC77: bad args");
            serde_json::to_vec(&s.get_result(a.decision_id)).unwrap()
        }
        "active_decisions" => {
            let s = state.as_ref().expect("DRC77: not initialised");
            serde_json::to_vec(&s.active_decisions()).unwrap()
        }
        _ => panic!("DRC77: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const ALICE: Address = [1u8; 32];
    const BOB: Address = [2u8; 32];
    const CAROL: Address = [3u8; 32];

    fn setup_decision() -> (SwarmConsensusState, u64) {
        let mut s = SwarmConsensusState::new(OWNER);
        let id = s.propose_decision(
            OWNER,
            "Which route should the swarm take?".into(),
            vec!["Route A".into(), "Route B".into(), "Route C".into()],
            10000,
            5000, // 50% threshold
        );
        s.register_voter(OWNER, id, ALICE, 10);
        s.register_voter(OWNER, id, BOB, 20);
        s.register_voter(OWNER, id, CAROL, 5);
        (s, id)
    }

    #[test]
    fn test_propose_and_vote() {
        let (mut s, id) = setup_decision();
        s.vote(ALICE, id, 0, 80, [0xAA; 32], 1000);
        s.vote(BOB, id, 0, 90, [0xBB; 32], 1001);
        s.vote(CAROL, id, 1, 50, [0xCC; 32], 1002);

        let result = s.resolve(id);
        assert_eq!(result, Some(0)); // Route A wins
    }

    #[test]
    fn test_no_consensus_reached() {
        let (mut s, id) = setup_decision();
        // Even split — no single option gets 50%
        s.vote(ALICE, id, 0, 80, [0xAA; 32], 1000); // 10*80 = 800
        s.vote(BOB, id, 1, 90, [0xBB; 32], 1001);   // 20*90 = 1800
        s.vote(CAROL, id, 2, 50, [0xCC; 32], 1002);  // 5*50 = 250

        // Route B has 1800/2850 = 63.1% > 50%, so it actually wins
        let result = s.resolve(id);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_high_threshold_no_consensus() {
        let mut s = SwarmConsensusState::new(OWNER);
        let id = s.propose_decision(
            OWNER, "Test".into(),
            vec!["A".into(), "B".into()],
            10000, 9000, // 90% threshold
        );
        s.register_voter(OWNER, id, ALICE, 10);
        s.register_voter(OWNER, id, BOB, 10);

        s.vote(ALICE, id, 0, 80, [0; 32], 1000);
        s.vote(BOB, id, 1, 80, [0; 32], 1001);

        let result = s.resolve(id);
        assert!(result.is_none()); // 50/50 split, needs 90%
    }

    #[test]
    fn test_active_decisions() {
        let (mut s, id) = setup_decision();
        assert_eq!(s.active_decisions().len(), 1);
        s.vote(ALICE, id, 0, 80, [0; 32], 1000);
        s.resolve(id);
        assert_eq!(s.active_decisions().len(), 0);
    }

    #[test]
    #[should_panic(expected = "not a registered voter")]
    fn test_unregistered_voter() {
        let (mut s, id) = setup_decision();
        let unknown: Address = [99u8; 32];
        s.vote(unknown, id, 0, 80, [0; 32], 1000);
    }

    #[test]
    #[should_panic(expected = "at least 2 options")]
    fn test_too_few_options() {
        let mut s = SwarmConsensusState::new(OWNER);
        s.propose_decision(OWNER, "Bad".into(), vec!["Only one".into()], 10000, 5000);
    }
}
