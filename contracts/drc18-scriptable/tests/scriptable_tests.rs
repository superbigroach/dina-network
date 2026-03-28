use drc18_scriptable::{dispatch, ScriptableTokenState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init() -> Option<ScriptableTokenState> {
    let mut state: Option<ScriptableTokenState> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn create_token(state: &mut Option<ScriptableTokenState>, caller: [u8; 32]) -> u64 {
    let result = dispatch(state, "create_token", b"", caller);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn create_token_returns_sequential_ids() {
    let mut state = init();
    let id1 = create_token(&mut state, addr(2));
    let id2 = create_token(&mut state, addr(2));
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn set_script_uri_stores_scripts() {
    let mut state = init();
    let id = create_token(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": id,
        "scripts": ["ipfs://script1", "ipfs://script2"]
    }))
    .unwrap();
    dispatch(&mut state, "set_script_uri", &args, addr(2));

    let query = serde_json::to_vec(&serde_json::json!({"token_id": id})).unwrap();
    let result = dispatch(&mut state, "script_uri", &query, addr(1));
    let scripts: Vec<String> = serde_json::from_slice(&result).unwrap();
    assert_eq!(scripts, vec!["ipfs://script1", "ipfs://script2"]);
}

#[test]
fn add_script_appends() {
    let mut state = init();
    let id = create_token(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "script_uri": "ipfs://a"
    }))
    .unwrap();
    dispatch(&mut state, "add_script", &args, addr(2));
    let args2 = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "script_uri": "ipfs://b"
    }))
    .unwrap();
    dispatch(&mut state, "add_script", &args2, addr(2));

    let query = serde_json::to_vec(&serde_json::json!({"token_id": id})).unwrap();
    let result = dispatch(&mut state, "script_uri", &query, addr(1));
    let scripts: Vec<String> = serde_json::from_slice(&result).unwrap();
    assert_eq!(scripts.len(), 2);
}

#[test]
fn remove_script_removes_by_index() {
    let mut state = init();
    let id = create_token(&mut state, addr(2));
    let add1 = serde_json::to_vec(&serde_json::json!({"token_id": id, "script_uri": "a"})).unwrap();
    let add2 = serde_json::to_vec(&serde_json::json!({"token_id": id, "script_uri": "b"})).unwrap();
    dispatch(&mut state, "add_script", &add1, addr(2));
    dispatch(&mut state, "add_script", &add2, addr(2));

    let rm = serde_json::to_vec(&serde_json::json!({"token_id": id, "index": 0usize})).unwrap();
    dispatch(&mut state, "remove_script", &rm, addr(2));

    let query = serde_json::to_vec(&serde_json::json!({"token_id": id})).unwrap();
    let result = dispatch(&mut state, "script_uri", &query, addr(1));
    let scripts: Vec<String> = serde_json::from_slice(&result).unwrap();
    assert_eq!(scripts, vec!["b"]);
}

#[test]
#[should_panic(expected = "only creator can set script URI")]
fn set_script_uri_by_non_creator_fails() {
    let mut state = init();
    let id = create_token(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "scripts": ["x"]
    }))
    .unwrap();
    dispatch(&mut state, "set_script_uri", &args, addr(99));
}

#[test]
#[should_panic(expected = "only creator can add script")]
fn add_script_by_non_creator_fails() {
    let mut state = init();
    let id = create_token(&mut state, addr(2));
    let args = serde_json::to_vec(&serde_json::json!({
        "token_id": id, "script_uri": "x"
    }))
    .unwrap();
    dispatch(&mut state, "add_script", &args, addr(99));
}

#[test]
#[should_panic(expected = "script index out of bounds")]
fn remove_script_out_of_bounds_fails() {
    let mut state = init();
    let id = create_token(&mut state, addr(2));
    let rm = serde_json::to_vec(&serde_json::json!({"token_id": id, "index": 5usize})).unwrap();
    dispatch(&mut state, "remove_script", &rm, addr(2));
}

#[test]
fn script_uri_returns_empty_for_new_token() {
    let mut state = init();
    let id = create_token(&mut state, addr(2));
    let query = serde_json::to_vec(&serde_json::json!({"token_id": id})).unwrap();
    let result = dispatch(&mut state, "script_uri", &query, addr(1));
    let scripts: Vec<String> = serde_json::from_slice(&result).unwrap();
    assert!(scripts.is_empty());
}
