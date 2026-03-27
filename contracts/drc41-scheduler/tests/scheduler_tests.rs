use drc41_scheduler::{dispatch, SchedulerState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_scheduler(admin: [u8; 32]) -> Option<SchedulerState> {
    let mut state: Option<SchedulerState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

fn schedule_one_shot(state: &mut Option<SchedulerState>, creator: [u8; 32], at: u64) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "target_contract": addr(99),
        "method": "process",
        "args": "{}",
        "execute_at": at,
        "recurring_interval": null
    }))
    .unwrap();
    let result = dispatch(state, "schedule_task", &args, creator);
    serde_json::from_slice(&result).unwrap()
}

fn schedule_recurring(
    state: &mut Option<SchedulerState>,
    creator: [u8; 32],
    at: u64,
    interval: u64,
) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "target_contract": addr(99),
        "method": "heartbeat",
        "args": "{}",
        "execute_at": at,
        "recurring_interval": interval
    }))
    .unwrap();
    let result = dispatch(state, "schedule_task", &args, creator);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn schedule_and_execute_one_shot() {
    let creator = addr(1);
    let mut state = init_scheduler(creator);
    let id = schedule_one_shot(&mut state, creator, 1000);

    // Not yet due
    let exec_args = serde_json::to_vec(&serde_json::json!({ "current_time": 999u64 })).unwrap();
    let result = dispatch(&mut state, "execute_due_tasks", &exec_args, creator);
    let executed: Vec<u64> = serde_json::from_slice(&result).unwrap();
    assert!(executed.is_empty());

    // Now due
    let exec_args2 =
        serde_json::to_vec(&serde_json::json!({ "current_time": 1000u64 })).unwrap();
    let result2 = dispatch(&mut state, "execute_due_tasks", &exec_args2, creator);
    let executed2: Vec<u64> = serde_json::from_slice(&result2).unwrap();
    assert_eq!(executed2, vec![id]);

    // One-shot should be deactivated
    let s = state.as_ref().unwrap();
    assert!(!s.get_task(id).unwrap().active);
}

#[test]
fn recurring_task_reschedules() {
    let creator = addr(1);
    let mut state = init_scheduler(creator);
    let id = schedule_recurring(&mut state, creator, 1000, 500);

    // Execute at 1000
    let args1 = serde_json::to_vec(&serde_json::json!({ "current_time": 1000u64 })).unwrap();
    dispatch(&mut state, "execute_due_tasks", &args1, creator);

    let s = state.as_ref().unwrap();
    let task = s.get_task(id).unwrap();
    assert!(task.active);
    assert_eq!(task.execute_at, 1500); // rescheduled
    assert_eq!(task.last_executed, Some(1000));

    // Execute again at 1500
    let args2 = serde_json::to_vec(&serde_json::json!({ "current_time": 1500u64 })).unwrap();
    dispatch(&mut state, "execute_due_tasks", &args2, creator);

    let s = state.as_ref().unwrap();
    assert_eq!(s.get_task(id).unwrap().execute_at, 2000);
}

#[test]
fn cancel_task_deactivates() {
    let creator = addr(1);
    let mut state = init_scheduler(creator);
    let id = schedule_one_shot(&mut state, creator, 1000);

    let args = serde_json::to_vec(&serde_json::json!({ "task_id": id })).unwrap();
    dispatch(&mut state, "cancel_task", &args, creator);

    let s = state.as_ref().unwrap();
    assert!(!s.get_task(id).unwrap().active);
    assert!(s.next_execution(id).is_none());
}

#[test]
fn my_scheduled_tasks_filters() {
    let alice = addr(1);
    let bob = addr(2);
    let mut state = init_scheduler(alice);
    schedule_one_shot(&mut state, alice, 1000);
    schedule_one_shot(&mut state, alice, 2000);
    schedule_one_shot(&mut state, bob, 3000);

    let s = state.as_ref().unwrap();
    assert_eq!(s.my_scheduled_tasks(&alice).len(), 2);
    assert_eq!(s.my_scheduled_tasks(&bob).len(), 1);
}

#[test]
fn next_execution_returns_time() {
    let creator = addr(1);
    let mut state = init_scheduler(creator);
    let id = schedule_one_shot(&mut state, creator, 5000);

    let s = state.as_ref().unwrap();
    assert_eq!(s.next_execution(id), Some(5000));
}

#[test]
#[should_panic(expected = "not authorized")]
fn non_creator_cannot_cancel() {
    let creator = addr(1);
    let rando = addr(99);
    let mut state = init_scheduler(creator);
    let id = schedule_one_shot(&mut state, creator, 1000);

    let args = serde_json::to_vec(&serde_json::json!({ "task_id": id })).unwrap();
    dispatch(&mut state, "cancel_task", &args, rando);
}
