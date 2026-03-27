use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-33  ML Model Registry
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelInfo {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub version: String,
    pub model_hash: String,
    pub accuracy_score: u64, // basis points (9500 = 95.00%)
    pub training_data_hash: String,
    pub framework: String,
    pub license: String,
    pub deployed_at: u64,
    pub deprecated: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MLRegistryState {
    pub admin: Address,
    pub models: BTreeMap<u64, ModelInfo>,
    pub next_id: u64,
}

impl MLRegistryState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            models: BTreeMap::new(),
            next_id: 1,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_model(
        &mut self,
        caller: Address,
        name: String,
        version: String,
        model_hash: String,
        accuracy_score: u64,
        training_data_hash: String,
        framework: String,
        license: String,
        deployed_at: u64,
    ) -> u64 {
        assert!(!name.is_empty(), "DRC33: name cannot be empty");
        assert!(!model_hash.is_empty(), "DRC33: model_hash cannot be empty");
        let id = self.next_id;
        self.next_id += 1;
        let model = ModelInfo {
            id,
            owner: caller,
            name,
            version,
            model_hash,
            accuracy_score,
            training_data_hash,
            framework,
            license,
            deployed_at,
            deprecated: false,
        };
        self.models.insert(id, model);
        id
    }

    pub fn update_model(
        &mut self,
        caller: Address,
        model_id: u64,
        version: Option<String>,
        model_hash: Option<String>,
        accuracy_score: Option<u64>,
    ) {
        let model = self.models.get_mut(&model_id).expect("DRC33: model not found");
        assert!(model.owner == caller, "DRC33: only owner can update model");
        assert!(!model.deprecated, "DRC33: model is deprecated");
        if let Some(v) = version {
            model.version = v;
        }
        if let Some(h) = model_hash {
            model.model_hash = h;
        }
        if let Some(a) = accuracy_score {
            model.accuracy_score = a;
        }
    }

    pub fn get_model(&self, id: u64) -> Option<&ModelInfo> {
        self.models.get(&id)
    }

    pub fn models_by_owner(&self, owner: &Address) -> Vec<&ModelInfo> {
        self.models
            .values()
            .filter(|m| m.owner == *owner && !m.deprecated)
            .collect()
    }

    pub fn verify_model_hash(&self, id: u64, hash: &str) -> bool {
        self.models
            .get(&id)
            .is_some_and(|m| m.model_hash == hash)
    }

    pub fn deprecate_model(&mut self, caller: Address, model_id: u64) {
        let model = self.models.get_mut(&model_id).expect("DRC33: model not found");
        assert!(
            model.owner == caller || caller == self.admin,
            "DRC33: not authorized"
        );
        model.deprecated = true;
    }

    pub fn search_by_framework(&self, framework: &str) -> Vec<&ModelInfo> {
        self.models
            .values()
            .filter(|m| !m.deprecated && m.framework == framework)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterModelArgs {
    name: String,
    version: String,
    model_hash: String,
    accuracy_score: u64,
    training_data_hash: String,
    framework: String,
    license: String,
    deployed_at: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateModelArgs {
    model_id: u64,
    version: Option<String>,
    model_hash: Option<String>,
    accuracy_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetModelArgs {
    id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModelsByOwnerArgs {
    owner: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifyHashArgs {
    id: u64,
    hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeprecateArgs {
    model_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchByFrameworkArgs {
    framework: String,
}

pub fn dispatch(
    state: &mut Option<MLRegistryState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC33: already initialised");
            *state = Some(MLRegistryState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "register_model" => {
            let s = state.as_mut().expect("DRC33: not initialised");
            let a: RegisterModelArgs =
                serde_json::from_slice(args).expect("DRC33: bad register_model args");
            let id = s.register_model(
                caller,
                a.name,
                a.version,
                a.model_hash,
                a.accuracy_score,
                a.training_data_hash,
                a.framework,
                a.license,
                a.deployed_at,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "update_model" => {
            let s = state.as_mut().expect("DRC33: not initialised");
            let a: UpdateModelArgs =
                serde_json::from_slice(args).expect("DRC33: bad update_model args");
            s.update_model(caller, a.model_id, a.version, a.model_hash, a.accuracy_score);
            serde_json::to_vec("ok").unwrap()
        }
        "get_model" => {
            let s = state.as_ref().expect("DRC33: not initialised");
            let a: GetModelArgs =
                serde_json::from_slice(args).expect("DRC33: bad get_model args");
            serde_json::to_vec(&s.get_model(a.id)).unwrap()
        }
        "models_by_owner" => {
            let s = state.as_ref().expect("DRC33: not initialised");
            let a: ModelsByOwnerArgs =
                serde_json::from_slice(args).expect("DRC33: bad models_by_owner args");
            serde_json::to_vec(&s.models_by_owner(&a.owner)).unwrap()
        }
        "verify_model_hash" => {
            let s = state.as_ref().expect("DRC33: not initialised");
            let a: VerifyHashArgs =
                serde_json::from_slice(args).expect("DRC33: bad verify_model_hash args");
            serde_json::to_vec(&s.verify_model_hash(a.id, &a.hash)).unwrap()
        }
        "deprecate_model" => {
            let s = state.as_mut().expect("DRC33: not initialised");
            let a: DeprecateArgs =
                serde_json::from_slice(args).expect("DRC33: bad deprecate_model args");
            s.deprecate_model(caller, a.model_id);
            serde_json::to_vec("ok").unwrap()
        }
        "search_by_framework" => {
            let s = state.as_ref().expect("DRC33: not initialised");
            let a: SearchByFrameworkArgs =
                serde_json::from_slice(args).expect("DRC33: bad search_by_framework args");
            let results = s.search_by_framework(&a.framework);
            serde_json::to_vec(&results).unwrap()
        }
        _ => panic!("DRC33: unknown method '{method}'"),
    }
}
