use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-68  On-Chain Knowledge Graph
// ---------------------------------------------------------------------------
// Store entity-relationship triples on-chain for AI agent reasoning.
// Enables querying by subject, predicate, or object, and traversing
// related entities with configurable depth.

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Triple {
    pub id: u64,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: u64, // basis points 0-10000
    pub source: Address,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KnowledgeGraphState {
    pub owner: Address,
    pub triples: Vec<Triple>,
    pub subject_index: BTreeMap<String, Vec<u64>>, // subject -> triple ids
    pub predicate_index: BTreeMap<String, Vec<u64>>, // predicate -> triple ids
    pub object_index: BTreeMap<String, Vec<u64>>,  // object -> triple ids
    pub source_index: BTreeMap<String, Vec<u64>>,  // hex(source) -> triple ids
    pub next_id: u64,
}

fn addr_key(a: &Address) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

impl KnowledgeGraphState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            triples: Vec::new(),
            subject_index: BTreeMap::new(),
            predicate_index: BTreeMap::new(),
            object_index: BTreeMap::new(),
            source_index: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn add_triple(
        &mut self,
        caller: Address,
        subject: String,
        predicate: String,
        object: String,
        confidence: u64,
        timestamp: u64,
    ) -> u64 {
        assert!(!subject.is_empty(), "DRC68: subject required");
        assert!(!predicate.is_empty(), "DRC68: predicate required");
        assert!(!object.is_empty(), "DRC68: object required");
        assert!(confidence <= 10000, "DRC68: confidence max 10000");

        let id = self.next_id;
        self.next_id += 1;

        self.subject_index
            .entry(subject.clone())
            .or_default()
            .push(id);
        self.predicate_index
            .entry(predicate.clone())
            .or_default()
            .push(id);
        self.object_index
            .entry(object.clone())
            .or_default()
            .push(id);
        self.source_index
            .entry(addr_key(&caller))
            .or_default()
            .push(id);

        self.triples.push(Triple {
            id,
            subject,
            predicate,
            object,
            confidence,
            source: caller,
            timestamp,
        });

        id
    }

    pub fn query_by_subject(&self, subject: &str) -> Vec<&Triple> {
        self.subject_index
            .get(subject)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.triples.iter().find(|t| t.id == *id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn query_by_predicate(&self, predicate: &str) -> Vec<&Triple> {
        self.predicate_index
            .get(predicate)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.triples.iter().find(|t| t.id == *id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn query_by_object(&self, object: &str) -> Vec<&Triple> {
        self.object_index
            .get(object)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.triples.iter().find(|t| t.id == *id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn related_entities(&self, entity: &str, depth: u32) -> Vec<String> {
        let mut visited = std::collections::BTreeSet::new();
        let mut queue = vec![entity.to_string()];
        visited.insert(entity.to_string());

        for _ in 0..depth {
            let mut next_queue = Vec::new();
            for current in &queue {
                // Entities connected via subject
                if let Some(ids) = self.subject_index.get(current) {
                    for id in ids {
                        if let Some(t) = self.triples.iter().find(|t| t.id == *id) {
                            if visited.insert(t.object.clone()) {
                                next_queue.push(t.object.clone());
                            }
                        }
                    }
                }
                // Entities connected via object
                if let Some(ids) = self.object_index.get(current) {
                    for id in ids {
                        if let Some(t) = self.triples.iter().find(|t| t.id == *id) {
                            if visited.insert(t.subject.clone()) {
                                next_queue.push(t.subject.clone());
                            }
                        }
                    }
                }
            }
            if next_queue.is_empty() {
                break;
            }
            queue = next_queue;
        }

        visited.into_iter().filter(|e| e != entity).collect()
    }

    pub fn remove_triple(&mut self, caller: Address, triple_id: u64) {
        let pos = self
            .triples
            .iter()
            .position(|t| t.id == triple_id)
            .expect("DRC68: triple not found");
        let triple = &self.triples[pos];
        assert!(
            caller == triple.source || caller == self.owner,
            "DRC68: not authorised"
        );

        // Remove from indexes
        if let Some(ids) = self.subject_index.get_mut(&triple.subject) {
            ids.retain(|id| *id != triple_id);
        }
        if let Some(ids) = self.predicate_index.get_mut(&triple.predicate) {
            ids.retain(|id| *id != triple_id);
        }
        if let Some(ids) = self.object_index.get_mut(&triple.object) {
            ids.retain(|id| *id != triple_id);
        }
        let src_key = addr_key(&triple.source);
        if let Some(ids) = self.source_index.get_mut(&src_key) {
            ids.retain(|id| *id != triple_id);
        }

        self.triples.remove(pos);
    }

    pub fn update_confidence(&mut self, caller: Address, triple_id: u64, new_confidence: u64) {
        assert!(new_confidence <= 10000, "DRC68: confidence max 10000");
        let triple = self
            .triples
            .iter_mut()
            .find(|t| t.id == triple_id)
            .expect("DRC68: triple not found");
        assert!(
            caller == triple.source || caller == self.owner,
            "DRC68: not authorised"
        );
        triple.confidence = new_confidence;
    }

    pub fn triples_by_source(&self, source: Address) -> Vec<&Triple> {
        let key = addr_key(&source);
        self.source_index
            .get(&key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.triples.iter().find(|t| t.id == *id))
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct AddTripleArgs {
    subject: String,
    predicate: String,
    object: String,
    confidence: u64,
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct QueryArgs {
    value: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct RelatedArgs {
    entity: String,
    depth: u32,
}
#[derive(Serialize, Deserialize, Debug)]
struct TripleIdArgs {
    triple_id: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct UpdateConfidenceArgs {
    triple_id: u64,
    new_confidence: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct SourceArgs {
    source: Address,
}

pub fn dispatch(
    state: &mut Option<KnowledgeGraphState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC68: already initialised");
            *state = Some(KnowledgeGraphState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "add_triple" => {
            let s = state.as_mut().expect("DRC68: not initialised");
            let a: AddTripleArgs = serde_json::from_slice(args).expect("DRC68: bad args");
            let id = s.add_triple(
                caller,
                a.subject,
                a.predicate,
                a.object,
                a.confidence,
                a.timestamp,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "query_by_subject" => {
            let s = state.as_ref().expect("DRC68: not initialised");
            let a: QueryArgs = serde_json::from_slice(args).expect("DRC68: bad args");
            serde_json::to_vec(&s.query_by_subject(&a.value)).unwrap()
        }
        "query_by_predicate" => {
            let s = state.as_ref().expect("DRC68: not initialised");
            let a: QueryArgs = serde_json::from_slice(args).expect("DRC68: bad args");
            serde_json::to_vec(&s.query_by_predicate(&a.value)).unwrap()
        }
        "query_by_object" => {
            let s = state.as_ref().expect("DRC68: not initialised");
            let a: QueryArgs = serde_json::from_slice(args).expect("DRC68: bad args");
            serde_json::to_vec(&s.query_by_object(&a.value)).unwrap()
        }
        "related_entities" => {
            let s = state.as_ref().expect("DRC68: not initialised");
            let a: RelatedArgs = serde_json::from_slice(args).expect("DRC68: bad args");
            serde_json::to_vec(&s.related_entities(&a.entity, a.depth)).unwrap()
        }
        "remove_triple" => {
            let s = state.as_mut().expect("DRC68: not initialised");
            let a: TripleIdArgs = serde_json::from_slice(args).expect("DRC68: bad args");
            s.remove_triple(caller, a.triple_id);
            serde_json::to_vec("ok").unwrap()
        }
        "update_confidence" => {
            let s = state.as_mut().expect("DRC68: not initialised");
            let a: UpdateConfidenceArgs = serde_json::from_slice(args).expect("DRC68: bad args");
            s.update_confidence(caller, a.triple_id, a.new_confidence);
            serde_json::to_vec("ok").unwrap()
        }
        "triples_by_source" => {
            let s = state.as_ref().expect("DRC68: not initialised");
            let a: SourceArgs = serde_json::from_slice(args).expect("DRC68: bad args");
            serde_json::to_vec(&s.triples_by_source(a.source)).unwrap()
        }
        _ => panic!("DRC68: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const AGENT_A: Address = [1u8; 32];
    const AGENT_B: Address = [2u8; 32];

    fn build_graph() -> KnowledgeGraphState {
        let mut s = KnowledgeGraphState::new(OWNER);
        s.add_triple(
            AGENT_A,
            "Rust".into(),
            "is_a".into(),
            "Language".into(),
            9500,
            100,
        );
        s.add_triple(
            AGENT_A,
            "Rust".into(),
            "used_by".into(),
            "Dina".into(),
            9000,
            200,
        );
        s.add_triple(
            AGENT_B,
            "Dina".into(),
            "is_a".into(),
            "Blockchain".into(),
            8500,
            300,
        );
        s.add_triple(
            AGENT_B,
            "Blockchain".into(),
            "enables".into(),
            "DeFi".into(),
            8000,
            400,
        );
        s
    }

    #[test]
    fn test_add_and_query_by_subject() {
        let s = build_graph();
        let results = s.query_by_subject("Rust");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].predicate, "is_a");
        assert_eq!(results[1].predicate, "used_by");
    }

    #[test]
    fn test_query_by_predicate() {
        let s = build_graph();
        let results = s.query_by_predicate("is_a");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_by_object() {
        let s = build_graph();
        let results = s.query_by_object("Dina");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].subject, "Rust");
    }

    #[test]
    fn test_related_entities_depth() {
        let s = build_graph();
        // From "Rust": depth 1 -> Language, Dina
        let related_1 = s.related_entities("Rust", 1);
        assert!(related_1.contains(&"Language".to_string()));
        assert!(related_1.contains(&"Dina".to_string()));

        // From "Rust": depth 2 -> also Blockchain
        let related_2 = s.related_entities("Rust", 2);
        assert!(related_2.contains(&"Blockchain".to_string()));

        // From "Rust": depth 3 -> also DeFi
        let related_3 = s.related_entities("Rust", 3);
        assert!(related_3.contains(&"DeFi".to_string()));
    }

    #[test]
    fn test_remove_triple() {
        let mut s = build_graph();
        assert_eq!(s.query_by_subject("Rust").len(), 2);
        s.remove_triple(AGENT_A, 1); // remove "Rust is_a Language"
        assert_eq!(s.query_by_subject("Rust").len(), 1);
        assert_eq!(s.query_by_object("Language").len(), 0);
    }

    #[test]
    fn test_update_confidence() {
        let mut s = build_graph();
        s.update_confidence(AGENT_A, 1, 7000);
        let t = s.triples.iter().find(|t| t.id == 1).unwrap();
        assert_eq!(t.confidence, 7000);
    }

    #[test]
    fn test_triples_by_source() {
        let s = build_graph();
        assert_eq!(s.triples_by_source(AGENT_A).len(), 2);
        assert_eq!(s.triples_by_source(AGENT_B).len(), 2);
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state = None;
        dispatch(&mut state, "init", b"{}", OWNER);
        let args = serde_json::to_vec(&AddTripleArgs {
            subject: "A".into(),
            predicate: "knows".into(),
            object: "B".into(),
            confidence: 9000,
            timestamp: 100,
        })
        .unwrap();
        let result = dispatch(&mut state, "add_triple", &args, AGENT_A);
        let id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(id, 1);
    }
}
