use serde::{Deserialize, Serialize};

use crate::types::{Address, Hash};

/// A structured on-chain event emitted by a transaction or contract execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainEvent {
    pub block_height: u64,
    pub tx_hash: Hash,
    pub contract: Option<Address>,
    pub event_name: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
    pub index: u64,
}

/// Filter criteria for querying events.
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub contract: Option<Address>,
    pub event_name: Option<String>,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
}

/// Append-only event log with query capabilities for chain event indexing.
pub struct EventLog {
    events: Vec<ChainEvent>,
    next_index: u64,
}

impl EventLog {
    /// Create an empty event log.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_index: 0,
        }
    }

    /// Emit (record) a new event, assigning it a monotonically increasing
    /// global index.
    pub fn emit(&mut self, mut event: ChainEvent) {
        event.index = self.next_index;
        self.next_index += 1;
        self.events.push(event);
    }

    /// Query events matching a filter. All filter fields are optional; when
    /// `None`, that criterion is not applied.
    pub fn query(&self, filter: &EventFilter) -> Vec<&ChainEvent> {
        self.events
            .iter()
            .filter(|e| {
                if let Some(ref addr) = filter.contract {
                    if e.contract.as_ref() != Some(addr) {
                        return false;
                    }
                }
                if let Some(ref name) = filter.event_name {
                    if e.event_name != *name {
                        return false;
                    }
                }
                if let Some(from) = filter.from_block {
                    if e.block_height < from {
                        return false;
                    }
                }
                if let Some(to) = filter.to_block {
                    if e.block_height > to {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    /// Return all events emitted in a specific block.
    pub fn events_in_block(&self, height: u64) -> Vec<&ChainEvent> {
        self.events
            .iter()
            .filter(|e| e.block_height == height)
            .collect()
    }

    /// Return all events emitted by a specific contract address.
    pub fn events_for_contract(&self, address: &Address) -> Vec<&ChainEvent> {
        self.events
            .iter()
            .filter(|e| e.contract.as_ref() == Some(address))
            .collect()
    }

    /// Return the most recent `limit` events (in emission order).
    pub fn latest_events(&self, limit: usize) -> Vec<&ChainEvent> {
        let start = self.events.len().saturating_sub(limit);
        self.events[start..].iter().collect()
    }

    /// Total number of events in the log.
    pub fn total_events(&self) -> u64 {
        self.events.len() as u64
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_address(byte: u8) -> Address {
        Address([byte; 32])
    }

    fn make_hash(byte: u8) -> Hash {
        Hash([byte; 32])
    }

    fn make_event(block: u64, contract_byte: Option<u8>, name: &str) -> ChainEvent {
        ChainEvent {
            block_height: block,
            tx_hash: make_hash(block as u8),
            contract: contract_byte.map(make_address),
            event_name: name.to_string(),
            data: serde_json::json!({"value": block}),
            timestamp: block * 1000,
            index: 0, // will be set by emit
        }
    }

    #[test]
    fn emit_assigns_sequential_indices() {
        let mut log = EventLog::new();
        log.emit(make_event(1, Some(1), "Transfer"));
        log.emit(make_event(1, Some(1), "Approval"));
        log.emit(make_event(2, Some(2), "Transfer"));
        assert_eq!(log.events[0].index, 0);
        assert_eq!(log.events[1].index, 1);
        assert_eq!(log.events[2].index, 2);
    }

    #[test]
    fn total_events() {
        let mut log = EventLog::new();
        assert_eq!(log.total_events(), 0);
        log.emit(make_event(1, Some(1), "Transfer"));
        assert_eq!(log.total_events(), 1);
        log.emit(make_event(2, Some(1), "Transfer"));
        assert_eq!(log.total_events(), 2);
    }

    #[test]
    fn events_in_block() {
        let mut log = EventLog::new();
        log.emit(make_event(1, Some(1), "A"));
        log.emit(make_event(1, Some(2), "B"));
        log.emit(make_event(2, Some(1), "C"));
        let block1 = log.events_in_block(1);
        assert_eq!(block1.len(), 2);
        let block2 = log.events_in_block(2);
        assert_eq!(block2.len(), 1);
        let block3 = log.events_in_block(3);
        assert!(block3.is_empty());
    }

    #[test]
    fn events_for_contract() {
        let mut log = EventLog::new();
        log.emit(make_event(1, Some(1), "A"));
        log.emit(make_event(2, Some(2), "B"));
        log.emit(make_event(3, Some(1), "C"));
        log.emit(make_event(4, None, "D"));
        let contract1 = log.events_for_contract(&make_address(1));
        assert_eq!(contract1.len(), 2);
        let contract2 = log.events_for_contract(&make_address(2));
        assert_eq!(contract2.len(), 1);
        let contract99 = log.events_for_contract(&make_address(99));
        assert!(contract99.is_empty());
    }

    #[test]
    fn latest_events() {
        let mut log = EventLog::new();
        for i in 0..10 {
            log.emit(make_event(i, Some(1), "E"));
        }
        let latest = log.latest_events(3);
        assert_eq!(latest.len(), 3);
        assert_eq!(latest[0].block_height, 7);
        assert_eq!(latest[1].block_height, 8);
        assert_eq!(latest[2].block_height, 9);
    }

    #[test]
    fn latest_events_more_than_available() {
        let mut log = EventLog::new();
        log.emit(make_event(1, Some(1), "E"));
        let latest = log.latest_events(100);
        assert_eq!(latest.len(), 1);
    }

    #[test]
    fn query_by_contract() {
        let mut log = EventLog::new();
        log.emit(make_event(1, Some(1), "Transfer"));
        log.emit(make_event(2, Some(2), "Transfer"));
        let filter = EventFilter {
            contract: Some(make_address(1)),
            ..Default::default()
        };
        let results = log.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].contract, Some(make_address(1)));
    }

    #[test]
    fn query_by_event_name() {
        let mut log = EventLog::new();
        log.emit(make_event(1, Some(1), "Transfer"));
        log.emit(make_event(2, Some(1), "Approval"));
        let filter = EventFilter {
            event_name: Some("Approval".to_string()),
            ..Default::default()
        };
        let results = log.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_name, "Approval");
    }

    #[test]
    fn query_by_block_range() {
        let mut log = EventLog::new();
        for i in 1..=10 {
            log.emit(make_event(i, Some(1), "E"));
        }
        let filter = EventFilter {
            from_block: Some(3),
            to_block: Some(7),
            ..Default::default()
        };
        let results = log.query(&filter);
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].block_height, 3);
        assert_eq!(results[4].block_height, 7);
    }

    #[test]
    fn query_combined_filters() {
        let mut log = EventLog::new();
        log.emit(make_event(1, Some(1), "Transfer"));
        log.emit(make_event(2, Some(1), "Approval"));
        log.emit(make_event(3, Some(2), "Transfer"));
        log.emit(make_event(4, Some(1), "Transfer"));
        let filter = EventFilter {
            contract: Some(make_address(1)),
            event_name: Some("Transfer".to_string()),
            from_block: Some(2),
            ..Default::default()
        };
        let results = log.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].block_height, 4);
    }

    #[test]
    fn query_empty_filter_returns_all() {
        let mut log = EventLog::new();
        log.emit(make_event(1, Some(1), "A"));
        log.emit(make_event(2, Some(2), "B"));
        let filter = EventFilter::default();
        assert_eq!(log.query(&filter).len(), 2);
    }

    #[test]
    fn empty_log_queries_return_empty() {
        let log = EventLog::new();
        assert!(log.events_in_block(1).is_empty());
        assert!(log.events_for_contract(&make_address(1)).is_empty());
        assert!(log.latest_events(10).is_empty());
        assert!(log.query(&EventFilter::default()).is_empty());
    }

    #[test]
    fn default_creates_empty_log() {
        let log = EventLog::default();
        assert_eq!(log.total_events(), 0);
    }

    #[test]
    fn chain_event_data_is_json() {
        let mut log = EventLog::new();
        let mut event = make_event(1, Some(1), "Transfer");
        event.data = serde_json::json!({"from": "0xaa", "to": "0xbb", "amount": 100});
        log.emit(event);
        let retrieved = &log.events[0];
        assert_eq!(retrieved.data["amount"], 100);
        assert_eq!(retrieved.data["from"], "0xaa");
    }
}
