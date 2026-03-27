import Link from "next/link";

export const metadata = {
  title: "WASM Runtime — Dina Network Developer Portal",
  description:
    "How Dina Network executes WASM smart contracts: host functions, memory model, gas metering, and example contract.",
};

const HOST_FUNCTIONS = [
  { name: "dina_log", signature: "(ptr: i32, len: i32)", description: "Write a UTF-8 string to the node log (debug only, stripped in production)" },
  { name: "dina_get_caller", signature: "() -> i32", description: "Returns pointer to 32-byte caller address in linear memory" },
  { name: "dina_get_block_height", signature: "() -> i64", description: "Current block height at execution time" },
  { name: "dina_get_block_time", signature: "() -> i64", description: "Block timestamp in milliseconds since epoch" },
  { name: "dina_get_balance", signature: "(addr_ptr: i32) -> i64", description: "USDC balance of an address (6 decimal places)" },
  { name: "dina_transfer", signature: "(to_ptr: i32, amount: i64) -> i32", description: "Transfer USDC from contract to address. Returns 0 on success." },
  { name: "dina_state_read", signature: "(key_ptr: i32, key_len: i32, buf_ptr: i32, buf_len: i32) -> i32", description: "Read state bytes by key. Returns bytes read." },
  { name: "dina_state_write", signature: "(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32) -> i32", description: "Write state bytes by key. Returns 0 on success." },
  { name: "dina_state_delete", signature: "(key_ptr: i32, key_len: i32) -> i32", description: "Delete a state key. Returns 0 on success." },
  { name: "dina_emit_event", signature: "(name_ptr: i32, name_len: i32, data_ptr: i32, data_len: i32)", description: "Emit a named event with JSON data payload" },
  { name: "dina_sha256", signature: "(data_ptr: i32, data_len: i32, out_ptr: i32)", description: "Compute SHA-256 hash, write 32 bytes to out_ptr" },
  { name: "dina_ed25519_verify", signature: "(msg_ptr: i32, msg_len: i32, sig_ptr: i32, pub_ptr: i32) -> i32", description: "Verify Ed25519 signature. Returns 1 if valid." },
  { name: "dina_abort", signature: "(msg_ptr: i32, msg_len: i32)", description: "Abort execution with an error message (rollback)" },
];

const GAS_COSTS = [
  { operation: "Memory allocation (per page, 64 KB)", gas: "1,000" },
  { operation: "State read (per call)", gas: "500" },
  { operation: "State write (per call)", gas: "2,000" },
  { operation: "State delete (per call)", gas: "500" },
  { operation: "SHA-256 hash", gas: "200" },
  { operation: "Ed25519 verify", gas: "3,000" },
  { operation: "Event emission", gas: "500" },
  { operation: "USDC transfer from contract", gas: "5,000" },
  { operation: "Log (debug, testnet only)", gas: "100" },
  { operation: "WASM instruction (per instruction)", gas: "1" },
];

export default function WasmRuntimePage() {
  return (
    <div>
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Smart Contracts
      </p>
      <h1 className="text-4xl font-bold tracking-tight text-white mb-4">
        WASM Runtime
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-10">
        Dina Network uses WebAssembly (WASM) as its smart contract execution
        engine -- not the EVM. Contracts are written in Rust, compiled to
        <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300 ml-1">
          wasm32-unknown-unknown
        </code>
        , and executed deterministically on every validator node. This gives you
        Rust&apos;s type safety, performance, and ecosystem while running in a
        sandboxed environment.
      </p>

      {/* Why WASM */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Why WASM Instead of EVM?
        </h2>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Property</th>
                <th className="px-4 py-3 text-left font-medium text-blue-400">Dina (WASM)</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Ethereum (EVM)</th>
              </tr>
            </thead>
            <tbody>
              {[
                { prop: "Language", wasm: "Rust (with full stdlib)", evm: "Solidity / Vyper" },
                { prop: "Execution speed", wasm: "Near-native (5-25x faster)", evm: "Interpreted bytecode" },
                { prop: "Type safety", wasm: "Compile-time (Rust borrow checker)", evm: "Runtime checks" },
                { prop: "Binary format", wasm: ".wasm (industry standard)", evm: "EVM bytecode (proprietary)" },
                { prop: "Determinism", wasm: "Guaranteed (no floats, no randomness)", evm: "Guaranteed" },
                { prop: "Memory model", wasm: "Linear memory (grow-only)", evm: "256-bit word stack" },
                { prop: "Tooling", wasm: "cargo, rustfmt, clippy, IDE support", evm: "Foundry, Hardhat" },
                { prop: "Testing", wasm: "Native Rust tests (cargo test)", evm: "JS/TS test frameworks" },
                { prop: "Dependencies", wasm: "Full crates.io ecosystem (no_std)", evm: "OpenZeppelin only" },
              ].map((row, i) => (
                <tr
                  key={row.prop}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 font-medium text-slate-300">{row.prop}</td>
                  <td className="px-4 py-3 text-blue-300">{row.wasm}</td>
                  <td className="px-4 py-3 text-slate-400">{row.evm}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Contract Execution Flow */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Execution Flow
        </h2>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
          <pre className="overflow-x-auto text-sm text-slate-300 leading-relaxed">
{`Transaction arrives
    |
    v
Validator loads contract WASM binary from storage
    |
    v
WASM runtime instantiates the module with host functions
    |
    v
Runtime calls dispatch(state, method, args, caller)
    |
    v
Contract executes, calling host functions as needed
    |
    v
Gas is metered per WASM instruction + host function calls
    |
    v
If gas exhausted -> abort, rollback all state changes
If success -> persist updated state, emit events, return result`}
          </pre>
        </div>
      </section>

      {/* Host Functions */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Available Host Functions
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Host functions are the interface between your WASM contract and the
          Dina blockchain. They are the only way for contracts to interact
          with the outside world. The WASM sandbox has no access to
          filesystem, network, or system clock.
        </p>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Function</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Signature</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Description</th>
              </tr>
            </thead>
            <tbody>
              {HOST_FUNCTIONS.map((fn, i) => (
                <tr
                  key={fn.name}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 font-mono text-blue-300 whitespace-nowrap">{fn.name}</td>
                  <td className="px-4 py-3 font-mono text-slate-400 text-xs whitespace-nowrap">{fn.signature}</td>
                  <td className="px-4 py-3 text-slate-400">{fn.description}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Memory Model */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Memory Model
        </h2>
        <div className="space-y-4">
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-white mb-2">Linear Memory</h3>
            <p className="text-sm text-slate-400 leading-relaxed mb-3">
              WASM contracts use a single contiguous block of linear memory.
              Memory starts at 1 page (64 KB) and can grow up to a maximum of
              256 pages (16 MB). Each page growth costs 1,000 gas.
            </p>
            <ul className="space-y-2">
              {[
                "Initial memory: 1 page (64 KB)",
                "Maximum memory: 256 pages (16 MB)",
                "Growth: memory.grow(n) allocates n additional pages",
                "Memory is zeroed on allocation",
                "Memory is NOT persisted between calls -- only state is persisted",
              ].map((item) => (
                <li key={item} className="flex items-start gap-2 text-sm text-slate-300">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-blue-500" />
                  {item}
                </li>
              ))}
            </ul>
          </div>
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-white mb-2">
              State Persistence
            </h3>
            <p className="text-sm text-slate-400 leading-relaxed">
              Contract state is stored as a JSON-serialized blob associated with
              the contract address. The{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">dispatch</code>{" "}
              function receives the current state as input and returns the updated
              state as output. The runtime handles serialization and persistence
              automatically. For advanced use cases, you can also use the{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">dina_state_read</code>{" "}
              and{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">dina_state_write</code>{" "}
              host functions for key-value storage.
            </p>
          </div>
        </div>
      </section>

      {/* Gas Metering */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Gas Metering
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Every WASM instruction and host function call costs gas. If a
          contract exhausts its gas budget, execution is aborted and all state
          changes are rolled back. The transaction fee is still consumed.
        </p>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Operation</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Gas Cost</th>
              </tr>
            </thead>
            <tbody>
              {GAS_COSTS.map((row, i) => (
                <tr
                  key={row.operation}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 text-slate-300">{row.operation}</td>
                  <td className="px-4 py-3 font-mono text-blue-300">{row.gas}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="mt-4 rounded-xl border border-amber-500/30 bg-amber-500/5 p-4">
          <p className="text-sm text-amber-300">
            <strong>Note:</strong> View calls (read-only queries) are not
            metered and do not consume gas. Gas is only charged for mutations
            that are included in a block.
          </p>
        </div>
      </section>

      {/* Determinism Rules */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Determinism Rules
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          All validators must produce identical execution results for
          consensus to work. The WASM sandbox enforces determinism:
        </p>
        <div className="space-y-3">
          {[
            {
              rule: "No floating point",
              desc: "WASM float operations (f32, f64) are disabled. Use integer arithmetic with fixed-point scaling.",
            },
            {
              rule: "No randomness",
              desc: "There is no random number generator. Use block hash or VRF oracle for pseudo-randomness.",
            },
            {
              rule: "No system clock",
              desc: "Use dina_get_block_time() for timestamps. The system clock is not accessible.",
            },
            {
              rule: "No filesystem or network",
              desc: "Contracts cannot access files, sockets, or HTTP. All external data must be passed as arguments.",
            },
            {
              rule: "Ordered collections only",
              desc: "Use BTreeMap/BTreeSet, never HashMap/HashSet. Unordered iteration breaks consensus.",
            },
            {
              rule: "Canonical serialization",
              desc: "serde_json produces deterministic output with sorted keys. Do not use custom serializers.",
            },
          ].map((item) => (
            <div
              key={item.rule}
              className="rounded-lg border border-slate-800 bg-slate-900/30 p-4"
            >
              <h3 className="text-sm font-semibold text-white">{item.rule}</h3>
              <p className="text-sm text-slate-400 mt-1">{item.desc}</p>
            </div>
          ))}
        </div>
      </section>

      {/* Minimal Example */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Example: Minimal Contract
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          The smallest possible contract that compiles and deploys. It stores
          a single value that anyone can get or set.
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Storage {
    pub value: u64,
}

pub fn dispatch(
    state: &mut Option<Storage>,
    method: &str,
    args: &[u8],
    _caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "already initialized");
            *state = Some(Storage { value: 0 });
            serde_json::to_vec("ok").unwrap()
        }
        "get" => {
            let s = state.as_ref().expect("not initialized");
            serde_json::to_vec(&s.value).unwrap()
        }
        "set" => {
            let s = state.as_mut().expect("not initialized");
            #[derive(Deserialize)]
            struct Args { value: u64 }
            let a: Args = serde_json::from_slice(args).expect("bad args");
            s.value = a.value;
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("unknown method '{method}'"),
    }
}`}</code>
        </pre>
        <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/50 p-4">
          <h3 className="text-sm font-semibold text-white mb-2">Build and Deploy</h3>
          <pre className="overflow-x-auto text-sm text-slate-300">
{`cargo build --target wasm32-unknown-unknown --release
wasm-opt -Oz target/wasm32-unknown-unknown/release/my_storage.wasm -o storage.wasm
dina contract deploy --wasm storage.wasm --init '{}' --network testnet`}
          </pre>
        </div>
      </section>

      {/* Advanced: Using Host Functions */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Using Host Functions Directly
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Most contracts use the high-level dispatch pattern (state is passed
          in and out automatically). For advanced use cases, you can call host
          functions directly via the{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">dina-sdk</code>{" "}
          crate:
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`use dina_sdk::host;

pub fn dispatch(
    state: &mut Option<MyState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "transfer_from_contract" => {
            let s = state.as_mut().expect("not initialized");
            assert!(caller == s.owner, "unauthorized");

            #[derive(serde::Deserialize)]
            struct Args { to: [u8; 32], amount: u64 }
            let a: Args = serde_json::from_slice(args).expect("bad args");

            // Use host function to transfer USDC from contract balance
            let result = host::transfer(&a.to, a.amount);
            assert!(result == 0, "transfer failed");

            // Emit an event
            host::emit_event("ContractTransfer", &serde_json::json!({
                "to": hex::encode(a.to),
                "amount": a.amount,
            }));

            // Log for debugging (testnet only)
            host::log(&format!("Transferred {} to {:?}", a.amount, a.to));

            serde_json::to_vec("ok").unwrap()
        }
        "check_block" => {
            let height = host::get_block_height();
            let time = host::get_block_time();
            serde_json::to_vec(&serde_json::json!({
                "height": height,
                "time": time,
            })).unwrap()
        }
        "verify_sig" => {
            #[derive(serde::Deserialize)]
            struct Args {
                message: Vec<u8>,
                signature: [u8; 64],
                pubkey: [u8; 32],
            }
            let a: Args = serde_json::from_slice(args).expect("bad args");
            let valid = host::ed25519_verify(&a.message, &a.signature, &a.pubkey);
            serde_json::to_vec(&valid).unwrap()
        }
        _ => panic!("unknown method '{method}'"),
    }
}`}</code>
        </pre>
      </section>

      {/* Size Optimization */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Binary Size Optimization
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Smaller WASM binaries cost less to deploy (21,000 + 200/byte gas).
          Here are techniques to minimize binary size:
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-4">
          <code className="text-slate-200">{`# Cargo.toml -- add these for release builds
[profile.release]
opt-level = "z"          # Optimize for size
lto = true               # Link-time optimization
codegen-units = 1        # Single codegen unit for better optimization
strip = true             # Strip debug symbols
panic = "abort"          # Smaller panic handler`}</code>
        </pre>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`# Post-build optimization with wasm-opt
wasm-opt -Oz input.wasm -o output.wasm

# Typical size reductions:
# Before wasm-opt:  45 KB
# After wasm-opt:   12 KB  (73% smaller)
# Deploy savings:   ~6,600 gas (~0.31 USDC)`}</code>
        </pre>
      </section>

      {/* Next Steps */}
      <div className="mt-10 flex flex-wrap gap-4">
        <Link
          href="/docs/contracts/deploy"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          &larr; Deploy Contract
        </Link>
        <Link
          href="/docs/contracts/standards"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          DRC Standards &rarr;
        </Link>
        <Link
          href="/docs/contracts/call"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Call Contract &rarr;
        </Link>
      </div>
    </div>
  );
}
