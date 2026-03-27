use drc33_machine_learning::{dispatch, MLRegistryState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_ml(admin: [u8; 32]) -> Option<MLRegistryState> {
    let mut state: Option<MLRegistryState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

fn register_test_model(state: &mut Option<MLRegistryState>, owner: [u8; 32]) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "name": "GPT-Dina",
        "version": "1.0.0",
        "model_hash": "abc123hash",
        "accuracy_score": 9500u64,
        "training_data_hash": "data_hash_xyz",
        "framework": "PyTorch",
        "license": "MIT",
        "deployed_at": 1000u64
    }))
    .unwrap();
    let result = dispatch(state, "register_model", &args, owner);
    serde_json::from_slice(&result).unwrap()
}

fn register_tf_model(state: &mut Option<MLRegistryState>, owner: [u8; 32]) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "name": "TF-Vision",
        "version": "2.0.0",
        "model_hash": "tf_hash_456",
        "accuracy_score": 9200u64,
        "training_data_hash": "tf_data_hash",
        "framework": "TensorFlow",
        "license": "Apache-2.0",
        "deployed_at": 2000u64
    }))
    .unwrap();
    let result = dispatch(state, "register_model", &args, owner);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn register_and_get_model() {
    let owner = addr(1);
    let mut state = init_ml(owner);
    let id = register_test_model(&mut state, owner);

    let s = state.as_ref().unwrap();
    let model = s.get_model(id).unwrap();
    assert_eq!(model.name, "GPT-Dina");
    assert_eq!(model.accuracy_score, 9500);
    assert!(!model.deprecated);
}

#[test]
fn update_model_version() {
    let owner = addr(1);
    let mut state = init_ml(owner);
    let id = register_test_model(&mut state, owner);

    let args = serde_json::to_vec(&serde_json::json!({
        "model_id": id,
        "version": "2.0.0",
        "model_hash": "new_hash_456",
        "accuracy_score": 9700u64
    }))
    .unwrap();
    dispatch(&mut state, "update_model", &args, owner);

    let s = state.as_ref().unwrap();
    let model = s.get_model(id).unwrap();
    assert_eq!(model.version, "2.0.0");
    assert_eq!(model.model_hash, "new_hash_456");
    assert_eq!(model.accuracy_score, 9700);
}

#[test]
fn verify_hash_correct_and_incorrect() {
    let owner = addr(1);
    let mut state = init_ml(owner);
    let id = register_test_model(&mut state, owner);

    let s = state.as_ref().unwrap();
    assert!(s.verify_model_hash(id, "abc123hash"));
    assert!(!s.verify_model_hash(id, "wrong_hash"));
}

#[test]
fn deprecate_model_hides_from_owner_list() {
    let owner = addr(1);
    let mut state = init_ml(owner);
    let id = register_test_model(&mut state, owner);
    register_test_model(&mut state, owner);

    let args = serde_json::to_vec(&serde_json::json!({ "model_id": id })).unwrap();
    dispatch(&mut state, "deprecate_model", &args, owner);

    let s = state.as_ref().unwrap();
    assert!(s.get_model(id).unwrap().deprecated);
    assert_eq!(s.models_by_owner(&owner).len(), 1);
}

#[test]
#[should_panic(expected = "only owner can update")]
fn non_owner_cannot_update() {
    let owner = addr(1);
    let other = addr(2);
    let mut state = init_ml(owner);
    let id = register_test_model(&mut state, owner);

    let args = serde_json::to_vec(&serde_json::json!({
        "model_id": id,
        "version": "hacked",
        "model_hash": null,
        "accuracy_score": null
    }))
    .unwrap();
    dispatch(&mut state, "update_model", &args, other);
}

#[test]
fn models_by_owner_returns_only_owned() {
    let owner1 = addr(1);
    let owner2 = addr(2);
    let mut state = init_ml(owner1);
    register_test_model(&mut state, owner1);
    register_test_model(&mut state, owner2);

    let s = state.as_ref().unwrap();
    assert_eq!(s.models_by_owner(&owner1).len(), 1);
    assert_eq!(s.models_by_owner(&owner2).len(), 1);
}

#[test]
fn search_by_framework_filters_correctly() {
    let owner = addr(1);
    let mut state = init_ml(owner);
    register_test_model(&mut state, owner); // PyTorch
    register_test_model(&mut state, owner); // PyTorch
    register_tf_model(&mut state, owner); // TensorFlow

    let s = state.as_ref().unwrap();
    assert_eq!(s.search_by_framework("PyTorch").len(), 2);
    assert_eq!(s.search_by_framework("TensorFlow").len(), 1);
    assert_eq!(s.search_by_framework("JAX").len(), 0);
}

#[test]
fn search_by_framework_via_dispatch() {
    let owner = addr(1);
    let mut state = init_ml(owner);
    register_test_model(&mut state, owner); // PyTorch
    register_tf_model(&mut state, owner); // TensorFlow

    let args = serde_json::to_vec(&serde_json::json!({ "framework": "PyTorch" })).unwrap();
    let result = dispatch(&mut state, "search_by_framework", &args, owner);
    let models: Vec<serde_json::Value> = serde_json::from_slice(&result).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0]["framework"], "PyTorch");
}

#[test]
fn search_by_framework_excludes_deprecated() {
    let owner = addr(1);
    let mut state = init_ml(owner);
    let id = register_test_model(&mut state, owner); // PyTorch
    register_test_model(&mut state, owner); // PyTorch

    // Deprecate the first one
    let dep_args = serde_json::to_vec(&serde_json::json!({ "model_id": id })).unwrap();
    dispatch(&mut state, "deprecate_model", &dep_args, owner);

    let s = state.as_ref().unwrap();
    assert_eq!(s.search_by_framework("PyTorch").len(), 1);
}

#[test]
fn get_model_via_dispatch() {
    let owner = addr(1);
    let mut state = init_ml(owner);
    let id = register_test_model(&mut state, owner);

    let args = serde_json::to_vec(&serde_json::json!({ "id": id })).unwrap();
    let result = dispatch(&mut state, "get_model", &args, owner);
    let model: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(model["name"], "GPT-Dina");
    assert_eq!(model["framework"], "PyTorch");
    assert_eq!(model["license"], "MIT");
}
