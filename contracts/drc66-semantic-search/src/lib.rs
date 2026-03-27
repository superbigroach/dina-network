use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-66  Semantic Search Marketplace
// ---------------------------------------------------------------------------
// Pay-per-query semantic search across distributed Cognitum Seeds.
// Providers host vector indexes, requesters pay for search results,
// and quality is tracked via accuracy scores.

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchProvider {
    pub address: Address,
    pub device_id: String,
    pub index_name: String,
    pub vector_count: u64,
    pub price_per_query: u64,
    pub accuracy_score: u64, // basis points 0-10000
    pub total_queries: u64,
    pub total_ratings: u64,
    pub rating_sum: u64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchResult {
    pub provider: Address,
    pub result_hash: Vec<u8>,
    pub relevance_score: u64, // basis points 0-10000
    pub latency_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchRequest {
    pub id: u64,
    pub requester: Address,
    pub query_embedding_hash: Vec<u8>,
    pub max_results: u64,
    pub max_cost: u64,
    pub providers_used: Vec<Address>,
    pub results: Vec<SearchResult>,
    pub total_cost: u64,
    pub timestamp: u64,
    pub completed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SemanticSearchState {
    pub owner: Address,
    pub providers: BTreeMap<String, SearchProvider>, // hex(address) -> provider
    pub search_requests: BTreeMap<u64, SearchRequest>,
    pub next_request_id: u64,
}

fn addr_key(a: &Address) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

impl SemanticSearchState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            providers: BTreeMap::new(),
            search_requests: BTreeMap::new(),
            next_request_id: 1,
        }
    }

    pub fn register_provider(
        &mut self,
        caller: Address,
        device_id: String,
        index_name: String,
        vector_count: u64,
        price_per_query: u64,
    ) {
        let key = addr_key(&caller);
        assert!(!self.providers.contains_key(&key), "DRC66: already registered");
        assert!(!index_name.is_empty(), "DRC66: index name required");
        assert!(price_per_query > 0, "DRC66: price must be > 0");
        self.providers.insert(key, SearchProvider {
            address: caller,
            device_id,
            index_name,
            vector_count,
            price_per_query,
            accuracy_score: 5000, // start at 50%
            total_queries: 0,
            total_ratings: 0,
            rating_sum: 0,
            active: true,
        });
    }

    pub fn search(
        &mut self,
        caller: Address,
        query_embedding_hash: Vec<u8>,
        max_results: u64,
        max_cost: u64,
        timestamp: u64,
    ) -> u64 {
        assert!(!query_embedding_hash.is_empty(), "DRC66: empty query hash");
        assert!(max_results > 0, "DRC66: max_results must be > 0");

        // Select best active providers within budget
        let mut eligible: Vec<&SearchProvider> = self.providers.values()
            .filter(|p| p.active && p.price_per_query <= max_cost)
            .collect();
        eligible.sort_by(|a, b| b.accuracy_score.cmp(&a.accuracy_score));
        eligible.truncate(max_results as usize);

        assert!(!eligible.is_empty(), "DRC66: no eligible providers");

        let providers_used: Vec<Address> = eligible.iter().map(|p| p.address).collect();
        let total_cost: u64 = eligible.iter().map(|p| p.price_per_query).sum();
        assert!(total_cost <= max_cost, "DRC66: total cost exceeds budget");

        // Increment query counts
        for p in &providers_used {
            let key = addr_key(p);
            if let Some(provider) = self.providers.get_mut(&key) {
                provider.total_queries += 1;
            }
        }

        let id = self.next_request_id;
        self.next_request_id += 1;
        self.search_requests.insert(id, SearchRequest {
            id,
            requester: caller,
            query_embedding_hash,
            max_results,
            max_cost,
            providers_used,
            results: Vec::new(),
            total_cost,
            timestamp,
            completed: false,
        });
        id
    }

    pub fn submit_search_result(
        &mut self,
        caller: Address,
        request_id: u64,
        result_hash: Vec<u8>,
        relevance_score: u64,
        latency_ms: u64,
    ) {
        let req = self.search_requests.get_mut(&request_id).expect("DRC66: request not found");
        assert!(req.providers_used.contains(&caller), "DRC66: not a selected provider");
        assert!(!req.completed, "DRC66: already completed");
        assert!(relevance_score <= 10000, "DRC66: relevance_score max 10000");

        // Ensure no duplicate submission
        assert!(!req.results.iter().any(|r| r.provider == caller), "DRC66: already submitted");

        req.results.push(SearchResult {
            provider: caller,
            result_hash,
            relevance_score,
            latency_ms,
        });

        if req.results.len() == req.providers_used.len() {
            req.completed = true;
        }
    }

    pub fn rate_result(
        &mut self,
        caller: Address,
        request_id: u64,
        provider: Address,
        rating: u64, // 0-10000
    ) {
        let req = self.search_requests.get(&request_id).expect("DRC66: request not found");
        assert!(caller == req.requester, "DRC66: only requester can rate");
        assert!(rating <= 10000, "DRC66: rating max 10000");

        let key = addr_key(&provider);
        let p = self.providers.get_mut(&key).expect("DRC66: provider not found");
        p.total_ratings += 1;
        p.rating_sum += rating;
        p.accuracy_score = p.rating_sum / p.total_ratings;
    }

    pub fn best_providers(&self, top_n: usize) -> Vec<&SearchProvider> {
        let mut active: Vec<&SearchProvider> = self.providers.values()
            .filter(|p| p.active)
            .collect();
        active.sort_by(|a, b| b.accuracy_score.cmp(&a.accuracy_score));
        active.truncate(top_n);
        active
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterArgs { device_id: String, index_name: String, vector_count: u64, price_per_query: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct SearchArgs { query_embedding_hash: Vec<u8>, max_results: u64, max_cost: u64, timestamp: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct SubmitSearchResultArgs { request_id: u64, result_hash: Vec<u8>, relevance_score: u64, latency_ms: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct RateResultArgs { request_id: u64, provider: Address, rating: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct TopNArgs { top_n: usize }

pub fn dispatch(
    state: &mut Option<SemanticSearchState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC66: already initialised");
            *state = Some(SemanticSearchState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "register_provider" => {
            let s = state.as_mut().expect("DRC66: not initialised");
            let a: RegisterArgs = serde_json::from_slice(args).expect("DRC66: bad args");
            s.register_provider(caller, a.device_id, a.index_name, a.vector_count, a.price_per_query);
            serde_json::to_vec("ok").unwrap()
        }
        "search" => {
            let s = state.as_mut().expect("DRC66: not initialised");
            let a: SearchArgs = serde_json::from_slice(args).expect("DRC66: bad args");
            let id = s.search(caller, a.query_embedding_hash, a.max_results, a.max_cost, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        "submit_search_result" => {
            let s = state.as_mut().expect("DRC66: not initialised");
            let a: SubmitSearchResultArgs = serde_json::from_slice(args).expect("DRC66: bad args");
            s.submit_search_result(caller, a.request_id, a.result_hash, a.relevance_score, a.latency_ms);
            serde_json::to_vec("ok").unwrap()
        }
        "rate_result" => {
            let s = state.as_mut().expect("DRC66: not initialised");
            let a: RateResultArgs = serde_json::from_slice(args).expect("DRC66: bad args");
            s.rate_result(caller, a.request_id, a.provider, a.rating);
            serde_json::to_vec("ok").unwrap()
        }
        "best_providers" => {
            let s = state.as_ref().expect("DRC66: not initialised");
            let a: TopNArgs = serde_json::from_slice(args).expect("DRC66: bad args");
            serde_json::to_vec(&s.best_providers(a.top_n)).unwrap()
        }
        _ => panic!("DRC66: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const PROV_A: Address = [1u8; 32];
    const PROV_B: Address = [2u8; 32];
    const REQUESTER: Address = [3u8; 32];

    fn setup_with_providers() -> SemanticSearchState {
        let mut s = SemanticSearchState::new(OWNER);
        s.register_provider(PROV_A, "seed-1".into(), "wiki-embeddings".into(), 100_000, 5);
        s.register_provider(PROV_B, "seed-2".into(), "code-embeddings".into(), 50_000, 8);
        s
    }

    #[test]
    fn test_register_provider() {
        let s = setup_with_providers();
        assert_eq!(s.providers.len(), 2);
        let pa = s.providers.get(&addr_key(&PROV_A)).unwrap();
        assert_eq!(pa.index_name, "wiki-embeddings");
        assert_eq!(pa.accuracy_score, 5000);
        assert!(pa.active);
    }

    #[test]
    fn test_search_selects_providers() {
        let mut s = setup_with_providers();
        let rid = s.search(REQUESTER, vec![0xAA, 0xBB], 2, 100, 1000);
        let req = s.search_requests.get(&rid).unwrap();
        assert_eq!(req.providers_used.len(), 2);
        assert_eq!(req.total_cost, 13); // 5 + 8
        assert!(!req.completed);
    }

    #[test]
    fn test_submit_results_completes_request() {
        let mut s = setup_with_providers();
        let rid = s.search(REQUESTER, vec![0xAA], 2, 100, 1000);

        s.submit_search_result(PROV_A, rid, vec![1, 2], 8500, 45);
        let req = s.search_requests.get(&rid).unwrap();
        assert!(!req.completed);

        s.submit_search_result(PROV_B, rid, vec![3, 4], 9200, 32);
        let req = s.search_requests.get(&rid).unwrap();
        assert!(req.completed);
        assert_eq!(req.results.len(), 2);
    }

    #[test]
    fn test_rate_result_updates_accuracy() {
        let mut s = setup_with_providers();
        let rid = s.search(REQUESTER, vec![0xCC], 1, 100, 1000);
        s.submit_search_result(PROV_A, rid, vec![1], 9000, 50);

        s.rate_result(REQUESTER, rid, PROV_A, 9000);
        let pa = s.providers.get(&addr_key(&PROV_A)).unwrap();
        assert_eq!(pa.accuracy_score, 9000);
        assert_eq!(pa.total_ratings, 1);
    }

    #[test]
    fn test_best_providers_sorted() {
        let mut s = setup_with_providers();
        // Rate PROV_B higher
        let rid = s.search(REQUESTER, vec![0xDD], 2, 100, 1000);
        s.submit_search_result(PROV_A, rid, vec![1], 5000, 100);
        s.submit_search_result(PROV_B, rid, vec![2], 9000, 20);
        s.rate_result(REQUESTER, rid, PROV_A, 3000);
        s.rate_result(REQUESTER, rid, PROV_B, 9500);

        let best = s.best_providers(10);
        assert_eq!(best[0].address, PROV_B);
        assert_eq!(best[1].address, PROV_A);
    }

    #[test]
    #[should_panic(expected = "already registered")]
    fn test_duplicate_registration_rejected() {
        let mut s = setup_with_providers();
        s.register_provider(PROV_A, "dup".into(), "dup".into(), 100, 1);
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state = None;
        dispatch(&mut state, "init", b"{}", OWNER);
        let reg = serde_json::to_vec(&RegisterArgs {
            device_id: "d1".into(), index_name: "idx".into(), vector_count: 500, price_per_query: 10,
        }).unwrap();
        dispatch(&mut state, "register_provider", &reg, PROV_A);
        let best = dispatch(&mut state, "best_providers",
            &serde_json::to_vec(&TopNArgs { top_n: 5 }).unwrap(), REQUESTER);
        let providers: Vec<SearchProvider> = serde_json::from_slice(&best).unwrap();
        assert_eq!(providers.len(), 1);
    }
}
