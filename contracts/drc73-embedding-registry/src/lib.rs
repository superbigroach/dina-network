use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-73  Vector Embedding Registry
// ---------------------------------------------------------------------------

type Address = [u8; 32];
type EmbeddingId = u64;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmbeddingRecord {
    pub id: EmbeddingId,
    pub owner: Address,
    pub dimensions: u16,
    pub model_used: String,
    pub content_hash: [u8; 32],
    pub semantic_label: String,
    pub created_at: u64,
    pub access_count: u64,
    pub price_per_access: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmbeddingState {
    pub owner: Address,
    pub embeddings: BTreeMap<EmbeddingId, EmbeddingRecord>,
    pub next_id: EmbeddingId,
    pub balances: BTreeMap<Address, u64>,
    pub revenue: BTreeMap<Address, u64>,
}

impl EmbeddingState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            embeddings: BTreeMap::new(),
            next_id: 1,
            balances: BTreeMap::new(),
            revenue: BTreeMap::new(),
        }
    }

    pub fn deposit(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC73: deposit must be positive");
        *self.balances.entry(caller).or_insert(0) += amount;
    }

    pub fn register_embedding(
        &mut self,
        caller: Address,
        dimensions: u16,
        model_used: String,
        content_hash: [u8; 32],
        semantic_label: String,
        created_at: u64,
        price_per_access: u64,
    ) -> EmbeddingId {
        assert!(dimensions > 0, "DRC73: dimensions must be positive");
        assert!(!model_used.is_empty(), "DRC73: model_used required");
        assert!(!semantic_label.is_empty(), "DRC73: semantic_label required");

        let id = self.next_id;
        self.next_id += 1;
        self.embeddings.insert(id, EmbeddingRecord {
            id,
            owner: caller,
            dimensions,
            model_used,
            content_hash,
            semantic_label,
            created_at,
            access_count: 0,
            price_per_access,
        });
        id
    }

    pub fn access_embedding(
        &mut self,
        caller: Address,
        id: EmbeddingId,
        payment: u64,
    ) -> &EmbeddingRecord {
        let record = self.embeddings.get(&id).expect("DRC73: embedding not found");
        let price = record.price_per_access;
        let emb_owner = record.owner;

        if price > 0 {
            assert!(payment >= price, "DRC73: insufficient payment");
            let bal = self.balances.get(&caller).copied().unwrap_or(0);
            assert!(bal >= payment, "DRC73: insufficient balance");
            self.balances.insert(caller, bal - price);
            *self.revenue.entry(emb_owner).or_insert(0) += price;
        }

        let record = self.embeddings.get_mut(&id).unwrap();
        record.access_count += 1;
        record
    }

    pub fn search_by_label(&self, label: &str) -> Vec<&EmbeddingRecord> {
        self.embeddings.values()
            .filter(|e| e.semantic_label.contains(label))
            .collect()
    }

    pub fn search_by_model(&self, model: &str) -> Vec<&EmbeddingRecord> {
        self.embeddings.values()
            .filter(|e| e.model_used == model)
            .collect()
    }

    pub fn embeddings_by_owner(&self, owner: &Address) -> Vec<&EmbeddingRecord> {
        self.embeddings.values()
            .filter(|e| &e.owner == owner)
            .collect()
    }

    pub fn update_price(&mut self, caller: Address, id: EmbeddingId, new_price: u64) {
        let record = self.embeddings.get_mut(&id).expect("DRC73: embedding not found");
        assert!(record.owner == caller, "DRC73: only owner can update price");
        record.price_per_access = new_price;
    }

    pub fn total_embeddings(&self) -> usize {
        self.embeddings.len()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterArgs {
    dimensions: u16, model_used: String, content_hash: [u8; 32],
    semantic_label: String, created_at: u64, price_per_access: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct AccessArgs { id: EmbeddingId, payment: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct LabelArgs { label: String }
#[derive(Serialize, Deserialize, Debug)]
struct ModelArgs { model: String }
#[derive(Serialize, Deserialize, Debug)]
struct OwnerArgs { owner: Address }
#[derive(Serialize, Deserialize, Debug)]
struct UpdatePriceArgs { id: EmbeddingId, new_price: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs { amount: u64 }

pub fn dispatch(
    state: &mut Option<EmbeddingState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC73: already initialised");
            *state = Some(EmbeddingState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "deposit" => {
            let s = state.as_mut().expect("DRC73: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC73: bad args");
            s.deposit(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "register_embedding" => {
            let s = state.as_mut().expect("DRC73: not initialised");
            let a: RegisterArgs = serde_json::from_slice(args).expect("DRC73: bad args");
            let id = s.register_embedding(caller, a.dimensions, a.model_used, a.content_hash, a.semantic_label, a.created_at, a.price_per_access);
            serde_json::to_vec(&id).unwrap()
        }
        "access_embedding" => {
            let s = state.as_mut().expect("DRC73: not initialised");
            let a: AccessArgs = serde_json::from_slice(args).expect("DRC73: bad args");
            let record = s.access_embedding(caller, a.id, a.payment);
            serde_json::to_vec(record).unwrap()
        }
        "search_by_label" => {
            let s = state.as_ref().expect("DRC73: not initialised");
            let a: LabelArgs = serde_json::from_slice(args).expect("DRC73: bad args");
            serde_json::to_vec(&s.search_by_label(&a.label)).unwrap()
        }
        "search_by_model" => {
            let s = state.as_ref().expect("DRC73: not initialised");
            let a: ModelArgs = serde_json::from_slice(args).expect("DRC73: bad args");
            serde_json::to_vec(&s.search_by_model(&a.model)).unwrap()
        }
        "embeddings_by_owner" => {
            let s = state.as_ref().expect("DRC73: not initialised");
            let a: OwnerArgs = serde_json::from_slice(args).expect("DRC73: bad args");
            serde_json::to_vec(&s.embeddings_by_owner(&a.owner)).unwrap()
        }
        "update_price" => {
            let s = state.as_mut().expect("DRC73: not initialised");
            let a: UpdatePriceArgs = serde_json::from_slice(args).expect("DRC73: bad args");
            s.update_price(caller, a.id, a.new_price);
            serde_json::to_vec("ok").unwrap()
        }
        "total_embeddings" => {
            let s = state.as_ref().expect("DRC73: not initialised");
            serde_json::to_vec(&s.total_embeddings()).unwrap()
        }
        _ => panic!("DRC73: unknown method '{method}'"),
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

    fn setup() -> EmbeddingState {
        let mut s = EmbeddingState::new(OWNER);
        s.register_embedding(
            ALICE, 768, "text-embedding-ada-002".into(), [0xAA; 32],
            "product-reviews".into(), 1000, 10,
        );
        s.register_embedding(
            ALICE, 1536, "text-embedding-ada-002".into(), [0xBB; 32],
            "customer-support".into(), 1001, 20,
        );
        s.register_embedding(
            BOB, 384, "all-MiniLM-L6".into(), [0xCC; 32],
            "product-descriptions".into(), 1002, 5,
        );
        s.deposit(BOB, 1000);
        s
    }

    #[test]
    fn test_register_and_total() {
        let s = setup();
        assert_eq!(s.total_embeddings(), 3);
        assert_eq!(s.embeddings_by_owner(&ALICE).len(), 2);
        assert_eq!(s.embeddings_by_owner(&BOB).len(), 1);
    }

    #[test]
    fn test_search_by_label() {
        let s = setup();
        let results = s.search_by_label("product");
        assert_eq!(results.len(), 2);
        let results = s.search_by_label("customer");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].semantic_label, "customer-support");
    }

    #[test]
    fn test_search_by_model() {
        let s = setup();
        let results = s.search_by_model("text-embedding-ada-002");
        assert_eq!(results.len(), 2);
        let results = s.search_by_model("all-MiniLM-L6");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_access_embedding_with_payment() {
        let mut s = setup();
        let record = s.access_embedding(BOB, 1, 10);
        assert_eq!(record.access_count, 1);
        assert_eq!(s.balances.get(&BOB).copied().unwrap_or(0), 990);
        assert_eq!(s.revenue.get(&ALICE).copied().unwrap_or(0), 10);

        // Access again
        s.access_embedding(BOB, 1, 10);
        let record = s.embeddings.get(&1).unwrap();
        assert_eq!(record.access_count, 2);
    }

    #[test]
    fn test_update_price() {
        let mut s = setup();
        s.update_price(ALICE, 1, 50);
        assert_eq!(s.embeddings.get(&1).unwrap().price_per_access, 50);
    }

    #[test]
    #[should_panic(expected = "only owner can update price")]
    fn test_non_owner_cannot_update_price() {
        let mut s = setup();
        s.update_price(BOB, 1, 50);
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state: Option<EmbeddingState> = None;
        dispatch(&mut state, "init", b"", OWNER);
        let args = serde_json::to_vec(&RegisterArgs {
            dimensions: 512, model_used: "bert-base".into(),
            content_hash: [0xDD; 32], semantic_label: "sentiment".into(),
            created_at: 5000, price_per_access: 15,
        }).unwrap();
        let id_bytes = dispatch(&mut state, "register_embedding", &args, ALICE);
        let id: u64 = serde_json::from_slice(&id_bytes).unwrap();
        assert_eq!(id, 1);

        let total_bytes = dispatch(&mut state, "total_embeddings", b"", OWNER);
        let total: usize = serde_json::from_slice(&total_bytes).unwrap();
        assert_eq!(total, 1);
    }
}
