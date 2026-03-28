use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-42  Cross-Chain Reputation Oracle
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReputationRecord {
    pub address: Address,
    pub scores: BTreeMap<String, u64>,
    pub aggregate_score: u64,
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReputationState {
    pub oracle: Address,
    pub records: BTreeMap<Address, ReputationRecord>,
}

impl ReputationState {
    pub fn new(oracle: Address) -> Self {
        Self {
            oracle,
            records: BTreeMap::new(),
        }
    }

    pub fn update_score(
        &mut self,
        caller: Address,
        source: String,
        addr: Address,
        score: u64,
        timestamp: u64,
    ) {
        assert!(
            caller == self.oracle,
            "DRC42: only oracle can update scores"
        );
        assert!(score <= 1000, "DRC42: score must be 0..=1000");

        let record = self
            .records
            .entry(addr)
            .or_insert_with(|| ReputationRecord {
                address: addr,
                scores: BTreeMap::new(),
                aggregate_score: 0,
                last_updated: 0,
            });

        record.scores.insert(source, score);
        record.last_updated = timestamp;

        // Recompute aggregate as the mean of all source scores
        let total: u64 = record.scores.values().sum();
        let count = record.scores.len() as u64;
        record.aggregate_score = total / count;
    }

    pub fn get_reputation(&self, addr: &Address) -> Option<&ReputationRecord> {
        self.records.get(addr)
    }

    pub fn get_score_from_source(&self, addr: &Address, source: &str) -> Option<u64> {
        self.records
            .get(addr)
            .and_then(|r| r.scores.get(source).copied())
    }

    pub fn top_rated(&self, limit: usize) -> Vec<&ReputationRecord> {
        let mut entries: Vec<&ReputationRecord> = self.records.values().collect();
        entries.sort_by(|a, b| b.aggregate_score.cmp(&a.aggregate_score));
        entries.truncate(limit);
        entries
    }

    pub fn aggregate_all(&mut self) {
        for record in self.records.values_mut() {
            if record.scores.is_empty() {
                record.aggregate_score = 0;
            } else {
                let total: u64 = record.scores.values().sum();
                let count = record.scores.len() as u64;
                record.aggregate_score = total / count;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct UpdateScoreArgs {
    source: String,
    addr: Address,
    score: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetReputationArgs {
    addr: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetScoreFromSourceArgs {
    addr: Address,
    source: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TopRatedArgs {
    limit: usize,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<ReputationState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC42: already initialised");
            *state = Some(ReputationState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "update_score" => {
            let s = state.as_mut().expect("DRC42: not initialised");
            let a: UpdateScoreArgs =
                serde_json::from_slice(args).expect("DRC42: bad update_score args");
            s.update_score(caller, a.source, a.addr, a.score, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }

        "get_reputation" => {
            let s = state.as_ref().expect("DRC42: not initialised");
            let a: GetReputationArgs =
                serde_json::from_slice(args).expect("DRC42: bad get_reputation args");
            let rep = s.get_reputation(&a.addr);
            serde_json::to_vec(&rep).unwrap()
        }

        "get_score_from_source" => {
            let s = state.as_ref().expect("DRC42: not initialised");
            let a: GetScoreFromSourceArgs =
                serde_json::from_slice(args).expect("DRC42: bad get_score_from_source args");
            let score = s.get_score_from_source(&a.addr, &a.source);
            serde_json::to_vec(&score).unwrap()
        }

        "top_rated" => {
            let s = state.as_ref().expect("DRC42: not initialised");
            let a: TopRatedArgs = serde_json::from_slice(args).expect("DRC42: bad top_rated args");
            let top = s.top_rated(a.limit);
            serde_json::to_vec(&top).unwrap()
        }

        "aggregate_all" => {
            let s = state.as_mut().expect("DRC42: not initialised");
            s.aggregate_all();
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC42: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ORACLE: Address = [1u8; 32];
    const ALICE: Address = [2u8; 32];
    const BOB: Address = [3u8; 32];
    const CAROL: Address = [4u8; 32];

    fn init() -> Option<ReputationState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", ORACLE);
        state
    }

    fn update(
        state: &mut Option<ReputationState>,
        source: &str,
        addr: Address,
        score: u64,
        ts: u64,
    ) {
        let args = serde_json::to_vec(&UpdateScoreArgs {
            source: source.to_string(),
            addr,
            score,
            timestamp: ts,
        })
        .unwrap();
        dispatch(state, "update_score", &args, ORACLE);
    }

    #[test]
    fn test_update_and_get_reputation() {
        let mut state = init();
        update(&mut state, "ethereum", ALICE, 800, 1000);
        update(&mut state, "polygon", ALICE, 600, 1001);

        let args = serde_json::to_vec(&GetReputationArgs { addr: ALICE }).unwrap();
        let result = dispatch(&mut state, "get_reputation", &args, ORACLE);
        let rep: Option<ReputationRecord> = serde_json::from_slice(&result).unwrap();
        let rep = rep.unwrap();
        assert_eq!(rep.aggregate_score, 700); // (800+600)/2
        assert_eq!(rep.scores.len(), 2);
        assert_eq!(rep.last_updated, 1001);
    }

    #[test]
    fn test_get_score_from_source() {
        let mut state = init();
        update(&mut state, "ethereum", ALICE, 900, 100);
        update(&mut state, "base", ALICE, 500, 200);

        let args = serde_json::to_vec(&GetScoreFromSourceArgs {
            addr: ALICE,
            source: "ethereum".to_string(),
        })
        .unwrap();
        let result = dispatch(&mut state, "get_score_from_source", &args, ORACLE);
        let score: Option<u64> = serde_json::from_slice(&result).unwrap();
        assert_eq!(score, Some(900));

        // Non-existent source
        let args = serde_json::to_vec(&GetScoreFromSourceArgs {
            addr: ALICE,
            source: "solana".to_string(),
        })
        .unwrap();
        let result = dispatch(&mut state, "get_score_from_source", &args, ORACLE);
        let score: Option<u64> = serde_json::from_slice(&result).unwrap();
        assert_eq!(score, None);
    }

    #[test]
    fn test_top_rated() {
        let mut state = init();
        update(&mut state, "eth", ALICE, 300, 1);
        update(&mut state, "eth", BOB, 900, 2);
        update(&mut state, "eth", CAROL, 600, 3);

        let args = serde_json::to_vec(&TopRatedArgs { limit: 2 }).unwrap();
        let result = dispatch(&mut state, "top_rated", &args, ORACLE);
        let top: Vec<ReputationRecord> = serde_json::from_slice(&result).unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].address, BOB); // 900
        assert_eq!(top[1].address, CAROL); // 600
    }

    #[test]
    fn test_aggregate_all() {
        let mut state = init();
        update(&mut state, "a", ALICE, 400, 1);
        update(&mut state, "b", ALICE, 800, 2);

        // Manually tamper aggregate to verify aggregate_all recomputes
        state
            .as_mut()
            .unwrap()
            .records
            .get_mut(&ALICE)
            .unwrap()
            .aggregate_score = 0;

        dispatch(&mut state, "aggregate_all", b"", ORACLE);

        let rec = state.as_ref().unwrap().records.get(&ALICE).unwrap();
        assert_eq!(rec.aggregate_score, 600); // (400+800)/2
    }

    #[test]
    #[should_panic(expected = "only oracle")]
    fn test_unauthorized_update() {
        let mut state = init();
        let args = serde_json::to_vec(&UpdateScoreArgs {
            source: "eth".to_string(),
            addr: ALICE,
            score: 500,
            timestamp: 1,
        })
        .unwrap();
        dispatch(&mut state, "update_score", &args, BOB); // BOB is not oracle
    }

    #[test]
    #[should_panic(expected = "score must be 0..=1000")]
    fn test_score_out_of_range() {
        let mut state = init();
        update(&mut state, "eth", ALICE, 1001, 1);
    }
}
