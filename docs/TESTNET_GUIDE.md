# Dina Network -- Testnet Guide

This guide covers everything you need to run, test, and develop on a local Dina Network testnet.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start (One Command)](#quick-start-one-command)
- [Manual Setup](#manual-setup)
- [Docker Setup](#docker-setup)
- [Using the CLI](#using-the-cli)
- [Using the Faucet](#using-the-faucet)
- [Connecting the TypeScript SDK](#connecting-the-typescript-sdk)
- [Connecting the Python SDK](#connecting-the-python-sdk)
- [Deploying Your First Contract](#deploying-your-first-contract)
- [Common Issues and Solutions](#common-issues-and-solutions)

---

## Prerequisites

### Required

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.70+ | Build validator, CLI, and faucet |
| WASM target | `wasm32-unknown-unknown` | Compile smart contracts |

### Optional

| Tool | Version | Purpose |
|------|---------|---------|
| Docker | 20+ | Run testnet in containers |
| Docker Compose | v2 | Orchestrate multi-container testnet |
| Node.js | 18+ | TypeScript SDK |
| Python | 3.9+ | Python SDK |
| curl | any | Health checks and faucet requests |

### Install Rust and WASM Target

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the WASM compilation target
rustup target add wasm32-unknown-unknown
```

---

## Quick Start (One Command)

The setup wizard handles everything: building binaries, generating keys, creating genesis, and starting all services.

```bash
cd dina_network
./scripts/setup-testnet.sh
```

This will:

1. Check that Rust and Cargo are installed
2. Build `dina-node` and `dina` CLI in release mode
3. Generate Ed25519 keys for 3 validators + faucet
4. Create `genesis.json` with validator stakes and faucet balance
5. Start 3 validator nodes (TurboBFT consensus)
6. Start 1 RPC node (JSON-RPC on port 8545, REST on port 8080)
7. Start the faucet server
8. Start the block explorer backend
9. Print connection URLs

Press `Ctrl+C` to cleanly shut down everything.

### Endpoints After Setup

| Service | URL |
|---------|-----|
| JSON-RPC | `http://localhost:8545` |
| REST API | `http://localhost:8080` |
| Faucet API | `http://localhost:8080/faucet/` |
| Explorer API | `http://localhost:8080/api/` |
| Health Check | `http://localhost:8080/health` |

---

## Manual Setup

If you prefer to run each step yourself:

### 1. Build the Binaries

```bash
cd dina_network
cargo build --release --bin dina-node --bin dina
```

### 2. Generate Validator Keys

```bash
./scripts/generate-keys.sh
```

This creates Ed25519 keypairs in `keys/validator-{1,2,3}/` and `keys/faucet/`, then writes `genesis.json`.

### 3. Start Validator Nodes

```bash
# Terminal 1 -- Validator 1 (seed node)
./target/release/dina-node \
    --data-dir .local-testnet/validator-1 \
    --listen /ip4/127.0.0.1/tcp/9944 \
    --rpc-port 8555 \
    --rest-port 8090 \
    --validator \
    --validator-key keys/validator-1/node_key \
    --chain-id dina-testnet-1

# Terminal 2 -- Validator 2
./target/release/dina-node \
    --data-dir .local-testnet/validator-2 \
    --listen /ip4/127.0.0.1/tcp/9945 \
    --rpc-port 8565 \
    --rest-port 8100 \
    --validator \
    --validator-key keys/validator-2/node_key \
    --chain-id dina-testnet-1 \
    --bootstrap /ip4/127.0.0.1/tcp/9944

# Terminal 3 -- Validator 3
./target/release/dina-node \
    --data-dir .local-testnet/validator-3 \
    --listen /ip4/127.0.0.1/tcp/9946 \
    --rpc-port 8575 \
    --rest-port 8110 \
    --validator \
    --validator-key keys/validator-3/node_key \
    --chain-id dina-testnet-1 \
    --bootstrap /ip4/127.0.0.1/tcp/9944
```

### 4. Start the RPC Node

```bash
# Terminal 4 -- RPC node (client-facing, non-validator)
./target/release/dina-node \
    --data-dir .local-testnet/rpc-node \
    --listen /ip4/127.0.0.1/tcp/9947 \
    --rpc-port 8545 \
    --rest-port 8080 \
    --chain-id dina-testnet-1 \
    --bootstrap /ip4/127.0.0.1/tcp/9944
```

### 5. Verify It Works

```bash
# Check node health
curl http://localhost:8080/health

# Check chain status
./target/release/dina --rpc-url http://localhost:8545 status
```

---

## Docker Setup

### Quick Docker Start

```bash
# Build and start (foreground)
./scripts/setup-testnet-docker.sh

# With faucet UI and explorer
./scripts/setup-testnet-docker.sh --with-faucet-ui --with-explorer

# Detached (background)
./scripts/setup-testnet-docker.sh --detach
```

### Using Docker Compose Directly

```bash
# Start the base testnet (3 validators + RPC node)
docker compose up --build

# Stop
docker compose down

# Stop and remove all data
docker compose down -v
```

### Docker Port Mappings

| Container | Host Port | Service |
|-----------|-----------|---------|
| dina-rpc | 8545 | JSON-RPC |
| dina-rpc | 8080 | REST API |
| dina-faucet-ui | 3000 | Faucet Web UI |
| dina-explorer | 3001 | Block Explorer |

### View Logs

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f rpc-node
docker compose logs -f validator-1
```

---

## Using the CLI

The `dina` CLI is your primary tool for interacting with the network.

### Generate a Keypair

```bash
# Generate a new Ed25519 keypair
./target/release/dina keygen

# Save to a specific file
./target/release/dina keygen --output my-wallet/key
# Creates: my-wallet/key (secret) and my-wallet/key.pub (public)
```

### Check Chain Status

```bash
./target/release/dina --rpc-url http://localhost:8545 status
```

### Check Balance

```bash
./target/release/dina --rpc-url http://localhost:8545 balance <ADDRESS>
```

### Transfer USDC

```bash
./target/release/dina --rpc-url http://localhost:8545 transfer \
    --from keys/test-wallets/alice/private_key \
    --to <RECIPIENT_ADDRESS> \
    --amount 50000000  # 50 USDC (6 decimal places)
```

### Deploy a Contract

```bash
./target/release/dina --rpc-url http://localhost:8545 deploy \
    --from keys/test-wallets/alice/private_key \
    --wasm contracts/my_contract.wasm \
    --gas-limit 1000000
```

### Create Test Wallets

```bash
# Creates 10 named wallets (Alice, Bob, etc.) and funds them
./scripts/create-test-wallets.sh
```

---

## Using the Faucet

The faucet dispenses testnet USDC for development. It is not real money.

### Web UI

Open the faucet web interface in your browser:

- Local testnet: `http://localhost:3000` (Docker) or served on the REST port
- Enter your Dina address (64 hex characters)
- Click "Request 100 USDC"
- Wait 60 seconds between requests

### Faucet REST API

#### Request Funds

```bash
curl -X POST http://localhost:8080/faucet/request \
    -H "Content-Type: application/json" \
    -d '{"address": "YOUR_64_CHAR_HEX_ADDRESS"}'
```

Response:

```json
{
  "success": true,
  "amount": 100000000,
  "amount_display": "100.000000 USDC",
  "address": "abcd1234...",
  "timestamp": 1711500000
}
```

#### Check Status

```bash
curl http://localhost:8080/faucet/status/YOUR_64_CHAR_HEX_ADDRESS
```

Response:

```json
{
  "address": "abcd1234...",
  "can_request": true,
  "seconds_until_next": 0,
  "total_received": 200000000,
  "request_count": 2
}
```

#### Faucet Stats

```bash
curl http://localhost:8080/faucet/stats
```

### Rate Limits

| Parameter | Value |
|-----------|-------|
| Amount per request | 100 USDC |
| Cooldown between requests | 60 seconds |
| Daily limit per address | 500 USDC |

---

## Connecting the TypeScript SDK

### Install

```bash
npm install @dinanetwork/sdk
# or
yarn add @dinanetwork/sdk
```

### Basic Usage

```typescript
import { DinaClient, Keypair, Amount } from '@dinanetwork/sdk';

// Connect to local testnet
const client = new DinaClient('http://localhost:8545');

// Generate a new keypair
const alice = Keypair.generate();
console.log('Address:', alice.address());

// Check balance
const balance = await client.getBalance(alice.address());
console.log('Balance:', Amount.fromMicroUnits(balance).toUSDC(), 'USDC');

// Request faucet funds
const faucet = await client.faucet(alice.address());
console.log('Received:', faucet.amount_display);

// Transfer USDC
const tx = await client.transfer({
  from: alice,
  to: 'RECIPIENT_ADDRESS_HEX',
  amount: Amount.usdc(50), // 50 USDC
});
console.log('Transaction hash:', tx.hash);

// Wait for confirmation
const receipt = await client.waitForTransaction(tx.hash);
console.log('Block height:', receipt.block_height);
```

### Smart Contract Interaction

```typescript
import { DinaClient, Keypair, Contract } from '@dinanetwork/sdk';

const client = new DinaClient('http://localhost:8545');
const deployer = Keypair.fromFile('keys/test-wallets/alice/private_key');

// Deploy a contract
const deployment = await client.deploy({
  from: deployer,
  wasmPath: './contracts/my_contract.wasm',
  gasLimit: 1_000_000,
});

console.log('Contract address:', deployment.contractAddress);

// Call a contract method
const contract = new Contract(deployment.contractAddress, MY_ABI);
const result = await client.call({
  from: deployer,
  contract,
  method: 'get_value',
  args: [],
});
```

---

## Connecting the Python SDK

### Install

```bash
pip install dina-sdk
```

### Basic Usage

```python
from dina_sdk import DinaClient, Keypair, Amount

# Connect to local testnet
client = DinaClient("http://localhost:8545")

# Generate a new keypair
alice = Keypair.generate()
print(f"Address: {alice.address()}")

# Check balance
balance = client.get_balance(alice.address())
print(f"Balance: {Amount.from_micro_units(balance).to_usdc()} USDC")

# Request faucet funds
result = client.faucet(alice.address())
print(f"Received: {result['amount_display']}")

# Transfer USDC
tx = client.transfer(
    from_key=alice,
    to="RECIPIENT_ADDRESS_HEX",
    amount=Amount.usdc(25),  # 25 USDC
)
print(f"TX hash: {tx.hash}")

# Wait for finality (sub-200ms on Dina)
receipt = client.wait_for_transaction(tx.hash)
print(f"Confirmed at block {receipt.block_height}")
```

### Load Test Wallets

```python
import json

with open("keys/test-wallets/test-wallets.json") as f:
    wallets = json.load(f)

for w in wallets["wallets"]:
    key = Keypair.from_file(w["key_file"])
    balance = client.get_balance(w["address"])
    print(f"{w['name']:>10}: {Amount.from_micro_units(balance).to_usdc()} USDC")
```

---

## Deploying Your First Contract

Dina Network uses WASM smart contracts written in Rust.

### 1. Create a Contract Project

```bash
cargo new --lib my_contract
cd my_contract
```

### 2. Configure Cargo.toml

```toml
[package]
name = "my_contract"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
dina-sdk = { path = "../crates/dina-sdk" }
```

### 3. Write the Contract

```rust
// src/lib.rs
use dina_sdk::prelude::*;

#[dina_contract]
mod counter {
    use super::*;

    #[state]
    struct Counter {
        value: u64,
        owner: Address,
    }

    #[init]
    fn new(ctx: &Context) -> Counter {
        Counter {
            value: 0,
            owner: ctx.sender(),
        }
    }

    #[call]
    fn increment(state: &mut Counter, _ctx: &Context) {
        state.value += 1;
    }

    #[call]
    fn get_value(state: &Counter) -> u64 {
        state.value
    }
}
```

### 4. Build the Contract

```bash
cargo build --target wasm32-unknown-unknown --release

# The WASM binary will be at:
# target/wasm32-unknown-unknown/release/my_contract.wasm
```

### 5. Deploy to Testnet

```bash
./target/release/dina --rpc-url http://localhost:8545 deploy \
    --from keys/test-wallets/alice/private_key \
    --wasm target/wasm32-unknown-unknown/release/my_contract.wasm
```

### 6. Interact with the Contract

```bash
# Call the increment method
./target/release/dina --rpc-url http://localhost:8545 call \
    --from keys/test-wallets/alice/private_key \
    --contract <CONTRACT_ADDRESS> \
    --method increment

# Query the value (read-only, no transaction)
./target/release/dina --rpc-url http://localhost:8545 query \
    --contract <CONTRACT_ADDRESS> \
    --method get_value
```

---

## Common Issues and Solutions

### Build fails with "linker not found"

**Cause:** Missing system libraries for the Rust build.

**Fix:**

```bash
# Ubuntu/Debian
sudo apt-get install build-essential pkg-config libssl-dev protobuf-compiler cmake

# macOS
brew install openssl protobuf cmake

# Windows (MSYS2/MinGW)
pacman -S mingw-w64-x86_64-toolchain pkg-config openssl
```

### "wasm32-unknown-unknown" target not found

**Fix:**

```bash
rustup target add wasm32-unknown-unknown
```

### Faucet returns "cooldown active"

**Cause:** You requested funds less than 60 seconds ago.

**Fix:** Wait for the cooldown timer to expire, then try again. The faucet status endpoint tells you how many seconds remain:

```bash
curl http://localhost:8080/faucet/status/YOUR_ADDRESS
# {"seconds_until_next": 42, ...}
```

### Faucet returns "daily limit exceeded"

**Cause:** You have received 500 USDC in the past 24 hours.

**Fix:** Wait 24 hours for the rolling window to reset, or use a different address.

### Nodes fail to connect to each other

**Cause:** Port conflicts or firewall blocking local connections.

**Fix:**

```bash
# Check what's using the ports
lsof -i :9944  # P2P
lsof -i :8545  # JSON-RPC
lsof -i :8080  # REST

# Kill any conflicting processes, then restart
```

### Docker build fails with "permission denied"

**Fix:**

```bash
# Ensure Docker can be run without sudo
sudo usermod -aG docker $USER
newgrp docker
```

### "genesis.json has no validators"

**Cause:** Keys were not generated before starting nodes.

**Fix:**

```bash
# Generate keys first
./scripts/generate-keys.sh

# Then start the testnet
./scripts/setup-testnet.sh
```

### Transaction stuck or not confirming

**Cause:** Consensus requires 2/3 of validators. If a validator is down, blocks may stall.

**Fix:**

```bash
# Check which validators are running
curl http://localhost:8080/api/validators

# Check logs for errors
tail -50 .local-testnet/validator-1.log
```

### WSL-specific: "address already in use"

**Cause:** Windows and WSL share port space on some configurations.

**Fix:**

```bash
# Use different ports
export BASE_RPC_PORT=18545
export BASE_REST_PORT=18080
./scripts/setup-testnet.sh
```

### Reset everything and start fresh

```bash
# Native
rm -rf .local-testnet/ keys/
./scripts/setup-testnet.sh

# Docker
docker compose down -v
./scripts/setup-testnet-docker.sh --clean
```

---

## Network Architecture

```
                          Internet / Local
                               |
                        +------+------+
                        |  RPC Node   |  :8545 JSON-RPC
                        | (non-val)   |  :8080 REST + Faucet
                        +------+------+
                               |
              +----------------+----------------+
              |                |                |
        +-----+-----+   +-----+-----+   +-----+-----+
        | Validator 1|   | Validator 2|   | Validator 3|
        |  (seed)    |   |            |   |            |
        |  :9944     |   |  :9945     |   |  :9946     |
        +------------+   +------------+   +------------+
              |                |                |
              +----------------+----------------+
                         TurboBFT
                      (sub-200ms finality)
```

## USDC Denominations

Dina Network uses USDC with 6 decimal places (micro-units):

| Display | Micro-units | Description |
|---------|-------------|-------------|
| 1 USDC | 1,000,000 | One US dollar |
| 0.01 USDC | 10,000 | One cent |
| 100 USDC | 100,000,000 | Faucet drip amount |
| 500 USDC | 500,000,000 | Faucet daily limit |
