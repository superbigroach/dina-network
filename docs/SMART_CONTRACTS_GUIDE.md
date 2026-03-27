# Dina Network Smart Contracts Guide

This guide walks you through building, testing, and deploying smart contracts on Dina Network. By the end, you will understand the contract model, have built your first contract, and know how to interact with it from the CLI, TypeScript, and Python.

---

## Table of Contents

1. [What Are DRC Contracts?](#what-are-drc-contracts)
2. [Your First Contract](#your-first-contract)
3. [Contract Structure](#contract-structure)
4. [Working with USDC in Contracts](#working-with-usdc-in-contracts)
5. [Testing Your Contract](#testing-your-contract)
6. [Deploying to Testnet](#deploying-to-testnet)
7. [Calling from the CLI](#calling-from-the-cli)
8. [Calling from TypeScript SDK](#calling-from-typescript-sdk)
9. [Calling from Python SDK](#calling-from-python-sdk)
10. [Security Best Practices](#security-best-practices)
11. [Gas Optimization Tips](#gas-optimization-tips)
12. [Available DRC Interfaces](#available-drc-interfaces)

---

## What Are DRC Contracts?

DRC (Dina Request for Comments) contracts are smart contracts that run on the Dina Network blockchain. They are written in Rust, compiled to WebAssembly (WASM), and executed by the Dina VM on every validator node.

Key properties:

- **Deterministic**: Given the same state and input, every node produces the same output. This is why we use `BTreeMap` (ordered) instead of `HashMap` (unordered).
- **Sandboxed**: Contracts run in a WASM sandbox with no access to the filesystem, network, or system clock. Time is provided as a parameter.
- **USDC-native**: Dina uses USDC as its native payment token. Contracts can hold, transfer, and escrow USDC natively.
- **State-per-contract**: Each deployed contract has its own isolated state. State is serialized as JSON between calls.

### How contracts execute

```
Caller --> Transaction --> Dina VM --> dispatch(state, method, args, caller) --> Updated State
```

1. A user or agent submits a signed transaction specifying a contract address, method name, and arguments.
2. The Dina VM loads the contract's current state from storage.
3. The VM calls your `dispatch` function with the state, method name, args (as JSON bytes), and the caller's address.
4. Your dispatch function routes to the appropriate method, modifies state, and returns a JSON response.
5. The VM saves the updated state back to storage.

For read-only "view" calls, the same flow applies but no state is persisted and no gas is charged.

---

## Your First Contract

Let's build the simplest possible contract: a greeting that the owner can update.

### Step 1: Create the project

```bash
mkdir -p examples/hello-world/src
```

Create `examples/hello-world/Cargo.toml`:

```toml
[package]
name = "example-hello-world"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
borsh = { workspace = true }

[lib]
crate-type = ["cdylib", "lib"]
```

The `crate-type = ["cdylib", "lib"]` line is important:
- `cdylib` produces the `.wasm` file that gets deployed on-chain
- `lib` allows other Rust code (like tests) to import your contract

### Step 2: Define state

Every contract needs a state struct that holds all on-chain data. It must derive `Serialize` and `Deserialize`.

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HelloWorldState {
    pub greeting: String,
    pub owner: [u8; 32],
}
```

Addresses on Dina are 32-byte ed25519 public keys, represented as `[u8; 32]`.

### Step 3: Implement methods

```rust
impl HelloWorldState {
    pub fn new(greeting: String, owner: [u8; 32]) -> Self {
        Self { greeting, owner }
    }

    pub fn get_greeting(&self) -> &str {
        &self.greeting
    }

    pub fn owner(&self) -> &[u8; 32] {
        &self.owner
    }

    pub fn set_greeting(&mut self, caller: [u8; 32], new_greeting: String) {
        assert!(caller == self.owner, "HelloWorld: only the owner can change the greeting");
        assert!(!new_greeting.is_empty(), "HelloWorld: greeting cannot be empty");
        self.greeting = new_greeting;
    }
}
```

Methods that read state take `&self`. Methods that modify state take `&mut self`. The `caller` parameter enables access control.

### Step 4: Write the dispatch function

```rust
#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    greeting: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetGreetingArgs {
    new_greeting: String,
}

pub fn dispatch(
    state: &mut Option<HelloWorldState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "HelloWorld: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("bad init args");
            *state = Some(HelloWorldState::new(a.greeting, caller));
            serde_json::to_vec("ok").unwrap()
        }
        "get_greeting" => {
            let s = state.as_ref().expect("not initialised");
            serde_json::to_vec(s.get_greeting()).unwrap()
        }
        "owner" => {
            let s = state.as_ref().expect("not initialised");
            serde_json::to_vec(s.owner()).unwrap()
        }
        "set_greeting" => {
            let s = state.as_mut().expect("not initialised");
            let a: SetGreetingArgs = serde_json::from_slice(args).expect("bad args");
            s.set_greeting(caller, a.new_greeting);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("unknown method '{method}'"),
    }
}
```

The dispatch function is a match statement that routes method names to implementations. This is the contract's ABI.

### Step 5: Add to workspace

Add the example to your workspace `Cargo.toml`:

```toml
[workspace]
members = [
    # ... existing members ...
    "examples/hello-world",
]
```

### Step 6: Build

```bash
cargo build -p example-hello-world --target wasm32-unknown-unknown --release
```

The compiled WASM file will be at:
```
target/wasm32-unknown-unknown/release/example_hello_world.wasm
```

---

## Contract Structure

Every Dina contract follows the same four-part structure:

### 1. State struct

The state struct holds all persistent data. Rules:

- Must derive `Serialize` and `Deserialize`
- Use `BTreeMap` instead of `HashMap` for deterministic ordering
- Use `[u8; 32]` for addresses
- Use `u64` for USDC amounts (6 decimal places, so 1 USDC = 1_000_000)
- Keep state as flat as possible for gas efficiency

### 2. Methods (impl block)

Implement your business logic as methods on the state struct:

- **Queries** (`&self`): Read-only methods. Free to call. No gas.
- **Mutations** (`&mut self`): State-changing methods. Cost gas. Always take a `caller: [u8; 32]` parameter for access control.

### 3. Dispatch argument structs

Each method that accepts arguments needs a corresponding struct with `Serialize` and `Deserialize`. These are deserialized from the JSON bytes passed to `dispatch()`.

### 4. Dispatch function

The entry point. Signature is always:

```rust
pub fn dispatch(
    state: &mut Option<YourState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8>
```

- `state` is `None` before `init` is called, `Some(...)` after
- `method` is the method name string
- `args` is JSON bytes
- `caller` is the transaction signer's address
- Returns JSON bytes

---

## Working with USDC in Contracts

Dina Network uses USDC as its native payment token. USDC amounts use 6 decimal places:

| Human Amount | Contract Value |
|-------------|---------------|
| 1.00 USDC   | 1_000_000     |
| 0.50 USDC   | 500_000       |
| 100.00 USDC | 100_000_000   |

### Accepting USDC in a transaction

When a user attaches USDC to a transaction, your dispatch receives it as a parameter in the args. Typically you'd include a `usdc_attached` field:

```rust
#[derive(Serialize, Deserialize)]
struct FundArgs {
    deal_id: u64,
    usdc_attached: u64,  // USDC sent with this transaction
}
```

### Holding USDC in escrow

Store the escrowed amount in your state. The USDC is implicitly held by the contract:

```rust
pub struct Deal {
    pub amount: u64,        // USDC locked in escrow
    pub status: DealStatus, // tracks whether funds are locked or released
}
```

### Releasing USDC

Return the transfer details from your dispatch function so the VM can execute the transfer:

```rust
"confirm_delivery" => {
    let (amount, recipient) = s.confirm_delivery(caller, deal_id);
    // The VM reads this result and executes the USDC transfer
    serde_json::to_vec(&TransferResult { amount, to: recipient }).unwrap()
}
```

See the `examples/escrow` contract for a complete USDC escrow implementation.

---

## Testing Your Contract

Testing Dina contracts is just testing regular Rust code. Call the `dispatch` function directly with different inputs and assert the outputs.

### Basic test structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create deterministic test addresses
    fn addr(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    #[test]
    fn test_init_and_query() {
        // Start with no state (fresh contract)
        let mut state: Option<YourState> = None;
        let owner = addr(1);

        // Initialize
        let args = serde_json::to_vec(&InitArgs { /* ... */ }).unwrap();
        let result = dispatch(&mut state, "init", &args, owner);
        assert_eq!(result, serde_json::to_vec("ok").unwrap());

        // Query
        let result = dispatch(&mut state, "some_query", b"{}", owner);
        let value: String = serde_json::from_slice(&result).unwrap();
        assert_eq!(value, "expected");
    }
}
```

### Testing error cases

Use `#[should_panic]` to test that invalid operations are rejected:

```rust
#[test]
#[should_panic(expected = "only the owner")]
fn test_unauthorized_access() {
    let mut state: Option<YourState> = None;
    let owner = addr(1);
    let attacker = addr(99);

    // ... init with owner ...

    // Attacker tries a privileged operation
    dispatch(&mut state, "admin_method", b"{}", attacker);
}
```

### Running tests

```bash
# Test a specific example
cargo test -p example-hello-world

# Test all examples
cargo test -p example-hello-world -p example-escrow -p example-voting \
           -p example-subscription -p example-marketplace

# With output
cargo test -p example-hello-world -- --nocapture
```

---

## Deploying to Testnet

### Prerequisites

1. Install the Dina CLI: see the main README
2. Create a wallet: `dina wallet create`
3. Get testnet USDC from the faucet: `dina faucet request`

### Build for WASM

```bash
cargo build -p example-hello-world --target wasm32-unknown-unknown --release
```

### Deploy

```bash
dina contract deploy \
  --wasm target/wasm32-unknown-unknown/release/example_hello_world.wasm \
  --init '{"greeting": "Hello, Dina Network!"}' \
  --network testnet
```

The CLI will output the contract address:

```
Contract deployed at: dina1abc123...xyz
Transaction hash: 0xdef456...
```

Save the contract address. You will need it for all subsequent calls.

---

## Calling from the CLI

### State-changing calls (mutations)

```bash
# Change the greeting (requires signing)
dina contract call \
  --address dina1abc123...xyz \
  --method set_greeting \
  --args '{"new_greeting": "Greetings from the Machine Economy!"}' \
  --network testnet
```

### Read-only calls (views)

```bash
# Read the greeting (free, no signing needed)
dina contract view \
  --address dina1abc123...xyz \
  --method get_greeting \
  --args '{}' \
  --network testnet
```

Output:
```json
"Greetings from the Machine Economy!"
```

### Calls with USDC attached

```bash
# Fund an escrow deal with 10 USDC
dina contract call \
  --address dina1escrow...xyz \
  --method fund_deal \
  --args '{"deal_id": 1, "usdc_attached": 10000000}' \
  --usdc 10.0 \
  --network testnet
```

---

## Calling from TypeScript SDK

Install the SDK:

```bash
npm install @dina-network/sdk
```

### Basic usage

```typescript
import { DinaClient, DinaContract, DinaWallet } from '@dina-network/sdk';

// Connect to testnet
const client = new DinaClient('https://rpc-testnet.dina.network');

// Load your wallet from a key file
const wallet = DinaWallet.fromKeyFile('./my-wallet.json');

// Create a contract instance
const contract = new DinaContract('dina1abc123...xyz', client);

// -- View call (free, no wallet needed) --
const greeting = await contract.view('get_greeting', {});
console.log('Current greeting:', greeting);
// => "Hello, Dina Network!"

// -- State-changing call (requires wallet) --
const txHash = await contract.call(
  'set_greeting',
  { new_greeting: 'Hello from TypeScript!' },
  wallet
);
console.log('Transaction:', txHash);

// -- Call with USDC attached --
const hireTx = await contract.call(
  'hire',
  { listing_id: 1, usdc_attached: 2000000 },
  wallet,
  BigInt(2_000_000) // attach 2 USDC
);
```

### Using typed contract wrappers

For standard DRC contracts, the SDK provides typed helpers:

```typescript
import { DinaContract } from '@dina-network/sdk';

// DRC-1 Token
const token = DinaContract.token('dina1token...xyz', client);
const balance = await token.balanceOf('dina1myaddr...');
await token.transfer(wallet, 'dina1recipient...', BigInt(5_000_000));

// DRC-101 Agent Wallet
const agentWallet = DinaContract.agentWallet('dina1agent...xyz', client);
const stats = await agentWallet.spendingStats();
console.log('Daily spent:', stats.dailySpent);
```

### Building a custom typed wrapper

For your own contracts, extend `DinaContract`:

```typescript
import { DinaClient, DinaContract, DinaWallet, Address, Hash } from '@dina-network/sdk';

class EscrowContract extends DinaContract {
  constructor(address: Address, client: DinaClient) {
    super(address, client);
  }

  async createDeal(
    wallet: DinaWallet,
    seller: Address,
    amount: bigint,
    description: string
  ): Promise<number> {
    const result = await this.call(
      'create_deal',
      { seller, amount: amount.toString(), description },
      wallet
    );
    // The CLI or SDK will parse the result
    return result as unknown as number;
  }

  async getDeal(dealId: number): Promise<Deal | null> {
    return this.view('get_deal', { deal_id: dealId }) as Promise<Deal | null>;
  }
}
```

---

## Calling from Python SDK

Install the SDK:

```bash
pip install dina-sdk
```

### Basic usage

```python
from dina import DinaClient, DinaWallet

# Connect to testnet
client = DinaClient("https://rpc-testnet.dina.network")

# Load wallet
wallet = DinaWallet.from_key_file("./my-wallet.json")

# View call (free)
greeting = client.view_contract(
    contract="dina1abc123...xyz",
    method="get_greeting",
    args={}
)
print(f"Greeting: {greeting}")

# State-changing call
tx = client.call_contract(
    wallet=wallet,
    contract="dina1abc123...xyz",
    method="set_greeting",
    args={"new_greeting": "Hello from Python!"}
)
print(f"Transaction: {tx.hash}")

# Call with USDC
tx = client.call_contract(
    wallet=wallet,
    contract="dina1escrow...xyz",
    method="fund_deal",
    args={"deal_id": 1, "usdc_attached": 10_000_000},
    usdc_attached=10_000_000
)
```

### Batch operations

```python
# Read multiple listings efficiently
for listing_id in range(1, 11):
    listing = client.view_contract(
        contract="dina1market...xyz",
        method="get_listing",
        args={"listing_id": listing_id}
    )
    if listing:
        print(f"Listing {listing_id}: {listing['service_type']} @ {listing['price']} USDC")
```

---

## Security Best Practices

### 1. Always validate the caller

Every mutation should check `caller` against an authorized address:

```rust
assert!(caller == self.owner, "unauthorized");
```

### 2. Validate all inputs

Never trust user input. Check lengths, ranges, and invariants:

```rust
assert!(amount > 0, "amount must be positive");
assert!(!description.is_empty(), "description required");
assert!(option_index < self.options.len(), "invalid option");
```

### 3. Use assert! for all invariants

`assert!` panics and rolls back the entire transaction on failure. This is safer than returning error codes, which callers might forget to check:

```rust
// GOOD: transaction is rolled back on failure
assert!(balance >= amount, "insufficient balance");

// AVOID: caller might ignore the error
if balance < amount {
    return serde_json::to_vec(&"error").unwrap();
}
```

### 4. Prevent re-entrancy by design

Dina contracts are single-threaded and do not support cross-contract calls within a single transaction. This eliminates the re-entrancy class of bugs by design. Update your state before returning transfer instructions:

```rust
// GOOD: state updated before returning transfer info
deal.status = DealStatus::Completed;  // update state first
(deal.amount, deal.seller)            // then return transfer info
```

### 5. Prevent double-initialization

Always check `state.is_none()` in your `init` handler:

```rust
"init" => {
    assert!(state.is_none(), "already initialised");
    // ...
}
```

### 6. Use BTreeMap, never HashMap

`HashMap` iteration order is non-deterministic, which breaks consensus. Always use `BTreeMap`:

```rust
// GOOD
pub votes: BTreeMap<[u8; 32], usize>,

// BAD — will cause consensus failures
pub votes: HashMap<[u8; 32], usize>,
```

### 7. Avoid integer overflow

For USDC amounts, be aware of u64 overflow. Maximum u64 is approximately 18.4 billion USDC, which is safe for most applications. For extremely large amounts, add overflow checks:

```rust
let new_balance = balance.checked_add(amount).expect("overflow");
```

---

## Gas Optimization Tips

### 1. Keep state small

Gas is proportional to the size of serialized state. Remove unnecessary fields and use compact types:

```rust
// GOOD: compact
pub status: u8,

// EXPENSIVE: large string
pub status: String,
```

### 2. Use indexes for lookups

Instead of scanning all entries, maintain reverse indexes:

```rust
// O(1) lookup by type
pub type_index: BTreeMap<String, Vec<ListingId>>,
```

### 3. Minimize serialization

Return only the data the caller needs, not the entire state:

```rust
// GOOD: return only the balance
serde_json::to_vec(&self.balance_of(&account)).unwrap()

// EXPENSIVE: return everything
serde_json::to_vec(&self).unwrap()
```

### 4. Batch operations when possible

If your contract supports batch operations, process multiple items in a single transaction to save gas:

```rust
pub fn batch_transfer(&mut self, caller: [u8; 32], transfers: Vec<(Address, u64)>) {
    for (to, amount) in transfers {
        self.transfer(caller, to, amount);
    }
}
```

### 5. Use u64 instead of String for enums in hot paths

Enum variants serialize as strings in JSON. If you have a high-frequency field, consider using numeric constants for internal storage.

---

## Available DRC Interfaces

| DRC | Name | Purpose | Example Use |
|-----|------|---------|-------------|
| DRC-1 | Fungible Token | ERC-20 equivalent token standard | USDC wrapper, loyalty points |
| DRC-2 | Device Identity | On-chain device registration | Robot identity, sensor auth |
| DRC-4 | Permit | Gasless approvals | Delegated spending |
| DRC-5 | Soulbound | Non-transferable tokens | Certifications, badges |
| DRC-6 | NFT | Non-fungible tokens | Unique device certificates |
| DRC-7 | Multi-Token | ERC-1155 equivalent | Mixed fungible/NFT collections |
| DRC-8 | Token-Bound | Token-bound accounts | NFTs that own assets |
| DRC-9 | Rental | Asset rental | Equipment leasing |
| DRC-10 | Royalties | Creator royalties | IP licensing payments |
| DRC-11 | Semi-Fungible | Semi-fungible tokens | Batch manufacturing IDs |
| DRC-12 | Vault | Asset vaults | Pooled staking |
| DRC-13 | Compliant | Compliance controls | KYC-gated transfers |
| DRC-14 | Contract Sig | Contract signatures | Multi-sig governance |
| DRC-15 | Meta-TX | Meta transactions | Gasless agent operations |
| DRC-16 | Proxy | Upgradeable proxy | Contract upgrades |
| DRC-17 | Hooks | Lifecycle hooks | Pre/post transfer logic |
| DRC-18 | Scriptable | Programmable logic | Custom automation rules |
| DRC-101 | Agent Wallet | AI agent spending wallet | Daily limits, emergency stop |
| DRC-102 | Capability | Machine capability registry | Service discovery |
| DRC-103 | Service Agreement | Escrow-based agreements | Agent-to-agent deals |
| DRC-104 | Swarm | Swarm coordination | Multi-agent task assignment |
| DRC-105 | Sensor Attestation | Verified sensor data | IoT data provenance |
| DRC-106 | Data Market | Data marketplace | Sensor data trading |
| DRC-107 | Reputation | On-chain reputation | Trust scores |
| DRC-108 | Resource | Resource management | Compute/storage allocation |
| DRC-109 | Emergency Stop | Circuit breaker | Fleet-wide halt |
| DRC-110 | Firmware | Firmware management | OTA update tracking |
| DRC-111 | Smart Wallet | Advanced wallet | Multi-sig + policies |
| DRC-112 | View Keys | Privacy views | Selective disclosure |
| DRC-113 | Relay | Cross-chain relay | Bridge messages |

---

## Next Steps

- Browse the example contracts in `examples/` for complete, working implementations
- Read the source code of the DRC standards in `contracts/` for production-grade patterns
- Join the Dina developer community for help and discussion
- Deploy your first contract to testnet and start building
