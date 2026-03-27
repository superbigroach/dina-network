import Link from "next/link";

export const metadata = {
  title: "Deploy Smart Contract — Dina Network Developer Portal",
  description:
    "Write, compile, and deploy WASM smart contracts on Dina Network using Rust.",
};

export default function DeployContractPage() {
  return (
    <div>
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Smart Contracts
      </p>
      <h1 className="text-4xl font-bold tracking-tight text-white mb-4">
        Deploy Smart Contract
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-10">
        Dina Network smart contracts are written in Rust, compiled to
        WebAssembly (WASM), and deployed on-chain. This guide walks through
        the complete workflow from writing your first contract to deploying it
        on testnet.
      </p>

      {/* Prerequisites */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Prerequisites
        </h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
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
      </section>

      {/* Step 1: Write */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Step 1: Write the Contract
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Every Dina contract has four parts: a state struct, methods, argument
          structs, and a dispatch function. Here is a complete counter contract:
        </p>

        <h3 className="text-lg font-semibold text-white mb-3">Cargo.toml</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`[package]
name = "my-counter"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[lib]
crate-type = ["cdylib", "lib"]`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">src/lib.rs</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`use serde::{Deserialize, Serialize};

// 1. State struct -- all on-chain data
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CounterState {
    pub count: u64,
    pub owner: [u8; 32],
}

// 2. Methods
impl CounterState {
    pub fn new(owner: [u8; 32]) -> Self {
        Self { count: 0, owner }
    }

    pub fn get_count(&self) -> u64 {
        self.count
    }

    pub fn increment(&mut self, caller: [u8; 32]) {
        // Anyone can increment
        self.count += 1;
    }

    pub fn reset(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "Counter: only owner can reset");
        self.count = 0;
    }
}

// 3. Argument structs (none needed for this simple contract)

// 4. Dispatch function -- the contract entry point
pub fn dispatch(
    state: &mut Option<CounterState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Counter: already initialized");
            *state = Some(CounterState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "get_count" => {
            let s = state.as_ref().expect("not initialized");
            serde_json::to_vec(&s.get_count()).unwrap()
        }
        "increment" => {
            let s = state.as_mut().expect("not initialized");
            s.increment(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "reset" => {
            let s = state.as_mut().expect("not initialized");
            s.reset(caller);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("unknown method '{method}'"),
    }
}`}</code>
        </pre>
      </section>

      {/* Step 2: Compile */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Step 2: Compile to WASM
        </h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-4">
          <code className="text-slate-200">{`# Build the contract targeting WASM
cargo build --target wasm32-unknown-unknown --release

# The compiled binary is at:
# target/wasm32-unknown-unknown/release/my_counter.wasm

# Optional: optimize the binary size (reduces deploy cost)
wasm-opt -Oz \\
  target/wasm32-unknown-unknown/release/my_counter.wasm \\
  -o target/wasm32-unknown-unknown/release/my_counter_optimized.wasm

# Check the size
ls -lh target/wasm32-unknown-unknown/release/my_counter*.wasm
# my_counter.wasm             45 KB
# my_counter_optimized.wasm   12 KB  (73% smaller)`}</code>
        </pre>
        <div className="rounded-xl border border-amber-500/30 bg-amber-500/5 p-4">
          <p className="text-sm text-amber-300">
            <strong>Tip:</strong> Always use{" "}
            <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs">wasm-opt</code>{" "}
            before deploying. Smaller binaries cost less gas.
            Deploy cost = 21,000 + (200 x bytes). A 33 KB reduction saves
            6,600 gas (~0.31 USDC).
          </p>
        </div>
      </section>

      {/* Step 3: Deploy */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Step 3: Deploy
        </h2>

        <h3 className="text-lg font-semibold text-white mb-3">CLI</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`# Deploy the contract (calls init automatically)
dina contract deploy \\
  --wasm target/wasm32-unknown-unknown/release/my_counter_optimized.wasm \\
  --init '{}' \\
  --network testnet

# Output:
# Contract deployed successfully!
# Address: dina1x7k9p...abc123
# TX hash: 0xdef456...789abc
# Gas used: 2,421,000
# Fee: 0.115 USDC
# Block: 148305`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">JavaScript SDK</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`import { DinaClient, DinaWallet } from '@dina-network/sdk';
import { readFileSync } from 'fs';

const client = new DinaClient('https://rpc-testnet.dina.network');
const wallet = DinaWallet.fromKeyFile('./my-wallet.json');

// Read the compiled WASM binary
const wasmBytes = readFileSync(
  './target/wasm32-unknown-unknown/release/my_counter_optimized.wasm'
);

// Deploy
const deployment = await client.deployContract({
  wallet,
  wasm: wasmBytes,
  initArgs: {},              // passed to the "init" method
});

console.log('Contract address:', deployment.address);
console.log('TX hash:', deployment.hash);
console.log('Gas used:', deployment.gasUsed);`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">Python SDK</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`from dina import DinaClient, DinaWallet

client = DinaClient("https://rpc-testnet.dina.network")
wallet = DinaWallet.from_key_file("./my-wallet.json")

# Read the WASM binary
with open("target/wasm32-unknown-unknown/release/my_counter_optimized.wasm", "rb") as f:
    wasm_bytes = f.read()

# Deploy
deployment = client.deploy_contract(
    wallet=wallet,
    wasm=wasm_bytes,
    init_args={},
)

print(f"Contract: {deployment.address}")
print(f"TX: {deployment.hash}")
print(f"Gas: {deployment.gas_used}")`}</code>
        </pre>
      </section>

      {/* Step 4: Verify */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Step 4: Verify Deployment
        </h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`# Query the contract to confirm it's working
dina contract view \\
  --address dina1x7k9p...abc123 \\
  --method get_count \\
  --args '{}' \\
  --network testnet

# Output: 0

# Call increment
dina contract call \\
  --address dina1x7k9p...abc123 \\
  --method increment \\
  --args '{}' \\
  --network testnet

# Query again
dina contract view \\
  --address dina1x7k9p...abc123 \\
  --method get_count \\
  --args '{}' \\
  --network testnet

# Output: 1`}</code>
        </pre>
      </section>

      {/* Contract Structure */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Contract Architecture
        </h2>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Component</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Purpose</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Rules</th>
              </tr>
            </thead>
            <tbody>
              {[
                {
                  component: "State struct",
                  purpose: "All persistent on-chain data",
                  rules: "Must derive Serialize + Deserialize. Use BTreeMap (not HashMap). Use [u8; 32] for addresses.",
                },
                {
                  component: "Methods (impl)",
                  purpose: "Business logic",
                  rules: "Queries take &self (free). Mutations take &mut self (cost gas). Always validate caller.",
                },
                {
                  component: "Arg structs",
                  purpose: "Typed parameters for dispatch",
                  rules: "One struct per method that accepts arguments. Derive Serialize + Deserialize.",
                },
                {
                  component: "dispatch()",
                  purpose: "Entry point / ABI router",
                  rules: "Signature: (state, method, args, caller) -> Vec<u8>. Must handle 'init' method.",
                },
              ].map((row, i) => (
                <tr
                  key={row.component}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 font-mono text-blue-300">{row.component}</td>
                  <td className="px-4 py-3 text-slate-300">{row.purpose}</td>
                  <td className="px-4 py-3 text-slate-400">{row.rules}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Security Checklist */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Security Checklist
        </h2>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
          <ul className="space-y-3">
            {[
              "Validate caller identity on every mutation (assert!(caller == self.owner))",
              "Prevent double-initialization (assert!(state.is_none()) in init)",
              "Use BTreeMap, never HashMap (non-deterministic ordering breaks consensus)",
              "Validate all inputs: lengths, ranges, non-zero amounts",
              "Use checked arithmetic for USDC amounts (checked_add, checked_sub)",
              "Update state before returning transfer instructions (prevent reentrancy by design)",
              "Test error cases with #[should_panic]",
              "Use wasm-opt to strip debug info from production builds",
            ].map((item) => (
              <li key={item} className="flex items-start gap-2 text-sm text-slate-300">
                <span className="mt-0.5 text-green-400">&#10003;</span>
                {item}
              </li>
            ))}
          </ul>
        </div>
      </section>

      {/* Next Steps */}
      <div className="mt-10 flex flex-wrap gap-4">
        <Link
          href="/docs/contracts/call"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Call Contract &rarr;
        </Link>
        <Link
          href="/docs/contracts/wasm"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          WASM Runtime &rarr;
        </Link>
        <Link
          href="/docs/contracts/standards"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          DRC Standards &rarr;
        </Link>
      </div>
    </div>
  );
}
