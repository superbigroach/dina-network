use drc32_task_queue::{dispatch, TaskQueueState, TaskStatus};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_queue(admin: [u8; 32]) -> Option<TaskQueueState> {
    let mut state: Option<TaskQueueState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

fn post_test_task(state: &mut Option<TaskQueueState>, poster: [u8; 32]) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "description": "Analyze dataset",
        "reward": 500u64,
        "deadline": 9999u64,
        "required_capabilities": ["nlp"]
    }))
    .unwrap();
    let result = dispatch(state, "post_task", &args, poster);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn post_and_retrieve_task() {
    let admin = addr(1);
    let mut state = init_queue(admin);
    let id = post_test_task(&mut state, admin);

    let s = state.as_ref().unwrap();
    let task = s.get_task(id).unwrap();
    assert_eq!(task.description, "Analyze dataset");
    assert_eq!(task.reward, 500);
    assert_eq!(task.status, TaskStatus::Open);
}

#[test]
fn claim_task_sets_assigned() {
    let poster = addr(1);
    let worker = addr(2);
    let mut state = init_queue(poster);
    let id = post_test_task(&mut state, poster);

    let args = serde_json::to_vec(&serde_json::json!({ "task_id": id })).unwrap();
    dispatch(&mut state, "claim_task", &args, worker);

    let s = state.as_ref().unwrap();
    let task = s.get_task(id).unwrap();
    assert_eq!(task.status, TaskStatus::Claimed);
    assert_eq!(task.assigned_to, Some(worker));
}

#[test]
#[should_panic(expected = "cannot claim own task")]
fn cannot_claim_own_task() {
    let poster = addr(1);
    let mut state = init_queue(poster);
    let id = post_test_task(&mut state, poster);

    let args = serde_json::to_vec(&serde_json::json!({ "task_id": id })).unwrap();
    dispatch(&mut state, "claim_task", &args, poster);
}

#[test]
fn full_lifecycle_post_claim_complete_verify() {
    let poster = addr(1);
    let worker = addr(2);
    let mut state = init_queue(poster);
    let id = post_test_task(&mut state, poster);

    // Claim
    let claim_args = serde_json::to_vec(&serde_json::json!({ "task_id": id })).unwrap();
    dispatch(&mut state, "claim_task", &claim_args, worker);

    // Complete
    let complete_args = serde_json::to_vec(&serde_json::json!({
        "task_id": id,
        "proof": "result_hash_abc123"
    }))
    .unwrap();
    dispatch(&mut state, "complete_task", &complete_args, worker);

    // Verify
    let verify_args = serde_json::to_vec(&serde_json::json!({
        "task_id": id,
        "approved": true
    }))
    .unwrap();
    dispatch(&mut state, "verify_completion", &verify_args, poster);

    let s = state.as_ref().unwrap();
    let task = s.get_task(id).unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    // Escrow should be released
    assert_eq!(*s.escrow.get(&poster).unwrap(), 0);
}

#[test]
fn cancel_open_task() {
    let poster = addr(1);
    let mut state = init_queue(poster);
    let id = post_test_task(&mut state, poster);

    let args = serde_json::to_vec(&serde_json::json!({ "task_id": id })).unwrap();
    dispatch(&mut state, "cancel_task", &args, poster);

    let s = state.as_ref().unwrap();
    assert_eq!(s.get_task(id).unwrap().status, TaskStatus::Expired);
    assert_eq!(s.available_tasks().len(), 0);
}

#[test]
fn available_tasks_filters_correctly() {
    let poster = addr(1);
    let worker = addr(2);
    let mut state = init_queue(poster);
    post_test_task(&mut state, poster);
    let id2 = post_test_task(&mut state, poster);

    // Claim one task
    let args = serde_json::to_vec(&serde_json::json!({ "task_id": id2 })).unwrap();
    dispatch(&mut state, "claim_task", &args, worker);

    let s = state.as_ref().unwrap();
    assert_eq!(s.available_tasks().len(), 1);
}

#[test]
fn my_tasks_returns_poster_and_worker_tasks() {
    let poster = addr(1);
    let worker = addr(2);
    let mut state = init_queue(poster);
    let id1 = post_test_task(&mut state, poster);
    post_test_task(&mut state, poster); // id2 stays open

    // Worker claims task 1
    let args = serde_json::to_vec(&serde_json::json!({ "task_id": id1 })).unwrap();
    dispatch(&mut state, "claim_task", &args, worker);

    // my_tasks via dispatch for poster (should see both tasks they posted)
    let poster_args = serde_json::to_vec(&serde_json::json!({ "address": poster })).unwrap();
    let poster_result = dispatch(&mut state, "my_tasks", &poster_args, poster);
    let poster_tasks: Vec<serde_json::Value> = serde_json::from_slice(&poster_result).unwrap();
    assert_eq!(poster_tasks.len(), 2);

    // my_tasks via dispatch for worker (should see 1 task they claimed)
    let worker_args = serde_json::to_vec(&serde_json::json!({ "address": worker })).unwrap();
    let worker_result = dispatch(&mut state, "my_tasks", &worker_args, worker);
    let worker_tasks: Vec<serde_json::Value> = serde_json::from_slice(&worker_result).unwrap();
    assert_eq!(worker_tasks.len(), 1);
}

#[test]
fn verify_rejected_sets_disputed() {
    let poster = addr(1);
    let worker = addr(2);
    let mut state = init_queue(poster);
    let id = post_test_task(&mut state, poster);

    // Claim
    let claim_args = serde_json::to_vec(&serde_json::json!({ "task_id": id })).unwrap();
    dispatch(&mut state, "claim_task", &claim_args, worker);

    // Complete with proof
    let complete_args = serde_json::to_vec(&serde_json::json!({
        "task_id": id,
        "proof": "bad_proof"
    }))
    .unwrap();
    dispatch(&mut state, "complete_task", &complete_args, worker);

    // Reject
    let verify_args = serde_json::to_vec(&serde_json::json!({
        "task_id": id,
        "approved": false
    }))
    .unwrap();
    dispatch(&mut state, "verify_completion", &verify_args, poster);

    let s = state.as_ref().unwrap();
    assert_eq!(s.get_task(id).unwrap().status, TaskStatus::Disputed);
    // Escrow NOT released on dispute
    assert_eq!(*s.escrow.get(&poster).unwrap(), 500);
}

#[test]
#[should_panic(expected = "only poster can cancel")]
fn non_poster_cannot_cancel() {
    let poster = addr(1);
    let other = addr(3);
    let mut state = init_queue(poster);
    let id = post_test_task(&mut state, poster);

    let args = serde_json::to_vec(&serde_json::json!({ "task_id": id })).unwrap();
    dispatch(&mut state, "cancel_task", &args, other);
}

#[test]
fn get_task_via_dispatch() {
    let poster = addr(1);
    let mut state = init_queue(poster);
    let id = post_test_task(&mut state, poster);

    let args = serde_json::to_vec(&serde_json::json!({ "task_id": id })).unwrap();
    let result = dispatch(&mut state, "get_task", &args, poster);
    let task: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(task["description"], "Analyze dataset");
    assert_eq!(task["reward"], 500);
}
