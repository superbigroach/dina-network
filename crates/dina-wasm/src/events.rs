use dina_core::error::DinaError;
use dina_core::types::{Address, Hash};

/// An event emitted by a contract during execution, with indexed topics
/// for efficient off-chain filtering and querying.
#[derive(Debug, Clone)]
pub struct ContractEvent {
    /// The contract that emitted this event.
    pub contract: Address,
    /// Human-readable event name (e.g. "Transfer", "Approval").
    pub name: String,
    /// Arbitrary serialized event payload.
    pub data: Vec<u8>,
    /// Indexed fields (hashed) for topic-based filtering.
    pub topics: Vec<Hash>,
    /// Block height at which the event was emitted.
    pub block_height: u64,
    /// Transaction index within the block.
    pub tx_index: u32,
    /// Log index within the transaction (auto-assigned).
    pub log_index: u32,
}

/// Collects contract events during execution and provides query methods.
///
/// Enforces a per-call event limit to prevent abuse. Events are assigned
/// sequential `log_index` values starting from 0.
#[derive(Debug)]
pub struct ContractEventCollector {
    events: Vec<ContractEvent>,
    max_events_per_call: u32,
}

impl ContractEventCollector {
    /// Create a new collector with the given per-call event limit.
    pub fn new(max_events: u32) -> Self {
        Self {
            events: Vec::new(),
            max_events_per_call: max_events,
        }
    }

    /// Emit a new event.
    ///
    /// The `log_index` is automatically assigned based on the current count.
    /// Returns an error if the event limit has been reached.
    pub fn emit(
        &mut self,
        contract: Address,
        name: String,
        data: Vec<u8>,
        topics: Vec<Hash>,
        block_height: u64,
        tx_index: u32,
    ) -> Result<(), DinaError> {
        if self.events.len() as u32 >= self.max_events_per_call {
            return Err(DinaError::WasmExecutionError(format!(
                "event limit exceeded: max {} events per call",
                self.max_events_per_call
            )));
        }

        let log_index = self.events.len() as u32;

        self.events.push(ContractEvent {
            contract,
            name,
            data,
            topics,
            block_height,
            tx_index,
            log_index,
        });

        Ok(())
    }

    /// All collected events.
    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    /// Filter events by the emitting contract address.
    pub fn events_by_contract(&self, contract: &Address) -> Vec<&ContractEvent> {
        self.events
            .iter()
            .filter(|e| &e.contract == contract)
            .collect()
    }

    /// Filter events that contain a specific topic hash.
    pub fn events_by_topic(&self, topic: &Hash) -> Vec<&ContractEvent> {
        self.events
            .iter()
            .filter(|e| e.topics.contains(topic))
            .collect()
    }

    /// Drain all events, returning them and leaving the collector empty.
    pub fn drain(&mut self) -> Vec<ContractEvent> {
        std::mem::take(&mut self.events)
    }

    /// Number of events collected so far.
    pub fn count(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(byte: u8) -> Address {
        Address([byte; 32])
    }

    fn topic(byte: u8) -> Hash {
        Hash([byte; 32])
    }

    #[test]
    fn new_collector_is_empty() {
        let collector = ContractEventCollector::new(100);
        assert_eq!(collector.count(), 0);
        assert!(collector.events().is_empty());
    }

    #[test]
    fn emit_adds_event() {
        let mut collector = ContractEventCollector::new(100);
        collector
            .emit(
                addr(1),
                "Transfer".into(),
                vec![1, 2, 3],
                vec![topic(0xAA)],
                42,
                0,
            )
            .unwrap();

        assert_eq!(collector.count(), 1);
        let event = &collector.events()[0];
        assert_eq!(event.contract, addr(1));
        assert_eq!(event.name, "Transfer");
        assert_eq!(event.data, vec![1, 2, 3]);
        assert_eq!(event.topics.len(), 1);
        assert_eq!(event.block_height, 42);
        assert_eq!(event.log_index, 0);
    }

    #[test]
    fn log_index_auto_increments() {
        let mut collector = ContractEventCollector::new(100);
        for i in 0..5 {
            collector
                .emit(addr(1), format!("Event{i}"), vec![], vec![], 1, 0)
                .unwrap();
        }
        for (i, event) in collector.events().iter().enumerate() {
            assert_eq!(event.log_index, i as u32);
        }
    }

    #[test]
    fn event_limit_enforced() {
        let mut collector = ContractEventCollector::new(2);
        collector
            .emit(addr(1), "A".into(), vec![], vec![], 1, 0)
            .unwrap();
        collector
            .emit(addr(1), "B".into(), vec![], vec![], 1, 0)
            .unwrap();
        let result = collector.emit(addr(1), "C".into(), vec![], vec![], 1, 0);
        assert!(result.is_err());
    }

    #[test]
    fn events_by_contract_filters_correctly() {
        let mut collector = ContractEventCollector::new(100);
        collector
            .emit(addr(1), "A".into(), vec![], vec![], 1, 0)
            .unwrap();
        collector
            .emit(addr(2), "B".into(), vec![], vec![], 1, 0)
            .unwrap();
        collector
            .emit(addr(1), "C".into(), vec![], vec![], 1, 0)
            .unwrap();

        let contract1_events = collector.events_by_contract(&addr(1));
        assert_eq!(contract1_events.len(), 2);
        assert_eq!(contract1_events[0].name, "A");
        assert_eq!(contract1_events[1].name, "C");

        let contract2_events = collector.events_by_contract(&addr(2));
        assert_eq!(contract2_events.len(), 1);
    }

    #[test]
    fn events_by_topic_filters_correctly() {
        let mut collector = ContractEventCollector::new(100);
        let transfer_topic = topic(0x01);
        let approval_topic = topic(0x02);

        collector
            .emit(
                addr(1),
                "Transfer".into(),
                vec![],
                vec![transfer_topic],
                1,
                0,
            )
            .unwrap();
        collector
            .emit(
                addr(1),
                "Approval".into(),
                vec![],
                vec![approval_topic],
                1,
                0,
            )
            .unwrap();
        collector
            .emit(
                addr(2),
                "Transfer".into(),
                vec![],
                vec![transfer_topic, approval_topic],
                1,
                0,
            )
            .unwrap();

        let transfer_events = collector.events_by_topic(&transfer_topic);
        assert_eq!(transfer_events.len(), 2);

        let approval_events = collector.events_by_topic(&approval_topic);
        assert_eq!(approval_events.len(), 2);
    }

    #[test]
    fn drain_clears_events() {
        let mut collector = ContractEventCollector::new(100);
        collector
            .emit(addr(1), "A".into(), vec![], vec![], 1, 0)
            .unwrap();
        collector
            .emit(addr(1), "B".into(), vec![], vec![], 1, 0)
            .unwrap();

        let drained = collector.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(collector.count(), 0);
        assert!(collector.events().is_empty());
    }

    #[test]
    fn events_by_nonexistent_contract_returns_empty() {
        let collector = ContractEventCollector::new(100);
        assert!(collector.events_by_contract(&addr(99)).is_empty());
    }

    #[test]
    fn events_by_nonexistent_topic_returns_empty() {
        let collector = ContractEventCollector::new(100);
        assert!(collector.events_by_topic(&topic(0xFF)).is_empty());
    }

    #[test]
    fn multiple_topics_per_event() {
        let mut collector = ContractEventCollector::new(100);
        let t1 = topic(0x01);
        let t2 = topic(0x02);
        let t3 = topic(0x03);

        collector
            .emit(addr(1), "Multi".into(), vec![42], vec![t1, t2, t3], 10, 5)
            .unwrap();

        assert_eq!(collector.events_by_topic(&t1).len(), 1);
        assert_eq!(collector.events_by_topic(&t2).len(), 1);
        assert_eq!(collector.events_by_topic(&t3).len(), 1);

        let event = &collector.events()[0];
        assert_eq!(event.tx_index, 5);
        assert_eq!(event.block_height, 10);
        assert_eq!(event.data, vec![42]);
    }
}
