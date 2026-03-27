use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-62  Digital Twins — On-Chain State Sync for Physical Devices
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Alert {
    pub id: u64,
    pub timestamp: u64,
    pub severity: Severity,
    pub message: String,
    pub acknowledged: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StateSnapshot {
    pub state: BTreeMap<String, String>,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DigitalTwin {
    pub id: u64,
    pub physical_device_id: String,
    pub owner: Address,
    pub state: BTreeMap<String, String>,
    pub last_sync: u64,
    pub sync_count: u64,
    pub alerts: Vec<Alert>,
    pub next_alert_id: u64,
    pub history: Vec<StateSnapshot>,
    pub max_history: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TwinState {
    pub owner: Address,
    pub twins: BTreeMap<u64, DigitalTwin>,
    pub device_index: BTreeMap<String, u64>, // physical_device_id -> twin id
    pub next_twin_id: u64,
}

impl TwinState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            twins: BTreeMap::new(),
            device_index: BTreeMap::new(),
            next_twin_id: 1,
        }
    }

    pub fn create_twin(
        &mut self,
        caller: Address,
        physical_device_id: String,
        max_history: usize,
    ) -> u64 {
        assert!(!physical_device_id.is_empty(), "DRC62: device ID required");
        assert!(
            !self.device_index.contains_key(&physical_device_id),
            "DRC62: twin already exists for this device"
        );
        let id = self.next_twin_id;
        self.next_twin_id += 1;
        self.device_index.insert(physical_device_id.clone(), id);
        self.twins.insert(id, DigitalTwin {
            id,
            physical_device_id,
            owner: caller,
            state: BTreeMap::new(),
            last_sync: 0,
            sync_count: 0,
            alerts: Vec::new(),
            next_alert_id: 1,
            history: Vec::new(),
            max_history: if max_history == 0 { 100 } else { max_history },
        });
        id
    }

    pub fn sync_state(
        &mut self,
        caller: Address,
        twin_id: u64,
        state_updates: BTreeMap<String, String>,
        timestamp: u64,
    ) {
        let twin = self.twins.get_mut(&twin_id).expect("DRC62: twin not found");
        assert!(caller == twin.owner || caller == self.owner, "DRC62: not authorised");
        assert!(!state_updates.is_empty(), "DRC62: empty update");

        // Save current state to history before updating
        if !twin.state.is_empty() {
            let snapshot = StateSnapshot {
                state: twin.state.clone(),
                timestamp: twin.last_sync,
            };
            twin.history.push(snapshot);
            // Trim history
            if twin.history.len() > twin.max_history {
                twin.history.remove(0);
            }
        }

        // Apply updates
        for (key, value) in state_updates {
            twin.state.insert(key, value);
        }
        twin.last_sync = timestamp;
        twin.sync_count += 1;
    }

    pub fn get_twin(&self, twin_id: u64) -> Option<&DigitalTwin> {
        self.twins.get(&twin_id)
    }

    pub fn get_twin_by_device(&self, physical_device_id: &str) -> Option<&DigitalTwin> {
        self.device_index.get(physical_device_id)
            .and_then(|id| self.twins.get(id))
    }

    pub fn set_alert(
        &mut self,
        caller: Address,
        twin_id: u64,
        severity: Severity,
        message: String,
        timestamp: u64,
    ) -> u64 {
        let twin = self.twins.get_mut(&twin_id).expect("DRC62: twin not found");
        assert!(caller == twin.owner || caller == self.owner, "DRC62: not authorised");
        let alert_id = twin.next_alert_id;
        twin.next_alert_id += 1;
        twin.alerts.push(Alert {
            id: alert_id,
            timestamp,
            severity,
            message,
            acknowledged: false,
        });
        alert_id
    }

    pub fn acknowledge_alert(&mut self, caller: Address, twin_id: u64, alert_id: u64) {
        let twin = self.twins.get_mut(&twin_id).expect("DRC62: twin not found");
        assert!(caller == twin.owner || caller == self.owner, "DRC62: not authorised");
        let alert = twin.alerts.iter_mut()
            .find(|a| a.id == alert_id)
            .expect("DRC62: alert not found");
        assert!(!alert.acknowledged, "DRC62: alert already acknowledged");
        alert.acknowledged = true;
    }

    pub fn twin_history(&self, twin_id: u64) -> Option<&Vec<StateSnapshot>> {
        self.twins.get(&twin_id).map(|t| &t.history)
    }

    pub fn unacknowledged_alerts(&self, twin_id: u64) -> Vec<&Alert> {
        let twin = self.twins.get(&twin_id).expect("DRC62: twin not found");
        twin.alerts.iter().filter(|a| !a.acknowledged).collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateTwinArgs { physical_device_id: String, max_history: usize }

#[derive(Serialize, Deserialize, Debug)]
struct SyncStateArgs { twin_id: u64, state_updates: BTreeMap<String, String>, timestamp: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct TwinIdArgs { twin_id: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct DeviceIdArgs { physical_device_id: String }

#[derive(Serialize, Deserialize, Debug)]
struct SetAlertArgs { twin_id: u64, severity: Severity, message: String, timestamp: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct AckAlertArgs { twin_id: u64, alert_id: u64 }

pub fn dispatch(
    state: &mut Option<TwinState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC62: already initialised");
            *state = Some(TwinState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_twin" => {
            let s = state.as_mut().expect("DRC62: not initialised");
            let a: CreateTwinArgs = serde_json::from_slice(args).expect("DRC62: bad args");
            let id = s.create_twin(caller, a.physical_device_id, a.max_history);
            serde_json::to_vec(&id).unwrap()
        }
        "sync_state" => {
            let s = state.as_mut().expect("DRC62: not initialised");
            let a: SyncStateArgs = serde_json::from_slice(args).expect("DRC62: bad args");
            s.sync_state(caller, a.twin_id, a.state_updates, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "get_twin" => {
            let s = state.as_ref().expect("DRC62: not initialised");
            let a: TwinIdArgs = serde_json::from_slice(args).expect("DRC62: bad args");
            serde_json::to_vec(&s.get_twin(a.twin_id)).unwrap()
        }
        "get_twin_by_device" => {
            let s = state.as_ref().expect("DRC62: not initialised");
            let a: DeviceIdArgs = serde_json::from_slice(args).expect("DRC62: bad args");
            serde_json::to_vec(&s.get_twin_by_device(&a.physical_device_id)).unwrap()
        }
        "set_alert" => {
            let s = state.as_mut().expect("DRC62: not initialised");
            let a: SetAlertArgs = serde_json::from_slice(args).expect("DRC62: bad args");
            let id = s.set_alert(caller, a.twin_id, a.severity, a.message, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        "acknowledge_alert" => {
            let s = state.as_mut().expect("DRC62: not initialised");
            let a: AckAlertArgs = serde_json::from_slice(args).expect("DRC62: bad args");
            s.acknowledge_alert(caller, a.twin_id, a.alert_id);
            serde_json::to_vec("ok").unwrap()
        }
        "twin_history" => {
            let s = state.as_ref().expect("DRC62: not initialised");
            let a: TwinIdArgs = serde_json::from_slice(args).expect("DRC62: bad args");
            serde_json::to_vec(&s.twin_history(a.twin_id)).unwrap()
        }
        "unacknowledged_alerts" => {
            let s = state.as_ref().expect("DRC62: not initialised");
            let a: TwinIdArgs = serde_json::from_slice(args).expect("DRC62: bad args");
            let alerts: Vec<&Alert> = s.unacknowledged_alerts(a.twin_id);
            serde_json::to_vec(&alerts).unwrap()
        }
        _ => panic!("DRC62: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const DEVICE_OWNER: Address = [1u8; 32];

    fn setup_twin() -> (TwinState, u64) {
        let mut s = TwinState::new(OWNER);
        let id = s.create_twin(DEVICE_OWNER, "robot-arm-001".into(), 5);
        (s, id)
    }

    #[test]
    fn test_create_and_get_twin() {
        let (s, twin_id) = setup_twin();
        let twin = s.get_twin(twin_id).unwrap();
        assert_eq!(twin.physical_device_id, "robot-arm-001");
        assert_eq!(twin.sync_count, 0);

        let by_device = s.get_twin_by_device("robot-arm-001").unwrap();
        assert_eq!(by_device.id, twin_id);
    }

    #[test]
    fn test_sync_state() {
        let (mut s, twin_id) = setup_twin();
        let mut updates = BTreeMap::new();
        updates.insert("temperature".into(), "72.5".into());
        updates.insert("rpm".into(), "1500".into());
        s.sync_state(DEVICE_OWNER, twin_id, updates, 100);

        let twin = s.get_twin(twin_id).unwrap();
        assert_eq!(twin.state["temperature"], "72.5");
        assert_eq!(twin.state["rpm"], "1500");
        assert_eq!(twin.sync_count, 1);
        assert_eq!(twin.last_sync, 100);
    }

    #[test]
    fn test_state_history_preserved() {
        let (mut s, twin_id) = setup_twin();
        let mut u1 = BTreeMap::new();
        u1.insert("temp".into(), "70".into());
        s.sync_state(DEVICE_OWNER, twin_id, u1, 100);

        let mut u2 = BTreeMap::new();
        u2.insert("temp".into(), "75".into());
        s.sync_state(DEVICE_OWNER, twin_id, u2, 200);

        let history = s.twin_history(twin_id).unwrap();
        assert_eq!(history.len(), 1); // first sync had empty state, second sync saved the old state
        assert_eq!(history[0].state["temp"], "70");
        assert_eq!(history[0].timestamp, 100);
    }

    #[test]
    fn test_alerts_and_acknowledge() {
        let (mut s, twin_id) = setup_twin();
        let a1 = s.set_alert(DEVICE_OWNER, twin_id, Severity::Warning, "Overheating".into(), 100);
        let a2 = s.set_alert(DEVICE_OWNER, twin_id, Severity::Critical, "Motor stall".into(), 200);

        let unacked = s.unacknowledged_alerts(twin_id);
        assert_eq!(unacked.len(), 2);

        s.acknowledge_alert(DEVICE_OWNER, twin_id, a1);
        let unacked = s.unacknowledged_alerts(twin_id);
        assert_eq!(unacked.len(), 1);
        assert_eq!(unacked[0].id, a2);
    }

    #[test]
    fn test_history_trim() {
        let (mut s, twin_id) = setup_twin(); // max_history = 5
        for i in 0..8 {
            let mut u = BTreeMap::new();
            u.insert("val".into(), format!("{i}"));
            s.sync_state(DEVICE_OWNER, twin_id, u, i * 100);
        }
        let history = s.twin_history(twin_id).unwrap();
        // 8 syncs: first has empty state (no snapshot), then 7 snapshots, trimmed to 5
        assert_eq!(history.len(), 5);
    }

    #[test]
    #[should_panic(expected = "twin already exists")]
    fn test_duplicate_device_rejected() {
        let (mut s, _) = setup_twin();
        s.create_twin(DEVICE_OWNER, "robot-arm-001".into(), 10);
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state = None;
        dispatch(&mut state, "init", b"{}", OWNER);
        let args = serde_json::to_vec(&CreateTwinArgs {
            physical_device_id: "sensor-42".into(),
            max_history: 10,
        }).unwrap();
        let result = dispatch(&mut state, "create_twin", &args, DEVICE_OWNER);
        let id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(id, 1);
    }
}
