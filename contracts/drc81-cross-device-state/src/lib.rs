use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-81  Cross-Device State Sync
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceState {
    pub device_id: Address,
    pub state_hash: [u8; 32],
    pub version: u64,
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncGroup {
    pub id: u64,
    pub owner: Address,
    pub devices: Vec<Address>,
    pub sync_interval: u64,
    pub last_sync: u64,
    pub state_hash: [u8; 32], // consensus state hash after last sync
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncEvent {
    pub group_id: u64,
    pub timestamp: u64,
    pub device_count: usize,
    pub conflict_detected: bool,
    pub resolved_hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CrossDeviceState {
    pub owner: Address,
    pub sync_groups: BTreeMap<u64, SyncGroup>,
    pub device_states: BTreeMap<(u64, Address), DeviceState>, // (group_id, device_addr)
    pub sync_history: Vec<SyncEvent>,
    pub next_group_id: u64,
}

impl CrossDeviceState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            sync_groups: BTreeMap::new(),
            device_states: BTreeMap::new(),
            sync_history: Vec::new(),
            next_group_id: 1,
        }
    }

    pub fn create_sync_group(&mut self, caller: Address, sync_interval: u64) -> u64 {
        assert!(sync_interval > 0, "DRC81: sync interval must be positive");

        let id = self.next_group_id;
        self.next_group_id += 1;
        self.sync_groups.insert(
            id,
            SyncGroup {
                id,
                owner: caller,
                devices: Vec::new(),
                sync_interval,
                last_sync: 0,
                state_hash: [0u8; 32],
            },
        );
        id
    }

    pub fn add_device(&mut self, caller: Address, group_id: u64, device: Address) {
        let group = self
            .sync_groups
            .get_mut(&group_id)
            .expect("DRC81: group not found");
        assert!(group.owner == caller, "DRC81: only owner can add devices");
        assert!(
            !group.devices.contains(&device),
            "DRC81: device already in group"
        );

        group.devices.push(device);
        self.device_states.insert(
            (group_id, device),
            DeviceState {
                device_id: device,
                state_hash: [0u8; 32],
                version: 0,
                last_updated: 0,
            },
        );
    }

    /// Each device reports its current state hash. If all match, sync is clean.
    /// If they diverge, a conflict is flagged.
    pub fn sync_state(
        &mut self,
        caller: Address,
        group_id: u64,
        device_hashes: Vec<(Address, [u8; 32], u64)>, // (device, hash, timestamp)
    ) -> bool {
        let group = self
            .sync_groups
            .get(&group_id)
            .expect("DRC81: group not found");
        assert!(group.owner == caller, "DRC81: only owner can trigger sync");

        // Update device states
        for (device, hash, timestamp) in &device_hashes {
            let key = (group_id, *device);
            if let Some(ds) = self.device_states.get_mut(&key) {
                ds.state_hash = *hash;
                ds.version += 1;
                ds.last_updated = *timestamp;
            }
        }

        // Check for conflicts (all hashes should match)
        let hashes: Vec<[u8; 32]> = device_hashes.iter().map(|(_, h, _)| *h).collect();
        let all_same = hashes.windows(2).all(|w| w[0] == w[1]);
        let resolved_hash = if all_same {
            hashes.first().copied().unwrap_or([0u8; 32])
        } else {
            [0u8; 32] // conflict — no consensus hash
        };

        let current_time = device_hashes.iter().map(|(_, _, t)| *t).max().unwrap_or(0);

        let group = self.sync_groups.get_mut(&group_id).unwrap();
        group.last_sync = current_time;
        if all_same {
            group.state_hash = resolved_hash;
        }

        self.sync_history.push(SyncEvent {
            group_id,
            timestamp: current_time,
            device_count: device_hashes.len(),
            conflict_detected: !all_same,
            resolved_hash,
        });

        all_same
    }

    /// Detect conflicting devices in a sync group (devices with different state hashes).
    pub fn detect_conflict(&self, group_id: u64) -> Vec<(Address, [u8; 32])> {
        let group = self
            .sync_groups
            .get(&group_id)
            .expect("DRC81: group not found");
        let mut hash_groups: BTreeMap<[u8; 32], Vec<Address>> = BTreeMap::new();

        for device in &group.devices {
            let key = (group_id, *device);
            if let Some(ds) = self.device_states.get(&key) {
                hash_groups.entry(ds.state_hash).or_default().push(*device);
            }
        }

        if hash_groups.len() <= 1 {
            return Vec::new(); // All in sync
        }

        // Return all devices with their hashes when conflict exists
        group
            .devices
            .iter()
            .filter_map(|d| {
                self.device_states
                    .get(&(group_id, *d))
                    .map(|ds| (*d, ds.state_hash))
            })
            .collect()
    }

    /// Resolve conflict by choosing a winner device. All other devices adopt its hash.
    pub fn resolve_conflict(
        &mut self,
        caller: Address,
        group_id: u64,
        winner_device: Address,
        timestamp: u64,
    ) {
        let group = self
            .sync_groups
            .get(&group_id)
            .expect("DRC81: group not found");
        assert!(
            group.owner == caller,
            "DRC81: only owner can resolve conflicts"
        );

        let winner_hash = self
            .device_states
            .get(&(group_id, winner_device))
            .expect("DRC81: winner device not in group")
            .state_hash;

        let devices = group.devices.clone();
        for device in &devices {
            let key = (group_id, *device);
            if let Some(ds) = self.device_states.get_mut(&key) {
                ds.state_hash = winner_hash;
                ds.version += 1;
                ds.last_updated = timestamp;
            }
        }

        let group = self.sync_groups.get_mut(&group_id).unwrap();
        group.state_hash = winner_hash;
        group.last_sync = timestamp;
    }

    pub fn sync_history_for(&self, group_id: u64) -> Vec<&SyncEvent> {
        self.sync_history
            .iter()
            .filter(|e| e.group_id == group_id)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateGroupArgs {
    sync_interval: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct AddDeviceArgs {
    group_id: u64,
    device: Address,
}
#[derive(Serialize, Deserialize, Debug)]
struct SyncArgs {
    group_id: u64,
    device_hashes: Vec<(Address, [u8; 32], u64)>,
}
#[derive(Serialize, Deserialize, Debug)]
struct GroupIdArgs {
    group_id: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct ResolveArgs {
    group_id: u64,
    winner_device: Address,
    timestamp: u64,
}

pub fn dispatch(
    state: &mut Option<CrossDeviceState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC81: already initialised");
            *state = Some(CrossDeviceState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_sync_group" => {
            let s = state.as_mut().expect("DRC81: not initialised");
            let a: CreateGroupArgs = serde_json::from_slice(args).expect("DRC81: bad args");
            let id = s.create_sync_group(caller, a.sync_interval);
            serde_json::to_vec(&id).unwrap()
        }
        "add_device" => {
            let s = state.as_mut().expect("DRC81: not initialised");
            let a: AddDeviceArgs = serde_json::from_slice(args).expect("DRC81: bad args");
            s.add_device(caller, a.group_id, a.device);
            serde_json::to_vec("ok").unwrap()
        }
        "sync_state" => {
            let s = state.as_mut().expect("DRC81: not initialised");
            let a: SyncArgs = serde_json::from_slice(args).expect("DRC81: bad args");
            let ok = s.sync_state(caller, a.group_id, a.device_hashes);
            serde_json::to_vec(&ok).unwrap()
        }
        "detect_conflict" => {
            let s = state.as_ref().expect("DRC81: not initialised");
            let a: GroupIdArgs = serde_json::from_slice(args).expect("DRC81: bad args");
            serde_json::to_vec(&s.detect_conflict(a.group_id)).unwrap()
        }
        "resolve_conflict" => {
            let s = state.as_mut().expect("DRC81: not initialised");
            let a: ResolveArgs = serde_json::from_slice(args).expect("DRC81: bad args");
            s.resolve_conflict(caller, a.group_id, a.winner_device, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "sync_history" => {
            let s = state.as_ref().expect("DRC81: not initialised");
            let a: GroupIdArgs = serde_json::from_slice(args).expect("DRC81: bad args");
            serde_json::to_vec(&s.sync_history_for(a.group_id)).unwrap()
        }
        _ => panic!("DRC81: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const DEV_1: Address = [1u8; 32];
    const DEV_2: Address = [2u8; 32];
    const DEV_3: Address = [3u8; 32];

    fn setup() -> (CrossDeviceState, u64) {
        let mut s = CrossDeviceState::new(OWNER);
        let gid = s.create_sync_group(OWNER, 60);
        s.add_device(OWNER, gid, DEV_1);
        s.add_device(OWNER, gid, DEV_2);
        s.add_device(OWNER, gid, DEV_3);
        (s, gid)
    }

    #[test]
    fn test_create_group_and_add_devices() {
        let (s, gid) = setup();
        let group = s.sync_groups.get(&gid).unwrap();
        assert_eq!(group.devices.len(), 3);
    }

    #[test]
    fn test_sync_state_clean() {
        let (mut s, gid) = setup();
        let hash = [0xAA; 32];
        let ok = s.sync_state(
            OWNER,
            gid,
            vec![
                (DEV_1, hash, 1000),
                (DEV_2, hash, 1000),
                (DEV_3, hash, 1000),
            ],
        );
        assert!(ok);
        assert_eq!(s.sync_groups.get(&gid).unwrap().state_hash, hash);
    }

    #[test]
    fn test_sync_state_conflict() {
        let (mut s, gid) = setup();
        let ok = s.sync_state(
            OWNER,
            gid,
            vec![
                (DEV_1, [0xAA; 32], 1000),
                (DEV_2, [0xBB; 32], 1000), // different hash
                (DEV_3, [0xAA; 32], 1000),
            ],
        );
        assert!(!ok); // conflict detected
    }

    #[test]
    fn test_detect_conflict() {
        let (mut s, gid) = setup();
        s.sync_state(
            OWNER,
            gid,
            vec![
                (DEV_1, [0xAA; 32], 1000),
                (DEV_2, [0xBB; 32], 1000),
                (DEV_3, [0xAA; 32], 1000),
            ],
        );
        let conflicts = s.detect_conflict(gid);
        assert_eq!(conflicts.len(), 3); // all devices listed with their hashes
    }

    #[test]
    fn test_resolve_conflict() {
        let (mut s, gid) = setup();
        s.sync_state(
            OWNER,
            gid,
            vec![
                (DEV_1, [0xAA; 32], 1000),
                (DEV_2, [0xBB; 32], 1000),
                (DEV_3, [0xAA; 32], 1000),
            ],
        );

        s.resolve_conflict(OWNER, gid, DEV_1, 2000);

        // All devices should now have DEV_1's hash
        let conflicts = s.detect_conflict(gid);
        assert_eq!(conflicts.len(), 0); // no conflict
        assert_eq!(s.sync_groups.get(&gid).unwrap().state_hash, [0xAA; 32]);
    }

    #[test]
    fn test_sync_history() {
        let (mut s, gid) = setup();
        let hash = [0xAA; 32];
        s.sync_state(
            OWNER,
            gid,
            vec![
                (DEV_1, hash, 1000),
                (DEV_2, hash, 1000),
                (DEV_3, hash, 1000),
            ],
        );
        s.sync_state(
            OWNER,
            gid,
            vec![
                (DEV_1, hash, 2000),
                (DEV_2, hash, 2000),
                (DEV_3, hash, 2000),
            ],
        );

        let history = s.sync_history_for(gid);
        assert_eq!(history.len(), 2);
        assert!(!history[0].conflict_detected);
    }

    #[test]
    #[should_panic(expected = "device already in group")]
    fn test_duplicate_device() {
        let (mut s, gid) = setup();
        s.add_device(OWNER, gid, DEV_1);
    }
}
