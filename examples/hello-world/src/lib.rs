// =============================================================================
// Hello World — Simplest Possible Dina Smart Contract
// =============================================================================
//
// This example teaches the basics of Dina smart contract development:
//   1. How to define contract STATE (the data your contract stores on-chain)
//   2. How to write METHODS that read and modify state
//   3. How to write a DISPATCH function that routes incoming calls
//
// Every Dina contract is compiled to WebAssembly (WASM) and executed by the
// Dina VM. The VM calls your `dispatch` function with:
//   - The current state (None if the contract is brand new)
//   - The method name the caller wants to invoke
//   - The arguments as a JSON byte slice
//   - The caller's 32-byte address (who signed the transaction)
//
// Your dispatch function must return a JSON-encoded response.
// =============================================================================

// We use `serde` for JSON serialization/deserialization of state and arguments.
// Every Dina contract needs these two imports.
use serde::{Deserialize, Serialize};

// =============================================================================
// Step 1: Define your contract's STATE
// =============================================================================
//
// The state struct holds ALL data your contract persists on-chain.
// It must derive Serialize + Deserialize so the VM can save/load it.
// Clone and Debug are optional but useful for testing.

/// The on-chain state for our Hello World contract.
///
/// This contract stores a single greeting string and tracks who deployed it
/// (the owner). Only the owner can change the greeting.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HelloWorldState {
    /// The current greeting message (e.g. "Hello, Dina Network!")
    pub greeting: String,

    /// The 32-byte address of the contract deployer.
    /// Dina uses ed25519 public keys as addresses, represented as [u8; 32].
    pub owner: [u8; 32],
}

// =============================================================================
// Step 2: Implement your contract's METHODS
// =============================================================================
//
// Methods are just regular Rust functions on your state struct.
// They fall into two categories:
//   - QUERIES:    &self methods that read state without modifying it
//   - MUTATIONS:  &mut self methods that change state (cost gas)

impl HelloWorldState {
    /// Create a new HelloWorldState.
    ///
    /// This is called once during the `init` dispatch to set up initial state.
    /// The deployer's address becomes the owner.
    pub fn new(greeting: String, owner: [u8; 32]) -> Self {
        Self { greeting, owner }
    }

    // -- Queries (read-only, no gas cost) ------------------------------------

    /// Returns the current greeting string.
    ///
    /// This is a VIEW method — it only reads state, never modifies it.
    /// View methods are free to call and don't require a signed transaction.
    pub fn get_greeting(&self) -> &str {
        &self.greeting
    }

    /// Returns the owner's address.
    ///
    /// Useful for verifying who controls this contract.
    pub fn owner(&self) -> &[u8; 32] {
        &self.owner
    }

    // -- Mutations (state-changing, cost gas) --------------------------------

    /// Updates the greeting to a new value.
    ///
    /// Only the contract owner can call this. The `caller` parameter is
    /// automatically provided by the Dina VM — it's the address that signed
    /// the transaction. This prevents impersonation.
    ///
    /// # Panics
    /// - If `caller` is not the contract owner
    /// - If `new_greeting` is empty
    pub fn set_greeting(&mut self, caller: [u8; 32], new_greeting: String) {
        // Access control: only the owner can change the greeting.
        // `assert!` will abort the transaction if the condition is false,
        // and the state change will be rolled back automatically.
        assert!(
            caller == self.owner,
            "HelloWorld: only the owner can change the greeting"
        );

        // Validate input: don't allow empty greetings
        assert!(
            !new_greeting.is_empty(),
            "HelloWorld: greeting cannot be empty"
        );

        // Update the state. This change will be persisted on-chain
        // after the transaction is confirmed.
        self.greeting = new_greeting;
    }
}

// =============================================================================
// Step 3: Define DISPATCH argument types
// =============================================================================
//
// Each method that takes arguments needs a corresponding arg struct.
// These are deserialized from the JSON bytes passed to dispatch().
// Query methods with no arguments (like `get_greeting`) don't need arg structs.

/// Arguments for the `init` method — called once when deploying the contract.
#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    /// The initial greeting message to store on-chain.
    greeting: String,
}

/// Arguments for the `set_greeting` method.
#[derive(Serialize, Deserialize, Debug)]
struct SetGreetingArgs {
    /// The new greeting message to replace the current one.
    new_greeting: String,
}

// =============================================================================
// Step 4: The DISPATCH function — the contract's entry point
// =============================================================================
//
// This is THE most important function in any Dina contract. The VM calls it
// for every transaction and every view call targeting your contract.
//
// Parameters:
//   state  — The current contract state wrapped in Option<T>.
//            None means the contract hasn't been initialized yet.
//            Some(T) means it has been initialized and T is the current state.
//   method — The name of the method the caller wants to invoke (e.g. "init",
//            "get_greeting", "set_greeting").
//   args   — The method arguments as a JSON-encoded byte slice.
//   caller — The 32-byte address of whoever signed this transaction.
//            For view calls, this is typically all zeros.
//
// Returns:
//   A JSON-encoded byte vector with the result. The caller will deserialize
//   this on their end.

pub fn dispatch(
    state: &mut Option<HelloWorldState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        // -- Initialization --------------------------------------------------
        // The `init` method is called exactly once when the contract is first
        // deployed. It creates the initial state. Calling init again will panic.
        "init" => {
            // Guard: prevent re-initialization
            assert!(state.is_none(), "HelloWorld: already initialised");

            // Parse the init arguments from JSON
            let a: InitArgs =
                serde_json::from_slice(args).expect("HelloWorld: bad init args");

            // Create the initial state. The `caller` (deployer) becomes owner.
            *state = Some(HelloWorldState::new(a.greeting, caller));

            // Return "ok" as JSON to indicate success
            serde_json::to_vec("ok").unwrap()
        }

        // -- Query: get_greeting ---------------------------------------------
        // Returns the current greeting. No arguments needed.
        "get_greeting" => {
            // `.as_ref()` borrows the state without taking ownership.
            // `.expect()` panics with a message if state is None (not initialized).
            let s = state.as_ref().expect("HelloWorld: not initialised");

            // Serialize the greeting string as JSON bytes and return it
            serde_json::to_vec(s.get_greeting()).unwrap()
        }

        // -- Query: owner ----------------------------------------------------
        // Returns the owner's address as a 32-byte array.
        "owner" => {
            let s = state.as_ref().expect("HelloWorld: not initialised");
            serde_json::to_vec(s.owner()).unwrap()
        }

        // -- Mutation: set_greeting ------------------------------------------
        // Changes the greeting. Requires the caller to be the owner.
        "set_greeting" => {
            // `.as_mut()` gives us a mutable reference to modify state.
            let s = state.as_mut().expect("HelloWorld: not initialised");

            // Parse the arguments
            let a: SetGreetingArgs =
                serde_json::from_slice(args).expect("HelloWorld: bad set_greeting args");

            // Call the method — access control happens inside
            s.set_greeting(caller, a.new_greeting);

            serde_json::to_vec("ok").unwrap()
        }

        // -- Catch-all for unknown methods -----------------------------------
        // If someone calls a method that doesn't exist, panic with a clear error.
        _ => panic!("HelloWorld: unknown method '{method}'"),
    }
}

// =============================================================================
// Tests
// =============================================================================
//
// You can run these with: cargo test -p example-hello-world
//
// Testing Dina contracts is just testing regular Rust code — call the dispatch
// function with different inputs and assert the outputs.

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a fake 32-byte address for testing
    fn make_address(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    #[test]
    fn test_init_and_get_greeting() {
        let mut state: Option<HelloWorldState> = None;
        let owner = make_address(1);

        // Initialize the contract
        let init_args = serde_json::to_vec(&InitArgs {
            greeting: "Hello, Dina!".to_string(),
        })
        .unwrap();

        let result = dispatch(&mut state, "init", &init_args, owner);
        assert_eq!(result, serde_json::to_vec("ok").unwrap());

        // Verify the greeting
        let result = dispatch(&mut state, "get_greeting", b"{}", owner);
        let greeting: String = serde_json::from_slice(&result).unwrap();
        assert_eq!(greeting, "Hello, Dina!");

        // Verify the owner
        let result = dispatch(&mut state, "owner", b"{}", owner);
        let returned_owner: [u8; 32] = serde_json::from_slice(&result).unwrap();
        assert_eq!(returned_owner, owner);
    }

    #[test]
    fn test_set_greeting_by_owner() {
        let mut state: Option<HelloWorldState> = None;
        let owner = make_address(1);

        // Init
        let init_args = serde_json::to_vec(&InitArgs {
            greeting: "Hello!".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "init", &init_args, owner);

        // Change greeting
        let set_args = serde_json::to_vec(&SetGreetingArgs {
            new_greeting: "Goodbye!".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "set_greeting", &set_args, owner);

        // Verify it changed
        let result = dispatch(&mut state, "get_greeting", b"{}", owner);
        let greeting: String = serde_json::from_slice(&result).unwrap();
        assert_eq!(greeting, "Goodbye!");
    }

    #[test]
    #[should_panic(expected = "only the owner")]
    fn test_set_greeting_by_non_owner_fails() {
        let mut state: Option<HelloWorldState> = None;
        let owner = make_address(1);
        let stranger = make_address(2);

        let init_args = serde_json::to_vec(&InitArgs {
            greeting: "Hello!".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "init", &init_args, owner);

        // A non-owner tries to change the greeting — should panic
        let set_args = serde_json::to_vec(&SetGreetingArgs {
            new_greeting: "Hacked!".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "set_greeting", &set_args, stranger);
    }

    #[test]
    #[should_panic(expected = "already initialised")]
    fn test_double_init_fails() {
        let mut state: Option<HelloWorldState> = None;
        let owner = make_address(1);

        let init_args = serde_json::to_vec(&InitArgs {
            greeting: "Hello!".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "init", &init_args, owner);
        // Second init should panic
        dispatch(&mut state, "init", &init_args, owner);
    }
}
