use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-70  AI Model Inference Marketplace (Enhanced)
// ---------------------------------------------------------------------------
// Enhanced marketplace for Cognitum-hosted AI models with benchmarking,
// comparison, and cost optimization.

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ModelType {
    LLM,
    Vision,
    Embedding,
    Classification,
    AudioTranscription,
    CodeGeneration,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum InferenceStatus {
    Pending,
    Completed,
    Failed,
    TimedOut,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelListing {
    pub id: u64,
    pub provider: Address,
    pub device_id: String,
    pub model_name: String,
    pub model_type: ModelType,
    pub input_format: String,
    pub output_format: String,
    pub price_per_call: u64,
    pub avg_latency_ms: u64,
    pub accuracy_benchmark: u64, // basis points
    pub hosted_on_cognitum: bool,
    pub total_calls: u64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Benchmark {
    pub model_id: u64,
    pub benchmark_name: String,
    pub score: u64,
    pub timestamp: u64,
    pub verifier: Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InferenceLog {
    pub id: u64,
    pub model_id: u64,
    pub requester: Address,
    pub input_hash: Vec<u8>,
    pub output_hash: Option<Vec<u8>>,
    pub cost: u64,
    pub latency_ms: u64,
    pub status: InferenceStatus,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelMarketplaceState {
    pub owner: Address,
    pub models: BTreeMap<u64, ModelListing>,
    pub benchmarks: BTreeMap<u64, Vec<Benchmark>>, // model_id -> benchmarks
    pub inference_logs: Vec<InferenceLog>,
    pub next_model_id: u64,
    pub next_inference_id: u64,
}

impl ModelMarketplaceState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            models: BTreeMap::new(),
            benchmarks: BTreeMap::new(),
            inference_logs: Vec::new(),
            next_model_id: 1,
            next_inference_id: 1,
        }
    }

    pub fn list_model(
        &mut self,
        caller: Address,
        device_id: String,
        model_name: String,
        model_type: ModelType,
        input_format: String,
        output_format: String,
        price_per_call: u64,
        hosted_on_cognitum: bool,
    ) -> u64 {
        assert!(!model_name.is_empty(), "DRC70: model name required");
        assert!(price_per_call > 0, "DRC70: price must be > 0");
        let id = self.next_model_id;
        self.next_model_id += 1;
        self.models.insert(
            id,
            ModelListing {
                id,
                provider: caller,
                device_id,
                model_name,
                model_type,
                input_format,
                output_format,
                price_per_call,
                avg_latency_ms: 0,
                accuracy_benchmark: 0,
                hosted_on_cognitum,
                total_calls: 0,
                active: true,
            },
        );
        id
    }

    pub fn request_inference(
        &mut self,
        caller: Address,
        model_id: u64,
        input_hash: Vec<u8>,
        payment: u64,
        timestamp: u64,
    ) -> u64 {
        let model = self
            .models
            .get_mut(&model_id)
            .expect("DRC70: model not found");
        assert!(model.active, "DRC70: model inactive");
        assert!(
            payment >= model.price_per_call,
            "DRC70: insufficient payment"
        );

        model.total_calls += 1;
        let iid = self.next_inference_id;
        self.next_inference_id += 1;

        self.inference_logs.push(InferenceLog {
            id: iid,
            model_id,
            requester: caller,
            input_hash,
            output_hash: None,
            cost: payment,
            latency_ms: 0,
            status: InferenceStatus::Pending,
            timestamp,
        });
        iid
    }

    pub fn submit_inference_result(
        &mut self,
        caller: Address,
        inference_id: u64,
        output_hash: Vec<u8>,
        latency_ms: u64,
    ) {
        let log = self
            .inference_logs
            .iter_mut()
            .find(|l| l.id == inference_id)
            .expect("DRC70: inference not found");
        let model = self
            .models
            .get(&log.model_id)
            .expect("DRC70: model not found");
        assert!(caller == model.provider, "DRC70: not the model provider");
        assert!(log.status == InferenceStatus::Pending, "DRC70: not pending");

        log.output_hash = Some(output_hash);
        log.latency_ms = latency_ms;
        log.status = InferenceStatus::Completed;

        // Update average latency using running average
        let m = self.models.get_mut(&log.model_id).unwrap();
        if m.avg_latency_ms == 0 {
            m.avg_latency_ms = latency_ms;
        } else {
            m.avg_latency_ms = (m.avg_latency_ms + latency_ms) / 2;
        }
    }

    pub fn benchmark_model(
        &mut self,
        caller: Address,
        model_id: u64,
        benchmark_name: String,
        score: u64,
        timestamp: u64,
    ) {
        assert!(
            self.models.contains_key(&model_id),
            "DRC70: model not found"
        );
        assert!(score <= 10000, "DRC70: score max 10000");

        self.benchmarks
            .entry(model_id)
            .or_default()
            .push(Benchmark {
                model_id,
                benchmark_name,
                score,
                timestamp,
                verifier: caller,
            });

        // Update model accuracy from latest benchmark average
        let benchmarks = self.benchmarks.get(&model_id).unwrap();
        let avg: u64 = benchmarks.iter().map(|b| b.score).sum::<u64>() / benchmarks.len() as u64;
        self.models.get_mut(&model_id).unwrap().accuracy_benchmark = avg;
    }

    pub fn compare_models(
        &self,
        model_ids: &[u64],
    ) -> Vec<(&ModelListing, Option<&Vec<Benchmark>>)> {
        model_ids
            .iter()
            .filter_map(|id| self.models.get(id).map(|m| (m, self.benchmarks.get(id))))
            .collect()
    }

    pub fn models_by_type(&self, model_type: &ModelType) -> Vec<&ModelListing> {
        self.models
            .values()
            .filter(|m| m.active && m.model_type == *model_type)
            .collect()
    }

    pub fn cheapest_for_task(&self, model_type: &ModelType) -> Option<&ModelListing> {
        self.models
            .values()
            .filter(|m| m.active && m.model_type == *model_type)
            .min_by_key(|m| m.price_per_call)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ListModelArgs {
    device_id: String,
    model_name: String,
    model_type: ModelType,
    input_format: String,
    output_format: String,
    price_per_call: u64,
    hosted_on_cognitum: bool,
}
#[derive(Serialize, Deserialize, Debug)]
struct RequestInferenceArgs {
    model_id: u64,
    input_hash: Vec<u8>,
    payment: u64,
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct SubmitInferenceArgs {
    inference_id: u64,
    output_hash: Vec<u8>,
    latency_ms: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct BenchmarkArgs {
    model_id: u64,
    benchmark_name: String,
    score: u64,
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct CompareArgs {
    model_ids: Vec<u64>,
}
#[derive(Serialize, Deserialize, Debug)]
struct TypeArgs {
    model_type: ModelType,
}

pub fn dispatch(
    state: &mut Option<ModelMarketplaceState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC70: already initialised");
            *state = Some(ModelMarketplaceState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "list_model" => {
            let s = state.as_mut().expect("DRC70: not initialised");
            let a: ListModelArgs = serde_json::from_slice(args).expect("DRC70: bad args");
            let id = s.list_model(
                caller,
                a.device_id,
                a.model_name,
                a.model_type,
                a.input_format,
                a.output_format,
                a.price_per_call,
                a.hosted_on_cognitum,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "request_inference" => {
            let s = state.as_mut().expect("DRC70: not initialised");
            let a: RequestInferenceArgs = serde_json::from_slice(args).expect("DRC70: bad args");
            let id = s.request_inference(caller, a.model_id, a.input_hash, a.payment, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        "submit_inference_result" => {
            let s = state.as_mut().expect("DRC70: not initialised");
            let a: SubmitInferenceArgs = serde_json::from_slice(args).expect("DRC70: bad args");
            s.submit_inference_result(caller, a.inference_id, a.output_hash, a.latency_ms);
            serde_json::to_vec("ok").unwrap()
        }
        "benchmark_model" => {
            let s = state.as_mut().expect("DRC70: not initialised");
            let a: BenchmarkArgs = serde_json::from_slice(args).expect("DRC70: bad args");
            s.benchmark_model(caller, a.model_id, a.benchmark_name, a.score, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "models_by_type" => {
            let s = state.as_ref().expect("DRC70: not initialised");
            let a: TypeArgs = serde_json::from_slice(args).expect("DRC70: bad args");
            serde_json::to_vec(&s.models_by_type(&a.model_type)).unwrap()
        }
        "cheapest_for_task" => {
            let s = state.as_ref().expect("DRC70: not initialised");
            let a: TypeArgs = serde_json::from_slice(args).expect("DRC70: bad args");
            serde_json::to_vec(&s.cheapest_for_task(&a.model_type)).unwrap()
        }
        _ => panic!("DRC70: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const PROVIDER_A: Address = [1u8; 32];
    const PROVIDER_B: Address = [2u8; 32];
    const USER: Address = [3u8; 32];
    const VERIFIER: Address = [4u8; 32];

    fn setup_with_model() -> (ModelMarketplaceState, u64) {
        let mut s = ModelMarketplaceState::new(OWNER);
        let id = s.list_model(
            PROVIDER_A,
            "seed-01".into(),
            "llama-7b".into(),
            ModelType::LLM,
            "text".into(),
            "text".into(),
            50,
            true,
        );
        (s, id)
    }

    #[test]
    fn test_list_and_find_model() {
        let (s, id) = setup_with_model();
        let model = s.models.get(&id).unwrap();
        assert_eq!(model.model_name, "llama-7b");
        assert_eq!(model.model_type, ModelType::LLM);
        assert!(model.hosted_on_cognitum);
        assert!(model.active);
    }

    #[test]
    fn test_request_and_submit_inference() {
        let (mut s, mid) = setup_with_model();
        let iid = s.request_inference(USER, mid, vec![0xAA], 50, 1000);
        assert_eq!(iid, 1);

        s.submit_inference_result(PROVIDER_A, iid, vec![0xBB], 120);
        let log = s.inference_logs.iter().find(|l| l.id == iid).unwrap();
        assert_eq!(log.status, InferenceStatus::Completed);
        assert_eq!(log.latency_ms, 120);
        assert_eq!(s.models.get(&mid).unwrap().avg_latency_ms, 120);
    }

    #[test]
    fn test_benchmark_updates_accuracy() {
        let (mut s, mid) = setup_with_model();
        s.benchmark_model(VERIFIER, mid, "MMLU".into(), 8000, 100);
        s.benchmark_model(VERIFIER, mid, "HumanEval".into(), 6000, 200);

        let model = s.models.get(&mid).unwrap();
        assert_eq!(model.accuracy_benchmark, 7000); // average
        assert_eq!(s.benchmarks.get(&mid).unwrap().len(), 2);
    }

    #[test]
    fn test_models_by_type_and_cheapest() {
        let mut s = ModelMarketplaceState::new(OWNER);
        s.list_model(
            PROVIDER_A,
            "s1".into(),
            "llama".into(),
            ModelType::LLM,
            "text".into(),
            "text".into(),
            50,
            true,
        );
        s.list_model(
            PROVIDER_B,
            "s2".into(),
            "mistral".into(),
            ModelType::LLM,
            "text".into(),
            "text".into(),
            30,
            true,
        );
        s.list_model(
            PROVIDER_A,
            "s1".into(),
            "clip".into(),
            ModelType::Vision,
            "image".into(),
            "text".into(),
            100,
            true,
        );

        assert_eq!(s.models_by_type(&ModelType::LLM).len(), 2);
        assert_eq!(s.models_by_type(&ModelType::Vision).len(), 1);

        let cheapest = s.cheapest_for_task(&ModelType::LLM).unwrap();
        assert_eq!(cheapest.model_name, "mistral");
        assert_eq!(cheapest.price_per_call, 30);
    }

    #[test]
    fn test_compare_models() {
        let (mut s, m1) = setup_with_model();
        let m2 = s.list_model(
            PROVIDER_B,
            "s2".into(),
            "gpt-neo".into(),
            ModelType::LLM,
            "text".into(),
            "text".into(),
            100,
            false,
        );
        s.benchmark_model(VERIFIER, m1, "test".into(), 8000, 100);

        let comparison = s.compare_models(&[m1, m2]);
        assert_eq!(comparison.len(), 2);
        assert!(comparison[0].1.is_some()); // m1 has benchmarks
        assert!(comparison[1].1.is_none()); // m2 has no benchmarks
    }

    #[test]
    #[should_panic(expected = "insufficient payment")]
    fn test_underpayment_rejected() {
        let (mut s, mid) = setup_with_model();
        s.request_inference(USER, mid, vec![1], 10, 1000); // price is 50
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state = None;
        dispatch(&mut state, "init", b"{}", OWNER);
        let args = serde_json::to_vec(&ListModelArgs {
            device_id: "d1".into(),
            model_name: "test".into(),
            model_type: ModelType::Embedding,
            input_format: "vec".into(),
            output_format: "vec".into(),
            price_per_call: 5,
            hosted_on_cognitum: true,
        })
        .unwrap();
        let result = dispatch(&mut state, "list_model", &args, PROVIDER_A);
        let id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(id, 1);
    }
}
