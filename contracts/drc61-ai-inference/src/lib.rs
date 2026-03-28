use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-61  AI Inference Registry
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ModelType {
    TextGeneration,
    ImageClassification,
    SpeechToText,
    Embedding,
    ObjectDetection,
    Translation,
    Custom(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RequestStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InferenceModel {
    pub id: u64,
    pub provider: Address,
    pub name: String,
    pub model_type: ModelType,
    pub price_per_call: u64,
    pub latency_ms: u32,
    pub accuracy: u32, // basis points, e.g. 9500 = 95.00%
    pub endpoint_hash: [u8; 32],
    pub active: bool,
    pub total_requests: u64,
    pub total_earned: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InferenceRequest {
    pub id: u64,
    pub requester: Address,
    pub model_id: u64,
    pub input_hash: [u8; 32],
    pub output_hash: Option<[u8; 32]>,
    pub cost: u64,
    pub status: RequestStatus,
    pub requested_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InferenceState {
    pub owner: Address,
    pub models: BTreeMap<u64, InferenceModel>,
    pub requests: BTreeMap<u64, InferenceRequest>,
    pub next_model_id: u64,
    pub next_request_id: u64,
    pub balances: BTreeMap<Address, u64>,
}

impl InferenceState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            models: BTreeMap::new(),
            requests: BTreeMap::new(),
            next_model_id: 1,
            next_request_id: 1,
            balances: BTreeMap::new(),
        }
    }

    pub fn deposit(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC61: deposit must be positive");
        let bal = self.balances.entry(caller).or_insert(0);
        *bal += amount;
    }

    pub fn register_model(
        &mut self,
        caller: Address,
        name: String,
        model_type: ModelType,
        price_per_call: u64,
        latency_ms: u32,
        accuracy: u32,
        endpoint_hash: [u8; 32],
    ) -> u64 {
        assert!(!name.is_empty(), "DRC61: model name required");
        assert!(price_per_call > 0, "DRC61: price must be positive");
        assert!(accuracy <= 10_000, "DRC61: accuracy max 10000 bps");

        let id = self.next_model_id;
        self.next_model_id += 1;
        self.models.insert(
            id,
            InferenceModel {
                id,
                provider: caller,
                name,
                model_type,
                price_per_call,
                latency_ms,
                accuracy,
                endpoint_hash,
                active: true,
                total_requests: 0,
                total_earned: 0,
            },
        );
        id
    }

    pub fn request_inference(
        &mut self,
        caller: Address,
        model_id: u64,
        input_hash: [u8; 32],
        requested_at: u64,
    ) -> u64 {
        let model = self.models.get(&model_id).expect("DRC61: model not found");
        assert!(model.active, "DRC61: model not active");
        let cost = model.price_per_call;

        let balance = self.balances.get(&caller).copied().unwrap_or(0);
        assert!(balance >= cost, "DRC61: insufficient balance");
        self.balances.insert(caller, balance - cost);

        let req_id = self.next_request_id;
        self.next_request_id += 1;
        self.requests.insert(
            req_id,
            InferenceRequest {
                id: req_id,
                requester: caller,
                model_id,
                input_hash,
                output_hash: None,
                cost,
                status: RequestStatus::Pending,
                requested_at,
                completed_at: None,
            },
        );
        req_id
    }

    pub fn submit_result(
        &mut self,
        caller: Address,
        request_id: u64,
        output_hash: [u8; 32],
        completed_at: u64,
    ) {
        let req = self
            .requests
            .get_mut(&request_id)
            .expect("DRC61: request not found");
        assert!(
            req.status == RequestStatus::Pending,
            "DRC61: request not pending"
        );
        let model = self
            .models
            .get(&req.model_id)
            .expect("DRC61: model not found");
        assert!(
            caller == model.provider,
            "DRC61: only model provider can submit result"
        );

        req.output_hash = Some(output_hash);
        req.status = RequestStatus::Completed;
        req.completed_at = Some(completed_at);

        // Pay provider
        let provider = model.provider;
        let cost = req.cost;
        let model_id = req.model_id;
        let bal = self.balances.entry(provider).or_insert(0);
        *bal += cost;

        // Update model stats
        let model = self.models.get_mut(&model_id).unwrap();
        model.total_requests += 1;
        model.total_earned += cost;
    }

    pub fn get_result(&self, request_id: u64) -> Option<&InferenceRequest> {
        self.requests.get(&request_id)
    }

    pub fn models_by_type(&self, model_type: &ModelType) -> Vec<&InferenceModel> {
        self.models
            .values()
            .filter(|m| m.active && &m.model_type == model_type)
            .collect()
    }

    pub fn provider_earnings(&self, provider: &Address) -> u64 {
        self.models
            .values()
            .filter(|m| m.provider == *provider)
            .map(|m| m.total_earned)
            .sum()
    }

    pub fn balance_of(&self, addr: &Address) -> u64 {
        self.balances.get(addr).copied().unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterModelArgs {
    name: String,
    model_type: ModelType,
    price_per_call: u64,
    latency_ms: u32,
    accuracy: u32,
    endpoint_hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct RequestInferenceArgs {
    model_id: u64,
    input_hash: [u8; 32],
    requested_at: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SubmitResultArgs {
    request_id: u64,
    output_hash: [u8; 32],
    completed_at: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RequestIdArgs {
    request_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModelTypeArgs {
    model_type: ModelType,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProviderArgs {
    provider: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddrArgs {
    addr: Address,
}

pub fn dispatch(
    state: &mut Option<InferenceState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC61: already initialised");
            *state = Some(InferenceState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "deposit" => {
            let s = state.as_mut().expect("DRC61: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC61: bad args");
            s.deposit(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "register_model" => {
            let s = state.as_mut().expect("DRC61: not initialised");
            let a: RegisterModelArgs = serde_json::from_slice(args).expect("DRC61: bad args");
            let id = s.register_model(
                caller,
                a.name,
                a.model_type,
                a.price_per_call,
                a.latency_ms,
                a.accuracy,
                a.endpoint_hash,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "request_inference" => {
            let s = state.as_mut().expect("DRC61: not initialised");
            let a: RequestInferenceArgs = serde_json::from_slice(args).expect("DRC61: bad args");
            let id = s.request_inference(caller, a.model_id, a.input_hash, a.requested_at);
            serde_json::to_vec(&id).unwrap()
        }
        "submit_result" => {
            let s = state.as_mut().expect("DRC61: not initialised");
            let a: SubmitResultArgs = serde_json::from_slice(args).expect("DRC61: bad args");
            s.submit_result(caller, a.request_id, a.output_hash, a.completed_at);
            serde_json::to_vec("ok").unwrap()
        }
        "get_result" => {
            let s = state.as_ref().expect("DRC61: not initialised");
            let a: RequestIdArgs = serde_json::from_slice(args).expect("DRC61: bad args");
            serde_json::to_vec(&s.get_result(a.request_id)).unwrap()
        }
        "models_by_type" => {
            let s = state.as_ref().expect("DRC61: not initialised");
            let a: ModelTypeArgs = serde_json::from_slice(args).expect("DRC61: bad args");
            let models: Vec<&InferenceModel> = s.models_by_type(&a.model_type);
            serde_json::to_vec(&models).unwrap()
        }
        "provider_earnings" => {
            let s = state.as_ref().expect("DRC61: not initialised");
            let a: ProviderArgs = serde_json::from_slice(args).expect("DRC61: bad args");
            serde_json::to_vec(&s.provider_earnings(&a.provider)).unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC61: not initialised");
            let a: AddrArgs = serde_json::from_slice(args).expect("DRC61: bad args");
            serde_json::to_vec(&s.balance_of(&a.addr)).unwrap()
        }
        _ => panic!("DRC61: unknown method '{method}'"),
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

    fn setup_with_model() -> (InferenceState, u64) {
        let mut s = InferenceState::new(OWNER);
        let model_id = s.register_model(
            PROVIDER,
            "GPT-Dina".into(),
            ModelType::TextGeneration,
            50,
            200,
            9500,
            [0xAB; 32],
        );
        s.deposit(CLIENT, 10_000);
        (s, model_id)
    }

    #[test]
    fn test_register_and_query_model() {
        let (s, model_id) = setup_with_model();
        let models = s.models_by_type(&ModelType::TextGeneration);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, model_id);
        assert_eq!(models[0].accuracy, 9500);
    }

    #[test]
    fn test_request_and_submit_result() {
        let (mut s, model_id) = setup_with_model();
        let req_id = s.request_inference(CLIENT, model_id, [0x11; 32], 1000);
        assert_eq!(s.balance_of(&CLIENT), 9950);

        s.submit_result(PROVIDER, req_id, [0x22; 32], 1001);
        let result = s.get_result(req_id).unwrap();
        assert_eq!(result.status, RequestStatus::Completed);
        assert_eq!(result.output_hash, Some([0x22; 32]));
        assert_eq!(s.balance_of(&PROVIDER), 50);
    }

    #[test]
    fn test_provider_earnings() {
        let (mut s, model_id) = setup_with_model();
        let r1 = s.request_inference(CLIENT, model_id, [0x11; 32], 1000);
        s.submit_result(PROVIDER, r1, [0x22; 32], 1001);
        let r2 = s.request_inference(CLIENT, model_id, [0x33; 32], 2000);
        s.submit_result(PROVIDER, r2, [0x44; 32], 2001);
        assert_eq!(s.provider_earnings(&PROVIDER), 100);
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_insufficient_balance() {
        let (mut s, model_id) = setup_with_model();
        let poor: Address = [99u8; 32];
        s.request_inference(poor, model_id, [0x11; 32], 1000);
    }

    #[test]
    #[should_panic(expected = "only model provider")]
    fn test_wrong_provider_cannot_submit() {
        let (mut s, model_id) = setup_with_model();
        let req_id = s.request_inference(CLIENT, model_id, [0x11; 32], 1000);
        s.submit_result(CLIENT, req_id, [0x22; 32], 1001);
    }

    #[test]
    fn test_models_by_type_filters() {
        let mut s = InferenceState::new(OWNER);
        s.register_model(
            PROVIDER,
            "Text".into(),
            ModelType::TextGeneration,
            10,
            100,
            9000,
            [0; 32],
        );
        s.register_model(
            PROVIDER,
            "Image".into(),
            ModelType::ImageClassification,
            20,
            50,
            8500,
            [0; 32],
        );
        s.register_model(
            PROVIDER,
            "Text2".into(),
            ModelType::TextGeneration,
            15,
            80,
            9200,
            [0; 32],
        );
        assert_eq!(s.models_by_type(&ModelType::TextGeneration).len(), 2);
        assert_eq!(s.models_by_type(&ModelType::ImageClassification).len(), 1);
        assert_eq!(s.models_by_type(&ModelType::SpeechToText).len(), 0);
    }
}
