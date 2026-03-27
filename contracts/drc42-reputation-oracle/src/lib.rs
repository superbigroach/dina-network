use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-42  Cross-Chain Reputation Oracle
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReputationRecord {
    pub address: Address,
    pub scores: BTreeMap<String, u64>, // source -> score
    pub aggregate_score: u64,
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReputationOracleState {
    pub admin: Address,
    pub authorized_sources: Vec<Address>,
    pub records: BTreeMap<Address, ReputationRecord>,
}

impl ReputationOracleState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            authorized_sources: vec![admin],
            records: BTreeMap::new(),
        }
    }

    pub fn authorize_source(&mut self, caller: Address, source: Address) {
        assert!(
            caller == self.admin,
            "DRC42: only admin can authorize sources"
        );
        assert!(
            !self.authorized_sources.contains(&source),
            "DRC42: source already authorized"
        );
        self.authorized_sources.push(source);
    }

    pub fn update_score(
        &mut self,
        caller: Address,
        source: String,
        address: Address,
        score: u64,
        timestamp: u64,
    ) {
        assert!(
            self.authorized_sources.contains(&caller) || caller == self.admin,
            "DRC42: caller not an authorized source"
        );
        assert!(score <= 10000, "DRC42: score must be 0-10000");

        let record = self
            .records
            .entry(address)
            .or_insert_with(|| ReputationRecord {
                address,
                scores: BTreeMap::new(),
                aggregate_score: 0,
                last_updated: 0,
            });

        record.scores.insert(source, score);
        record.last_updated = timestamp;

        // Recalculate aggregate as average of all source scores
        if record.scores.is_empty() {
            record.aggregate_score = 0;
        } else {
            let sum: u64 = record.scores.values().sum();
            record.aggregate_score = sum / record.scores.len() as u64;
        }
    }

    pub fn get_reputation(&self, address: &Address) -> Option<&ReputationRecord> {
        self.records.get(address)
    }

    pub fn get_score_from_source(&self, address: &Address, source: &str) -> Option<u64> {
        self.records
            .get(address)
            .and_then(|r| r.scores.get(source).copied())
    }

    pub fn top_rated(&self, limit: usize) -> Vec<&ReputationRecord> {
        let mut sorted: Vec<&ReputationRecord> = self.records.values().collect();
        sorted.sort_by(|a, b| b.aggregate_score.cmp(&a.aggregate_score));
        sorted.truncate(limit);
        sorted
    }

    pub fn aggregate_all(&self) -> Vec<(Address, u64)> {
        self.records
            .values()
            .map(|r| (r.address, r.aggregate_score))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct AuthorizeSourceArgs {
    source: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateScoreArgs {
    source: String,
    address: Address,
    score: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetReputationArgs {
    address: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetScoreFromSourceArgs {
    address: Address,
    source: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TopRatedArgs {
    limit: usize,
}

pub fn dispatch(
    state: &mut Option<ReputationOracleState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC42: already initialised");
            *state = Some(ReputationOracleState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "authorize_source" => {
            let s = state.as_mut().expect("DRC42: not initialised");
            let a: AuthorizeSourceArgs =
                serde_json::from_slice(args).expect("DRC42: bad authorize_source args");
            s.authorize_source(caller, a.source);
            serde_json::to_vec("ok").unwrap()
        }
        "update_score" => {
            let s = state.as_mut().expect("DRC42: not initialised");
            let a: UpdateScoreArgs =
                serde_json::from_slice(args).expect("DRC42: bad update_score args");
            s.update_score(caller, a.source, a.address, a.score, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "get_reputation" => {
            let s = state.as_ref().expect("DRC42: not initialised");
            let a: GetReputationArgs =
                serde_json::from_slice(args).expect("DRC42: bad get_reputation args");
            serde_json::to_vec(&s.get_reputation(&a.address)).unwrap()
        }
        "get_score_from_source" => {
            let s = state.as_ref().expect("DRC42: not initialised");
            let a: GetScoreFromSourceArgs =
                serde_json::from_slice(args).expect("DRC42: bad get_score_from_source args");
            serde_json::to_vec(&s.get_score_from_source(&a.address, &a.source)).unwrap()
        }
        "top_rated" => {
            let s = state.as_ref().expect("DRC42: not initialised");
            let a: TopRatedArgs =
                serde_json::from_slice(args).expect("DRC42: bad top_rated args");
            serde_json::to_vec(&s.top_rated(a.limit)).unwrap()
        }
        "aggregate_all" => {
            let s = state.as_ref().expect("DRC42: not initialised");
            serde_json::to_vec(&s.aggregate_all()).unwrap()
        }
        _ => panic!("DRC42: unknown method '{method}'"),
    }
}
