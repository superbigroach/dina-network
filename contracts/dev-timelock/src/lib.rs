use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Dev-Timelock — Governance timelock for queuing and executing operations
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueuedOperation {
    pub target: String,
    pub method: String,
    pub args: Vec<u8>,
    pub value: u64,
    pub eta: u64, // earliest execution time
    pub queued_at: u64,
    pub executed: bool,
    pub cancelled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimelockState {
    pub admin: String,
    pub min_delay: u64,
    pub queued_ops: HashMap<String, QueuedOperation>,
    pub next_op_id: u64,
}

impl TimelockState {
    pub fn new(admin: String, min_delay: u64) -> Self {
        Self {
            admin,
            min_delay,
            queued_ops: HashMap::new(),
            next_op_id: 1,
        }
    }

    /// Queue an operation with at least `min_delay` seconds before execution.
    /// Returns the operation ID.
    pub fn queue_operation(
        &mut self,
        caller: &str,
        target: String,
        method: String,
        args: Vec<u8>,
        value: u64,
        delay: u64,
        current_time: u64,
    ) -> String {
        assert!(caller == self.admin, "Timelock: only admin can queue");
        assert!(
            delay >= self.min_delay,
            "Timelock: delay ({}) below minimum ({})",
            delay,
            self.min_delay
        );

        let op_id = format!("op_{}", self.next_op_id);
        self.next_op_id += 1;

        let op = QueuedOperation {
            target,
            method,
            args,
            value,
            eta: current_time + delay,
            queued_at: current_time,
            executed: false,
            cancelled: false,
        };
        self.queued_ops.insert(op_id.clone(), op);
        op_id
    }

    /// Execute a queued operation after its eta has passed.
    pub fn execute_operation(
        &mut self,
        caller: &str,
        op_id: &str,
        current_time: u64,
    ) -> &QueuedOperation {
        assert!(caller == self.admin, "Timelock: only admin can execute");
        let op = self
            .queued_ops
            .get_mut(op_id)
            .expect("Timelock: operation not found");
        assert!(!op.executed, "Timelock: already executed");
        assert!(!op.cancelled, "Timelock: operation was cancelled");
        assert!(
            current_time >= op.eta,
            "Timelock: not ready (now={}, eta={})",
            current_time,
            op.eta
        );

        op.executed = true;
        self.queued_ops.get(op_id).unwrap()
    }

    /// Cancel a pending operation.
    pub fn cancel_operation(&mut self, caller: &str, op_id: &str) {
        assert!(caller == self.admin, "Timelock: only admin can cancel");
        let op = self
            .queued_ops
            .get_mut(op_id)
            .expect("Timelock: operation not found");
        assert!(!op.executed, "Timelock: cannot cancel executed operation");
        assert!(!op.cancelled, "Timelock: already cancelled");
        op.cancelled = true;
    }

    /// Change the minimum delay. This change is itself subject to the
    /// current min_delay via the normal queue mechanism externally.
    pub fn set_min_delay(&mut self, caller: &str, new_delay: u64) {
        assert!(
            caller == self.admin,
            "Timelock: only admin can change min_delay"
        );
        assert!(new_delay > 0, "Timelock: min_delay must be > 0");
        self.min_delay = new_delay;
    }

    pub fn get_operation(&self, op_id: &str) -> Option<&QueuedOperation> {
        self.queued_ops.get(op_id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    admin: String,
    min_delay: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct QueueArgs {
    target: String,
    method: String,
    args: Vec<u8>,
    value: u64,
    delay: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteArgs {
    op_id: String,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CancelArgs {
    op_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetMinDelayArgs {
    new_delay: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetOperationArgs {
    op_id: String,
}

pub fn dispatch(
    state: &mut Option<TimelockState>,
    method: &str,
    args: &[u8],
    caller: &str,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Timelock: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("Timelock: bad init args");
            *state = Some(TimelockState::new(a.admin, a.min_delay));
            serde_json::to_vec("ok").unwrap()
        }

        "queue_operation" => {
            let s = state.as_mut().expect("Timelock: not initialised");
            let a: QueueArgs = serde_json::from_slice(args).expect("Timelock: bad queue args");
            let op_id = s.queue_operation(
                caller,
                a.target,
                a.method,
                a.args,
                a.value,
                a.delay,
                a.current_time,
            );
            serde_json::to_vec(&op_id).unwrap()
        }

        "execute_operation" => {
            let s = state.as_mut().expect("Timelock: not initialised");
            let a: ExecuteArgs = serde_json::from_slice(args).expect("Timelock: bad execute args");
            let op = s.execute_operation(caller, &a.op_id, a.current_time);
            serde_json::to_vec(op).unwrap()
        }

        "cancel_operation" => {
            let s = state.as_mut().expect("Timelock: not initialised");
            let a: CancelArgs = serde_json::from_slice(args).expect("Timelock: bad cancel args");
            s.cancel_operation(caller, &a.op_id);
            serde_json::to_vec("ok").unwrap()
        }

        "set_min_delay" => {
            let s = state.as_mut().expect("Timelock: not initialised");
            let a: SetMinDelayArgs =
                serde_json::from_slice(args).expect("Timelock: bad set_min_delay args");
            s.set_min_delay(caller, a.new_delay);
            serde_json::to_vec("ok").unwrap()
        }

        "get_operation" => {
            let s = state.as_ref().expect("Timelock: not initialised");
            let a: GetOperationArgs =
                serde_json::from_slice(args).expect("Timelock: bad get_operation args");
            serde_json::to_vec(&s.get_operation(&a.op_id)).unwrap()
        }

        _ => panic!("Timelock: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: &str = "admin_addr";
    const ALICE: &str = "alice_addr";
    const MIN_DELAY: u64 = 3600;

    fn init() -> Option<TimelockState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            admin: ADMIN.to_string(),
            min_delay: MIN_DELAY,
        })
        .unwrap();
        dispatch(&mut state, "init", &args, ADMIN);
        state
    }

    fn queue(state: &mut Option<TimelockState>, delay: u64, current_time: u64) -> String {
        let args = serde_json::to_vec(&QueueArgs {
            target: "token_contract".to_string(),
            method: "transfer".to_string(),
            args: vec![1, 2, 3],
            value: 1000,
            delay,
            current_time,
        })
        .unwrap();
        let result = dispatch(state, "queue_operation", &args, ADMIN);
        serde_json::from_slice(&result).unwrap()
    }

    #[test]
    fn test_queue_and_execute() {
        let mut state = init();
        let op_id = queue(&mut state, MIN_DELAY, 1000);
        assert_eq!(op_id, "op_1");

        // Execute after eta
        let args = serde_json::to_vec(&ExecuteArgs {
            op_id: op_id.clone(),
            current_time: 1000 + MIN_DELAY,
        })
        .unwrap();
        dispatch(&mut state, "execute_operation", &args, ADMIN);

        let s = state.as_ref().unwrap();
        let op = s.queued_ops.get(&op_id).unwrap();
        assert!(op.executed);
    }

    #[test]
    fn test_cancel_operation() {
        let mut state = init();
        let op_id = queue(&mut state, MIN_DELAY, 1000);

        let args = serde_json::to_vec(&CancelArgs {
            op_id: op_id.clone(),
        })
        .unwrap();
        dispatch(&mut state, "cancel_operation", &args, ADMIN);

        let s = state.as_ref().unwrap();
        assert!(s.queued_ops.get(&op_id).unwrap().cancelled);
    }

    #[test]
    #[should_panic(expected = "Timelock: not ready")]
    fn test_execute_before_eta() {
        let mut state = init();
        let op_id = queue(&mut state, MIN_DELAY, 1000);

        let args = serde_json::to_vec(&ExecuteArgs {
            op_id,
            current_time: 1000 + MIN_DELAY - 1,
        })
        .unwrap();
        dispatch(&mut state, "execute_operation", &args, ADMIN);
    }

    #[test]
    #[should_panic(expected = "Timelock: delay (100) below minimum (3600)")]
    fn test_delay_below_minimum() {
        let mut state = init();
        queue(&mut state, 100, 1000);
    }

    #[test]
    #[should_panic(expected = "Timelock: only admin can queue")]
    fn test_non_admin_cannot_queue() {
        let mut state = init();
        let args = serde_json::to_vec(&QueueArgs {
            target: "x".to_string(),
            method: "y".to_string(),
            args: vec![],
            value: 0,
            delay: MIN_DELAY,
            current_time: 1000,
        })
        .unwrap();
        dispatch(&mut state, "queue_operation", &args, ALICE);
    }

    #[test]
    fn test_set_min_delay() {
        let mut state = init();
        let args = serde_json::to_vec(&SetMinDelayArgs { new_delay: 7200 }).unwrap();
        dispatch(&mut state, "set_min_delay", &args, ADMIN);
        assert_eq!(state.as_ref().unwrap().min_delay, 7200);
    }

    #[test]
    fn test_get_operation() {
        let mut state = init();
        let op_id = queue(&mut state, MIN_DELAY, 1000);

        let args = serde_json::to_vec(&GetOperationArgs { op_id }).unwrap();
        let result = dispatch(&mut state, "get_operation", &args, ADMIN);
        let op: Option<QueuedOperation> = serde_json::from_slice(&result).unwrap();
        assert!(op.is_some());
        assert_eq!(op.unwrap().value, 1000);
    }

    #[test]
    #[should_panic(expected = "Timelock: already executed")]
    fn test_cannot_execute_twice() {
        let mut state = init();
        let op_id = queue(&mut state, MIN_DELAY, 1000);

        let args = serde_json::to_vec(&ExecuteArgs {
            op_id,
            current_time: 1000 + MIN_DELAY,
        })
        .unwrap();
        dispatch(&mut state, "execute_operation", &args, ADMIN);
        dispatch(&mut state, "execute_operation", &args, ADMIN);
    }
}
