use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-76  Persistent Agent Memory Store
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ValueType {
    Text,
    Json,
    Binary,
    VectorRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemoryEntry {
    pub key: String,
    pub value: Vec<u8>,
    pub value_type: ValueType,
    pub created_at: u64,
    pub updated_at: u64,
    pub access_count: u64,
    pub max_age: Option<u64>,
    /// Other agents that have been granted read access.
    pub shared_with: Vec<Address>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub total_bytes: usize,
    pub by_type: BTreeMap<String, usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentMemoryState {
    pub owner: Address,
    /// Key: (agent_address, memory_key)
    pub memories: BTreeMap<(Address, String), MemoryEntry>,
}

impl AgentMemoryState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            memories: BTreeMap::new(),
        }
    }

    pub fn store(
        &mut self,
        caller: Address,
        key: String,
        value: Vec<u8>,
        value_type: ValueType,
        timestamp: u64,
        max_age: Option<u64>,
    ) {
        assert!(!key.is_empty(), "DRC76: key required");
        assert!(!value.is_empty(), "DRC76: value required");

        let composite = (caller, key.clone());
        if let Some(entry) = self.memories.get_mut(&composite) {
            entry.value = value;
            entry.value_type = value_type;
            entry.updated_at = timestamp;
            entry.max_age = max_age;
        } else {
            self.memories.insert(composite, MemoryEntry {
                key,
                value,
                value_type,
                created_at: timestamp,
                updated_at: timestamp,
                access_count: 0,
                max_age,
                shared_with: Vec::new(),
            });
        }
    }

    pub fn recall(&mut self, caller: Address, key: &str) -> Option<&MemoryEntry> {
        let composite = (caller, key.to_string());
        // First check own memory
        if self.memories.contains_key(&composite) {
            let entry = self.memories.get_mut(&composite).unwrap();
            entry.access_count += 1;
            return self.memories.get(&composite);
        }
        // Check if any other agent shared this key with caller
        let shared_key = self.memories.iter()
            .find(|((agent, k), entry)| {
                k == key && *agent != caller && entry.shared_with.contains(&caller)
            })
            .map(|(k, _)| k.clone());

        if let Some(sk) = shared_key {
            let entry = self.memories.get_mut(&sk).unwrap();
            entry.access_count += 1;
            return self.memories.get(&sk);
        }
        None
    }

    pub fn forget(&mut self, caller: Address, key: &str) -> bool {
        let composite = (caller, key.to_string());
        self.memories.remove(&composite).is_some()
    }

    pub fn list_memories(&self, caller: &Address) -> Vec<&MemoryEntry> {
        self.memories.iter()
            .filter(|((addr, _), _)| addr == caller)
            .map(|(_, entry)| entry)
            .collect()
    }

    pub fn memory_stats(&self, caller: &Address) -> MemoryStats {
        let entries: Vec<&MemoryEntry> = self.list_memories(caller);
        let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
        let mut total_bytes = 0usize;
        for e in &entries {
            total_bytes += e.value.len();
            let type_key = format!("{:?}", e.value_type);
            *by_type.entry(type_key).or_insert(0) += 1;
        }
        MemoryStats {
            total_entries: entries.len(),
            total_bytes,
            by_type,
        }
    }

    pub fn search_by_prefix(&self, caller: &Address, prefix: &str) -> Vec<&MemoryEntry> {
        self.memories.iter()
            .filter(|((addr, key), _)| addr == caller && key.starts_with(prefix))
            .map(|(_, entry)| entry)
            .collect()
    }

    pub fn cleanup_expired(&mut self, current_time: u64) -> u32 {
        let mut removed = 0u32;
        let expired_keys: Vec<(Address, String)> = self.memories.iter()
            .filter(|(_, entry)| {
                if let Some(max_age) = entry.max_age {
                    current_time > entry.created_at + max_age
                } else {
                    false
                }
            })
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired_keys {
            self.memories.remove(&key);
            removed += 1;
        }
        removed
    }

    pub fn share_memory(&mut self, caller: Address, key: &str, with_agent: Address) {
        let composite = (caller, key.to_string());
        let entry = self.memories.get_mut(&composite).expect("DRC76: memory not found");
        if !entry.shared_with.contains(&with_agent) {
            entry.shared_with.push(with_agent);
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct StoreArgs { key: String, value: Vec<u8>, value_type: ValueType, timestamp: u64, max_age: Option<u64> }
#[derive(Serialize, Deserialize, Debug)]
struct KeyArgs { key: String }
#[derive(Serialize, Deserialize, Debug)]
struct PrefixArgs { prefix: String }
#[derive(Serialize, Deserialize, Debug)]
struct CleanupArgs { current_time: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct ShareArgs { key: String, with_agent: Address }

pub fn dispatch(
    state: &mut Option<AgentMemoryState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC76: already initialised");
            *state = Some(AgentMemoryState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "store" => {
            let s = state.as_mut().expect("DRC76: not initialised");
            let a: StoreArgs = serde_json::from_slice(args).expect("DRC76: bad args");
            s.store(caller, a.key, a.value, a.value_type, a.timestamp, a.max_age);
            serde_json::to_vec("ok").unwrap()
        }
        "recall" => {
            let s = state.as_mut().expect("DRC76: not initialised");
            let a: KeyArgs = serde_json::from_slice(args).expect("DRC76: bad args");
            serde_json::to_vec(&s.recall(caller, &a.key)).unwrap()
        }
        "forget" => {
            let s = state.as_mut().expect("DRC76: not initialised");
            let a: KeyArgs = serde_json::from_slice(args).expect("DRC76: bad args");
            let removed = s.forget(caller, &a.key);
            serde_json::to_vec(&removed).unwrap()
        }
        "list_memories" => {
            let s = state.as_ref().expect("DRC76: not initialised");
            serde_json::to_vec(&s.list_memories(&caller)).unwrap()
        }
        "memory_stats" => {
            let s = state.as_ref().expect("DRC76: not initialised");
            serde_json::to_vec(&s.memory_stats(&caller)).unwrap()
        }
        "search_by_prefix" => {
            let s = state.as_ref().expect("DRC76: not initialised");
            let a: PrefixArgs = serde_json::from_slice(args).expect("DRC76: bad args");
            serde_json::to_vec(&s.search_by_prefix(&caller, &a.prefix)).unwrap()
        }
        "cleanup_expired" => {
            let s = state.as_mut().expect("DRC76: not initialised");
            let a: CleanupArgs = serde_json::from_slice(args).expect("DRC76: bad args");
            let count = s.cleanup_expired(a.current_time);
            serde_json::to_vec(&count).unwrap()
        }
        "share_memory" => {
            let s = state.as_mut().expect("DRC76: not initialised");
            let a: ShareArgs = serde_json::from_slice(args).expect("DRC76: bad args");
            s.share_memory(caller, &a.key, a.with_agent);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC76: unknown method '{method}'"),
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

    fn setup() -> AgentMemoryState {
        let mut s = AgentMemoryState::new(OWNER);
        s.store(AGENT_A, "goal".into(), b"reach destination X".to_vec(), ValueType::Text, 1000, None);
        s.store(AGENT_A, "context.user".into(), b"{\"name\":\"Bob\"}".to_vec(), ValueType::Json, 1001, None);
        s.store(AGENT_A, "context.session".into(), b"{\"id\":42}".to_vec(), ValueType::Json, 1002, Some(3600));
        s
    }

    #[test]
    fn test_store_and_recall() {
        let mut s = setup();
        let entry = s.recall(AGENT_A, "goal").unwrap();
        assert_eq!(entry.value, b"reach destination X");
        assert_eq!(entry.value_type, ValueType::Text);
        assert_eq!(entry.access_count, 1);
    }

    #[test]
    fn test_update_existing_key() {
        let mut s = setup();
        s.store(AGENT_A, "goal".into(), b"new goal".to_vec(), ValueType::Text, 2000, None);
        let entry = s.recall(AGENT_A, "goal").unwrap();
        assert_eq!(entry.value, b"new goal");
        assert_eq!(entry.created_at, 1000); // original create time preserved
        assert_eq!(entry.updated_at, 2000);
    }

    #[test]
    fn test_forget() {
        let mut s = setup();
        assert!(s.forget(AGENT_A, "goal"));
        assert!(s.recall(AGENT_A, "goal").is_none());
        assert!(!s.forget(AGENT_A, "nonexistent"));
    }

    #[test]
    fn test_search_by_prefix() {
        let s = setup();
        let results = s.search_by_prefix(&AGENT_A, "context.");
        assert_eq!(results.len(), 2);
        let results = s.search_by_prefix(&AGENT_A, "goal");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_memory_stats() {
        let s = setup();
        let stats = s.memory_stats(&AGENT_A);
        assert_eq!(stats.total_entries, 3);
        assert_eq!(*stats.by_type.get("Text").unwrap_or(&0), 1);
        assert_eq!(*stats.by_type.get("Json").unwrap_or(&0), 2);
    }

    #[test]
    fn test_cleanup_expired() {
        let mut s = setup();
        // context.session has max_age 3600, created_at 1002
        // Should expire after 1002 + 3600 = 4602
        let removed = s.cleanup_expired(5000);
        assert_eq!(removed, 1);
        assert_eq!(s.list_memories(&AGENT_A).len(), 2);
    }

    #[test]
    fn test_share_memory() {
        let mut s = setup();
        s.share_memory(AGENT_A, "goal", AGENT_B);
        // AGENT_B should be able to recall AGENT_A's shared memory
        let entry = s.recall(AGENT_B, "goal").unwrap();
        assert_eq!(entry.value, b"reach destination X");
    }
}
