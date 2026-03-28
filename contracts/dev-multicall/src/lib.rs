use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Dev-Multicall — Batch multiple contract calls into one transaction
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Call {
    pub target: String,
    pub method: String,
    pub args: Vec<u8>,
    pub value: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MulticallResult {
    pub success: bool,
    pub return_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MulticallState {
    pub owner: String,
    pub total_calls_executed: u64,
}

impl MulticallState {
    pub fn new(owner: String) -> Self {
        Self {
            owner,
            total_calls_executed: 0,
        }
    }

    /// Execute multiple calls atomically. All must succeed or the entire
    /// batch reverts.
    pub fn multicall(&mut self, calls: &[Call]) -> Vec<MulticallResult> {
        assert!(!calls.is_empty(), "Multicall: empty call list");

        let mut results = Vec::with_capacity(calls.len());
        for call in calls {
            // In a real runtime this would invoke the target contract.
            // Here we simulate successful execution and record the call.
            let return_data =
                serde_json::to_vec(&format!("executed:{}:{}", call.target, call.method)).unwrap();
            results.push(MulticallResult {
                success: true,
                return_data,
            });
        }
        self.total_calls_executed += calls.len() as u64;
        results
    }

    /// Execute multiple calls, continuing even if individual calls fail.
    /// Each result indicates success or failure independently.
    pub fn try_multicall(&mut self, calls: &[Call]) -> Vec<MulticallResult> {
        assert!(!calls.is_empty(), "Multicall: empty call list");

        let mut results = Vec::with_capacity(calls.len());
        for call in calls {
            // Simulate: calls with empty method name "fail"
            if call.method.is_empty() {
                results.push(MulticallResult {
                    success: false,
                    return_data: b"error: empty method".to_vec(),
                });
            } else {
                let return_data =
                    serde_json::to_vec(&format!("executed:{}:{}", call.target, call.method))
                        .unwrap();
                results.push(MulticallResult {
                    success: true,
                    return_data,
                });
                self.total_calls_executed += 1;
            }
        }
        results
    }

    /// Execute multiple calls with USDC value attached per call.
    pub fn multicall_with_value(&mut self, calls: &[Call], values: &[u64]) -> Vec<MulticallResult> {
        assert!(!calls.is_empty(), "Multicall: empty call list");
        assert_eq!(
            calls.len(),
            values.len(),
            "Multicall: calls and values length mismatch"
        );

        let mut results = Vec::with_capacity(calls.len());
        for (call, &value) in calls.iter().zip(values.iter()) {
            let return_data = serde_json::to_vec(&format!(
                "executed:{}:{}:value={}",
                call.target, call.method, value
            ))
            .unwrap();
            results.push(MulticallResult {
                success: true,
                return_data,
            });
        }
        self.total_calls_executed += calls.len() as u64;
        results
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    owner: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MulticallArgs {
    calls: Vec<Call>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MulticallWithValueArgs {
    calls: Vec<Call>,
    values: Vec<u64>,
}

pub fn dispatch(
    state: &mut Option<MulticallState>,
    method: &str,
    args: &[u8],
    caller: &str,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Multicall: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("Multicall: bad init args");
            *state = Some(MulticallState::new(a.owner));
            serde_json::to_vec("ok").unwrap()
        }

        "multicall" => {
            let s = state.as_mut().expect("Multicall: not initialised");
            let a: MulticallArgs =
                serde_json::from_slice(args).expect("Multicall: bad multicall args");
            let results = s.multicall(&a.calls);
            serde_json::to_vec(&results).unwrap()
        }

        "try_multicall" => {
            let s = state.as_mut().expect("Multicall: not initialised");
            let a: MulticallArgs =
                serde_json::from_slice(args).expect("Multicall: bad try_multicall args");
            let results = s.try_multicall(&a.calls);
            serde_json::to_vec(&results).unwrap()
        }

        "multicall_with_value" => {
            let s = state.as_mut().expect("Multicall: not initialised");
            let a: MulticallWithValueArgs =
                serde_json::from_slice(args).expect("Multicall: bad multicall_with_value args");
            let results = s.multicall_with_value(&a.calls, &a.values);
            serde_json::to_vec(&results).unwrap()
        }

        "get_total_calls" => {
            let s = state.as_ref().expect("Multicall: not initialised");
            serde_json::to_vec(&s.total_calls_executed).unwrap()
        }

        _ => panic!("Multicall: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: &str = "owner_addr";
    const ALICE: &str = "alice_addr";

    fn make_call(target: &str, method: &str, value: u64) -> Call {
        Call {
            target: target.to_string(),
            method: method.to_string(),
            args: vec![],
            value,
        }
    }

    fn init() -> Option<MulticallState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            owner: OWNER.to_string(),
        })
        .unwrap();
        dispatch(&mut state, "init", &args, OWNER);
        state
    }

    #[test]
    fn test_batch_three_calls() {
        let mut state = init();
        let calls = vec![
            make_call("token", "transfer", 100),
            make_call("nft", "mint", 0),
            make_call("vault", "deposit", 500),
        ];
        let args = serde_json::to_vec(&MulticallArgs { calls }).unwrap();
        let result = dispatch(&mut state, "multicall", &args, ALICE);
        let results: Vec<MulticallResult> = serde_json::from_slice(&result).unwrap();

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.success));
        assert_eq!(state.as_ref().unwrap().total_calls_executed, 3);
    }

    #[test]
    fn test_try_multicall_handles_failures() {
        let mut state = init();
        let calls = vec![
            make_call("token", "transfer", 100),
            Call {
                target: "bad".to_string(),
                method: "".to_string(), // empty method triggers failure
                args: vec![],
                value: 0,
            },
            make_call("vault", "deposit", 500),
        ];
        let args = serde_json::to_vec(&MulticallArgs { calls }).unwrap();
        let result = dispatch(&mut state, "try_multicall", &args, ALICE);
        let results: Vec<MulticallResult> = serde_json::from_slice(&result).unwrap();

        assert_eq!(results.len(), 3);
        assert!(results[0].success);
        assert!(!results[1].success);
        assert!(results[2].success);
        // Only 2 successful calls counted
        assert_eq!(state.as_ref().unwrap().total_calls_executed, 2);
    }

    #[test]
    fn test_multicall_with_value() {
        let mut state = init();
        let calls = vec![
            make_call("token", "transfer", 0),
            make_call("vault", "deposit", 0),
        ];
        let values = vec![100, 500];
        let args = serde_json::to_vec(&MulticallWithValueArgs { calls, values }).unwrap();
        let result = dispatch(&mut state, "multicall_with_value", &args, ALICE);
        let results: Vec<MulticallResult> = serde_json::from_slice(&result).unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.success));

        // Check value was included in return data
        let data: String = serde_json::from_slice(&results[1].return_data).unwrap();
        assert!(data.contains("value=500"));
    }

    #[test]
    #[should_panic(expected = "Multicall: empty call list")]
    fn test_empty_calls_panics() {
        let mut state = init();
        let args = serde_json::to_vec(&MulticallArgs { calls: vec![] }).unwrap();
        dispatch(&mut state, "multicall", &args, ALICE);
    }

    #[test]
    #[should_panic(expected = "Multicall: calls and values length mismatch")]
    fn test_value_length_mismatch() {
        let mut state = init();
        let calls = vec![make_call("token", "transfer", 0)];
        let values = vec![100, 200]; // 2 values for 1 call
        let args = serde_json::to_vec(&MulticallWithValueArgs { calls, values }).unwrap();
        dispatch(&mut state, "multicall_with_value", &args, ALICE);
    }

    #[test]
    fn test_total_calls_counter() {
        let mut state = init();

        let calls = vec![make_call("a", "b", 0), make_call("c", "d", 0)];
        let args = serde_json::to_vec(&MulticallArgs { calls }).unwrap();
        dispatch(&mut state, "multicall", &args, ALICE);

        let result = dispatch(&mut state, "get_total_calls", b"", ALICE);
        let total: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(total, 2);

        // Another batch
        let calls = vec![make_call("e", "f", 0)];
        let args = serde_json::to_vec(&MulticallArgs { calls }).unwrap();
        dispatch(&mut state, "multicall", &args, ALICE);

        let result = dispatch(&mut state, "get_total_calls", b"", ALICE);
        let total: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(total, 3);
    }
}
