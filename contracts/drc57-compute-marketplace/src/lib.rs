use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-57  Compute Marketplace — Buy/Sell CPU, GPU, Storage Between Devices
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComputeOffer {
    pub id: u64,
    pub provider: Address,
    pub cpu_cores: u32,
    pub gpu_available: bool,
    pub memory_gb: u32,
    pub storage_gb: u32,
    pub price_per_hour: u64,
    pub available: bool,
    pub total_jobs: u64,
    pub total_earned: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResourceRequest {
    pub cpu_cores: u32,
    pub gpu_needed: bool,
    pub memory_gb: u32,
    pub storage_gb: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComputeJob {
    pub id: u64,
    pub client: Address,
    pub provider: Address,
    pub offer_id: u64,
    pub resources: ResourceRequest,
    pub duration_hours: u32,
    pub total_cost: u64,
    pub status: JobStatus,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub result_hash: Option<[u8; 32]>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComputeState {
    pub owner: Address,
    pub offers: BTreeMap<u64, ComputeOffer>,
    pub jobs: BTreeMap<u64, ComputeJob>,
    pub next_offer_id: u64,
    pub next_job_id: u64,
    pub balances: BTreeMap<Address, u64>,
}

impl ComputeState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            offers: BTreeMap::new(),
            jobs: BTreeMap::new(),
            next_offer_id: 1,
            next_job_id: 1,
            balances: BTreeMap::new(),
        }
    }

    pub fn list_compute(
        &mut self,
        caller: Address,
        cpu_cores: u32,
        gpu_available: bool,
        memory_gb: u32,
        storage_gb: u32,
        price_per_hour: u64,
    ) -> u64 {
        assert!(cpu_cores > 0 || storage_gb > 0, "DRC57: must offer some resources");
        assert!(price_per_hour > 0, "DRC57: price must be positive");
        let id = self.next_offer_id;
        self.next_offer_id += 1;
        self.offers.insert(id, ComputeOffer {
            id,
            provider: caller,
            cpu_cores,
            gpu_available,
            memory_gb,
            storage_gb,
            price_per_hour,
            available: true,
            total_jobs: 0,
            total_earned: 0,
        });
        id
    }

    pub fn hire_compute(
        &mut self,
        caller: Address,
        offer_id: u64,
        resources: ResourceRequest,
        duration_hours: u32,
        started_at: u64,
    ) -> u64 {
        let offer = self.offers.get(&offer_id).expect("DRC57: offer not found");
        assert!(offer.available, "DRC57: offer not available");
        assert!(resources.cpu_cores <= offer.cpu_cores, "DRC57: not enough CPU");
        assert!(resources.memory_gb <= offer.memory_gb, "DRC57: not enough memory");
        assert!(resources.storage_gb <= offer.storage_gb, "DRC57: not enough storage");
        if resources.gpu_needed {
            assert!(offer.gpu_available, "DRC57: GPU not available");
        }
        assert!(duration_hours > 0, "DRC57: duration must be positive");

        let total_cost = offer.price_per_hour * duration_hours as u64;
        let provider = offer.provider;
        let job_id = self.next_job_id;
        self.next_job_id += 1;

        self.jobs.insert(job_id, ComputeJob {
            id: job_id,
            client: caller,
            provider,
            offer_id,
            resources,
            duration_hours,
            total_cost,
            status: JobStatus::Pending,
            started_at,
            completed_at: None,
            result_hash: None,
        });
        job_id
    }

    pub fn start_job(&mut self, caller: Address, job_id: u64) {
        let job = self.jobs.get_mut(&job_id).expect("DRC57: job not found");
        assert!(caller == job.provider, "DRC57: only provider can start job");
        assert!(job.status == JobStatus::Pending, "DRC57: job not pending");
        job.status = JobStatus::Running;
    }

    pub fn complete_job(&mut self, caller: Address, job_id: u64, result_hash: [u8; 32], completed_at: u64) {
        let job = self.jobs.get_mut(&job_id).expect("DRC57: job not found");
        assert!(caller == job.provider, "DRC57: only provider can complete");
        assert!(job.status == JobStatus::Running, "DRC57: job not running");
        job.status = JobStatus::Completed;
        job.result_hash = Some(result_hash);
        job.completed_at = Some(completed_at);

        let provider = job.provider;
        let cost = job.total_cost;
        let offer_id = job.offer_id;

        // Pay provider
        let balance = self.balances.entry(provider).or_insert(0);
        *balance += cost;

        // Update offer stats
        if let Some(offer) = self.offers.get_mut(&offer_id) {
            offer.total_jobs += 1;
            offer.total_earned += cost;
        }
    }

    pub fn cancel_job(&mut self, caller: Address, job_id: u64) {
        let job = self.jobs.get_mut(&job_id).expect("DRC57: job not found");
        assert!(caller == job.client, "DRC57: only client can cancel");
        assert!(
            job.status == JobStatus::Pending || job.status == JobStatus::Running,
            "DRC57: job cannot be cancelled"
        );
        job.status = JobStatus::Cancelled;
    }

    pub fn available_compute(&self) -> Vec<&ComputeOffer> {
        self.offers.values().filter(|o| o.available).collect()
    }

    pub fn get_job(&self, job_id: u64) -> Option<&ComputeJob> {
        self.jobs.get(&job_id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ListComputeArgs { cpu_cores: u32, gpu_available: bool, memory_gb: u32, storage_gb: u32, price_per_hour: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct HireComputeArgs { offer_id: u64, resources: ResourceRequest, duration_hours: u32, started_at: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct StartJobArgs { job_id: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct CompleteJobArgs { job_id: u64, result_hash: [u8; 32], completed_at: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct CancelJobArgs { job_id: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct GetJobArgs { job_id: u64 }

pub fn dispatch(
    state: &mut Option<ComputeState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC57: already initialised");
            *state = Some(ComputeState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "list_compute" => {
            let s = state.as_mut().expect("DRC57: not initialised");
            let a: ListComputeArgs = serde_json::from_slice(args).expect("DRC57: bad args");
            let id = s.list_compute(caller, a.cpu_cores, a.gpu_available, a.memory_gb, a.storage_gb, a.price_per_hour);
            serde_json::to_vec(&id).unwrap()
        }
        "hire_compute" => {
            let s = state.as_mut().expect("DRC57: not initialised");
            let a: HireComputeArgs = serde_json::from_slice(args).expect("DRC57: bad args");
            let id = s.hire_compute(caller, a.offer_id, a.resources, a.duration_hours, a.started_at);
            serde_json::to_vec(&id).unwrap()
        }
        "start_job" => {
            let s = state.as_mut().expect("DRC57: not initialised");
            let a: StartJobArgs = serde_json::from_slice(args).expect("DRC57: bad args");
            s.start_job(caller, a.job_id);
            serde_json::to_vec("ok").unwrap()
        }
        "complete_job" => {
            let s = state.as_mut().expect("DRC57: not initialised");
            let a: CompleteJobArgs = serde_json::from_slice(args).expect("DRC57: bad args");
            s.complete_job(caller, a.job_id, a.result_hash, a.completed_at);
            serde_json::to_vec("ok").unwrap()
        }
        "cancel_job" => {
            let s = state.as_mut().expect("DRC57: not initialised");
            let a: CancelJobArgs = serde_json::from_slice(args).expect("DRC57: bad args");
            s.cancel_job(caller, a.job_id);
            serde_json::to_vec("ok").unwrap()
        }
        "available_compute" => {
            let s = state.as_ref().expect("DRC57: not initialised");
            let offers: Vec<&ComputeOffer> = s.available_compute();
            serde_json::to_vec(&offers).unwrap()
        }
        "get_job" => {
            let s = state.as_ref().expect("DRC57: not initialised");
            let a: GetJobArgs = serde_json::from_slice(args).expect("DRC57: bad args");
            serde_json::to_vec(&s.get_job(a.job_id)).unwrap()
        }
        _ => panic!("DRC57: unknown method '{method}'"),
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
    const CLIENT: Address = [2u8; 32];

    fn setup_with_offer() -> (ComputeState, u64) {
        let mut s = ComputeState::new(OWNER);
        let offer_id = s.list_compute(PROVIDER, 8, true, 32, 500, 100);
        (s, offer_id)
    }

    #[test]
    fn test_list_and_query_offers() {
        let (s, offer_id) = setup_with_offer();
        let offers = s.available_compute();
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].id, offer_id);
        assert_eq!(offers[0].cpu_cores, 8);
        assert!(offers[0].gpu_available);
    }

    #[test]
    fn test_hire_and_complete_job() {
        let (mut s, offer_id) = setup_with_offer();
        let resources = ResourceRequest { cpu_cores: 4, gpu_needed: true, memory_gb: 16, storage_gb: 100 };
        let job_id = s.hire_compute(CLIENT, offer_id, resources, 5, 1000);
        let job = s.get_job(job_id).unwrap();
        assert_eq!(job.total_cost, 500); // 100 * 5
        assert_eq!(job.status, JobStatus::Pending);

        s.start_job(PROVIDER, job_id);
        assert_eq!(s.get_job(job_id).unwrap().status, JobStatus::Running);

        s.complete_job(PROVIDER, job_id, [0xFF; 32], 2000);
        assert_eq!(s.get_job(job_id).unwrap().status, JobStatus::Completed);
        assert_eq!(*s.balances.get(&PROVIDER).unwrap(), 500);
    }

    #[test]
    fn test_cancel_job() {
        let (mut s, offer_id) = setup_with_offer();
        let resources = ResourceRequest { cpu_cores: 2, gpu_needed: false, memory_gb: 8, storage_gb: 50 };
        let job_id = s.hire_compute(CLIENT, offer_id, resources, 1, 1000);
        s.cancel_job(CLIENT, job_id);
        assert_eq!(s.get_job(job_id).unwrap().status, JobStatus::Cancelled);
    }

    #[test]
    #[should_panic(expected = "not enough CPU")]
    fn test_exceed_cpu_resources() {
        let (mut s, offer_id) = setup_with_offer();
        let resources = ResourceRequest { cpu_cores: 16, gpu_needed: false, memory_gb: 8, storage_gb: 50 };
        s.hire_compute(CLIENT, offer_id, resources, 1, 1000);
    }

    #[test]
    #[should_panic(expected = "only provider can start")]
    fn test_client_cannot_start_job() {
        let (mut s, offer_id) = setup_with_offer();
        let resources = ResourceRequest { cpu_cores: 2, gpu_needed: false, memory_gb: 8, storage_gb: 50 };
        let job_id = s.hire_compute(CLIENT, offer_id, resources, 1, 1000);
        s.start_job(CLIENT, job_id);
    }

    #[test]
    fn test_offer_stats_after_completion() {
        let (mut s, offer_id) = setup_with_offer();
        let resources = ResourceRequest { cpu_cores: 2, gpu_needed: false, memory_gb: 8, storage_gb: 50 };
        let job_id = s.hire_compute(CLIENT, offer_id, resources, 3, 1000);
        s.start_job(PROVIDER, job_id);
        s.complete_job(PROVIDER, job_id, [0xAA; 32], 2000);
        let offer = s.offers.get(&offer_id).unwrap();
        assert_eq!(offer.total_jobs, 1);
        assert_eq!(offer.total_earned, 300);
    }
}
