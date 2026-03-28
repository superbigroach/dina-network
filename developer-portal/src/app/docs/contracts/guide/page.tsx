import Link from "next/link";

export const metadata = {
  title: "Smart Contract Developer Guide — Dina Network Developer Portal",
  description:
    "The definitive guide to building, testing, deploying, and calling smart contracts on Dina Network. Covers Rust/WASM contracts, host functions, gas costs, common patterns, and migration from Solidity.",
};

const HOST_FUNCTIONS = [
  {
    name: "dina_get_caller",
    signature: "() -> i32",
    description: "Returns pointer to 32-byte caller address in linear memory",
  },
  {
    name: "dina_get_block_time",
    signature: "() -> i64",
    description: "Current block timestamp in milliseconds since epoch",
  },
  {
    name: "dina_get_block_height",
    signature: "() -> i64",
    description: "Current block height at execution time",
  },
  {
    name: "dina_get_balance",
    signature: "(addr_ptr: i32) -> i64",
    description: "USDC balance of an address (6 decimal places)",
  },
  {
    name: "dina_state_read",
    signature: "(key_ptr: i32, key_len: i32, buf_ptr: i32, buf_len: i32) -> i32",
    description: "Read state bytes by key. Returns bytes read.",
  },
  {
    name: "dina_state_write",
    signature: "(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32) -> i32",
    description: "Write state bytes by key. Returns 0 on success.",
  },
  {
    name: "dina_state_delete",
    signature: "(key_ptr: i32, key_len: i32) -> i32",
    description: "Delete a state key. Returns 0 on success.",
  },
  {
    name: "dina_transfer",
    signature: "(to_ptr: i32, amount: i64) -> i32",
    description: "Transfer USDC from contract to address. Returns 0 on success.",
  },
  {
    name: "dina_emit_event",
    signature: "(name_ptr: i32, name_len: i32, data_ptr: i32, data_len: i32)",
    description: "Emit a named event with JSON data for indexing",
  },
  {
    name: "dina_sha256",
    signature: "(data_ptr: i32, data_len: i32, out_ptr: i32)",
    description: "Compute SHA-256 hash, write 32 bytes to out_ptr",
  },
  {
    name: "dina_ed25519_verify",
    signature: "(msg_ptr: i32, msg_len: i32, sig_ptr: i32, pub_ptr: i32) -> i32",
    description: "Verify Ed25519 signature. Returns 1 if valid.",
  },
  {
    name: "dina_log",
    signature: "(ptr: i32, len: i32)",
    description: "Debug logging (testnet only, stripped in production)",
  },
  {
    name: "dina_abort",
    signature: "(msg_ptr: i32, msg_len: i32)",
    description: "Abort execution with an error message (rollback all state)",
  },
];

const GAS_TABLE = [
  { operation: "Contract deploy", gas: "21,000 + 200/byte of WASM" },
  { operation: "Contract call (base)", gas: "30,000" },
  { operation: "WASM instruction (each)", gas: "1" },
  { operation: "State read", gas: "500" },
  { operation: "State write", gas: "2,000" },
  { operation: "State delete", gas: "500" },
  { operation: "SHA-256 hash", gas: "200" },
  { operation: "Ed25519 verify", gas: "3,000" },
  { operation: "Event emission", gas: "500" },
  { operation: "USDC transfer", gas: "5,000" },
  { operation: "Memory page (64 KB)", gas: "1,000" },
  { operation: "Log (debug, testnet)", gas: "100" },
];

const SOLIDITY_COMPARISON = [
  {
    solidity: "contract MyContract { ... }",
    dina: "pub struct MyContract { ... }",
    note: "State is a serializable Rust struct",
  },
  {
    solidity: "constructor(uint256 x)",
    dina: 'dispatch(state, "new", args, caller)',
    note: "Init is just the first dispatch call",
  },
  {
    solidity: "function foo() public",
    dina: "pub fn foo(&mut self)",
    note: "Methods on the state struct",
  },
  {
    solidity: "function bar() view returns (uint256)",
    dina: "pub fn bar(&self) -> u64",
    note: "View functions take &self (immutable)",
  },
  {
    solidity: "msg.sender",
    dina: "caller: String (dispatch parameter)",
    note: "Passed by runtime, not a global",
  },
  {
    solidity: "msg.value",
    dina: "usdc_attached: u64 (dispatch parameter)",
    note: "USDC amount attached to call",
  },
  {
    solidity: "block.timestamp",
    dina: "dina_get_block_time()",
    note: "Host function, milliseconds",
  },
  {
    solidity: "block.number",
    dina: "dina_get_block_height()",
    note: "Host function",
  },
  {
    solidity: "require(condition, msg)",
    dina: 'assert!(condition, "msg")',
    note: "Rust assert macro",
  },
  {
    solidity: "emit Transfer(from, to, amount)",
    dina: 'host::emit_event("Transfer", &data)',
    note: "Named events with JSON payload",
  },
  {
    solidity: "mapping(address => uint256)",
    dina: "HashMap<String, u64>",
    note: "Use BTreeMap for determinism",
  },
  {
    solidity: "modifier onlyOwner",
    dina: 'assert!(caller == self.owner, "...")',
    note: "Inline checks, no modifier syntax",
  },
  {
    solidity: "payable",
    dina: "Check usdc_attached > 0",
    note: "No payable keyword, check manually",
  },
  {
    solidity: "abi.encode / abi.decode",
    dina: "serde_json::to_vec / from_slice",
    note: "JSON serialization, not ABI encoding",
  },
  {
    solidity: "inheritance (is Ownable)",
    dina: "Composition or Rust traits",
    note: "No inheritance, use composition",
  },
];

const COMMON_PATTERNS = [
  {
    name: "Ownable",
    description: "Restrict admin functions to the contract deployer",
    code: `#[derive(Serialize, Deserialize, Clone)]
pub struct Ownable {
    owner: String,
}

impl Ownable {
    pub fn new(owner: String) -> Self {
        Self { owner }
    }

    pub fn require_owner(&self, caller: &str) {
        assert!(caller == self.owner, "Ownable: caller is not the owner");
    }

    pub fn transfer_ownership(&mut self, caller: &str, new_owner: String) {
        self.require_owner(caller);
        self.owner = new_owner;
    }
}`,
  },
  {
    name: "Pausable",
    description: "Emergency stop mechanism for critical situations",
    code: `#[derive(Serialize, Deserialize, Clone)]
pub struct Pausable {
    paused: bool,
    owner: String,
}

impl Pausable {
    pub fn require_not_paused(&self) {
        assert!(!self.paused, "Pausable: contract is paused");
    }

    pub fn pause(&mut self, caller: &str) {
        assert!(caller == self.owner, "Only owner can pause");
        self.paused = true;
    }

    pub fn unpause(&mut self, caller: &str) {
        assert!(caller == self.owner, "Only owner can unpause");
        self.paused = false;
    }
}`,
  },
  {
    name: "DRC-1 Token",
    description: "Fungible token with transfer and approval",
    code: `#[derive(Serialize, Deserialize, Clone)]
pub struct Token {
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: u64,
    balances: BTreeMap<String, u64>,
    allowances: BTreeMap<String, BTreeMap<String, u64>>,
}

impl Token {
    pub fn transfer(&mut self, from: &str, to: &str, amount: u64) {
        let balance = self.balances.get(from).copied().unwrap_or(0);
        assert!(balance >= amount, "Insufficient balance");
        *self.balances.entry(from.to_string()).or_default() -= amount;
        *self.balances.entry(to.to_string()).or_default() += amount;
    }
}`,
  },
  {
    name: "Access Control",
    description: "Role-based permissions for multi-admin contracts",
    code: `use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, Deserialize, Clone)]
pub struct AccessControl {
    roles: BTreeMap<String, BTreeSet<String>>,
}

impl AccessControl {
    pub fn has_role(&self, role: &str, account: &str) -> bool {
        self.roles.get(role).map_or(false, |s| s.contains(account))
    }

    pub fn grant_role(&mut self, role: &str, account: &str, caller: &str) {
        assert!(self.has_role("ADMIN", caller), "Not admin");
        self.roles
            .entry(role.to_string())
            .or_default()
            .insert(account.to_string());
    }

    pub fn require_role(&self, role: &str, account: &str) {
        assert!(self.has_role(role, account), "Missing role: {role}");
    }
}`,
  },
];

const EXAMPLE_CONTRACTS = [
  {
    name: "DRC-1 Fungible Token",
    path: "contracts/drc1-token/",
    description: "Standard fungible token with transfer, approve, and transferFrom",
  },
  {
    name: "DRC-6 NFT",
    path: "contracts/drc6-nft/",
    description: "Non-fungible token with mint, burn, and metadata",
  },
  {
    name: "DinaDEX Swap",
    path: "contracts/dex-swap/",
    description: "Automated market maker with constant-product formula",
  },
  {
    name: "Yield Vault",
    path: "contracts/defi-vault/",
    description: "Deposit USDC and earn yield with share-based accounting",
  },
  {
    name: "Lending Pool",
    path: "contracts/defi-lending/",
    description: "Collateralized lending with variable interest rates",
  },
  {
    name: "Parallel Wallet (DRC-63)",
    path: "contracts/drc63-swarm-wallet/",
    description: "Multi-party wallet with swarm-based approval",
  },
  {
    name: "Agent Wallet (DRC-101)",
    path: "contracts/drc101-agent-wallet/",
    description: "Autonomous agent wallet with capability-based permissions",
  },
  {
    name: "Multisig (DRC-21)",
    path: "contracts/drc21-multisig/",
    description: "N-of-M multisignature wallet",
  },
];

export default function ContractDevGuidePage() {
  return (
    <div>
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Smart Contracts
      </p>
      <h1 className="text-4xl font-bold tracking-tight text-white mb-4">
        Smart Contract Developer Guide
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-6">
        The complete guide to building, testing, deploying, and calling smart
        contracts on Dina Network. Contracts are written in Rust, compiled to
        WebAssembly, and executed deterministically across all validators with
        100ms block times and 10,000 TPS throughput.
      </p>

      {/* Table of Contents */}
      <nav className="mb-12 rounded-xl border border-slate-800 bg-slate-900/50 p-6">
        <h2 className="text-sm font-semibold uppercase tracking-wider text-slate-400 mb-4">
          On This Page
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-2">
          {[
            { label: "Getting Started", anchor: "#getting-started" },
            { label: "Contract Structure", anchor: "#contract-structure" },
            { label: "The Dispatch Function", anchor: "#dispatch" },
            { label: "Available Host Functions", anchor: "#host-functions" },
            { label: "Testing Locally", anchor: "#testing" },
            { label: "Building", anchor: "#building" },
            { label: "Deploying", anchor: "#deploying" },
            { label: "Calling Contracts", anchor: "#calling" },
            { label: "Gas Costs", anchor: "#gas" },
            { label: "Common Patterns", anchor: "#patterns" },
            { label: "Example Contracts", anchor: "#examples" },
            { label: "Differences from Solidity", anchor: "#solidity-comparison" },
          ].map((item) => (
            <a
              key={item.anchor}
              href={item.anchor}
              className="text-sm text-slate-300 hover:text-blue-400 transition-colors py-0.5"
            >
              {item.label}
            </a>
          ))}
        </div>
      </nav>

      {/* ──────────────────────────────────────────────── */}
      {/* Getting Started */}
      {/* ──────────────────────────────────────────────── */}
      <section id="getting-started" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Getting Started
        </h2>

        <h3 className="text-lg font-semibold text-white mb-3">Prerequisites</h3>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          You need the Rust toolchain with the WASM compilation target. If you
          already have Rust installed, just add the target.
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the WASM compilation target
rustup target add wasm32-unknown-unknown

# Install the Dina CLI
cargo install dina-cli

# Create a wallet and get testnet USDC
dina wallet create
dina faucet request --network testnet

# Optional: install wasm-opt for binary size optimization
cargo install wasm-opt`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">
          Clone the Starter Template
        </h3>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          The fastest way to start is by copying the starter template. It
          includes a working contract with tests, a Cargo.toml with
          release optimizations, and a README with instructions.
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`# Copy the starter template
cp -r templates/contract-starter my-contract
cd my-contract

# Run the tests to verify everything works
cargo test

# You should see:
# running 2 tests
# test tests::test_full_lifecycle ... ok
# test tests::test_only_owner_can_reset ... ok
# test result: ok. 2 passed; 0 failed`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">
          Project Structure
        </h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`my-contract/
  Cargo.toml          # Dependencies + release profile (size-optimized)
  src/
    lib.rs            # Contract state, methods, dispatch, and tests`}</code>
        </pre>
        <p className="text-sm text-slate-400 leading-relaxed mt-3">
          For larger contracts you can split into multiple files (e.g.,{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            src/state.rs
          </code>
          ,{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            src/dispatch.rs
          </code>
          ,{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            src/access.rs
          </code>
          ), but a single{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            lib.rs
          </code>{" "}
          works fine for most contracts.
        </p>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Contract Structure */}
      {/* ──────────────────────────────────────────────── */}
      <section id="contract-structure" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Contract Structure
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          Every Dina contract has three parts: a state struct, methods on that
          struct, and a dispatch function that routes incoming calls.
        </p>

        <div className="space-y-6">
          {/* State Struct */}
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-white mb-2">
              1. State Struct
            </h3>
            <p className="text-sm text-slate-400 leading-relaxed mb-4">
              The state struct holds all data stored on-chain. It must derive{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
                Serialize
              </code>{" "}
              and{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
                Deserialize
              </code>{" "}
              so the runtime can persist it between calls. The entire struct is
              JSON-serialized and stored as a blob associated with your contract
              address.
            </p>
            <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
              <code className="text-slate-200">{`#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MyContract {
    owner: String,          // Deployer address
    counter: u64,           // Simple counter
    balances: BTreeMap<String, u64>,  // User balances
}`}</code>
            </pre>
            <div className="mt-3 rounded-lg border border-amber-500/30 bg-amber-500/5 p-3">
              <p className="text-sm text-amber-300">
                <strong>Determinism rule:</strong> Use{" "}
                <code className="text-xs">BTreeMap</code> /{" "}
                <code className="text-xs">BTreeSet</code> instead of{" "}
                <code className="text-xs">HashMap</code> /{" "}
                <code className="text-xs">HashSet</code> in production contracts.
                Unordered iteration produces different serialization on different
                validators, breaking consensus. The starter template uses HashMap
                for simplicity, but real contracts should use BTreeMap.
              </p>
            </div>
          </div>

          {/* Methods */}
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-white mb-2">
              2. Methods
            </h3>
            <p className="text-sm text-slate-400 leading-relaxed mb-4">
              Methods are regular Rust functions on your state struct. Methods
              that modify state take{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
                &amp;mut self
              </code>
              . View functions (read-only) take{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
                &amp;self
              </code>
              . View calls are free and do not consume gas.
            </p>
            <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
              <code className="text-slate-200">{`impl MyContract {
    // Mutation -- costs gas, included in a block
    pub fn increment(&mut self) -> u64 {
        self.counter += 1;
        self.counter
    }

    // View -- free, no gas, not included in a block
    pub fn get_counter(&self) -> u64 {
        self.counter
    }

    // Access-controlled mutation
    pub fn reset(&mut self, caller: &str) {
        assert!(caller == self.owner, "Only owner can reset");
        self.counter = 0;
    }
}`}</code>
            </pre>
          </div>

          {/* Dispatch */}
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-white mb-2">
              3. Dispatch Function
            </h3>
            <p className="text-sm text-slate-400 leading-relaxed mb-4">
              The dispatch function is the entry point called by the Dina
              runtime. It receives the current state, the method name, serialized
              arguments, and the caller address. It routes to the correct method
              and returns the result as serialized bytes.
            </p>
            <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
              <code className="text-slate-200">{`pub fn dispatch(
    state: &mut Option<MyContract>,  // None if not yet initialized
    method: &str,                     // Method name from the transaction
    args: &[u8],                      // JSON-serialized arguments
    caller: String,                   // Address of the caller
) -> Vec<u8> {                        // JSON-serialized return value
    match method {
        "new" => {
            assert!(state.is_none(), "Already initialized");
            *state = Some(MyContract::new(caller));
            serde_json::to_vec("initialized").unwrap()
        }
        "increment" => {
            let s = state.as_mut().expect("Not initialized");
            let val = s.increment();
            serde_json::to_vec(&val).unwrap()
        }
        "get_counter" => {
            let s = state.as_ref().expect("Not initialized");
            serde_json::to_vec(&s.get_counter()).unwrap()
        }
        _ => panic!("Unknown method: {method}"),
    }
}`}</code>
            </pre>
          </div>
        </div>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Host Functions */}
      {/* ──────────────────────────────────────────────── */}
      <section id="host-functions" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Available Host Functions
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Host functions are the interface between your WASM contract and the
          Dina blockchain. They are the only way for contracts to interact with
          the outside world -- the WASM sandbox has no access to filesystem,
          network, or system clock. Use them via the{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            dina-sdk
          </code>{" "}
          crate or call them directly through the WASM FFI.
        </p>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Function
                </th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Signature
                </th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Description
                </th>
              </tr>
            </thead>
            <tbody>
              {HOST_FUNCTIONS.map((fn_, i) => (
                <tr
                  key={fn_.name}
                  className={
                    i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"
                  }
                >
                  <td className="px-4 py-3 font-mono text-blue-300 whitespace-nowrap">
                    {fn_.name}
                  </td>
                  <td className="px-4 py-3 font-mono text-slate-400 text-xs whitespace-nowrap">
                    {fn_.signature}
                  </td>
                  <td className="px-4 py-3 text-slate-400">
                    {fn_.description}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="mt-6">
          <h3 className="text-lg font-semibold text-white mb-3">
            Using Host Functions via dina-sdk
          </h3>
          <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
            <code className="text-slate-200">{`use dina_sdk::host;

// Get block info
let height = host::get_block_height();
let time = host::get_block_time();

// Transfer USDC from contract balance
let result = host::transfer(&recipient_addr, 1_000_000); // 1 USDC
assert!(result == 0, "transfer failed");

// Emit an event for indexers to pick up
host::emit_event("Transfer", &serde_json::json!({
    "to": recipient,
    "amount": 1_000_000,
}));

// Verify a signature
let valid = host::ed25519_verify(&message, &signature, &pubkey);

// Hash data
let hash = host::sha256(&data);

// Debug logging (testnet only)
host::log(&format!("Balance: {}", balance));`}</code>
          </pre>
        </div>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Testing */}
      {/* ──────────────────────────────────────────────── */}
      <section id="testing" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Testing Locally
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Dina contracts are standard Rust libraries, so you can use{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            cargo test
          </code>{" "}
          to run unit tests without deploying to a network. This is the fastest
          way to iterate on contract logic.
        </p>

        <h3 className="text-lg font-semibold text-white mb-3">
          Unit Tests with cargo test
        </h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> String {
        format!("0x{:064x}", n)
    }

    #[test]
    fn test_increment() {
        let mut state = None;

        // Initialize
        dispatch(&mut state, "new", b"", addr(1));
        assert!(state.is_some());

        // Increment twice
        dispatch(&mut state, "increment", b"", addr(2));
        let result = dispatch(&mut state, "increment", b"", addr(2));
        let val: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(val, 2);
    }

    #[test]
    #[should_panic(expected = "Only owner")]
    fn test_access_control() {
        let mut state = None;
        dispatch(&mut state, "new", b"", addr(1));

        // addr(2) is not the owner -- should panic
        dispatch(&mut state, "reset", b"", addr(2));
    }

    #[test]
    fn test_greeting_round_trip() {
        let mut state = None;
        dispatch(&mut state, "new", b"", addr(1));

        let msg = serde_json::to_vec("Hello Dina!").unwrap();
        dispatch(&mut state, "set_greeting", &msg, addr(1));

        let query = serde_json::to_vec(&addr(1)).unwrap();
        let result = dispatch(&mut state, "get_greeting", &query, addr(2));
        let greeting: String = serde_json::from_slice(&result).unwrap();
        assert_eq!(greeting, "Hello Dina!");
    }
}`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">
          Mocking Host Functions
        </h3>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          If your contract uses host functions (via{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            dina-sdk
          </code>
          ), you can mock them in tests by feature-gating the imports:
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`// In your contract:
#[cfg(not(test))]
fn get_block_time() -> u64 {
    dina_sdk::host::get_block_time()
}

#[cfg(test)]
fn get_block_time() -> u64 {
    1_700_000_000_000  // Fixed timestamp for tests
}`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">
          Integration Testing with the CLI
        </h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`# Deploy to testnet
dina deploy --wasm target/wasm32-unknown-unknown/release/my_dina_contract.wasm \\
  --network testnet

# Call a method and check the result
RESULT=$(dina call --contract 0x... --method increment --network testnet)
echo "$RESULT"  # {"result": 1}

# View call (free, no wallet needed)
dina view --contract 0x... --method get_counter --network testnet`}</code>
        </pre>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Building */}
      {/* ──────────────────────────────────────────────── */}
      <section id="building" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">Building</h2>

        <h3 className="text-lg font-semibold text-white mb-3">
          Compile to WASM
        </h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`cargo build --target wasm32-unknown-unknown --release`}</code>
        </pre>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          The output is at{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            target/wasm32-unknown-unknown/release/my_dina_contract.wasm
          </code>
          . The starter template includes release profile optimizations in Cargo.toml
          that minimize binary size.
        </p>

        <h3 className="text-lg font-semibold text-white mb-3">
          Optimize with wasm-opt (Optional)
        </h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-4">
          <code className="text-slate-200">{`# Install wasm-opt
cargo install wasm-opt

# Optimize for size (-Oz) or speed (-O4)
wasm-opt -Oz target/wasm32-unknown-unknown/release/my_dina_contract.wasm \\
  -o optimized.wasm

# Typical size reductions:
# Before wasm-opt:  45 KB
# After wasm-opt:   12 KB  (73% smaller)
# Deploy savings:   ~6,600 gas (~0.31 USDC)`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">
          Release Profile (Cargo.toml)
        </h3>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          The starter template includes these optimizations. Add them to your
          Cargo.toml if you started from scratch:
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-4">
          <code className="text-slate-200">{`[profile.release]
opt-level = "z"          # Optimize for size
lto = true               # Link-time optimization
codegen-units = 1        # Single codegen unit for better optimization
strip = true             # Strip debug symbols
panic = "abort"          # Smaller panic handler`}</code>
        </pre>

        <div className="rounded-xl border border-amber-500/30 bg-amber-500/5 p-4">
          <p className="text-sm text-amber-300">
            <strong>Size limit:</strong> Maximum WASM bytecode size is 512 KB.
            Contracts exceeding this limit will be rejected at deploy time. Most
            contracts are well under 50 KB after optimization.
          </p>
        </div>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Deploying */}
      {/* ──────────────────────────────────────────────── */}
      <section id="deploying" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">Deploying</h2>

        <h3 className="text-lg font-semibold text-white mb-3">Via CLI</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`# Deploy to testnet
dina deploy --wasm target/wasm32-unknown-unknown/release/my_dina_contract.wasm \\
  --network testnet

# Deploy with initialization arguments
dina deploy --wasm optimized.wasm \\
  --init '{"name": "My Token", "symbol": "MTK"}' \\
  --network testnet

# Output:
# Contract deployed at: 0x7a3f...e291
# Transaction hash:     0xab12...ff90
# Gas used:             34,200
# Cost:                 0.0034 USDC`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">Via SDK</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`import { DinaClient, Wallet } from '@dina-network/sdk';
import { readFileSync } from 'fs';

const client = new DinaClient('https://rpc.dina.network');
const wallet = Wallet.fromMnemonic('your mnemonic here...');

const wasmBytes = readFileSync('./optimized.wasm');

const result = await client.deployContract(wallet, {
  wasmBytes,
  initArgs: { name: "My Token", symbol: "MTK" },
});

console.log('Contract address:', result.contractAddress);
console.log('Tx hash:', result.txHash);
console.log('Gas used:', result.gasUsed);`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">
          Contract Address Derivation
        </h3>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Contract addresses are deterministic:{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            SHA-256(deployer_address ++ nonce)
          </code>
          , truncated to 32 bytes. This means you can predict the contract
          address before deploying by knowing your address and current nonce.
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`// Predict contract address before deployment
const nonce = await client.getAccount(wallet.address);
const predicted = DinaClient.predictContractAddress(
  wallet.address,
  nonce.nonce
);
console.log('Contract will deploy at:', predicted);`}</code>
        </pre>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Calling Contracts */}
      {/* ──────────────────────────────────────────────── */}
      <section id="calling" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Calling Contracts
        </h2>

        <div className="space-y-6">
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-white mb-3">
              Via CLI
            </h3>
            <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
              <code className="text-slate-200">{`# Mutation call (costs gas, included in block)
dina call --contract 0x7a3f...e291 \\
  --method increment \\
  --network testnet

# Mutation with arguments
dina call --contract 0x7a3f...e291 \\
  --method set_greeting \\
  --args '"Hello from CLI!"' \\
  --network testnet

# View call (free, no gas, instant response)
dina view --contract 0x7a3f...e291 \\
  --method get_counter \\
  --network testnet`}</code>
            </pre>
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-white mb-3">
              Via JavaScript/TypeScript SDK
            </h3>
            <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
              <code className="text-slate-200">{`import { DinaClient, Wallet } from '@dina-network/sdk';

const client = new DinaClient('https://rpc.dina.network');
const wallet = Wallet.fromMnemonic('your mnemonic...');

// Mutation call (costs gas)
const tx = await client.callContract(wallet, {
  contract: '0x7a3f...e291',
  method: 'increment',
  args: {},
});
console.log('New counter value:', tx.result); // 1

// View call (free, no gas)
const counter = await client.viewContract({
  contract: '0x7a3f...e291',
  method: 'get_counter',
  args: {},
});
console.log('Counter:', counter); // 1

// Call with arguments
await client.callContract(wallet, {
  contract: '0x7a3f...e291',
  method: 'set_greeting',
  args: 'Hello from SDK!',
});`}</code>
            </pre>
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-white mb-3">
              Via JSON-RPC
            </h3>
            <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
              <code className="text-slate-200">{`curl -X POST https://rpc.dina.network \\
  -H "Content-Type: application/json" \\
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "dina_callContract",
    "params": [{
      "contract": "0x7a3f...e291",
      "method": "get_counter",
      "args": {}
    }]
  }'`}</code>
            </pre>
          </div>
        </div>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Gas Costs */}
      {/* ──────────────────────────────────────────────── */}
      <section id="gas" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">Gas Costs</h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Gas is paid in USDC (6 decimals). View calls are free. All gas costs
          below are additive -- a contract call that writes state and emits an
          event pays the base call cost + state write cost + event emission cost
          + per-instruction cost.
        </p>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Operation
                </th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Gas Cost
                </th>
              </tr>
            </thead>
            <tbody>
              {GAS_TABLE.map((row, i) => (
                <tr
                  key={row.operation}
                  className={
                    i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"
                  }
                >
                  <td className="px-4 py-3 text-slate-300">{row.operation}</td>
                  <td className="px-4 py-3 font-mono text-blue-300">
                    {row.gas}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="mt-4 rounded-xl border border-amber-500/30 bg-amber-500/5 p-4">
          <p className="text-sm text-amber-300">
            <strong>Note:</strong> View calls (read-only queries) are not
            metered and do not consume gas. Gas is only charged for mutations
            that are included in a block. If a contract runs out of gas during
            execution, all state changes are rolled back but the transaction fee
            is still consumed.
          </p>
        </div>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Common Patterns */}
      {/* ──────────────────────────────────────────────── */}
      <section id="patterns" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Common Patterns
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          These patterns appear in most production contracts. Copy and adapt them
          for your own use.
        </p>
        <div className="space-y-6">
          {COMMON_PATTERNS.map((pattern) => (
            <div
              key={pattern.name}
              className="rounded-xl border border-slate-800 bg-slate-900/50 p-6"
            >
              <h3 className="text-lg font-semibold text-white mb-1">
                {pattern.name}
              </h3>
              <p className="text-sm text-slate-400 mb-4">
                {pattern.description}
              </p>
              <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
                <code className="text-slate-200">{pattern.code}</code>
              </pre>
            </div>
          ))}
        </div>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Example Contracts */}
      {/* ──────────────────────────────────────────────── */}
      <section id="examples" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Example Contracts
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          Production-quality reference contracts in the Dina Network repository.
          Each includes full source, tests, and deployment scripts.
        </p>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {EXAMPLE_CONTRACTS.map((example) => (
            <div
              key={example.name}
              className="rounded-xl border border-slate-800 bg-slate-900/30 p-5 hover:border-blue-500/30 transition-colors"
            >
              <h3 className="text-sm font-semibold text-white mb-1">
                {example.name}
              </h3>
              <p className="text-xs text-slate-500 font-mono mb-2">
                {example.path}
              </p>
              <p className="text-sm text-slate-400">{example.description}</p>
            </div>
          ))}
        </div>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Solidity Comparison */}
      {/* ──────────────────────────────────────────────── */}
      <section id="solidity-comparison" className="mb-16">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Differences from Solidity / EVM
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          If you are coming from Ethereum / Solidity development, this table
          maps every major concept to its Dina equivalent. The mental model is
          similar -- state, methods, access control, events -- but the syntax
          and tooling are completely different.
        </p>

        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Solidity / EVM
                </th>
                <th className="px-4 py-3 text-left font-medium text-blue-400">
                  Dina / WASM
                </th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Notes
                </th>
              </tr>
            </thead>
            <tbody>
              {SOLIDITY_COMPARISON.map((row, i) => (
                <tr
                  key={row.solidity}
                  className={
                    i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"
                  }
                >
                  <td className="px-4 py-3 font-mono text-slate-300 text-xs">
                    {row.solidity}
                  </td>
                  <td className="px-4 py-3 font-mono text-blue-300 text-xs">
                    {row.dina}
                  </td>
                  <td className="px-4 py-3 text-slate-400">{row.note}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="mt-6 space-y-4">
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-white mb-3">
              Key Architectural Differences
            </h3>
            <div className="space-y-3">
              {[
                {
                  title: "Rust instead of Solidity",
                  desc: "Full Rust language with borrow checker, generics, traits, and the crates.io ecosystem. No need for a domain-specific language.",
                },
                {
                  title: "WASM instead of EVM bytecode",
                  desc: "Industry-standard WebAssembly runs 5-25x faster than EVM. Same format used by browsers, Cloudflare Workers, and other blockchains.",
                },
                {
                  title: "JSON serialization instead of ABI encoding",
                  desc: "Arguments and return values are JSON-serialized with serde. No ABI encoding/decoding, no function selectors, no calldata packing.",
                },
                {
                  title: "No inheritance -- use composition",
                  desc: "Rust has no inheritance. Use struct composition (embed one struct in another) or traits for shared behavior. This produces simpler, more predictable contracts.",
                },
                {
                  title: "USDC-native instead of ETH-native",
                  desc: "Gas is priced in USDC (6 decimals). No native token volatility. No need to convert between ETH and stablecoins.",
                },
                {
                  title: "cargo test instead of Hardhat/Foundry",
                  desc: "Unit tests run natively with cargo test -- no blockchain fork needed, no JS test runner, no Anvil/Ganache. Tests execute in milliseconds.",
                },
              ].map((item) => (
                <div
                  key={item.title}
                  className="rounded-lg border border-slate-800 bg-slate-900/30 p-4"
                >
                  <h4 className="text-sm font-semibold text-white">
                    {item.title}
                  </h4>
                  <p className="text-sm text-slate-400 mt-1">{item.desc}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* ──────────────────────────────────────────────── */}
      {/* Next Steps */}
      {/* ──────────────────────────────────────────────── */}
      <section className="mb-8">
        <h2 className="text-2xl font-semibold text-white mb-4">Next Steps</h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          {[
            {
              title: "Deploy Contract",
              href: "/docs/contracts/deploy",
              desc: "Step-by-step deploy walkthrough",
            },
            {
              title: "Call Contract",
              href: "/docs/contracts/call",
              desc: "Interact with deployed contracts",
            },
            {
              title: "DRC Standards",
              href: "/docs/contracts/standards",
              desc: "Token, NFT, and DeFi standards",
            },
            {
              title: "WASM Runtime",
              href: "/docs/contracts/wasm",
              desc: "Deep dive into the execution engine",
            },
            {
              title: "JavaScript SDK",
              href: "/docs/sdk/javascript",
              desc: "Build dApps with TypeScript",
            },
            {
              title: "CLI Reference",
              href: "/docs/sdk/cli",
              desc: "All CLI commands and flags",
            },
          ].map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className="rounded-xl border border-slate-800 bg-slate-900/30 p-5 hover:border-blue-500/40 transition-all group"
            >
              <h3 className="text-sm font-semibold text-white group-hover:text-blue-400 transition-colors">
                {item.title}
              </h3>
              <p className="text-xs text-slate-500 mt-1">{item.desc}</p>
            </Link>
          ))}
        </div>
      </section>
    </div>
  );
}
