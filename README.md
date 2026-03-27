# Dina Network

**The fastest blockchain for payments, AI agents, and IoT devices.**

[![TPS](https://img.shields.io/badge/TPS-100K+-blue)]()
[![Finality](https://img.shields.io/badge/Finality-100ms-green)]()
[![Standards](https://img.shields.io/badge/DRC_Standards-94-orange)]()
[![Gas](https://img.shields.io/badge/Gas-USDC_Native-brightgreen)]()
[![Bridges](https://img.shields.io/badge/Bridges-8_Integrations-purple)]()
[![License](https://img.shields.io/badge/License-MIT-lightgrey)]()

---

## What Makes Dina Different

- **Parallel Execution.** Block-STM lane-based transaction processing that scales with CPU cores. A 4-core machine handles 400K TPS. A 64-core machine handles 6.4M TPS. No chain redesign needed -- just add cores.

- **Parallel Wallets (DRC-63).** One authority key controls N independent wallets. Each wallet has its own nonce, so all N can transact in the same 100ms block. No other chain offers per-user parallelism. A single user with 100 parallel wallets achieves 1,000 on-chain transactions per second.

- **USDC Native Gas.** Transaction fees are paid in USDC. There is no gas token. No ETH. No SOL. No bridge-and-swap before you can use the chain. Stablecoin in, stablecoin out.

- **100ms BFT Finality.** TurboBFT consensus produces irreversible blocks every 100ms. This is not optimistic finality with a 7-day challenge window. Not soft confirmation that can be rolled back. Finality means finality.

- **94 DRC Standards.** More built-in smart contract standards than any chain. 18 ported from Ethereum ERCs, 76 purpose-built for AI agents, IoT devices, machine economies, and DeFi. Every standard has a complete WASM reference implementation.

---

## Quick Start

### JavaScript / TypeScript

```bash
npm install dina-js
```

```typescript
import { DinaWallet, DinaClient } from 'dina-js';

// Generate a new wallet (Ed25519)
const wallet = DinaWallet.generate();
console.log('Address:', wallet.address);

// Connect to testnet
const client = new DinaClient('http://35.184.213.248:8545');

// Check USDC balance
const balance = await client.getBalance(wallet.address);
console.log('Balance:', balance, 'USDC');

// Send 5 USDC (6 decimals)
const txHash = await client.transfer(wallet, {
  to: '0xRecipientAddress...',
  amount: 5_000_000n,
});

// Confirmed in ~100ms
const receipt = await client.waitForTransaction(txHash);
console.log('Confirmed in block:', receipt.blockNumber);
```

### Python

```bash
pip install dina-network
```

```python
from dina_network import Wallet, Client

# Generate wallet
wallet = Wallet.generate()
print(f"Address: {wallet.address}")

# Connect and send
client = Client("http://35.184.213.248:8545")
tx = client.transfer(wallet, to="0x...", amount=5_000_000)
receipt = client.wait_for_transaction(tx)
```

### Rust

```bash
# Add to Cargo.toml
[dependencies]
dina-sdk = { path = "crates/dina-sdk" }
```

### CLI

```bash
# Build from source
cargo build --release

# Generate a keypair
./target/release/dina keygen

# Check balance
./target/release/dina balance <ADDRESS>

# Send USDC
./target/release/dina transfer --to <ADDRESS> --amount 1000000
```

---

## Performance Comparison

| Chain | TPS (actual) | Finality | Gas Token | VM | Smart Contract Standards | Per-User Parallel Txs |
|-------|-------------|----------|-----------|-----|--------------------------|----------------------|
| **Dina** | **100,000+** | **100ms** | **USDC** | **WASM** | **94 DRC** | **Yes (DRC-63)** |
| Ethereum | 15 | 12 min | ETH | EVM | ~40 ERCs widely used | No |
| Base | 100 | 2 sec (L2) | ETH | EVM | Inherits Ethereum | No |
| Solana | 4,000 | 400ms (soft) | SOL | SVM | ~10 SPL programs | No |
| Sui | 10,000 | 500ms | SUI | Move | ~15 standards | No |
| Aptos | 10,000 | 900ms | APT | Move | ~15 standards | No |
| Arbitrum | 40,000 | 250ms (soft) | ETH | EVM | Inherits Ethereum | No |
| Optimism | 2,000 | 2 sec (L2) | ETH | EVM | Inherits Ethereum | No |
| Polygon | 7,000 | 2 sec | POL | EVM | Inherits Ethereum | No |
| NEAR | 1,000 | 1-2 sec | NEAR | WASM | ~10 NEPs | No |
| Sei | 20,000 | 400ms | SEI | EVM+WASM | ~10 standards | No |

**Notes on the comparison:**

- TPS figures reflect observed throughput, not theoretical maximums. Dina's 100K+ TPS is measured on an 8-core machine with parallel execution enabled.
- "Soft" finality means the transaction is likely final but can theoretically be reverted. Dina, Ethereum (post-merge), and Sui provide hard finality.
- Arbitrum and Optimism inherit Ethereum's security but have a 7-day challenge window for finality on L1.
- Every chain listed except Dina requires a volatile gas token, meaning users must acquire and hold a non-stablecoin asset to transact.
- "Per-User Parallel Txs" means a single user can submit multiple independent transactions in the same block without nonce conflicts. Only Dina supports this via Parallel Wallets.

**Trade-offs to be aware of:**

- Dina currently runs 3-21 validators. This is significantly fewer than Ethereum (~900K validators) or Solana (~1,800 validators). Fewer validators means faster consensus but less decentralization.
- Dina's WASM VM means developers write contracts in Rust, not Solidity. This is a higher barrier to entry but produces faster, safer contracts.
- The ecosystem is early-stage. Ethereum and Solana have years of tooling, auditors, and battle-tested infrastructure.

---

## Parallel Wallets (DRC-63)

*Previously called "Swarm Wallets" -- renamed to "Parallel Wallets" for clarity.*

### The Problem

Every blockchain limits users to sequential transactions per account. This is enforced by a **nonce** -- a counter that forces transaction ordering. Transaction #5 cannot execute until transaction #4 is confirmed.

```
How ALL blockchains work today:

  Your Account (nonce: 0)
    -> Transaction #1 (nonce: 0)  ... confirmed
    -> Transaction #2 (nonce: 1)  ... confirmed
    -> Transaction #3 (nonce: 2)  ... confirmed
    -> ... must wait for each one ...

  Per-user throughput is capped at 1 transaction per block.

  Ethereum:  1 tx per 12-second block  =  0.08 tx/s per user
  Solana:    1 tx per 400ms block      =  2.5 tx/s per user
  Dina:      1 tx per 100ms block      =  10 tx/s per user (without Parallel Wallets)
```

### The Solution

One authority key controls N wallets. Each wallet has its own nonce. All N can transact independently in the same block.

```
Traditional (every chain):

  User --> 1 Wallet --> sequential --> 10 tx/s max


Dina Parallel Wallet (DRC-63):

  User --> Authority Key
            |-- Wallet #1 (nonce: 0) --> tx A --+
            |-- Wallet #2 (nonce: 0) --> tx B --+
            |-- Wallet #3 (nonce: 0) --> tx C --+  ALL IN THE SAME
            |-- Wallet #4 (nonce: 0) --> tx D --+  100ms BLOCK
            |-- ...                             +------------------
            '-- Wallet #N (nonce: 0) --> tx N --+  N transactions/block
```

Each wallet is a full independent account with its own address (derived from authority + wallet_id), its own nonce, its own balance, its own spending limit, and its own purpose label. The authority controls all of them, but they operate independently on-chain.

### Throughput Levels

```
Level 1: Normal (what every chain gives you)
  1 wallet --> 1 tx/block --> 10 tx/s

Level 2: Parallel Wallets (DRC-63)
  100 wallets --> 100 tx/block --> 1,000 tx/s

Level 3: Parallel Wallets + Batch Transfer (DRC-63 + DRC-19)
  100 wallets x 100 payments per batch tx = 10,000 payments/block
  At 10 blocks/second = 100,000 payments/second from ONE user

Level 4: Parallel Wallets + Payment Channels (DRC-63 + Channels)
  1,000 wallets, each with an open payment channel
  Offline: 1,000 channels x 200 tx/s each = 200,000 tx/s (no validators needed)
  Settlement: batch-close all 1,000 channels in 1-10 seconds

Level 5: Maximum Theoretical (Parallel + Batch + Channels)
  10,000 wallets with payment channels, 1 hour of offline activity:
  10,000 x 3,600s x 200 tx/s = 7.2 billion transactions/hour
  Settled on-chain in 10 seconds via batched channel closes
```

### Per-User TPS Comparison

```
                        Per-User TPS      Network Max TPS
                        (what YOU get)    (what the chain handles)
                        --------------    ----------------------
Ethereum                0.08              15
Base                    0.5               100
Solana                  2.5               4,000
Aptos                   10                10,000
Sui                     10                10,000
---------------------------------------------------------
Dina (no parallel)      10                100,000
Dina (100 wallets)      1,000             100,000
Dina (1K wallets)       10,000            100,000
```

With Parallel Wallets, a single Dina user can saturate the entire chain's capacity. On Ethereum, you would need 600+ separate accounts to match what one Dina Parallel Wallet user can do.

---

## Architecture

```
Consensus:        TurboBFT (3-21 validators, 100ms blocks, BFT finality)
Execution:        Parallel lanes (Block-STM style, scales with CPU cores)
Smart Contracts:  WASM (write in Rust, compiled via wasm32-unknown-unknown)
Cryptography:     Ed25519 keys, SHA-256 address derivation
Storage:          redb embedded key-value database
Networking:       libp2p (gossipsub, Kademlia DHT, mDNS discovery)
RPC:              JSON-RPC 2.0 + REST API (axum + jsonrpsee) + WebSocket
```

### Crate Map

```
dina-core              Foundation types, Ed25519 crypto, genesis, USDC accounting
  |
  +-- dina-consensus   TurboBFT consensus engine (propose, prevote, precommit)
  +-- dina-network     libp2p peer discovery, gossipsub, block propagation
  +-- dina-storage     Persistent block and state storage via redb (8 tables)
  +-- dina-privacy     Stealth addresses, view keys, encrypted memos, ZK proofs
  +-- dina-channels    Offline device-to-device payment channels (~5ms latency)
  +-- dina-relay       BLE mesh relay protocol for offline settlement propagation
  +-- dina-wasm        wasmtime contract execution runtime
  +-- dina-rpc         JSON-RPC 2.0 + REST API server
  +-- dina-sdk         Rust SDK for authoring DRC-compatible contracts
  +-- dina-sdk-macros  Proc-macros: #[dina_contract], #[dina_impl], #[view], #[init]
  +-- dina-mcp         MCP tool server (12 tools for IoT devices)
  +-- dina-bench       Performance benchmarks
  +-- dina-faucet      Testnet faucet server
  +-- dina-monitoring  Prometheus metrics export
  +-- dina-explorer    Block explorer backend
```

### Binaries

| Binary | Description |
|--------|-------------|
| `dina-node` | Full validator or full-node: runs consensus, networking, RPC, WASM runtime |
| `dina` (CLI) | Command-line wallet and admin tool: keygen, transfers, queries, contract deploy |

---

## Bridges -- Connecting to 60+ Chains

Dina has Dina-side contracts ready for 7 bridge protocols. All bridges mint the same Bridged USDC (USDC.e) token on Dina.

| Bridge | Speed | Status | What's Needed |
|--------|-------|--------|---------------|
| **Base Direct** | ~5 min | **READY TO DEPLOY** | Deploy Solidity to Base Sepolia + run relayer. No approval needed. |
| **Across** | 1-3 min | NOT ACTIVE | Apply at across.to for spoke pool listing |
| **Stargate** | 1-3 min | NOT ACTIVE | Requires LayerZero endpoint first |
| **LayerZero** | 3-10 min | NOT ACTIVE | Apply at layerzero.network for endpoint |
| **Wormhole** | ~15 min | NOT ACTIVE | Apply at wormhole.com for Guardian support |
| **Axelar** | ~15 min | NOT ACTIVE | Apply at axelar.network for gateway |
| **Circle CCTP** | ~15 min | NOT ACTIVE | Requires legal entity, security audit, Circle approval (6-12 months) |

**Only the Base bridge works without third-party approval.** All other bridges have Dina-side contracts deployed and ready -- they activate when the third-party protocol approves Dina as a supported chain. Application guides are in `bridges/third-party/`.

### Bridge Architecture

```
                         Dina Network
                  +-----------------------+
                  |                       |
                  |   Bridged USDC Token  |  <-- Single token, multiple bridge minters
                  |      (USDC.e)         |
                  |                       |
                  +---+---+---+---+---+---+
                      |   |   |   |   |
                 CCTP | A | S | L | W | Axelar | Base
                      | c | t | Z | o |        |
                      | r | a | e | r |        |
                      | o | r | r | m |        |
                      | s | g | o | h |        |
                      | s | a |   | o |        |
                      |   | t |   | l |        |
                      |   | e |   | e |        |
                  +---+---+---+---+---+---+---+---+
                  |                                |
                  |        External Chains          |
                  | Ethereum, Base, Solana,         |
                  | Arbitrum, Polygon, Cosmos, ...  |
                  +--------------------------------+
```

### Circle Upgrade Path

The Bridged USDC contract is designed for Circle to eventually assume ownership and upgrade USDC.e to native USDC -- no migration, no token swap, same contract address. This follows Circle's documented process for new chains.

---

## DRC Standards (All 94)

Dina has 94 implemented DRC (Dina Request for Comments) standards. Each is a complete WASM smart contract in `contracts/`. Standards marked "ERC Port" are adapted from Ethereum; all others are Dina-original.

### Token Standards (DRC-1 to DRC-18)

| DRC | Name | Origin | Description |
|-----|------|--------|-------------|
| DRC-1 | Fungible Token | ERC-20 port | Standard fungible token interface |
| DRC-2 | Device Identity | Dina original | Hardware device identity attestation with Ed25519 keys |
| DRC-4 | Permit | ERC-2612 port | Gasless token approvals via Ed25519 signatures |
| DRC-5 | Soulbound Token | ERC-5192 port | Non-transferable tokens for credentials and reputation |
| DRC-6 | NFT | ERC-721 port | Non-fungible token standard |
| DRC-7 | Multi-Token | ERC-1155 port | Fungible + non-fungible tokens in one contract |
| DRC-8 | Token-Bound Account | ERC-6551 port | NFTs that own assets and call contracts |
| DRC-9 | Rental / Lending | ERC-4907 port | Time-limited NFT usage rights |
| DRC-10 | Royalties | ERC-2981 port | Secondary sale royalty information |
| DRC-11 | Semi-Fungible Token | ERC-3525 port | Tokens with value and slot attributes |
| DRC-12 | Vault (Yield) | ERC-4626 port | Tokenized yield-bearing vaults |
| DRC-13 | Compliant Token | ERC-3643 port | KYC/AML-gated transfer restrictions |
| DRC-14 | Contract Signature | ERC-1271 port | Contract-based signature validation |
| DRC-15 | Meta-Transactions | ERC-2771 port | Gasless transactions via trusted forwarder |
| DRC-16 | Proxy (Upgradeable) | ERC-1967 port | Upgradeable proxy contract pattern |
| DRC-17 | Hooks | ERC-777 port | Token send/receive hook callbacks |
| DRC-18 | Scriptable | ERC-5169 port | Off-chain scripts attached to tokens |

### Financial Standards (DRC-19 to DRC-30)

| DRC | Name | Origin | Description |
|-----|------|--------|-------------|
| DRC-19 | Batch Transfer | Dina original | Send to 100+ recipients in one transaction |
| DRC-20 | Timelock | Dina original | Time-delayed execution for governance safety |
| DRC-21 | Multisig | Dina original | M-of-N multi-signature authorization |
| DRC-22 | Vesting | Dina original | Token vesting schedules with cliff and linear release |
| DRC-23 | Oracle | Dina original | On-chain price and data feed interface |
| DRC-24 | DAO | Dina original | Decentralized governance with proposals and voting |
| DRC-25 | Staking Pool | Dina original | Pooled staking with reward distribution |
| DRC-26 | Access Control | Dina original | Role-based permission management |
| DRC-27 | Payment Splitter | Dina original | Automatic revenue splitting among recipients |
| DRC-28 | Crowdfund | Dina original | On-chain crowdfunding with milestones |
| DRC-29 | Insurance | Dina original | Parametric insurance contracts |
| DRC-30 | Identity Resolver | Dina original | Address-to-identity resolution |

### Machine Economy Standards (DRC-31 to DRC-42)

| DRC | Name | Origin | Description |
|-----|------|--------|-------------|
| DRC-31 | Agent Registry | Dina original | Global registry of AI agents with metadata |
| DRC-32 | Task Queue | Dina original | On-chain task assignment and completion tracking |
| DRC-33 | Machine Learning | Dina original | Model registration and inference payment |
| DRC-34 | Energy Market | Dina original | Peer-to-peer energy trading for IoT devices |
| DRC-35 | Fleet Manager | Dina original | Vehicle/robot fleet coordination and dispatch |
| DRC-36 | Supply Chain | Dina original | Product tracking from manufacture to delivery |
| DRC-37 | Agent Communication | Dina original | Structured inter-agent messaging protocol |
| DRC-38 | Bounty | Dina original | Task bounties with submission and judging |
| DRC-39 | Loyalty | Dina original | Points-based loyalty and rewards programs |
| DRC-40 | Warranty | Dina original | On-chain product warranty registration |
| DRC-41 | Scheduler | Dina original | Cron-style scheduled contract execution |
| DRC-42 | Reputation Oracle | Dina original | Cross-protocol reputation aggregation |

### Advanced Infrastructure (DRC-43 to DRC-63)

| DRC | Name | Origin | Description |
|-----|------|--------|-------------|
| DRC-43 | Payable Token | Dina original | Tokens with built-in payment callbacks |
| DRC-44 | Flash Loan | Dina original | Uncollateralized single-block loans |
| DRC-45 | Hybrid Token | Dina original | Tokens that switch between fungible and non-fungible |
| DRC-46 | Modular Account | Dina original | Plugin-based smart account architecture |
| DRC-47 | Minimal Multi-Token | Dina original | Gas-optimized ERC-1155 alternative |
| DRC-48 | Minimal Proxy | Dina original | Lightweight clone factory pattern |
| DRC-49 | Contract Metadata | Dina original | On-chain contract description and ABI |
| DRC-50 | Pausable | Dina original | Emergency pause/unpause for any contract |
| DRC-51 | Snapshot | Dina original | Point-in-time balance snapshots for governance |
| DRC-52 | Enumerable NFT | Dina original | NFTs with on-chain enumeration |
| DRC-53 | Wrapped USDC | Dina original | Wrapped USDC with additional DeFi hooks |
| DRC-54 | Agent Escrow | Dina original | Escrow specifically for AI agent transactions |
| DRC-55 | Device Mesh | Dina original | IoT device mesh network coordination |
| DRC-56 | Data NFT | Dina original | NFTs representing ownership of datasets |
| DRC-57 | Compute Marketplace | Dina original | Buy/sell compute resources on-chain |
| DRC-58 | Autonomous Payment | Dina original | Self-executing payments based on conditions |
| DRC-59 | Geo-Fence | Dina original | Location-based smart contract triggers |
| DRC-60 | Token Bridge Escrow | Dina original | Escrow for cross-chain token transfers |
| DRC-61 | AI Inference | Dina original | On-chain AI inference request and payment |
| DRC-62 | Device Twins | Dina original | Digital twin representation of physical devices |
| DRC-63 | Parallel Wallet | Dina original | One authority, N independent wallets, N parallel txs |

### AI Agent Standards (DRC-64 to DRC-82)

| DRC | Name | Origin | Description |
|-----|------|--------|-------------|
| DRC-64 | Vector Index | Dina original | On-chain vector similarity search index |
| DRC-65 | Agent Swarm | Dina original | Autonomous multi-agent swarm coordination |
| DRC-66 | Semantic Search | Dina original | Natural language contract and data queries |
| DRC-67 | Multi-Agent Escrow | Dina original | Escrow for multi-party agent collaborations |
| DRC-68 | Knowledge Graph | Dina original | On-chain knowledge graph for agent reasoning |
| DRC-69 | Agent Delegation | Dina original | Hierarchical agent permission delegation |
| DRC-70 | Model Marketplace | Dina original | Buy/sell/license AI models on-chain |
| DRC-71 | Collaborative Training | Dina original | Federated learning coordination and rewards |
| DRC-72 | Device Cluster | Dina original | IoT device clustering and group management |
| DRC-73 | Embedding Registry | Dina original | Shared vector embedding storage |
| DRC-74 | Agent Reputation Network | Dina original | Cross-agent reputation with graph analysis |
| DRC-75 | Conditional Payment | Dina original | Payments triggered by oracle-verified conditions |
| DRC-76 | Agent Memory | Dina original | Persistent memory storage for AI agents |
| DRC-77 | Swarm Consensus | Dina original | Multi-agent decision-making and voting |
| DRC-78 | Device Attestation Chain | Dina original | Chain of custody for device attestations |
| DRC-79 | Micro-Payment Stream | Dina original | Continuous payment streaming by the second |
| DRC-80 | Task Decomposer | Dina original | Break complex tasks into subtasks for agents |
| DRC-81 | Cross-Device State | Dina original | Synchronized state across device clusters |
| DRC-82 | Agent Marketplace v2 | Dina original | Agent hiring, rating, and payment marketplace |

### Agent / IoT / Privacy Standards (DRC-101 to DRC-113)

| DRC | Name | Origin | Description |
|-----|------|--------|-------------|
| DRC-101 | Agent Wallet | Dina original | AI agent-owned wallets with spending policies and allowlists |
| DRC-102 | Capability | Dina original | Delegated permission tokens with expiry and revocation |
| DRC-103 | Service Agreement | Dina original | Machine-to-machine SLA contracts with dispute resolution |
| DRC-104 | Swarm Coordination | Dina original | Multi-agent task distribution and reward splitting |
| DRC-105 | Sensor Attestation | Dina original | IoT sensor data authenticity proofs (Ed25519 signed) |
| DRC-106 | Data Market | Dina original | Buy/sell sensor data and AI training datasets |
| DRC-107 | Reputation | Dina original | On-chain reputation scoring for agents and devices |
| DRC-108 | Resource | Dina original | Tokenized compute, bandwidth, and storage resources |
| DRC-109 | Emergency Stop | Dina original | Circuit breaker for autonomous systems |
| DRC-110 | Firmware | Dina original | On-chain firmware registry and integrity verification |
| DRC-111 | Smart Wallet | Dina original | Account abstraction with session keys and social recovery |
| DRC-112 | View Keys | Dina original | Selective disclosure for privacy compliance |
| DRC-113 | Relay | Dina original | Mesh relay incentive and routing protocol |

---

## Validator Guide

### Hardware Requirements

**Testnet (minimum):**

| Component | Requirement |
|-----------|-------------|
| CPU | 2 cores, x86_64 or aarch64 |
| RAM | 4 GB |
| Storage | 50 GB SSD |
| Network | 10 Mbps, stable connection |

**Mainnet (recommended):**

| Component | Requirement |
|-----------|-------------|
| CPU | 4+ cores with AVX2 support |
| RAM | 16 GB |
| Storage | 500 GB NVMe SSD |
| Network | 100 Mbps, static IP, 99.9% uptime |

### Running a Validator

```bash
# Install Rust and WASM target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Clone and build
git clone https://github.com/superbigroach/dina-network.git
cd dina-network
cargo build --release

# Start a validator node
./target/release/dina-node --validator --data-dir ./data
```

### Validator Count vs. Finality Speed

| Validators | Finality | Communication Rounds |
|------------|----------|---------------------|
| 3 | ~100ms | 2 rounds (propose + commit) |
| 7 | ~150ms | 2 rounds, more signatures |
| 13 | ~250ms | 2 rounds, network latency dominates |
| 21 | ~400ms | 2 rounds, signature aggregation overhead |

The validator set is currently permissioned. Operators apply to join and are vetted for uptime and infrastructure quality. The goal is to expand to 21+ validators while keeping finality under 500ms.

**Trade-off:** Fewer validators means faster consensus and lower latency, but less decentralization. This is a deliberate design choice for payment and IoT workloads where speed matters more than maximum decentralization. The protocol is designed to scale to 21 validators without architectural changes.

---

## SDKs and Tools

| SDK | Install | Features |
|-----|---------|----------|
| **JavaScript/TypeScript** | `npm install dina-js` | Wallet, client, contracts, channels, DRC-1 tokens, DRC-101 agent wallets |
| **Python** | `pip install dina-network` | Wallet, client, contract interaction |
| **Rust** | `dina-sdk` crate (workspace) | Contract authoring SDK with proc-macros |
| **CLI** | `cargo build --release` | Key management, transfers, queries, contract deployment |

### Developer Portal

[https://dina-developer-portal.web.app](https://dina-developer-portal.web.app)

API reference, SDK documentation, contract examples, and deployment guides.

---

## Testnet

| Property | Value |
|----------|-------|
| Chain ID | `dina-testnet-1` |
| Validators | 3x e2-medium on GCP (us-central1) |
| RPC (JSON-RPC) | `http://35.184.213.248:8545` |
| RPC (REST) | `http://35.184.213.248:8080` |
| WebSocket | `ws://35.184.213.248:8546` |
| Block time | 100ms |
| Finality | ~100ms (3 validators) |
| Faucet | `http://35.184.213.248:8081/faucet` |
| Explorer | `http://35.184.213.248:3000` |

Request testnet USDC:

```bash
curl -X POST http://35.184.213.248:8081/faucet \
  -H "Content-Type: application/json" \
  -d '{"address": "0xYourAddress"}'
```

---

## Repository Structure

```
dina_network/
  crates/
    dina-core/             Core types: Address, Hash, Transaction, Block, Account
    dina-consensus/        TurboBFT consensus (3-21 validators, 100ms finality)
    dina-network/          libp2p networking (gossipsub, Kademlia, mDNS)
    dina-storage/          Persistent storage via redb
    dina-privacy/          Stealth addresses, view keys, encrypted memos, ZK proofs
    dina-channels/         Offline payment channels (~5ms device-to-device)
    dina-relay/            BLE mesh relay for offline settlement
    dina-wasm/             WASM contract runtime (wasmtime)
    dina-rpc/              JSON-RPC + REST API server (axum + jsonrpsee)
    dina-sdk/              Rust SDK for writing contracts
    dina-sdk-macros/       Proc-macros for contract boilerplate
    dina-mcp/              MCP tool server for IoT devices
    dina-bench/            Performance benchmarks
    dina-faucet/           Testnet faucet
    dina-monitoring/       Prometheus metrics export
    dina-explorer/         Block explorer backend
  node/
    dina-node              Full validator/full-node binary
  cli/
    dina                   Command-line wallet and admin tool
  contracts/
    drc1-token/            94 DRC standard reference implementations
    drc2-device-identity/  (each compiles to standalone WASM)
    ...
    drc113-relay/
    bridge-cctp/           8 bridge contract implementations
    bridge-across/
    ...
    bridge-wormhole/
  sdk/
    dina-js/               TypeScript/JavaScript SDK
    dina-py/               Python SDK
  examples/
    hello-world/           Minimal contract example
    escrow/                Escrow contract
    marketplace/           NFT marketplace
    subscription/          Subscription payments
    voting/                DAO voting
  deploy/
    ansible/               Ansible playbooks
    kubernetes/            Kubernetes manifests
    terraform/             Terraform configs
  developer-portal/        Developer documentation site
  explorer/                Block explorer frontend
  faucet-app/              Faucet web UI
  docs/                    Technical documentation
  benches/                 Benchmark harnesses
  tests/                   Integration tests
  genesis.json             Default testnet genesis configuration
```

---

## Building from Source

```bash
# Prerequisites: Rust 1.70+, wasm32-unknown-unknown target
rustup target add wasm32-unknown-unknown

# Build everything
cargo build --release

# Run all tests
cargo test --workspace

# Run clippy
cargo clippy --workspace -- -D warnings

# Run benchmarks
cargo bench --bench throughput
```

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Ensure all tests pass (`cargo test --workspace`)
4. Ensure no clippy warnings (`cargo clippy --workspace -- -D warnings`)
5. Submit a pull request

All smart contracts must include unit tests and follow the `dispatch(state, method, args, caller)` pattern established by `dina-sdk`.

---

## License

MIT
