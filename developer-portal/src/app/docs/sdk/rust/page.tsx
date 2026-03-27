"use client";

import { CodeBlock } from "@/components/code-block";

function H2({ id, children }: { id: string; children: React.ReactNode }) {
  return (
    <h2
      id={id}
      className="mb-4 mt-12 scroll-mt-24 border-b border-slate-800/60 pb-2 text-2xl font-bold tracking-tight"
    >
      {children}
    </h2>
  );
}

function H3({ id, children }: { id: string; children: React.ReactNode }) {
  return (
    <h3
      id={id}
      className="mb-3 mt-8 scroll-mt-24 text-xl font-semibold tracking-tight"
    >
      {children}
    </h3>
  );
}

function Badge({ children }: { children: React.ReactNode }) {
  return (
    <span className="ml-2 rounded-full bg-orange-600/15 px-2 py-0.5 text-xs font-medium text-orange-400">
      {children}
    </span>
  );
}

function TypeRow({ name, description }: { name: string; description: string }) {
  return (
    <div className="my-3 rounded-lg border border-slate-800/60 bg-slate-900/40 p-4">
      <code className="text-sm text-orange-400">{name}</code>
      <p className="mt-1.5 text-sm text-slate-400">{description}</p>
    </div>
  );
}

export default function RustSdkPage() {
  return (
    <>
      {/* Header */}
      <div className="mb-2 flex items-center gap-2 text-sm text-slate-500">
        SDKs
        <span className="text-slate-700">/</span>
        Rust
      </div>
      <h1 className="text-4xl font-extrabold tracking-tight">
        Rust SDK
        <Badge>dina-core</Badge>
      </h1>
      <p className="mt-4 text-lg leading-relaxed text-slate-400">
        The core Rust crate that powers the Dina Network. Use it to embed a Dina
        node, build validators, process blocks, or write WASM smart contracts.
      </p>

      {/* ---- Installation ---- */}
      <H2 id="installation">Installation</H2>
      <p className="mb-4 text-sm text-slate-400">
        Add <code className="text-slate-300">dina-core</code> to your project
        via the GitHub repository:
      </p>

      <CodeBlock
        language="toml"
        filename="Cargo.toml"
        code={`[dependencies]
dina-core = { git = "https://github.com/superbigroach/dina-network" }

# Optional: enable parallel block execution (requires nightly)
# dina-core = { git = "https://github.com/superbigroach/dina-network", features = ["parallel"] }`}
      />

      <p className="mt-4 text-sm text-slate-400">
        Full API documentation is available on{" "}
        <a
          href="https://docs.rs/dina-core"
          target="_blank"
          rel="noopener noreferrer"
          className="text-blue-400 hover:underline"
        >
          docs.rs/dina-core
        </a>
        .
      </p>

      {/* ---- Quick Start ---- */}
      <H2 id="quick-start">Quick Start</H2>

      <CodeBlock
        language="rust"
        filename="main.rs"
        code={`use dina_core::{Address, Transaction, BlockExecutor};
use dina_core::wallet::Wallet;
use dina_core::client::RpcClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate a new wallet
    let wallet = Wallet::generate();
    println!("Address: {}", wallet.address());

    // Connect to testnet
    let client = RpcClient::new("https://rpc.dina.network")?;

    // Check balance
    let balance = client.get_balance(&wallet.address())?;
    println!("Balance: {} USDC", balance);

    // Build and sign a transaction
    let tx = Transaction::builder()
        .to("dina1recipient...".parse()?)
        .amount(10_000_000)  // 10.0 USDC (6 decimals)
        .sign(&wallet)?;

    // Submit
    let hash = client.send_transaction(&tx)?;
    println!("TX Hash: {}", hash);

    // Wait for confirmation
    let receipt = client.wait_for_transaction(&hash, None)?;
    println!("Status: {:?}", receipt.status);

    Ok(())
}`}
      />

      {/* ---- Key Types ---- */}
      <H2 id="key-types">Key Types</H2>
      <p className="mb-4 text-sm text-slate-400">
        The core data types used throughout the Dina Network.
      </p>

      <TypeRow
        name="Address"
        description="A 20-byte account address. Implements Display, FromStr, Serialize/Deserialize. Created from public keys via Address::from_public_key()."
      />
      <TypeRow
        name="Hash"
        description="A 32-byte Blake3 hash. Used for transaction hashes, block hashes, and Merkle roots."
      />
      <TypeRow
        name="Transaction"
        description="A signed transaction with sender, recipient, amount, nonce, data (optional), and Ed25519 signature. Use Transaction::builder() to construct."
      />
      <TypeRow
        name="Block"
        description="A finalized block containing header (number, hash, parent_hash, timestamp, validator), list of transactions, state root, and gas used."
      />
      <TypeRow
        name="Account"
        description="On-chain account state: address, balance (u64 micro-USDC), nonce (u64), optional code_hash for contracts, and storage_root."
      />
      <TypeRow
        name="AccountState"
        description="Mutable account state used during block execution. Provides get_balance(), set_balance(), increment_nonce(), and storage access."
      />

      <CodeBlock
        language="rust"
        filename="types_example.rs"
        code={`use dina_core::{Address, Hash, Transaction, Block, Account};

// Parse an address from a string
let addr: Address = "dina1abc123...".parse()?;

// Create a transaction
let tx = Transaction::builder()
    .to(addr)
    .amount(5_000_000)      // 5 USDC
    .data(b"hello".to_vec()) // optional contract call data
    .nonce(42)
    .sign(&wallet)?;

// Inspect a block
let block: Block = client.get_block(100)?;
println!("Block {} has {} txs", block.header.number, block.transactions.len());
println!("Validator: {}", block.header.validator);
println!("State root: {}", block.header.state_root);

// Access account state
let account: Account = client.get_account(&addr)?;
println!("Balance: {} micro-USDC", account.balance);
println!("Nonce: {}", account.nonce);`}
      />

      {/* ---- Block Execution ---- */}
      <H2 id="block-execution">Block Execution</H2>

      <H3 id="block-executor">BlockExecutor</H3>
      <p className="mb-4 text-sm text-slate-400">
        Sequential block executor. Processes transactions one at a time,
        updating state between each. Good for testing and single-threaded
        environments.
      </p>

      <CodeBlock
        language="rust"
        filename="block_executor.rs"
        code={`use dina_core::execution::{BlockExecutor, ExecutionConfig};
use dina_core::state::StateDb;

// Create a state database
let state = StateDb::open("./data/state")?;

// Configure the executor
let config = ExecutionConfig {
    max_gas_per_block: 10_000_000,
    fee_recipient: validator_address,
    chain_id: "dina-testnet-1".to_string(),
};

// Execute a block
let executor = BlockExecutor::new(state, config);
let result = executor.execute_block(&block)?;

println!("Executed {} txs", result.receipts.len());
println!("Gas used: {}", result.gas_used);
println!("New state root: {}", result.state_root);

// Check individual transaction results
for receipt in &result.receipts {
    match receipt.status {
        TxStatus::Success => println!("  TX {} OK", receipt.tx_hash),
        TxStatus::Failed(ref err) => println!("  TX {} FAILED: {}", receipt.tx_hash, err),
    }
}`}
      />

      <H3 id="parallel-block-executor">ParallelBlockExecutor</H3>
      <p className="mb-4 text-sm text-slate-400">
        Lane-based parallel block executor using Block-STM. Automatically
        detects transaction conflicts and re-executes as needed. Scales linearly
        with CPU cores. Requires the <code className="text-slate-300">parallel</code>{" "}
        feature flag.
      </p>

      <CodeBlock
        language="rust"
        filename="parallel_executor.rs"
        code={`use dina_core::execution::{ParallelBlockExecutor, ParallelConfig};
use dina_core::state::StateDb;

let state = StateDb::open("./data/state")?;

let config = ParallelConfig {
    num_lanes: 8,           // number of parallel execution lanes
    max_retries: 3,         // max conflict retries per tx
    max_gas_per_block: 100_000_000,
    fee_recipient: validator_address,
    chain_id: "dina-mainnet-1".to_string(),
};

let executor = ParallelBlockExecutor::new(state, config);

// Transactions are automatically sorted into non-conflicting lanes
let result = executor.execute_block(&block)?;

println!("Executed {} txs across {} lanes", result.receipts.len(), result.lanes_used);
println!("Conflicts detected: {}", result.conflict_count);
println!("Speedup: {:.1}x vs sequential", result.speedup_factor);`}
      />

      {/* ---- Wallet ---- */}
      <H2 id="wallet">Wallet</H2>

      <CodeBlock
        language="rust"
        filename="wallet.rs"
        code={`use dina_core::wallet::{Wallet, Mnemonic};

// Generate a new wallet with a random keypair
let wallet = Wallet::generate();
println!("Address: {}", wallet.address());
println!("Public key: {}", hex::encode(wallet.public_key()));

// Export and import private key
let private_key = wallet.private_key_bytes();
let restored = Wallet::from_private_key(&private_key)?;
assert_eq!(wallet.address(), restored.address());

// HD wallet from mnemonic
let mnemonic = Mnemonic::generate(12)?; // 12-word phrase
println!("Mnemonic: {}", mnemonic.phrase());

let hd_wallet = Wallet::from_mnemonic(&mnemonic, 0)?;
let hd_wallet_1 = Wallet::from_mnemonic(&mnemonic, 1)?;
assert_ne!(hd_wallet.address(), hd_wallet_1.address());

// Sign and verify
let message = b"Hello Dina";
let signature = wallet.sign(message);
assert!(wallet.verify(message, &signature));`}
      />

      {/* ---- RPC Client ---- */}
      <H2 id="rpc-client">RPC Client</H2>

      <CodeBlock
        language="rust"
        filename="rpc_client.rs"
        code={`use dina_core::client::RpcClient;
use std::time::Duration;

let client = RpcClient::builder()
    .url("https://rpc.dina.network")
    .timeout(Duration::from_secs(10))
    .max_retries(3)
    .build()?;

// Query balance
let balance = client.get_balance(&address)?;
println!("{} micro-USDC", balance);

// Get latest block
let block = client.get_latest_block()?;
println!("Block #{}: {} txs", block.header.number, block.transactions.len());

// Get specific block
let block_100 = client.get_block(100)?;

// Submit a signed transaction
let hash = client.send_transaction(&signed_tx)?;

// Wait for confirmation with timeout
let receipt = client.wait_for_transaction(&hash, Some(Duration::from_secs(30)))?;
println!("Confirmed: {:?}", receipt.status);

// Estimate gas
let gas = client.estimate_gas(&unsigned_tx)?;
println!("Estimated gas: {}", gas);`}
      />

      {/* ---- Smart Contracts ---- */}
      <H2 id="smart-contracts">Smart Contract Development</H2>
      <p className="mb-4 text-sm text-slate-400">
        Dina smart contracts are compiled to WASM. Write them in Rust using the
        contract SDK macros.
      </p>

      <CodeBlock
        language="rust"
        filename="my_contract.rs"
        code={`use dina_core::contract::{contract, storage, msg};

#[contract]
pub struct Counter {
    #[storage]
    count: u64,

    #[storage]
    owner: Address,
}

#[contract]
impl Counter {
    /// Initialize the contract
    #[msg(init)]
    pub fn new(owner: Address) -> Self {
        Counter { count: 0, owner }
    }

    /// Increment the counter (anyone can call)
    #[msg(execute)]
    pub fn increment(&mut self) {
        self.count += 1;
    }

    /// Reset the counter (owner only)
    #[msg(execute)]
    pub fn reset(&mut self, caller: Address) -> Result<(), ContractError> {
        if caller != self.owner {
            return Err(ContractError::Unauthorized);
        }
        self.count = 0;
        Ok(())
    }

    /// Read the current count
    #[msg(query)]
    pub fn get_count(&self) -> u64 {
        self.count
    }
}`}
      />

      <p className="mt-4 text-sm text-slate-400">
        Build the contract to WASM:
      </p>

      <CodeBlock
        language="bash"
        filename="terminal"
        code={`# Build optimized WASM
cargo build --target wasm32-unknown-unknown --release

# The output binary is at:
# target/wasm32-unknown-unknown/release/my_contract.wasm`}
      />

      {/* ---- Testing ---- */}
      <H2 id="testing">Testing</H2>

      <CodeBlock
        language="rust"
        filename="tests.rs"
        code={`#[cfg(test)]
mod tests {
    use dina_core::testing::TestEnv;
    use dina_core::wallet::Wallet;

    #[test]
    fn test_transfer() {
        // TestEnv spins up an in-memory chain
        let mut env = TestEnv::new();

        let alice = Wallet::generate();
        let bob = Wallet::generate();

        // Fund Alice with 1000 USDC
        env.fund(&alice.address(), 1_000_000_000);

        // Transfer 100 USDC
        let receipt = env.transfer(&alice, &bob.address(), 100_000_000).unwrap();
        assert_eq!(receipt.status, TxStatus::Success);

        // Verify balances
        assert_eq!(env.balance(&alice.address()), 899_999_000); // minus fee
        assert_eq!(env.balance(&bob.address()), 100_000_000);
    }

    #[test]
    fn test_parallel_execution() {
        let mut env = TestEnv::with_parallel_lanes(8);

        // Create 1000 non-conflicting transactions
        let wallets: Vec<_> = (0..1000)
            .map(|_| {
                let w = Wallet::generate();
                env.fund(&w.address(), 1_000_000);
                w
            })
            .collect();

        let recipient = Wallet::generate();

        // Submit all at once -- processed in parallel
        let result = env.execute_batch(
            wallets.iter().map(|w| {
                env.build_transfer(w, &recipient.address(), 500_000)
            }).collect()
        ).unwrap();

        assert_eq!(result.receipts.len(), 1000);
        assert!(result.lanes_used > 1);
        println!("Speedup: {:.1}x", result.speedup_factor);
    }
}`}
      />

      {/* ---- Feature Flags ---- */}
      <H2 id="feature-flags">Feature Flags</H2>
      <div className="overflow-x-auto">
        <table className="mt-4 w-full text-sm">
          <thead>
            <tr className="border-b border-slate-800/60 text-left">
              <th className="py-3 pr-6 font-semibold text-slate-300">Feature</th>
              <th className="py-3 pr-6 font-semibold text-slate-300">Default</th>
              <th className="py-3 font-semibold text-slate-300">Description</th>
            </tr>
          </thead>
          <tbody className="text-slate-400">
            <tr className="border-b border-slate-800/40">
              <td className="py-3 pr-6"><code className="text-orange-400">parallel</code></td>
              <td className="py-3 pr-6">off</td>
              <td className="py-3">Enable ParallelBlockExecutor with Block-STM lanes</td>
            </tr>
            <tr className="border-b border-slate-800/40">
              <td className="py-3 pr-6"><code className="text-orange-400">wasm-runtime</code></td>
              <td className="py-3 pr-6">on</td>
              <td className="py-3">WASM smart contract execution engine</td>
            </tr>
            <tr className="border-b border-slate-800/40">
              <td className="py-3 pr-6"><code className="text-orange-400">rpc-client</code></td>
              <td className="py-3 pr-6">on</td>
              <td className="py-3">HTTP/WebSocket RPC client</td>
            </tr>
            <tr className="border-b border-slate-800/40">
              <td className="py-3 pr-6"><code className="text-orange-400">serde</code></td>
              <td className="py-3 pr-6">on</td>
              <td className="py-3">Serialize/Deserialize derives for all types</td>
            </tr>
            <tr className="border-b border-slate-800/40">
              <td className="py-3 pr-6"><code className="text-orange-400">testing</code></td>
              <td className="py-3 pr-6">off</td>
              <td className="py-3">In-memory test environment (TestEnv)</td>
            </tr>
          </tbody>
        </table>
      </div>

      <CodeBlock
        language="toml"
        filename="Cargo.toml"
        code={`# Enable parallel execution and testing helpers
[dependencies]
dina-core = { git = "https://github.com/superbigroach/dina-network", features = ["parallel"] }

[dev-dependencies]
dina-core = { git = "https://github.com/superbigroach/dina-network", features = ["testing"] }`}
      />
    </>
  );
}
