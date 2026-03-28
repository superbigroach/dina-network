use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-74  Decentralized Agent Reputation Network
// ---------------------------------------------------------------------------

type Address = [u8; 32];
type VouchId = u64;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vouch {
    pub id: VouchId,
    pub from: Address,
    pub for_agent: Address,
    pub score: u8, // 1-100
    pub category: String,
    pub timestamp: u64,
    pub stake: u64, // tokens staked behind this vouch
    pub revoked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReputationState {
    pub owner: Address,
    pub vouches: BTreeMap<VouchId, Vouch>,
    pub next_vouch_id: VouchId,
    /// Raw sum of weighted scores per agent (numerator for weighted average)
    pub reputation_scores: BTreeMap<Address, u64>,
    /// Sum of weights per agent (denominator for weighted average)
    pub reputation_weights: BTreeMap<Address, u64>,
    pub balances: BTreeMap<Address, u64>,
}

impl ReputationState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            vouches: BTreeMap::new(),
            next_vouch_id: 1,
            reputation_scores: BTreeMap::new(),
            reputation_weights: BTreeMap::new(),
            balances: BTreeMap::new(),
        }
    }

    pub fn deposit(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC74: deposit must be positive");
        *self.balances.entry(caller).or_insert(0) += amount;
    }

    /// Vouch for an agent. Score 1-100, stake tokens behind it.
    /// The voucher's own reputation acts as a weight multiplier (PageRank-like).
    pub fn vouch_for(
        &mut self,
        caller: Address,
        for_agent: Address,
        score: u8,
        category: String,
        stake: u64,
        timestamp: u64,
    ) -> VouchId {
        assert!(score >= 1 && score <= 100, "DRC74: score must be 1-100");
        assert!(!category.is_empty(), "DRC74: category required");
        assert!(caller != for_agent, "DRC74: cannot vouch for yourself");

        if stake > 0 {
            let bal = self.balances.get(&caller).copied().unwrap_or(0);
            assert!(bal >= stake, "DRC74: insufficient balance for stake");
            self.balances.insert(caller, bal - stake);
        }

        let id = self.next_vouch_id;
        self.next_vouch_id += 1;

        // Weight = voucher's own reputation (minimum 1) * stake (minimum 1)
        let voucher_rep = self.get_reputation(&caller);
        let weight = (voucher_rep.max(1) as u64) * stake.max(1);

        *self.reputation_scores.entry(for_agent).or_insert(0) += (score as u64) * weight;
        *self.reputation_weights.entry(for_agent).or_insert(0) += weight;

        self.vouches.insert(
            id,
            Vouch {
                id,
                from: caller,
                for_agent,
                score,
                category,
                timestamp,
                stake,
                revoked: false,
            },
        );

        id
    }

    /// Revoke a vouch — removes it from reputation calculation, returns stake.
    pub fn revoke_vouch(&mut self, caller: Address, vouch_id: VouchId) {
        let vouch = self.vouches.get(&vouch_id).expect("DRC74: vouch not found");
        assert!(vouch.from == caller, "DRC74: only voucher can revoke");
        assert!(!vouch.revoked, "DRC74: already revoked");

        // Capture values before mutating
        let voucher_rep = self.get_reputation(&caller);
        let stake = vouch.stake;
        let for_agent = vouch.for_agent;
        let score = vouch.score;
        let weight = (voucher_rep.max(1) as u64) * stake.max(1);
        let score_contribution = (score as u64) * weight;

        // Now mutate
        self.vouches.get_mut(&vouch_id).unwrap().revoked = true;

        let total_score = self.reputation_scores.entry(for_agent).or_insert(0);
        *total_score = total_score.saturating_sub(score_contribution);
        let total_weight = self.reputation_weights.entry(for_agent).or_insert(0);
        *total_weight = total_weight.saturating_sub(weight);

        // Return stake
        if stake > 0 {
            *self.balances.entry(caller).or_insert(0) += stake;
        }
    }

    /// Get weighted-average reputation for an agent (0-100).
    pub fn get_reputation(&self, agent: &Address) -> u32 {
        let total_weight = self.reputation_weights.get(agent).copied().unwrap_or(0);
        if total_weight == 0 {
            return 0;
        }
        let total_score = self.reputation_scores.get(agent).copied().unwrap_or(0);
        (total_score / total_weight) as u32
    }

    /// Top agents in a given category, sorted by reputation descending.
    pub fn top_agents_by_category(&self, category: &str, limit: usize) -> Vec<(Address, u32)> {
        // Collect agents that have vouches in this category
        let mut agents_in_category: BTreeMap<Address, bool> = BTreeMap::new();
        for v in self.vouches.values() {
            if !v.revoked && v.category == category {
                agents_in_category.insert(v.for_agent, true);
            }
        }

        let mut results: Vec<(Address, u32)> = agents_in_category
            .keys()
            .map(|a| (*a, self.get_reputation(a)))
            .collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.truncate(limit);
        results
    }

    /// Get all vouchers who have vouched for an agent.
    pub fn vouchers_of(&self, agent: &Address) -> Vec<&Vouch> {
        self.vouches
            .values()
            .filter(|v| &v.for_agent == agent && !v.revoked)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct VouchForArgs {
    for_agent: Address,
    score: u8,
    category: String,
    stake: u64,
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct RevokeArgs {
    vouch_id: VouchId,
}
#[derive(Serialize, Deserialize, Debug)]
struct AgentArgs {
    agent: Address,
}
#[derive(Serialize, Deserialize, Debug)]
struct CategoryArgs {
    category: String,
    limit: usize,
}
#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    amount: u64,
}

pub fn dispatch(
    state: &mut Option<ReputationState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC74: already initialised");
            *state = Some(ReputationState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "deposit" => {
            let s = state.as_mut().expect("DRC74: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC74: bad args");
            s.deposit(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "vouch_for" => {
            let s = state.as_mut().expect("DRC74: not initialised");
            let a: VouchForArgs = serde_json::from_slice(args).expect("DRC74: bad args");
            let id = s.vouch_for(
                caller,
                a.for_agent,
                a.score,
                a.category,
                a.stake,
                a.timestamp,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "revoke_vouch" => {
            let s = state.as_mut().expect("DRC74: not initialised");
            let a: RevokeArgs = serde_json::from_slice(args).expect("DRC74: bad args");
            s.revoke_vouch(caller, a.vouch_id);
            serde_json::to_vec("ok").unwrap()
        }
        "get_reputation" => {
            let s = state.as_ref().expect("DRC74: not initialised");
            let a: AgentArgs = serde_json::from_slice(args).expect("DRC74: bad args");
            serde_json::to_vec(&s.get_reputation(&a.agent)).unwrap()
        }
        "top_agents_by_category" => {
            let s = state.as_ref().expect("DRC74: not initialised");
            let a: CategoryArgs = serde_json::from_slice(args).expect("DRC74: bad args");
            serde_json::to_vec(&s.top_agents_by_category(&a.category, a.limit)).unwrap()
        }
        "vouchers_of" => {
            let s = state.as_ref().expect("DRC74: not initialised");
            let a: AgentArgs = serde_json::from_slice(args).expect("DRC74: bad args");
            serde_json::to_vec(&s.vouchers_of(&a.agent)).unwrap()
        }
        _ => panic!("DRC74: unknown method '{method}'"),
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
    const DAVE: Address = [4u8; 32];

    fn setup() -> ReputationState {
        let mut s = ReputationState::new(OWNER);
        s.deposit(ALICE, 10_000);
        s.deposit(BOB, 10_000);
        s.deposit(CAROL, 10_000);
        s
    }

    #[test]
    fn test_vouch_and_reputation() {
        let mut s = setup();
        s.vouch_for(ALICE, BOB, 80, "inference".into(), 100, 1000);
        let rep = s.get_reputation(&BOB);
        assert_eq!(rep, 80);
    }

    #[test]
    fn test_multiple_vouches_weighted() {
        let mut s = setup();
        // Alice vouches 80 with stake 100 => weight = 1*100 = 100, contribution = 8000
        s.vouch_for(ALICE, DAVE, 80, "inference".into(), 100, 1000);
        // Bob vouches 60 with stake 200 => weight = 1*200 = 200, contribution = 12000
        s.vouch_for(BOB, DAVE, 60, "inference".into(), 200, 1001);
        // total_score = 8000 + 12000 = 20000, total_weight = 300
        // reputation = 20000 / 300 = 66
        let rep = s.get_reputation(&DAVE);
        assert_eq!(rep, 66);
    }

    #[test]
    fn test_revoke_returns_stake() {
        let mut s = setup();
        let vid = s.vouch_for(ALICE, BOB, 90, "navigation".into(), 500, 1000);
        assert_eq!(s.balances.get(&ALICE).copied().unwrap_or(0), 9500);
        assert!(s.get_reputation(&BOB) > 0);

        s.revoke_vouch(ALICE, vid);
        assert_eq!(s.balances.get(&ALICE).copied().unwrap_or(0), 10_000);
        assert_eq!(s.get_reputation(&BOB), 0);
    }

    #[test]
    fn test_top_agents_by_category() {
        let mut s = setup();
        s.vouch_for(ALICE, BOB, 90, "inference".into(), 100, 1000);
        s.vouch_for(ALICE, CAROL, 70, "inference".into(), 100, 1001);
        s.vouch_for(BOB, DAVE, 50, "navigation".into(), 100, 1002);

        let top = s.top_agents_by_category("inference", 10);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, BOB); // 90 > 70
        assert_eq!(top[1].0, CAROL);
    }

    #[test]
    fn test_vouchers_of() {
        let mut s = setup();
        s.vouch_for(ALICE, DAVE, 80, "sensor".into(), 50, 1000);
        s.vouch_for(BOB, DAVE, 70, "sensor".into(), 60, 1001);
        let vouchers = s.vouchers_of(&DAVE);
        assert_eq!(vouchers.len(), 2);
    }

    #[test]
    #[should_panic(expected = "cannot vouch for yourself")]
    fn test_cannot_self_vouch() {
        let mut s = setup();
        s.vouch_for(ALICE, ALICE, 100, "cheat".into(), 0, 1000);
    }

    #[test]
    #[should_panic(expected = "score must be 1-100")]
    fn test_invalid_score() {
        let mut s = setup();
        s.vouch_for(ALICE, BOB, 0, "test".into(), 0, 1000);
    }
}
