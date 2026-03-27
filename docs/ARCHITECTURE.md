# Dina Network Architecture

## Overview

Dina Network is a Rust-native Layer 1 blockchain purpose-built for machine-to-machine payments, AI agent transactions, and IoT/robotics commerce. It uses USDC-as-gas (no native token), Ed25519 cryptography, TurboBFT consensus, and WASM smart contracts.

The codebase is structured as a Cargo workspace with 14 library crates, 2 binary crates (node and CLI), and 30 smart contract crates.

## System Diagram

```
                           +-----------------------------+
                           |        External Clients      |
                           |  (SDKs, Wallets, Cognitum)   |
                           +-------+-------+-------+------+
                                   |       |       |
                          JSON-RPC |  REST |  WS   | MCP
                                   |       |       |
                     +-------------+-------+-------+----------+
                     |              dina-rpc                    |
                     |  axum REST | jsonrpsee RPC | WebSocket  |
                     +------+------+------+-------+---+--------+
                            |             |           |
             +--------------+---+  +------+------+ +--+----------+
             |   dina-mcp       |  | dina-wasm   | | dina-explorer|
             | (MCP tool server)|  | (wasmtime   | | (block       |
             | 12 tools for     |  |  contract   | |  explorer)   |
             | Cognitum Seeds   |  |  runtime)   | |              |
             +------------------+  +------+------+ +--------------+
                                          |
    +-------------------------------------+-----------------------------+
    |                        dina-core                                  |
    |  Address | Hash | Transaction | Block | Account | Crypto | Merkle |
    +---+-------+-------+-------+-------+-------+-------+---------+----+
        |       |       |       |       |       |       |         |
   +----+--+ +--+---+ +-+-----+ +------+-+ +---+----+ ++--------++
   | dina- | | dina-| | dina- | | dina-  | | dina-  | | dina-    |
   |consen-| | net- | |storag-| |privacy | |channel-| | relay    |
   | sus   | | work | | e     | |        | | s      | | (BLE     |
   |       | |      | |       | |        | |        | |  mesh)   |
   +-------+ +------+ +-------+ +--------+ +--------+ +----------+
   TurboBFT  libp2p    redb     stealth    payment     settlement
   3-7 vals  gossipsub tables   addresses  channels    propagation
   <200ms    Kademlia  8 tables view keys  offline     BLE broadcast
             mDNS               enc memos  device-to-  QR code
                                ZK proofs  device      relay fees

   +------------------+   +------------------+
   | dina-sdk         |   | dina-sdk-macros  |
   | Contract SDK     |   | #[dina_contract] |
   | types, host,     |   | #[dina_impl]     |
   | storage, prelude |   | #[view] #[init]  |
   +------------------+   +------------------+

   +------------------+   +------------------+   +------------------+
   | dina-bench       |   | dina-faucet      |   | dina-monitoring  |
   | Benchmarks       |   | Testnet faucet   |   | Metrics export   |
   +------------------+   +------------------+   +------------------+

   +------------------+
   | dina-bridge      |
   | Cross-chain      |
   | bridging         |
   +------------------+
```

## Data Flow: Transaction Lifecycle

### 1. Transaction Submission

```
Client                  RPC Server              Mempool
  |                        |                       |
  |-- dina_sendTransaction |                       |
  |   (signed tx hex) --->|                        |
  |                       |-- validate tx -------->|
  |                       |   (sig, nonce, fee)    |
  |                       |                        |-- add to BTreeMap
  |                       |                        |   (ordered by fee)
  |<-- tx_hash -----------|                        |
```

### 2. Consensus (TurboBFT)

```
Mempool        Leader Validator       Other Validators
  |                  |                      |
  |-- get_pending -->|                      |
  |   (top N txs)    |                      |
  |                  |-- Propose ---------->|
  |                  |   (block with txs)   |
  |                  |                      |-- verify proposal
  |                  |                      |-- verify leader
  |                  |<---- Prevote --------|
  |                  |   (block_hash, sig)  |
  |                  |                      |
  |                  |   [2/3+ prevotes?]   |
  |                  |   Lock on block      |
  |                  |                      |
  |                  |<---- Precommit ------|
  |                  |   (block_hash, sig)  |
  |                  |                      |
  |                  |   [2/3+ precommits?] |
  |                  |   COMMIT + cert      |
```

### 3. Block Execution

```
Committed Block         WASM Runtime           Account State
     |                      |                       |
     |-- for each tx:       |                       |
     |   Transfer --------->|                       |
     |                      |-- deduct fee -------->|
     |                      |-- transfer amount --->|
     |                      |-- increment nonce --->|
     |                      |                       |
     |   CallContract ----->|                       |
     |                      |-- load contract WASM  |
     |                      |-- set fuel (gas)      |
     |                      |-- call __dispatch()   |
     |                      |-- read/write storage  |
     |                      |-- emit events         |
     |                      |-- pending transfers ->|
     |                      |                       |
     |   DeployContract --->|                       |
     |                      |-- compile WASM module |
     |                      |-- call __init()       |
     |                      |-- store code + state  |
     |                      |-- return contract addr|
```

### 4. State Persistence

```
Account State           DinaDB (redb)
     |                      |
     |-- set_account ------>|  ACCOUNTS table
     |                      |  key: [u8; 32] address
     |                      |  val: bincode(Account)
     |                      |
     |-- store_block ------>|  BLOCKS table
     |                      |  key: u64 height
     |                      |  val: bincode(Block)
     |                      |
     |                      |  BLOCK_HASHES table
     |                      |  key: [u8; 32] hash
     |                      |  val: u64 height
     |                      |
     |                      |  STATE_METADATA table
     |                      |  key: "latest_block_height"
     |                      |  val: u64 (le bytes)
```

## Crate Dependency Graph

```
dina-core  (foundation: Address, Hash, Transaction, Block, Account, Crypto)
   ^
   |
   +--- dina-consensus   depends on: dina-core, ed25519-dalek, sha2, chrono, tokio
   |
   +--- dina-network     depends on: dina-core, libp2p, tokio
   |
   +--- dina-storage     depends on: dina-core, redb, bincode, tempfile
   |
   +--- dina-privacy     depends on: dina-core, x25519-dalek, chacha20poly1305, sha2
   |
   +--- dina-channels    depends on: dina-core, ed25519-dalek, sha2, chrono
   |
   +--- dina-relay       depends on: dina-core, ed25519-dalek, sha2, bincode
   |
   +--- dina-wasm        depends on: dina-core, wasmtime, sha2
   |
   +--- dina-rpc         depends on: dina-core, dina-wasm, axum, jsonrpsee, tokio
   |
   +--- dina-sdk         depends on: dina-sdk-macros (proc-macro crate)
   |
   +--- dina-mcp         depends on: dina-core, serde_json
   |
   +--- dina-explorer    depends on: dina-core, dina-storage, dina-rpc
   |
   +--- dina-bridge      depends on: dina-core
   |
   +--- dina-bench       depends on: dina-core, criterion
   |
   +--- dina-faucet      depends on: dina-core, dina-rpc
   |
   +--- dina-monitoring  depends on: dina-core
```

## Network Topology

### Node Types

| Node Type | Consensus | Block Storage | RPC | P2P |
|-----------|-----------|--------------|-----|-----|
| Validator | Active (proposes + votes) | Full chain | Yes | Full mesh |
| Full Node | Passive (receives blocks) | Full chain | Yes | Gossip |
| Light Client | None | Headers only | Query only | Minimal |
| Cognitum Seed | None | None | MCP client | BLE mesh |

### Peer-to-Peer Layer (libp2p)

The network layer uses libp2p with the following protocols:

- **Transport**: TCP with Noise encryption and Yamux multiplexing
- **Discovery**: mDNS for local networks, Kademlia DHT for wide-area
- **Block/TX propagation**: GossipSub with topic-based publishing
- **Peer identity**: Ed25519 keypairs (same as on-chain identity)

```
Validator A <--TCP+Noise--> Validator B
     |                          |
     +--gossipsub: /blocks------+
     +--gossipsub: /txs---------+
     +--gossipsub: /consensus---+
     |                          |
     +--kademlia: DHT discovery-+
     +--mdns: local discovery---+
```

### Cognitum Seed Connectivity

Cognitum Seeds (hardware devices) connect via two paths:

1. **Online**: JSON-RPC or MCP protocol over TCP/TLS to any full node
2. **Offline**: BLE mesh relay for payment channel settlements

```
Cognitum Seed A                    Cognitum Seed B
     |                                  |
     |-- BLE broadcast (RelayBlob) ---->|
     |                                  |
     |   [no internet needed]           |
     |                                  |
     |   When online:                   |
     +-- MCP tool call --------------> Full Node --> On-chain settlement
```

## Storage Architecture

### redb Tables

Dina uses redb, an embedded key-value store, with 8 tables:

| Table | Key Type | Value Type | Purpose |
|-------|----------|------------|---------|
| `accounts` | `[u8; 32]` (address) | bincode(Account) | Account balances, nonces, code hashes |
| `blocks` | `u64` (height) | bincode(Block) | Full block data by height |
| `block_hashes` | `[u8; 32]` (hash) | `u64` (height) | Reverse index: hash to height |
| `transactions` | `[u8; 32]` (tx hash) | bincode(Transaction) | Transaction lookup by hash |
| `contract_code` | `[u8; 32]` (code hash) | `[u8]` (WASM bytes) | Deployed contract bytecode |
| `contract_storage` | `[u8]` (addr+slot) | `[u8]` (value) | Per-contract key-value storage |
| `device_registry` | `[u8; 32]` (device addr) | bincode(DeviceIdentity) | Registered device records |
| `state_metadata` | `&str` (key name) | `[u8]` (value) | Schema version, latest height |

### State Management

Account state is managed both in-memory (for fast consensus) and on-disk (for persistence):

- **In-memory**: `AccountState` (HashMap-based) for consensus execution
- **On-disk**: `DinaDB` (redb-backed) for crash recovery and historical queries
- **Synchronization**: After each committed block, the in-memory state is flushed to redb

## Consensus Architecture (TurboBFT)

### Design

TurboBFT is a pipelined BFT consensus protocol for 3-7 validators, based on Tendermint/HotStuff principles. It achieves sub-200ms finality for small validator sets.

### State Machine

```
                    +----------+
            +------>| Propose  |<-------+
            |       +----+-----+        |
            |            |              |
            |    leader creates block   |
            |            |              |
            |       +----v-----+        |
            |       | Prevote  |        |
            |       +----+-----+        |
            |            |              |
            |    2/3+ prevotes?         |
            |     yes: lock block       |
            |            |              | timeout:
            |       +----v-----+        | view change
            |       |Precommit |--------+
            |       +----+-----+
            |            |
            |    2/3+ precommits?
            |     yes: commit
            |            |
            |       +----v-----+
            +-------| Commit   |
                    +----------+
                 advance height
```

### Key Properties

- **Safety**: A locked block persists across rounds within the same height. A conflicting proposal is only accepted if its round exceeds the locked round.
- **Liveness**: Timeouts trigger view changes that rotate the leader via round-robin: `validators[(height + round) % n]`.
- **Finality**: Once 2/3+ precommit votes are collected, the block is final. A `CommitCertificate` proves finality with the aggregated signatures.
- **Quorum**: `ceil(2n/3)` validators must agree. For 3 validators, quorum is 2. For 7, quorum is 5.

### Timing

| Parameter | Default | Description |
|-----------|---------|-------------|
| `block_time_ms` | 2000 | Target block production interval |
| `timeout_ms` | 10000 | Round timeout before view change |
| Check interval | 100-200ms | Polling interval within consensus loop |

## Privacy Architecture (4 Layers)

### Layer 1: Encrypted Memos

Transactions can carry encrypted memos visible only to the recipient.

- **Algorithm**: X25519 ECDH key agreement + XChaCha20-Poly1305 AEAD
- **Flow**: Sender generates ephemeral X25519 keypair, performs ECDH with recipient's public key, derives a symmetric key via SHA-256, encrypts with XChaCha20-Poly1305
- **Properties**: Forward secrecy (new ephemeral key per memo), authenticated encryption (tamper detection)

### Layer 2: Stealth Addresses

One-time addresses for unlinkable payments, adapted from EIP-5564.

- **Meta-address**: Recipient publishes `(scan_pubkey, spend_pubkey)` as X25519 keys
- **Derivation**: Sender generates ephemeral keypair, computes `address = SHA-256(SHA-256(ECDH(ephemeral, scan_pubkey) || spend_pubkey))`
- **Detection**: Recipient scans the chain using their scan secret to identify payments
- **Spending**: Recipient derives `spending_key = SHA-256(ECDH(scan_secret, ephemeral_pubkey) || spend_secret)`

### Layer 3: View Keys (DRC-112)

Selective disclosure for compliance. Key holders can grant view-only access to specific auditors or regulators without exposing spending keys.

- **Permission types**: FullAccess, ViewOnly, TransferOnly, ContractCallOnly, DeviceControl, SessionKey, Custom
- **Session keys**: Time-limited keys with nested permission restrictions
- **Key rotation**: Keys can be rotated while preserving permission structure

### Layer 4: Zero-Knowledge Proofs

Planned for mainnet. Will enable private transfers where amounts and participants are hidden from validators while preserving verifiability.

## Payment Channel Architecture

### Channel Lifecycle

```
    Open                  Off-chain Updates           Close
  (on-chain)              (device-to-device)         (on-chain)
     |                         |                        |
  lock funds              signed state updates      settle balances
  A: 1M, B: 1M           seq 1: A=900K, B=1.1M    final: A=850K, B=1.15M
                          seq 2: A=850K, B=1.15M
                          (~5ms per update)
```

### Close Modes

1. **Cooperative**: Both parties sign the final state. Immediate settlement, no challenge period.
2. **Unilateral**: One party submits a signed state. Starts a 100-block challenge period.
3. **Disputed**: Counter-party submits a newer state (higher sequence number) during challenge.
4. **Finalized**: After challenge period expires, the latest submitted state is settled.

### Invariants

- `balance_a + balance_b == total_locked` (conservation of funds)
- Sequence numbers are monotonically increasing
- Both parties must sign every state update
- Challenge states must have strictly higher sequence numbers

## MCP Integration Architecture

The Model Context Protocol (MCP) integration allows Cognitum Seed devices to interact with the Dina Network through 12 standardized tools:

```
Cognitum Seed (AI Agent)
     |
     |-- MCP Tool Call: dina/transfer
     |-- MCP Tool Call: dina/balance
     |-- MCP Tool Call: dina/deploy_contract
     |-- MCP Tool Call: dina/call_contract
     |-- MCP Tool Call: dina/register_device
     |-- MCP Tool Call: dina/verify_device
     |-- MCP Tool Call: dina/channel_open
     |-- MCP Tool Call: dina/channel_pay
     |-- MCP Tool Call: dina/channel_close
     |-- MCP Tool Call: dina/peers
     |-- MCP Tool Call: dina/block_info
     |-- MCP Tool Call: dina/network_status
     |
     v
  dina-mcp server
     |
     v
  dina-rpc (JSON-RPC/REST)
     |
     v
  Dina Network (consensus + execution)
```

Each tool accepts JSON input conforming to a declared JSON Schema and returns a `McpToolResult` with success/failure status, data payload, and optional error message.

## Binary Architecture

### dina-node

The validator/full-node binary (`node/src/main.rs`) orchestrates all subsystems:

1. Parse CLI arguments (clap)
2. Initialize tracing/logging
3. Load or generate Ed25519 node identity
4. Open redb database
5. Load genesis configuration
6. Initialize genesis block and accounts
7. Create shared `NodeState` for RPC
8. Initialize mempool (BTreeMap ordered by fee)
9. Initialize WASM runtime (wasmtime with fuel metering)
10. Start JSON-RPC server (port 8545) and REST server (port 8080)
11. If validator: start TurboBFT consensus loop with mempool feeder
12. Start periodic mempool maintenance (expire old transactions)
13. Wait for SIGINT/SIGTERM for graceful shutdown

### dina (CLI)

The CLI binary provides key management, balance queries, and transaction submission:

- `dina keygen` -- Generate a new Ed25519 keypair
- `dina balance <ADDRESS>` -- Query account balance
- `dina status` -- Query network status
- `dina transfer` -- Send USDC
- `dina deploy` -- Deploy a contract

## Gas and Fee Model

### USDC-as-Gas

Dina has no native token. All fees are paid in USDC (6 decimal places, stored as `u64` micro-units).

- 1 gas unit = 1 micro-USDC (0.000001 USDC)
- Gas is metered using wasmtime's fuel mechanism

### Gas Cost Table

| Operation | Gas Cost | USDC Cost |
|-----------|----------|-----------|
| Base WASM instruction | 1 | 0.000001 |
| Memory read (host) | 5 | 0.000005 |
| Memory write (host) | 5 | 0.000005 |
| Storage read | 100 | 0.000100 |
| Storage write | 500 | 0.000500 |
| USDC transfer | 200 | 0.000200 |
| Cross-contract call | 1000 | 0.001000 |
| SHA-256 hash | 50 | 0.000050 |
| Ed25519 verify | 300 | 0.000300 |
| Emit event | 100 | 0.000100 |

### Runtime Limits

| Limit | Default |
|-------|---------|
| Max gas per call | 10,000,000 (10 USDC) |
| Max WASM memory | 16 MiB |
| Max call depth | 10 |
| Mempool size | 10,000 transactions |
| Transaction expiry | 1 hour |
