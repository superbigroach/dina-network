use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Dev-Events — On-chain event indexer for easier querying
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IndexedEvent {
    pub id: u64,
    pub emitter: String,
    pub topic: String,
    pub data: Vec<u8>,
    pub block_height: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EventIndexState {
    pub events: Vec<IndexedEvent>,
    pub subscribers: HashMap<String, Vec<String>>, // topic → subscriber addresses
    pub next_id: u64,
}

impl EventIndexState {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            subscribers: HashMap::new(),
            next_id: 1,
        }
    }

    /// Emit an event. Any contract can call this to log an indexed event.
    pub fn emit(
        &mut self,
        emitter: &str,
        topic: String,
        data: Vec<u8>,
        block_height: u64,
        timestamp: u64,
    ) -> u64 {
        assert!(!topic.is_empty(), "Events: topic cannot be empty");

        let id = self.next_id;
        self.next_id += 1;

        self.events.push(IndexedEvent {
            id,
            emitter: emitter.to_string(),
            topic,
            data,
            block_height,
            timestamp,
        });
        id
    }

    /// Query events by topic, starting from `from_id`, returning up to `limit`.
    pub fn get_events(&self, topic: &str, from_id: u64, limit: usize) -> Vec<&IndexedEvent> {
        self.events
            .iter()
            .filter(|e| e.topic == topic && e.id >= from_id)
            .take(limit)
            .collect()
    }

    /// Query events by emitter address, starting from `from_id`, up to `limit`.
    pub fn get_events_by_emitter(
        &self,
        emitter: &str,
        from_id: u64,
        limit: usize,
    ) -> Vec<&IndexedEvent> {
        self.events
            .iter()
            .filter(|e| e.emitter == emitter && e.id >= from_id)
            .take(limit)
            .collect()
    }

    /// Subscribe an address to a topic.
    pub fn subscribe(&mut self, topic: String, subscriber: String) {
        let subs = self.subscribers.entry(topic).or_default();
        if !subs.contains(&subscriber) {
            subs.push(subscriber);
        }
    }

    /// Get the most recent events, up to `limit`.
    pub fn get_latest_events(&self, limit: usize) -> Vec<&IndexedEvent> {
        let len = self.events.len();
        let start = if len > limit { len - limit } else { 0 };
        self.events[start..].iter().collect()
    }

    /// Get subscribers for a topic.
    pub fn get_subscribers(&self, topic: &str) -> Vec<&str> {
        self.subscribers
            .get(topic)
            .map(|s| s.iter().map(|x| x.as_str()).collect())
            .unwrap_or_default()
    }

    /// Total event count.
    pub fn event_count(&self) -> u64 {
        self.next_id - 1
    }
}

impl Default for EventIndexState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct EmitArgs {
    topic: String,
    data: Vec<u8>,
    block_height: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetEventsArgs {
    topic: String,
    from_id: u64,
    limit: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetEventsByEmitterArgs {
    emitter: String,
    from_id: u64,
    limit: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct SubscribeArgs {
    topic: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetLatestArgs {
    limit: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetSubscribersArgs {
    topic: String,
}

pub fn dispatch(
    state: &mut Option<EventIndexState>,
    method: &str,
    args: &[u8],
    caller: &str,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Events: already initialised");
            *state = Some(EventIndexState::new());
            serde_json::to_vec("ok").unwrap()
        }

        "emit" => {
            let s = state.as_mut().expect("Events: not initialised");
            let a: EmitArgs =
                serde_json::from_slice(args).expect("Events: bad emit args");
            let id = s.emit(caller, a.topic, a.data, a.block_height, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }

        "get_events" => {
            let s = state.as_ref().expect("Events: not initialised");
            let a: GetEventsArgs =
                serde_json::from_slice(args).expect("Events: bad get_events args");
            let events = s.get_events(&a.topic, a.from_id, a.limit);
            serde_json::to_vec(&events).unwrap()
        }

        "get_events_by_emitter" => {
            let s = state.as_ref().expect("Events: not initialised");
            let a: GetEventsByEmitterArgs =
                serde_json::from_slice(args).expect("Events: bad get_events_by_emitter args");
            let events = s.get_events_by_emitter(&a.emitter, a.from_id, a.limit);
            serde_json::to_vec(&events).unwrap()
        }

        "subscribe" => {
            let s = state.as_mut().expect("Events: not initialised");
            let a: SubscribeArgs =
                serde_json::from_slice(args).expect("Events: bad subscribe args");
            s.subscribe(a.topic, caller.to_string());
            serde_json::to_vec("ok").unwrap()
        }

        "get_latest_events" => {
            let s = state.as_ref().expect("Events: not initialised");
            let a: GetLatestArgs =
                serde_json::from_slice(args).expect("Events: bad get_latest args");
            let events = s.get_latest_events(a.limit);
            serde_json::to_vec(&events).unwrap()
        }

        "get_subscribers" => {
            let s = state.as_ref().expect("Events: not initialised");
            let a: GetSubscribersArgs =
                serde_json::from_slice(args).expect("Events: bad get_subscribers args");
            let subs = s.get_subscribers(&a.topic);
            serde_json::to_vec(&subs).unwrap()
        }

        "event_count" => {
            let s = state.as_ref().expect("Events: not initialised");
            serde_json::to_vec(&s.event_count()).unwrap()
        }

        _ => panic!("Events: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN_CONTRACT: &str = "token_contract";
    const NFT_CONTRACT: &str = "nft_contract";
    const ALICE: &str = "alice_addr";

    fn init() -> Option<EventIndexState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", "deployer");
        state
    }

    fn emit_event(
        state: &mut Option<EventIndexState>,
        caller: &str,
        topic: &str,
        data: &[u8],
        block: u64,
        ts: u64,
    ) -> u64 {
        let args = serde_json::to_vec(&EmitArgs {
            topic: topic.to_string(),
            data: data.to_vec(),
            block_height: block,
            timestamp: ts,
        })
        .unwrap();
        let result = dispatch(state, "emit", &args, caller);
        serde_json::from_slice(&result).unwrap()
    }

    #[test]
    fn test_emit_and_query_by_topic() {
        let mut state = init();
        emit_event(&mut state, TOKEN_CONTRACT, "Transfer", b"tx1", 100, 1000);
        emit_event(&mut state, TOKEN_CONTRACT, "Transfer", b"tx2", 101, 1001);
        emit_event(&mut state, TOKEN_CONTRACT, "Approval", b"ap1", 102, 1002);

        let args = serde_json::to_vec(&GetEventsArgs {
            topic: "Transfer".to_string(),
            from_id: 1,
            limit: 10,
        })
        .unwrap();
        let result = dispatch(&mut state, "get_events", &args, ALICE);
        let events: Vec<IndexedEvent> = serde_json::from_slice(&result).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, b"tx1");
        assert_eq!(events[1].data, b"tx2");
    }

    #[test]
    fn test_query_by_emitter() {
        let mut state = init();
        emit_event(&mut state, TOKEN_CONTRACT, "Transfer", b"t1", 100, 1000);
        emit_event(&mut state, NFT_CONTRACT, "Transfer", b"n1", 101, 1001);
        emit_event(&mut state, TOKEN_CONTRACT, "Transfer", b"t2", 102, 1002);

        let args = serde_json::to_vec(&GetEventsByEmitterArgs {
            emitter: TOKEN_CONTRACT.to_string(),
            from_id: 1,
            limit: 10,
        })
        .unwrap();
        let result = dispatch(&mut state, "get_events_by_emitter", &args, ALICE);
        let events: Vec<IndexedEvent> = serde_json::from_slice(&result).unwrap();
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| e.emitter == TOKEN_CONTRACT));
    }

    #[test]
    fn test_subscribe() {
        let mut state = init();
        let args = serde_json::to_vec(&SubscribeArgs {
            topic: "Transfer".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "subscribe", &args, ALICE);
        dispatch(&mut state, "subscribe", &args, TOKEN_CONTRACT);
        // Duplicate subscribe should not add twice
        dispatch(&mut state, "subscribe", &args, ALICE);

        let get_args = serde_json::to_vec(&GetSubscribersArgs {
            topic: "Transfer".to_string(),
        })
        .unwrap();
        let result = dispatch(&mut state, "get_subscribers", &get_args, ALICE);
        let subs: Vec<String> = serde_json::from_slice(&result).unwrap();
        assert_eq!(subs.len(), 2);
        assert!(subs.contains(&ALICE.to_string()));
    }

    #[test]
    fn test_get_latest_events() {
        let mut state = init();
        for i in 0..10 {
            emit_event(
                &mut state,
                TOKEN_CONTRACT,
                "Transfer",
                &[i as u8],
                100 + i,
                1000 + i,
            );
        }

        let args = serde_json::to_vec(&GetLatestArgs { limit: 3 }).unwrap();
        let result = dispatch(&mut state, "get_latest_events", &args, ALICE);
        let events: Vec<IndexedEvent> = serde_json::from_slice(&result).unwrap();
        assert_eq!(events.len(), 3);
        // Should be the last 3 events (ids 8, 9, 10)
        assert_eq!(events[0].id, 8);
        assert_eq!(events[2].id, 10);
    }

    #[test]
    fn test_event_count() {
        let mut state = init();
        emit_event(&mut state, TOKEN_CONTRACT, "Transfer", b"t1", 100, 1000);
        emit_event(&mut state, NFT_CONTRACT, "Mint", b"m1", 101, 1001);

        let result = dispatch(&mut state, "event_count", b"", ALICE);
        let count: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    #[should_panic(expected = "Events: topic cannot be empty")]
    fn test_empty_topic_panics() {
        let mut state = init();
        emit_event(&mut state, TOKEN_CONTRACT, "", b"data", 100, 1000);
    }

    #[test]
    fn test_from_id_filtering() {
        let mut state = init();
        emit_event(&mut state, TOKEN_CONTRACT, "Transfer", b"old", 100, 1000);
        emit_event(&mut state, TOKEN_CONTRACT, "Transfer", b"mid", 101, 1001);
        emit_event(&mut state, TOKEN_CONTRACT, "Transfer", b"new", 102, 1002);

        // Only events with id >= 3
        let args = serde_json::to_vec(&GetEventsArgs {
            topic: "Transfer".to_string(),
            from_id: 3,
            limit: 10,
        })
        .unwrap();
        let result = dispatch(&mut state, "get_events", &args, ALICE);
        let events: Vec<IndexedEvent> = serde_json::from_slice(&result).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, b"new");
    }
}
