use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-72  Device Compute Cluster
// ---------------------------------------------------------------------------
// Pool multiple Cognitum Seeds into a compute cluster for larger workloads.
// Manages device resources, workload queue, and automatic assignment.

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ClusterStatus {
    Active,
    Paused,
    Disbanded,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WorkloadStatus {
    Queued,
    Assigned,
    Running,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClusterDevice {
    pub device_id: String,
    pub address: Address,
    pub compute_power: u64, // abstract units (e.g., TFLOPS * 100)
    pub memory: u64,        // MB
    pub storage: u64,       // MB
    pub available: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Workload {
    pub id: u64,
    pub requester: Address,
    pub description: String,
    pub required_compute: u64,
    pub required_memory: u64,
    pub assigned_devices: Vec<String>, // device_ids
    pub status: WorkloadStatus,
    pub cost: u64,
    pub result_hash: Option<Vec<u8>>,
    pub submitted_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceCluster {
    pub id: u64,
    pub owner: Address,
    pub devices: Vec<ClusterDevice>,
    pub total_compute_power: u64,
    pub total_memory: u64,
    pub total_storage: u64,
    pub status: ClusterStatus,
    pub workload_queue: Vec<u64>, // workload ids
    pub price_per_compute_unit: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClusterState {
    pub admin: Address,
    pub clusters: BTreeMap<u64, DeviceCluster>,
    pub workloads: BTreeMap<u64, Workload>,
    pub next_cluster_id: u64,
    pub next_workload_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClusterStats {
    pub total_compute: u64,
    pub available_compute: u64,
    pub total_memory: u64,
    pub available_memory: u64,
    pub total_storage: u64,
    pub device_count: usize,
    pub available_devices: usize,
    pub queued_workloads: usize,
}

impl ClusterState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            clusters: BTreeMap::new(),
            workloads: BTreeMap::new(),
            next_cluster_id: 1,
            next_workload_id: 1,
        }
    }

    pub fn create_cluster(
        &mut self,
        caller: Address,
        price_per_compute_unit: u64,
    ) -> u64 {
        let id = self.next_cluster_id;
        self.next_cluster_id += 1;
        self.clusters.insert(id, DeviceCluster {
            id,
            owner: caller,
            devices: Vec::new(),
            total_compute_power: 0,
            total_memory: 0,
            total_storage: 0,
            status: ClusterStatus::Active,
            workload_queue: Vec::new(),
            price_per_compute_unit,
        });
        id
    }

    pub fn add_device(
        &mut self,
        caller: Address,
        cluster_id: u64,
        device_id: String,
        device_address: Address,
        compute_power: u64,
        memory: u64,
        storage: u64,
    ) {
        let cluster = self.clusters.get_mut(&cluster_id).expect("DRC72: cluster not found");
        assert!(caller == cluster.owner, "DRC72: not cluster owner");
        assert!(cluster.status == ClusterStatus::Active, "DRC72: cluster not active");
        assert!(!device_id.is_empty(), "DRC72: device_id required");
        assert!(!cluster.devices.iter().any(|d| d.device_id == device_id),
            "DRC72: device already in cluster");

        cluster.devices.push(ClusterDevice {
            device_id,
            address: device_address,
            compute_power,
            memory,
            storage,
            available: true,
        });
        cluster.total_compute_power += compute_power;
        cluster.total_memory += memory;
        cluster.total_storage += storage;
    }

    pub fn remove_device(
        &mut self,
        caller: Address,
        cluster_id: u64,
        device_id: &str,
    ) {
        let cluster = self.clusters.get_mut(&cluster_id).expect("DRC72: cluster not found");
        assert!(caller == cluster.owner, "DRC72: not cluster owner");
        let pos = cluster.devices.iter().position(|d| d.device_id == device_id)
            .expect("DRC72: device not found");
        let device = cluster.devices.remove(pos);
        cluster.total_compute_power -= device.compute_power;
        cluster.total_memory -= device.memory;
        cluster.total_storage -= device.storage;
    }

    pub fn submit_workload(
        &mut self,
        caller: Address,
        cluster_id: u64,
        description: String,
        required_compute: u64,
        required_memory: u64,
        payment: u64,
        timestamp: u64,
    ) -> u64 {
        let cluster = self.clusters.get_mut(&cluster_id).expect("DRC72: cluster not found");
        assert!(cluster.status == ClusterStatus::Active, "DRC72: cluster not active");
        let expected_cost = required_compute * cluster.price_per_compute_unit;
        assert!(payment >= expected_cost, "DRC72: insufficient payment");

        let wid = self.next_workload_id;
        self.next_workload_id += 1;

        self.workloads.insert(wid, Workload {
            id: wid,
            requester: caller,
            description,
            required_compute,
            required_memory,
            assigned_devices: Vec::new(),
            status: WorkloadStatus::Queued,
            cost: payment,
            result_hash: None,
            submitted_at: timestamp,
        });

        cluster.workload_queue.push(wid);
        wid
    }

    pub fn assign_workload(
        &mut self,
        caller: Address,
        cluster_id: u64,
        workload_id: u64,
        device_ids: Vec<String>,
    ) {
        let cluster = self.clusters.get_mut(&cluster_id).expect("DRC72: cluster not found");
        assert!(caller == cluster.owner, "DRC72: not cluster owner");

        // Validate devices exist and are available
        let mut total_compute = 0u64;
        let mut total_memory = 0u64;
        for did in &device_ids {
            let dev = cluster.devices.iter().find(|d| &d.device_id == did)
                .expect("DRC72: device not in cluster");
            assert!(dev.available, "DRC72: device not available");
            total_compute += dev.compute_power;
            total_memory += dev.memory;
        }

        let workload = self.workloads.get_mut(&workload_id).expect("DRC72: workload not found");
        assert!(workload.status == WorkloadStatus::Queued, "DRC72: workload not queued");
        assert!(total_compute >= workload.required_compute, "DRC72: insufficient compute");
        assert!(total_memory >= workload.required_memory, "DRC72: insufficient memory");

        workload.assigned_devices = device_ids.clone();
        workload.status = WorkloadStatus::Assigned;

        // Mark devices as unavailable
        for did in &device_ids {
            if let Some(dev) = cluster.devices.iter_mut().find(|d| &d.device_id == did) {
                dev.available = false;
            }
        }
    }

    pub fn complete_workload(
        &mut self,
        caller: Address,
        cluster_id: u64,
        workload_id: u64,
        result_hash: Vec<u8>,
    ) {
        let cluster = self.clusters.get_mut(&cluster_id).expect("DRC72: cluster not found");
        assert!(caller == cluster.owner, "DRC72: not cluster owner");

        let workload = self.workloads.get_mut(&workload_id).expect("DRC72: workload not found");
        assert!(workload.status == WorkloadStatus::Assigned, "DRC72: workload not assigned");
        workload.result_hash = Some(result_hash);
        workload.status = WorkloadStatus::Completed;

        // Free devices
        for did in &workload.assigned_devices {
            if let Some(dev) = cluster.devices.iter_mut().find(|d| &d.device_id == did) {
                dev.available = true;
            }
        }

        // Remove from queue
        cluster.workload_queue.retain(|id| *id != workload_id);
    }

    pub fn cluster_stats(&self, cluster_id: u64) -> ClusterStats {
        let cluster = self.clusters.get(&cluster_id).expect("DRC72: cluster not found");
        let available_compute: u64 = cluster.devices.iter()
            .filter(|d| d.available).map(|d| d.compute_power).sum();
        let available_memory: u64 = cluster.devices.iter()
            .filter(|d| d.available).map(|d| d.memory).sum();
        let available_devices = cluster.devices.iter().filter(|d| d.available).count();

        ClusterStats {
            total_compute: cluster.total_compute_power,
            available_compute,
            total_memory: cluster.total_memory,
            available_memory,
            total_storage: cluster.total_storage,
            device_count: cluster.devices.len(),
            available_devices,
            queued_workloads: cluster.workload_queue.len(),
        }
    }

    pub fn available_capacity(&self, cluster_id: u64) -> (u64, u64) {
        let cluster = self.clusters.get(&cluster_id).expect("DRC72: cluster not found");
        let compute: u64 = cluster.devices.iter()
            .filter(|d| d.available).map(|d| d.compute_power).sum();
        let memory: u64 = cluster.devices.iter()
            .filter(|d| d.available).map(|d| d.memory).sum();
        (compute, memory)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateClusterArgs { price_per_compute_unit: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct AddDeviceArgs { cluster_id: u64, device_id: String, device_address: Address, compute_power: u64, memory: u64, storage: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct RemoveDeviceArgs { cluster_id: u64, device_id: String }
#[derive(Serialize, Deserialize, Debug)]
struct SubmitWorkloadArgs { cluster_id: u64, description: String, required_compute: u64, required_memory: u64, payment: u64, timestamp: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct AssignWorkloadArgs { cluster_id: u64, workload_id: u64, device_ids: Vec<String> }
#[derive(Serialize, Deserialize, Debug)]
struct CompleteWorkloadArgs { cluster_id: u64, workload_id: u64, result_hash: Vec<u8> }
#[derive(Serialize, Deserialize, Debug)]
struct ClusterIdArgs { cluster_id: u64 }

pub fn dispatch(
    state: &mut Option<ClusterState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC72: already initialised");
            *state = Some(ClusterState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_cluster" => {
            let s = state.as_mut().expect("DRC72: not initialised");
            let a: CreateClusterArgs = serde_json::from_slice(args).expect("DRC72: bad args");
            let id = s.create_cluster(caller, a.price_per_compute_unit);
            serde_json::to_vec(&id).unwrap()
        }
        "add_device" => {
            let s = state.as_mut().expect("DRC72: not initialised");
            let a: AddDeviceArgs = serde_json::from_slice(args).expect("DRC72: bad args");
            s.add_device(caller, a.cluster_id, a.device_id, a.device_address, a.compute_power, a.memory, a.storage);
            serde_json::to_vec("ok").unwrap()
        }
        "remove_device" => {
            let s = state.as_mut().expect("DRC72: not initialised");
            let a: RemoveDeviceArgs = serde_json::from_slice(args).expect("DRC72: bad args");
            s.remove_device(caller, a.cluster_id, &a.device_id);
            serde_json::to_vec("ok").unwrap()
        }
        "submit_workload" => {
            let s = state.as_mut().expect("DRC72: not initialised");
            let a: SubmitWorkloadArgs = serde_json::from_slice(args).expect("DRC72: bad args");
            let id = s.submit_workload(caller, a.cluster_id, a.description, a.required_compute, a.required_memory, a.payment, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        "assign_workload" => {
            let s = state.as_mut().expect("DRC72: not initialised");
            let a: AssignWorkloadArgs = serde_json::from_slice(args).expect("DRC72: bad args");
            s.assign_workload(caller, a.cluster_id, a.workload_id, a.device_ids);
            serde_json::to_vec("ok").unwrap()
        }
        "complete_workload" => {
            let s = state.as_mut().expect("DRC72: not initialised");
            let a: CompleteWorkloadArgs = serde_json::from_slice(args).expect("DRC72: bad args");
            s.complete_workload(caller, a.cluster_id, a.workload_id, a.result_hash);
            serde_json::to_vec("ok").unwrap()
        }
        "cluster_stats" => {
            let s = state.as_ref().expect("DRC72: not initialised");
            let a: ClusterIdArgs = serde_json::from_slice(args).expect("DRC72: bad args");
            serde_json::to_vec(&s.cluster_stats(a.cluster_id)).unwrap()
        }
        "available_capacity" => {
            let s = state.as_ref().expect("DRC72: not initialised");
            let a: ClusterIdArgs = serde_json::from_slice(args).expect("DRC72: bad args");
            let cap = s.available_capacity(a.cluster_id);
            serde_json::to_vec(&cap).unwrap()
        }
        _ => panic!("DRC72: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: Address = [0u8; 32];
    const CLUSTER_OWNER: Address = [1u8; 32];
    const USER: Address = [2u8; 32];
    const DEV_ADDR_A: Address = [10u8; 32];
    const DEV_ADDR_B: Address = [11u8; 32];

    fn setup_cluster_with_devices() -> (ClusterState, u64) {
        let mut s = ClusterState::new(ADMIN);
        let cid = s.create_cluster(CLUSTER_OWNER, 10);
        s.add_device(CLUSTER_OWNER, cid, "gpu-node-1".into(), DEV_ADDR_A, 100, 16000, 500000);
        s.add_device(CLUSTER_OWNER, cid, "gpu-node-2".into(), DEV_ADDR_B, 80, 8000, 250000);
        (s, cid)
    }

    #[test]
    fn test_create_cluster_and_add_devices() {
        let (s, cid) = setup_cluster_with_devices();
        let cluster = s.clusters.get(&cid).unwrap();
        assert_eq!(cluster.devices.len(), 2);
        assert_eq!(cluster.total_compute_power, 180);
        assert_eq!(cluster.total_memory, 24000);
        assert_eq!(cluster.total_storage, 750000);
    }

    #[test]
    fn test_submit_and_assign_workload() {
        let (mut s, cid) = setup_cluster_with_devices();
        let wid = s.submit_workload(USER, cid, "train model".into(), 50, 8000, 500, 1000);
        assert_eq!(s.workloads.get(&wid).unwrap().status, WorkloadStatus::Queued);

        s.assign_workload(CLUSTER_OWNER, cid, wid, vec!["gpu-node-1".into()]);
        assert_eq!(s.workloads.get(&wid).unwrap().status, WorkloadStatus::Assigned);

        // gpu-node-1 should be unavailable now
        let cluster = s.clusters.get(&cid).unwrap();
        assert!(!cluster.devices.iter().find(|d| d.device_id == "gpu-node-1").unwrap().available);
    }

    #[test]
    fn test_complete_workload_frees_devices() {
        let (mut s, cid) = setup_cluster_with_devices();
        let wid = s.submit_workload(USER, cid, "infer".into(), 50, 4000, 500, 1000);
        s.assign_workload(CLUSTER_OWNER, cid, wid, vec!["gpu-node-2".into()]);
        s.complete_workload(CLUSTER_OWNER, cid, wid, vec![0xDE, 0xAD]);

        let w = s.workloads.get(&wid).unwrap();
        assert_eq!(w.status, WorkloadStatus::Completed);
        assert_eq!(w.result_hash, Some(vec![0xDE, 0xAD]));

        let cluster = s.clusters.get(&cid).unwrap();
        assert!(cluster.devices.iter().all(|d| d.available));
        assert!(!cluster.workload_queue.contains(&wid));
    }

    #[test]
    fn test_cluster_stats() {
        let (mut s, cid) = setup_cluster_with_devices();
        let wid = s.submit_workload(USER, cid, "work".into(), 50, 4000, 500, 1000);
        s.assign_workload(CLUSTER_OWNER, cid, wid, vec!["gpu-node-1".into()]);

        let stats = s.cluster_stats(cid);
        assert_eq!(stats.device_count, 2);
        assert_eq!(stats.available_devices, 1); // gpu-node-2 only
        assert_eq!(stats.available_compute, 80); // gpu-node-2's compute
        assert_eq!(stats.queued_workloads, 1);
    }

    #[test]
    fn test_remove_device() {
        let (mut s, cid) = setup_cluster_with_devices();
        s.remove_device(CLUSTER_OWNER, cid, "gpu-node-1");
        let cluster = s.clusters.get(&cid).unwrap();
        assert_eq!(cluster.devices.len(), 1);
        assert_eq!(cluster.total_compute_power, 80);
    }

    #[test]
    fn test_available_capacity() {
        let (s, cid) = setup_cluster_with_devices();
        let (compute, memory) = s.available_capacity(cid);
        assert_eq!(compute, 180);
        assert_eq!(memory, 24000);
    }

    #[test]
    #[should_panic(expected = "insufficient payment")]
    fn test_underpayment_rejected() {
        let (mut s, cid) = setup_cluster_with_devices();
        // price_per_compute_unit = 10, required_compute = 50, so need 500
        s.submit_workload(USER, cid, "cheap".into(), 50, 4000, 100, 1000);
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state = None;
        dispatch(&mut state, "init", b"{}", ADMIN);
        let args = serde_json::to_vec(&CreateClusterArgs { price_per_compute_unit: 5 }).unwrap();
        let result = dispatch(&mut state, "create_cluster", &args, CLUSTER_OWNER);
        let id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(id, 1);

        let stats_args = serde_json::to_vec(&ClusterIdArgs { cluster_id: 1 }).unwrap();
        let stats_result = dispatch(&mut state, "cluster_stats", &stats_args, USER);
        let stats: ClusterStats = serde_json::from_slice(&stats_result).unwrap();
        assert_eq!(stats.device_count, 0);
    }
}
