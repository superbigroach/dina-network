use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Your contract state -- all data stored on-chain
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MyContract {
    owner: String,
    counter: u64,
    greetings: HashMap<String, String>,
}

impl MyContract {
    // Initialize the contract
    pub fn new(owner: String) -> Self {
        Self {
            owner,
            counter: 0,
            greetings: HashMap::new(),
        }
    }

    // Increment the counter (anyone can call)
    pub fn increment(&mut self) -> u64 {
        self.counter += 1;
        self.counter
    }

    // Set a greeting for your address
    pub fn set_greeting(&mut self, caller: String, message: String) {
        self.greetings.insert(caller, message);
    }

    // Read the counter (view function, no state change)
    pub fn get_counter(&self) -> u64 {
        self.counter
    }

    // Read a greeting
    pub fn get_greeting(&self, address: &str) -> Option<&String> {
        self.greetings.get(address)
    }

    // Only owner can reset
    pub fn reset(&mut self, caller: &str) {
        assert!(caller == self.owner, "Only owner can reset");
        self.counter = 0;
    }
}

// Dispatch function -- routes incoming calls to the right method
// This is the entry point that the Dina runtime calls
pub fn dispatch(
    state: &mut Option<MyContract>,
    method: &str,
    args: &[u8],
    caller: String,
) -> Vec<u8> {
    match method {
        "new" => {
            *state = Some(MyContract::new(caller));
            serde_json::to_vec("initialized").unwrap()
        }
        "increment" => {
            let s = state.as_mut().expect("Not initialized");
            let val = s.increment();
            serde_json::to_vec(&val).unwrap()
        }
        "set_greeting" => {
            let s = state.as_mut().expect("Not initialized");
            let msg: String = serde_json::from_slice(args).expect("Bad args");
            s.set_greeting(caller, msg);
            serde_json::to_vec("ok").unwrap()
        }
        "get_counter" => {
            let s = state.as_ref().expect("Not initialized");
            serde_json::to_vec(&s.get_counter()).unwrap()
        }
        "get_greeting" => {
            let s = state.as_ref().expect("Not initialized");
            let addr: String = serde_json::from_slice(args).expect("Bad args");
            let greeting = s.get_greeting(&addr).cloned().unwrap_or_default();
            serde_json::to_vec(&greeting).unwrap()
        }
        "reset" => {
            let s = state.as_mut().expect("Not initialized");
            s.reset(&caller);
            serde_json::to_vec("reset").unwrap()
        }
        _ => panic!("Unknown method: {method}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> String {
        format!("0x{:064x}", n)
    }

    #[test]
    fn test_full_lifecycle() {
        let mut state = None;

        // Initialize
        dispatch(&mut state, "new", b"", addr(1));
        assert!(state.is_some());

        // Increment
        let result = dispatch(&mut state, "increment", b"", addr(1));
        let val: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(val, 1);

        // Set greeting
        let msg = serde_json::to_vec("Hello Dina!").unwrap();
        dispatch(&mut state, "set_greeting", &msg, addr(1));

        // Read greeting
        let addr_arg = serde_json::to_vec(&addr(1)).unwrap();
        let result = dispatch(&mut state, "get_greeting", &addr_arg, addr(2));
        let greeting: String = serde_json::from_slice(&result).unwrap();
        assert_eq!(greeting, "Hello Dina!");

        // Reset (only owner)
        dispatch(&mut state, "reset", b"", addr(1));
        let result = dispatch(&mut state, "get_counter", b"", addr(1));
        let val: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(val, 0);
    }

    #[test]
    #[should_panic(expected = "Only owner")]
    fn test_only_owner_can_reset() {
        let mut state = None;
        dispatch(&mut state, "new", b"", addr(1));
        dispatch(&mut state, "reset", b"", addr(2)); // not owner
    }
}
