use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-20  Timelock Controller
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimelockOp {
    pub id: u64,
    pub target: Address,
    pub data: Vec<u8>,
    pub value: u64,
    pub scheduled_at: u64,
    pub execute_after: u64,
    pub executed: bool,
    pub cancelled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimelockState {
    pub admin: Address,
    pub min_delay: u64,
    pub pending_ops: BTreeMap<u64, TimelockOp>,
    pub next_id: u64,
}

impl TimelockState {
    pub fn new(admin: Address, min_delay: u64) -> Self {
        Self {
            admin,
            min_delay,
            pending_ops: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Schedule a new timelock operation. Returns the operation id.
    pub fn schedule(
        &mut self,
        caller: Address,
        target: Address,
        data: Vec<u8>,
        value: u64,
        delay: u64,
        current_time: u64,
    ) -> u64 {
        assert!(caller == self.admin, "DRC20: only admin can schedule");
        assert!(
            delay >= self.min_delay,
            "DRC20: delay ({delay}) is below minimum ({})",
            self.min_delay
        );

        let id = self.next_id;
        self.next_id += 1;

        let op = TimelockOp {
            id,
            target,
            data,
            value,
            scheduled_at: current_time,
            execute_after: current_time + delay,
            executed: false,
            cancelled: false,
        };
        self.pending_ops.insert(id, op);
        id
    }

    /// Execute a ready operation.
    pub fn execute(&mut self, caller: Address, id: u64, current_time: u64) -> &TimelockOp {
        assert!(caller == self.admin, "DRC20: only admin can execute");
        let op = self
            .pending_ops
            .get_mut(&id)
            .expect("DRC20: operation not found");
        assert!(!op.executed, "DRC20: operation already executed");
        assert!(!op.cancelled, "DRC20: operation was cancelled");
        assert!(
            current_time >= op.execute_after,
            "DRC20: timelock not yet expired (now={current_time}, ready_at={})",
            op.execute_after
        );

        op.executed = true;
        self.pending_ops.get(&id).unwrap()
    }

    /// Cancel a pending operation.
    pub fn cancel(&mut self, caller: Address, id: u64) {
        assert!(caller == self.admin, "DRC20: only admin can cancel");
        let op = self
            .pending_ops
            .get_mut(&id)
            .expect("DRC20: operation not found");
        assert!(!op.executed, "DRC20: cannot cancel executed operation");
        assert!(!op.cancelled, "DRC20: operation already cancelled");
        op.cancelled = true;
    }

    /// Check whether an operation is ready to execute.
    pub fn is_ready(&self, id: u64, current_time: u64) -> bool {
        match self.pending_ops.get(&id) {
            Some(op) => !op.executed && !op.cancelled && current_time >= op.execute_after,
            None => false,
        }
    }

    pub fn get_operation(&self, id: u64) -> Option<&TimelockOp> {
        self.pending_ops.get(&id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    min_delay: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ScheduleArgs {
    target: Address,
    data: Vec<u8>,
    value: u64,
    delay: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteArgs {
    id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CancelArgs {
    id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsReadyArgs {
    id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetOperationArgs {
    id: u64,
}

pub fn dispatch(
    state: &mut Option<TimelockState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC20: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC20: bad init args");
            *state = Some(TimelockState::new(caller, a.min_delay));
            serde_json::to_vec("ok").unwrap()
        }

        "schedule" => {
            let s = state.as_mut().expect("DRC20: not initialised");
            let a: ScheduleArgs =
                serde_json::from_slice(args).expect("DRC20: bad schedule args");
            let id = s.schedule(caller, a.target, a.data, a.value, a.delay, a.current_time);
            serde_json::to_vec(&id).unwrap()
        }

        "execute" => {
            let s = state.as_mut().expect("DRC20: not initialised");
            let a: ExecuteArgs =
                serde_json::from_slice(args).expect("DRC20: bad execute args");
            let op = s.execute(caller, a.id, a.current_time);
            serde_json::to_vec(op).unwrap()
        }

        "cancel" => {
            let s = state.as_mut().expect("DRC20: not initialised");
            let a: CancelArgs = serde_json::from_slice(args).expect("DRC20: bad cancel args");
            s.cancel(caller, a.id);
            serde_json::to_vec("ok").unwrap()
        }

        "is_ready" => {
            let s = state.as_ref().expect("DRC20: not initialised");
            let a: IsReadyArgs =
                serde_json::from_slice(args).expect("DRC20: bad is_ready args");
            serde_json::to_vec(&s.is_ready(a.id, a.current_time)).unwrap()
        }

        "get_operation" => {
            let s = state.as_ref().expect("DRC20: not initialised");
            let a: GetOperationArgs =
                serde_json::from_slice(args).expect("DRC20: bad get_operation args");
            serde_json::to_vec(&s.get_operation(a.id)).unwrap()
        }

        _ => panic!("DRC20: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: Address = [1u8; 32];
    const ALICE: Address = [2u8; 32];
    const TARGET: Address = [10u8; 32];

    fn init_state(min_delay: u64) -> Option<TimelockState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs { min_delay }).unwrap();
        dispatch(&mut state, "init", &args, ADMIN);
        state
    }

    fn schedule_op(state: &mut Option<TimelockState>, delay: u64, current_time: u64) -> u64 {
        let args = serde_json::to_vec(&ScheduleArgs {
            target: TARGET,
            data: vec![1, 2, 3],
            value: 1000,
            delay,
            current_time,
        })
        .unwrap();
        let result = dispatch(state, "schedule", &args, ADMIN);
        serde_json::from_slice(&result).unwrap()
    }

    #[test]
    fn test_schedule_and_execute() {
        let mut state = init_state(100);
        let id = schedule_op(&mut state, 100, 1000);
        assert_eq!(id, 1);

        // Not ready yet at t=1099
        let ready_args = serde_json::to_vec(&IsReadyArgs {
            id,
            current_time: 1099,
        })
        .unwrap();
        let result = dispatch(&mut state, "is_ready", &ready_args, ADMIN);
        let ready: bool = serde_json::from_slice(&result).unwrap();
        assert!(!ready);

        // Ready at t=1100
        let ready_args = serde_json::to_vec(&IsReadyArgs {
            id,
            current_time: 1100,
        })
        .unwrap();
        let result = dispatch(&mut state, "is_ready", &ready_args, ADMIN);
        let ready: bool = serde_json::from_slice(&result).unwrap();
        assert!(ready);

        // Execute
        let exec_args = serde_json::to_vec(&ExecuteArgs {
            id,
            current_time: 1100,
        })
        .unwrap();
        dispatch(&mut state, "execute", &exec_args, ADMIN);

        let s = state.as_ref().unwrap();
        assert!(s.pending_ops.get(&id).unwrap().executed);
    }

    #[test]
    fn test_cancel_operation() {
        let mut state = init_state(100);
        let id = schedule_op(&mut state, 100, 1000);

        let cancel_args = serde_json::to_vec(&CancelArgs { id }).unwrap();
        dispatch(&mut state, "cancel", &cancel_args, ADMIN);

        let s = state.as_ref().unwrap();
        assert!(s.pending_ops.get(&id).unwrap().cancelled);

        // No longer ready
        let ready_args = serde_json::to_vec(&IsReadyArgs {
            id,
            current_time: 2000,
        })
        .unwrap();
        let result = dispatch(&mut state, "is_ready", &ready_args, ADMIN);
        let ready: bool = serde_json::from_slice(&result).unwrap();
        assert!(!ready);
    }

    #[test]
    #[should_panic(expected = "DRC20: timelock not yet expired")]
    fn test_execute_too_early_fails() {
        let mut state = init_state(100);
        let id = schedule_op(&mut state, 100, 1000);

        let exec_args = serde_json::to_vec(&ExecuteArgs {
            id,
            current_time: 1050,
        })
        .unwrap();
        dispatch(&mut state, "execute", &exec_args, ADMIN);
    }

    #[test]
    #[should_panic(expected = "DRC20: delay (50) is below minimum (100)")]
    fn test_delay_below_minimum_fails() {
        let mut state = init_state(100);
        schedule_op(&mut state, 50, 1000);
    }

    #[test]
    #[should_panic(expected = "DRC20: only admin can schedule")]
    fn test_non_admin_cannot_schedule() {
        let mut state = init_state(100);
        let args = serde_json::to_vec(&ScheduleArgs {
            target: TARGET,
            data: vec![],
            value: 0,
            delay: 100,
            current_time: 1000,
        })
        .unwrap();
        dispatch(&mut state, "schedule", &args, ALICE);
    }

    #[test]
    fn test_multiple_operations_independent() {
        let mut state = init_state(10);
        let id1 = schedule_op(&mut state, 10, 100);
        let id2 = schedule_op(&mut state, 20, 100);
        assert_ne!(id1, id2);

        // Execute id1, cancel id2
        let exec_args = serde_json::to_vec(&ExecuteArgs {
            id: id1,
            current_time: 110,
        })
        .unwrap();
        dispatch(&mut state, "execute", &exec_args, ADMIN);

        let cancel_args = serde_json::to_vec(&CancelArgs { id: id2 }).unwrap();
        dispatch(&mut state, "cancel", &cancel_args, ADMIN);

        let s = state.as_ref().unwrap();
        assert!(s.pending_ops.get(&id1).unwrap().executed);
        assert!(s.pending_ops.get(&id2).unwrap().cancelled);
    }

    #[test]
    #[should_panic(expected = "DRC20: operation already executed")]
    fn test_cannot_execute_twice() {
        let mut state = init_state(10);
        let id = schedule_op(&mut state, 10, 100);

        let exec_args = serde_json::to_vec(&ExecuteArgs {
            id,
            current_time: 200,
        })
        .unwrap();
        dispatch(&mut state, "execute", &exec_args, ADMIN);
        dispatch(&mut state, "execute", &exec_args, ADMIN);
    }
}
