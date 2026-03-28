use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-64  On-Chain Vector Index Registry
// ---------------------------------------------------------------------------
// Register vector indexes stored on Cognitum Seeds. Agents discover and
// query vector databases through on-chain metadata and payment rails.

type Address = [u8; 32];
type IndexId = u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DistanceMetric {
    Cosine,
    L2,
    DotProduct,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VectorIndex {
    pub id: IndexId,
    pub owner: Address,
    pub device_id: String,
    pub name: String,
    pub dimensions: u16,
    pub vector_count: u64,
    pub distance_metric: DistanceMetric,
    pub description: String,
    pub endpoint_hash: Vec<u8>,
    pub created_at: u64,
    pub last_updated: u64,
    pub price_per_query: u64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VectorQuery {
    pub id: u64,
    pub requester: Address,
    pub index_id: IndexId,
    pub query_hash: Vec<u8>,
    pub result_hash: Option<Vec<u8>>,
    pub cost: u64,
    pub timestamp: u64,
    pub fulfilled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VectorIndexState {
    pub owner: Address,
    pub indexes: BTreeMap<IndexId, VectorIndex>,
    pub queries: BTreeMap<u64, VectorQuery>,
    pub next_index_id: u64,
    pub next_query_id: u64,
    pub total_queries: u64,
    pub total_revenue: u64,
}

impl VectorIndexState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            indexes: BTreeMap::new(),
            queries: BTreeMap::new(),
            next_index_id: 1,
            next_query_id: 1,
            total_queries: 0,
            total_revenue: 0,
        }
    }

    pub fn register_index(
        &mut self,
        caller: Address,
        device_id: String,
        name: String,
        dimensions: u16,
        vector_count: u64,
        distance_metric: DistanceMetric,
        description: String,
        endpoint_hash: Vec<u8>,
        price_per_query: u64,
        timestamp: u64,
    ) -> IndexId {
        assert!(!name.is_empty(), "DRC64: name required");
        assert!(dimensions > 0, "DRC64: dimensions must be > 0");
        let id = self.next_index_id;
        self.next_index_id += 1;
        self.indexes.insert(
            id,
            VectorIndex {
                id,
                owner: caller,
                device_id,
                name,
                dimensions,
                vector_count,
                distance_metric,
                description,
                endpoint_hash,
                created_at: timestamp,
                last_updated: timestamp,
                price_per_query,
                active: true,
            },
        );
        id
    }

    pub fn update_index(
        &mut self,
        caller: Address,
        index_id: IndexId,
        vector_count: u64,
        endpoint_hash: Vec<u8>,
        timestamp: u64,
    ) {
        let idx = self
            .indexes
            .get_mut(&index_id)
            .expect("DRC64: index not found");
        assert!(caller == idx.owner, "DRC64: not index owner");
        idx.vector_count = vector_count;
        idx.endpoint_hash = endpoint_hash;
        idx.last_updated = timestamp;
    }

    pub fn query_index(
        &mut self,
        caller: Address,
        index_id: IndexId,
        query_hash: Vec<u8>,
        payment: u64,
        timestamp: u64,
    ) -> u64 {
        let idx = self.indexes.get(&index_id).expect("DRC64: index not found");
        assert!(idx.active, "DRC64: index inactive");
        assert!(
            payment >= idx.price_per_query,
            "DRC64: insufficient payment"
        );

        let qid = self.next_query_id;
        self.next_query_id += 1;
        self.total_queries += 1;
        self.total_revenue += payment;

        self.queries.insert(
            qid,
            VectorQuery {
                id: qid,
                requester: caller,
                index_id,
                query_hash,
                result_hash: None,
                cost: payment,
                timestamp,
                fulfilled: false,
            },
        );
        qid
    }

    pub fn submit_result(&mut self, caller: Address, query_id: u64, result_hash: Vec<u8>) {
        let q = self
            .queries
            .get_mut(&query_id)
            .expect("DRC64: query not found");
        let idx = self
            .indexes
            .get(&q.index_id)
            .expect("DRC64: index not found");
        assert!(
            caller == idx.owner,
            "DRC64: only index owner can submit results"
        );
        assert!(!q.fulfilled, "DRC64: already fulfilled");
        q.result_hash = Some(result_hash);
        q.fulfilled = true;
    }

    pub fn deregister(&mut self, caller: Address, index_id: IndexId) {
        let idx = self
            .indexes
            .get_mut(&index_id)
            .expect("DRC64: index not found");
        assert!(caller == idx.owner, "DRC64: not index owner");
        idx.active = false;
    }

    pub fn search_by_dimensions(&self, dimensions: u16) -> Vec<&VectorIndex> {
        self.indexes
            .values()
            .filter(|idx| idx.active && idx.dimensions == dimensions)
            .collect()
    }

    pub fn search_by_metric(&self, metric: &DistanceMetric) -> Vec<&VectorIndex> {
        self.indexes
            .values()
            .filter(|idx| idx.active && idx.distance_metric == *metric)
            .collect()
    }

    pub fn indexes_by_device(&self, device_id: &str) -> Vec<&VectorIndex> {
        self.indexes
            .values()
            .filter(|idx| idx.device_id == device_id)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterIndexArgs {
    device_id: String,
    name: String,
    dimensions: u16,
    vector_count: u64,
    distance_metric: DistanceMetric,
    description: String,
    endpoint_hash: Vec<u8>,
    price_per_query: u64,
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct UpdateIndexArgs {
    index_id: IndexId,
    vector_count: u64,
    endpoint_hash: Vec<u8>,
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct QueryIndexArgs {
    index_id: IndexId,
    query_hash: Vec<u8>,
    payment: u64,
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct SubmitResultArgs {
    query_id: u64,
    result_hash: Vec<u8>,
}
#[derive(Serialize, Deserialize, Debug)]
struct IndexIdArgs {
    index_id: IndexId,
}
#[derive(Serialize, Deserialize, Debug)]
struct DimensionsArgs {
    dimensions: u16,
}
#[derive(Serialize, Deserialize, Debug)]
struct MetricArgs {
    metric: DistanceMetric,
}
#[derive(Serialize, Deserialize, Debug)]
struct DeviceIdArgs {
    device_id: String,
}

pub fn dispatch(
    state: &mut Option<VectorIndexState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC64: already initialised");
            *state = Some(VectorIndexState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "register_index" => {
            let s = state.as_mut().expect("DRC64: not initialised");
            let a: RegisterIndexArgs = serde_json::from_slice(args).expect("DRC64: bad args");
            let id = s.register_index(
                caller,
                a.device_id,
                a.name,
                a.dimensions,
                a.vector_count,
                a.distance_metric,
                a.description,
                a.endpoint_hash,
                a.price_per_query,
                a.timestamp,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "update_index" => {
            let s = state.as_mut().expect("DRC64: not initialised");
            let a: UpdateIndexArgs = serde_json::from_slice(args).expect("DRC64: bad args");
            s.update_index(
                caller,
                a.index_id,
                a.vector_count,
                a.endpoint_hash,
                a.timestamp,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "query_index" => {
            let s = state.as_mut().expect("DRC64: not initialised");
            let a: QueryIndexArgs = serde_json::from_slice(args).expect("DRC64: bad args");
            let qid = s.query_index(caller, a.index_id, a.query_hash, a.payment, a.timestamp);
            serde_json::to_vec(&qid).unwrap()
        }
        "submit_result" => {
            let s = state.as_mut().expect("DRC64: not initialised");
            let a: SubmitResultArgs = serde_json::from_slice(args).expect("DRC64: bad args");
            s.submit_result(caller, a.query_id, a.result_hash);
            serde_json::to_vec("ok").unwrap()
        }
        "deregister" => {
            let s = state.as_mut().expect("DRC64: not initialised");
            let a: IndexIdArgs = serde_json::from_slice(args).expect("DRC64: bad args");
            s.deregister(caller, a.index_id);
            serde_json::to_vec("ok").unwrap()
        }
        "search_by_dimensions" => {
            let s = state.as_ref().expect("DRC64: not initialised");
            let a: DimensionsArgs = serde_json::from_slice(args).expect("DRC64: bad args");
            serde_json::to_vec(&s.search_by_dimensions(a.dimensions)).unwrap()
        }
        "search_by_metric" => {
            let s = state.as_ref().expect("DRC64: not initialised");
            let a: MetricArgs = serde_json::from_slice(args).expect("DRC64: bad args");
            serde_json::to_vec(&s.search_by_metric(&a.metric)).unwrap()
        }
        "indexes_by_device" => {
            let s = state.as_ref().expect("DRC64: not initialised");
            let a: DeviceIdArgs = serde_json::from_slice(args).expect("DRC64: bad args");
            serde_json::to_vec(&s.indexes_by_device(&a.device_id)).unwrap()
        }
        _ => panic!("DRC64: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const PROVIDER: Address = [1u8; 32];
    const QUERIER: Address = [2u8; 32];

    fn setup_with_index() -> (VectorIndexState, IndexId) {
        let mut s = VectorIndexState::new(OWNER);
        let id = s.register_index(
            PROVIDER,
            "seed-001".into(),
            "embeddings-384".into(),
            384,
            50_000,
            DistanceMetric::Cosine,
            "General text embeddings".into(),
            vec![0xAB, 0xCD],
            10,
            1000,
        );
        (s, id)
    }

    #[test]
    fn test_register_and_query_index() {
        let (s, id) = setup_with_index();
        let idx = s.indexes.get(&id).unwrap();
        assert_eq!(idx.name, "embeddings-384");
        assert_eq!(idx.dimensions, 384);
        assert_eq!(idx.vector_count, 50_000);
        assert_eq!(idx.distance_metric, DistanceMetric::Cosine);
        assert!(idx.active);
    }

    #[test]
    fn test_query_and_submit_result() {
        let (mut s, idx_id) = setup_with_index();
        let qid = s.query_index(QUERIER, idx_id, vec![1, 2, 3], 10, 2000);
        assert_eq!(qid, 1);
        assert_eq!(s.total_queries, 1);
        assert_eq!(s.total_revenue, 10);

        s.submit_result(PROVIDER, qid, vec![0xDE, 0xAD]);
        let q = s.queries.get(&qid).unwrap();
        assert!(q.fulfilled);
        assert_eq!(q.result_hash, Some(vec![0xDE, 0xAD]));
    }

    #[test]
    fn test_search_by_dimensions() {
        let mut s = VectorIndexState::new(OWNER);
        s.register_index(
            PROVIDER,
            "s1".into(),
            "small".into(),
            128,
            1000,
            DistanceMetric::L2,
            "".into(),
            vec![],
            5,
            100,
        );
        s.register_index(
            PROVIDER,
            "s2".into(),
            "large".into(),
            768,
            5000,
            DistanceMetric::Cosine,
            "".into(),
            vec![],
            20,
            200,
        );
        s.register_index(
            PROVIDER,
            "s3".into(),
            "also-small".into(),
            128,
            2000,
            DistanceMetric::DotProduct,
            "".into(),
            vec![],
            8,
            300,
        );

        let results = s.search_by_dimensions(128);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_deregister_hides_from_search() {
        let (mut s, idx_id) = setup_with_index();
        assert_eq!(s.search_by_metric(&DistanceMetric::Cosine).len(), 1);
        s.deregister(PROVIDER, idx_id);
        assert_eq!(s.search_by_metric(&DistanceMetric::Cosine).len(), 0);
    }

    #[test]
    fn test_indexes_by_device() {
        let mut s = VectorIndexState::new(OWNER);
        s.register_index(
            PROVIDER,
            "seed-A".into(),
            "idx1".into(),
            256,
            100,
            DistanceMetric::Cosine,
            "".into(),
            vec![],
            5,
            100,
        );
        s.register_index(
            PROVIDER,
            "seed-A".into(),
            "idx2".into(),
            512,
            200,
            DistanceMetric::L2,
            "".into(),
            vec![],
            10,
            200,
        );
        s.register_index(
            PROVIDER,
            "seed-B".into(),
            "idx3".into(),
            256,
            300,
            DistanceMetric::Cosine,
            "".into(),
            vec![],
            15,
            300,
        );

        assert_eq!(s.indexes_by_device("seed-A").len(), 2);
        assert_eq!(s.indexes_by_device("seed-B").len(), 1);
        assert_eq!(s.indexes_by_device("seed-C").len(), 0);
    }

    #[test]
    #[should_panic(expected = "insufficient payment")]
    fn test_underpayment_rejected() {
        let (mut s, idx_id) = setup_with_index();
        s.query_index(QUERIER, idx_id, vec![1], 5, 3000); // price is 10
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state = None;
        dispatch(&mut state, "init", b"{}", OWNER);
        let reg = serde_json::to_vec(&RegisterIndexArgs {
            device_id: "dev-1".into(),
            name: "test-idx".into(),
            dimensions: 256,
            vector_count: 1000,
            distance_metric: DistanceMetric::Cosine,
            description: "test".into(),
            endpoint_hash: vec![0xFF],
            price_per_query: 5,
            timestamp: 100,
        })
        .unwrap();
        let result = dispatch(&mut state, "register_index", &reg, PROVIDER);
        let id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(id, 1);
    }
}
