# Developer Guide

## Overview

This guide covers everything you need to build on the Dina Network: setting up your environment, deploying smart contracts, using the SDKs, and integrating advanced features like payment channels, privacy, and Cognitum Seed hardware.

## Prerequisites

### Required

- **Rust** 1.70+ with `wasm32-unknown-unknown` target
- **Git** for cloning the repository

### Install Rust

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Windows
# Download and run rustup-init.exe from https://rustup.rs
```

### Install WASM Target

```bash
rustup target add wasm32-unknown-unknown
```

### Windows-Specific Setup

The codebase requires the GNU toolchain on Windows:

```powershell
# Install MSYS2, then in MSYS2 shell:
pacman -S mingw-w64-x86_64-toolchain

# Add to PATH
$env:PATH += ";C:\msys64\mingw64\bin"

# Verify
dlltool --version
```

## Building from Source

```bash
git clone https://github.com/lucilla-app/dina_network.git
cd dina_network

# Build everything
cargo build

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Build only the node
cargo build --bin dina-node

# Build only the CLI
cargo build --bin dina

# Build a specific contract to WASM
cargo build -p drc1-token --target wasm32-unknown-unknown
```

## Running a Local Testnet

### Single Validator

The simplest way to get a local chain running:

```bash
cargo run --bin dina-node -- --validator --data-dir ./data
```

This starts:
- A validator node with auto-generated Ed25519 identity
- JSON-RPC server on `http://127.0.0.1:8545`
- REST API server on `http://0.0.0.0:8080`
- P2P listener on `/ip4/0.0.0.0/tcp/9944`
- Default testnet genesis with a faucet account holding 1 billion USDC

### Multi-Validator Local Testnet

```bash
# Terminal 1: Validator A
cargo run --bin dina-node -- \
  --validator \
  --data-dir ./data-a \
  --rpc-port 8545 \
  --rest-port 8080 \
  --listen /ip4/127.0.0.1/tcp/9944

# Terminal 2: Validator B
cargo run --bin dina-node -- \
  --validator \
  --data-dir ./data-b \
  --rpc-port 8546 \
  --rest-port 8081 \
  --listen /ip4/127.0.0.1/tcp/9945 \
  --bootstrap /ip4/127.0.0.1/tcp/9944

# Terminal 3: Validator C
cargo run --bin dina-node -- \
  --validator \
  --data-dir ./data-c \
  --rpc-port 8547 \
  --rest-port 8082 \
  --listen /ip4/127.0.0.1/tcp/9946 \
  --bootstrap /ip4/127.0.0.1/tcp/9944
```

### Docker Compose

```bash
docker-compose up
```

The `docker-compose.yml` in the project root starts a 3-validator testnet.

### Verify the Network is Running

```bash
# Health check
curl http://localhost:8080/health

# Get network info
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"dina_networkInfo","params":[],"id":1}'

# Check balance of the faucet
curl http://localhost:8080/v1/balance/0000000000000000000000000000000000000000000000000000000000000001
```

## SDK Quickstart

### TypeScript SDK (dina-js)

```bash
cd sdk/dina-js
npm install
```

```typescript
import { DinaClient, Wallet } from 'dina-js';

// Connect to local testnet
const client = new DinaClient('http://localhost:8545');

// Generate a wallet
const wallet = Wallet.generate();
console.log('Address:', wallet.address);

// Check balance
const balance = await client.getBalance(wallet.address);
console.log('Balance:', balance, 'micro-USDC');

// Send a transfer
const txHash = await client.transfer({
  from: wallet,
  to: '0xrecipient_address_hex',
  amount: 1_000_000, // 1 USDC
  fee: 1000,
});
console.log('TX Hash:', txHash);

// Query a block
const block = await client.getLatestBlock();
console.log('Block height:', block.block_number);
```

### Python SDK (dina-py)

```bash
cd sdk/dina-py
pip install -e .
```

```python
from dina_py import DinaClient, Wallet

# Connect to local testnet
client = DinaClient("http://localhost:8545")

# Generate a wallet
wallet = Wallet.generate()
print(f"Address: {wallet.address}")

# Check balance
balance = client.get_balance(wallet.address)
print(f"Balance: {balance} micro-USDC")

# Send a transfer
tx_hash = client.transfer(
    from_wallet=wallet,
    to="0xrecipient_address_hex",
    amount=1_000_000,  # 1 USDC
    fee=1000,
)
print(f"TX Hash: {tx_hash}")
```

### Rust SDK (dina-sdk)

The Rust SDK is used for writing smart contracts that compile to WASM. Add it to your `Cargo.toml`:

```toml
[dependencies]
dina-sdk = { path = "../dina_network/crates/dina-sdk" }

[lib]
crate-type = ["cdylib"]
```

## Deploying Contracts

### Writing a Contract

Every Dina contract implements the `dispatch(state, method, args, caller)` pattern:

```rust
// contracts/my-contract/src/lib.rs

use dina_sdk::prelude::*;

#[dina_contract]
pub struct MyToken {
    name: String,
    symbol: String,
    total_supply: u64,
    balances: Map<Address, u64>,
}

#[dina_impl]
impl MyToken {
    #[init]
    pub fn new(name: String, symbol: String, initial_supply: u64) -> Self {
        let mut balances = Map::new();
        balances.insert(caller(), initial_supply);
        Self {
            name,
            symbol,
            total_supply: initial_supply,
            balances,
        }
    }

    #[view]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[view]
    pub fn balance_of(&self, owner: Address) -> u64 {
        self.balances.get(&owner).copied().unwrap_or(0)
    }

    pub fn transfer(&mut self, to: Address, amount: u64) -> bool {
        let sender = caller();
        let sender_balance = self.balance_of(sender);
        if sender_balance < amount {
            return false;
        }
        self.balances.insert(sender, sender_balance - amount);
        let recipient_balance = self.balance_of(to);
        self.balances.insert(to, recipient_balance + amount);
        emit_event("Transfer", &(sender, to, amount));
        true
    }

    #[payable]
    pub fn buy(&mut self) -> u64 {
        let payment = attached_usdc();
        let buyer = caller();
        // 1 USDC = 100 tokens
        let tokens = payment * 100;
        let balance = self.balance_of(buyer);
        self.balances.insert(buyer, balance + tokens);
        self.total_supply += tokens;
        tokens
    }
}
```

### Compiling to WASM

```bash
cargo build -p my-contract --target wasm32-unknown-unknown --release

# The WASM file will be at:
# target/wasm32-unknown-unknown/release/my_contract.wasm
```

### Deploying via CLI

```bash
# Generate a deployer keypair
./target/release/dina keygen --output deployer.key

# Deploy the contract
./target/release/dina deploy \
  --wasm target/wasm32-unknown-unknown/release/my_contract.wasm \
  --init '{"name":"My Token","symbol":"MTK","initial_supply":1000000}' \
  --key deployer.key
```

### Deploying via JSON-RPC

```bash
# Hex-encode the WASM file
WASM_HEX=$(xxd -p target/wasm32-unknown-unknown/release/my_contract.wasm | tr -d '\n')

# Submit a DeployContract transaction
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d "{
    \"jsonrpc\": \"2.0\",
    \"method\": \"dina_sendTransaction\",
    \"params\": [\"$SIGNED_TX_HEX\"],
    \"id\": 1
  }"
```

## Interacting with Contracts

### Call a Contract Method

```bash
# Via JSON-RPC
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "dina_sendTransaction",
    "params": ["<hex-encoded-CallContract-tx>"],
    "id": 1
  }'
```

### Contract Dispatch Model

Contracts expose a single entry point: `__dispatch(method_ptr, method_len, args_ptr, args_len) -> i64`. The WASM runtime:

1. Allocates memory for the method name and arguments via `__alloc`
2. Writes the method name and argument bytes into guest memory
3. Calls `__dispatch` which routes to the correct Rust function
4. Reads the return value from the packed i64 (high 32 bits = ptr, low 32 bits = len)

### Host Functions Available to Contracts

Contracts can call these host functions:

| Function | Gas Cost | Description |
|----------|----------|-------------|
| `dina_storage_read(key_ptr, key_len)` | 100 | Read from persistent storage |
| `dina_storage_write(key_ptr, key_len, val_ptr, val_len)` | 500 | Write to persistent storage |
| `dina_transfer(to_ptr, amount)` | 200 | Transfer USDC to an address |
| `dina_caller()` | 5 | Get the caller's address |
| `dina_self_address()` | 5 | Get the contract's own address |
| `dina_attached_value()` | 5 | Get USDC attached to the call |
| `dina_block_time()` | 5 | Get current block timestamp |
| `dina_block_height()` | 5 | Get current block height |
| `dina_emit_event(topic_ptr, topic_len, data_ptr, data_len)` | 100 | Emit an event |
| `dina_sha256(data_ptr, data_len, out_ptr)` | 50 | Compute SHA-256 hash |
| `dina_ed25519_verify(msg_ptr, msg_len, sig_ptr, pubkey_ptr)` | 300 | Verify Ed25519 signature |
| `dina_cross_call(addr_ptr, method_ptr, method_len, args_ptr, args_len)` | 1000 | Call another contract |

## Using Payment Channels

Payment channels enable offline device-to-device transactions at approximately 5ms per update.

### Opening a Channel

```typescript
// TypeScript SDK
const channel = await client.openChannel({
  counterparty: '0xdevice_b_address',
  deposit: 10_000_000, // 10 USDC
  wallet: myWallet,
});
console.log('Channel ID:', channel.channelId);
```

### Off-Chain Payments

Once a channel is open, payments happen locally without blockchain interaction:

```typescript
// Send 0.50 USDC through the channel
const update = await channel.pay(500_000);
// ~5ms, no gas, no network required

// Send another payment
const update2 = await channel.pay(250_000);
// State: A has 9.25 USDC, B has 0.75 USDC
```

### Closing a Channel

```typescript
// Cooperative close (both parties agree)
await channel.closeCooperative();

// Unilateral close (one party submits state)
await channel.closeUnilateral();
// Starts 100-block challenge period
```

### Channel State Model

Each state update contains:
- `channel_id`: 32-byte identifier
- `balance_a`, `balance_b`: Current balances (must sum to `total_locked`)
- `sequence`: Monotonically increasing counter
- `timestamp`: When the update was created

Both parties sign every state update. Higher sequence numbers supersede lower ones during disputes.

## Privacy Features

### Encrypted Memos

Attach an encrypted memo that only the recipient can read:

```typescript
// Encrypt a memo for the recipient
const memo = await client.encryptMemo(
  recipientX25519PublicKey,
  'Payment for invoice #1234'
);

// Send transfer with encrypted memo
await client.transfer({
  from: wallet,
  to: recipientAddress,
  amount: 5_000_000,
  memo: memo,
});
```

```python
# Python
memo = client.encrypt_memo(recipient_x25519_pubkey, b"Payment for invoice #1234")
client.transfer(wallet, to_address, amount=5_000_000, memo=memo)
```

### Stealth Addresses

Send payments that are unlinkable to the recipient's public address:

```typescript
// Recipient publishes their stealth meta-address
const meta = recipient.getStealthMetaAddress();
// Contains: scan_pubkey, spend_pubkey

// Sender derives a one-time address
const stealth = deriveStealthAddress(meta);
// stealth.address is unique to this transaction
// stealth.ephemeral_pubkey must be published

// Send to the stealth address
await client.transfer({
  from: wallet,
  to: stealth.address,
  amount: 1_000_000,
});

// Recipient scans the chain for their payments
const payments = await recipient.scanForStealthPayments();
```

### View Keys

Grant auditors read-only access to your transaction history:

```typescript
// Grant a viewer limited access
await wallet.grantViewAccess(auditorAddress, {
  scope: 'transfers_only',
  expiry: Date.now() + 86400 * 30 * 1000, // 30 days
});

// Revoke access
await wallet.revokeViewAccess(auditorAddress);
```

## Cognitum Seed Integration

### MCP Tool Calls

Cognitum Seed devices interact with Dina through the Model Context Protocol:

```rust
// On the Cognitum Seed firmware (Rust)
use dina_mcp::McpToolCall;

// Send a transfer
let call = McpToolCall {
    tool_name: "dina/transfer".to_string(),
    arguments: serde_json::json!({
        "to": "0xrecipient",
        "amount": 50000,
    }),
};

let result = mcp_client.call_tool(call).await?;
if result.success {
    println!("Transfer sent: {:?}", result.data);
}
```

### Device Registration

```rust
// Register the device on-chain
let call = McpToolCall {
    tool_name: "dina/register_device".to_string(),
    arguments: serde_json::json!({
        "device_pubkey": hex::encode(device_ed25519_pubkey),
        "owner": hex::encode(owner_address),
        "firmware_hash": hex::encode(firmware_sha256),
        "attestation_signature": hex::encode(attestation_sig),
    }),
};
```

### BLE Mesh Relay

When offline, Cognitum Seeds can settle payment channels through the BLE mesh relay:

```rust
use dina_relay::{RelayBlob, RelayBroadcaster, BroadcastConfig};

// Create a settlement blob
let blob = RelayBlob {
    version: 1,
    sender: my_address,
    receiver: counterparty_address,
    amount: 50_000,
    sequence: channel_sequence,
    created_at: current_unix_time,
    ttl_secs: 300,
    relay_fee: 10,
    channel_state_hash: state_hash,
    sender_signature: sign(&signing_key, &blob.signing_bytes()),
    receiver_signature: countersign,
    hop_count: 0,
    max_hops: 10,
};

// Broadcast over BLE
let broadcaster = RelayBroadcaster::new(BroadcastConfig::default());
broadcaster.broadcast(&blob).await?;
```

## Example Contracts

The `examples/` directory contains complete example contracts:

| Example | Description | Path |
|---------|-------------|------|
| `hello-world` | Minimal contract demonstrating dispatch pattern | `examples/hello-world/` |
| `escrow` | Two-party escrow with release and refund | `examples/escrow/` |
| `voting` | On-chain voting with proposal and ballot | `examples/voting/` |
| `subscription` | Recurring payment subscription service | `examples/subscription/` |
| `marketplace` | P2P marketplace with listings and purchases | `examples/marketplace/` |

### Running Example Tests

```bash
cargo test -p hello-world
cargo test -p escrow
cargo test -p voting
```

## Contributing to the Codebase

### Code Style

- Use `thiserror` for error types in library crates, `anyhow` in binaries only
- Never use `unwrap()` in library crates; return `Result` with descriptive errors
- All public APIs must have doc comments
- Smart contracts must be `no_std` compatible
- Use Rust edition 2021 (not 2024)

### Testing Requirements

- Every crate must have unit tests
- Integration tests go in `tests/` at workspace root
- Contract tests use the SDK's `TestRuntime` harness
- Run `cargo test --workspace` before submitting any PR
- Run `cargo clippy --workspace -- -D warnings` for lint

### Pull Request Process

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Write code with tests
4. Ensure all tests pass: `cargo test --workspace`
5. Ensure no clippy warnings: `cargo clippy --workspace -- -D warnings`
6. Submit a pull request

### Adding a New DRC Contract

1. Create `contracts/drc{N}-{name}/` with:
   - `Cargo.toml` (set `crate-type = ["cdylib"]`, depend on `dina-sdk`)
   - `src/lib.rs` implementing `dispatch(state, method, args, caller) -> Result<Vec<u8>>`
2. Add the crate to `members` in root `Cargo.toml`
3. Write tests using `dina-sdk::TestRuntime`
4. Document the standard in `docs/DRC_STANDARDS.md`

### Project Layout

```
dina_network/
  crates/            14 library crates (core, consensus, network, etc.)
  node/              Validator/full-node binary (dina-node)
  cli/               CLI wallet and admin tool (dina)
  contracts/         30 DRC standard reference implementations
  sdk/
    dina-js/         TypeScript/JavaScript SDK
    dina-py/         Python SDK
  examples/          Example contracts (hello-world, escrow, voting, etc.)
  tests/             Workspace-level integration tests
  benches/           Benchmarks
  docs/              Documentation (you are here)
  genesis.json       Default testnet genesis configuration
  docker-compose.yml Multi-validator Docker setup
```

## Troubleshooting

### Contract compilation fails with "unknown import"

**Cause**: The contract uses a host function that is not registered.

**Fix**: Check that your `dina-sdk` version matches the node version. Only use host functions listed in the host function table above.

### "out of gas" error

**Cause**: The contract execution exceeded the gas limit.

**Fix**: Increase the `fee` field in your transaction. The default maximum gas is 10,000,000 (10 USDC). Optimize your contract to use fewer storage operations (500 gas each).

### Contract call returns empty data

**Cause**: The contract's `__dispatch` function returned a zero-length result.

**Fix**: Ensure your contract method returns data. View methods should serialize their return value. Check that the method name in the call matches the contract exactly.

### WASM module too large

**Cause**: Release builds with debug info or unoptimized code.

**Fix**: Build with optimization:
```bash
cargo build -p my-contract --target wasm32-unknown-unknown --release
```

Add to your contract's `Cargo.toml`:
```toml
[profile.release]
opt-level = "z"    # Optimize for size
lto = true         # Link-time optimization
strip = true       # Strip debug info
```

### Transaction rejected: "invalid nonce"

**Cause**: The transaction nonce does not match the account's current nonce.

**Fix**: Query the account's current nonce with `dina_getAccount` and use that value for the next transaction.
