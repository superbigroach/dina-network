# Changelog

All notable changes to the Dina Network project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Staking and slashing for validators
- USDC bridge (federated, then CCTP integration)
- Zero-knowledge proof integration (Layer 4 privacy)
- Prometheus metrics export via dina-monitoring
- Light client implementation
- Block explorer web application
- NAT traversal for P2P networking
- Formal verification of TurboBFT consensus

## [0.1.0] - 2026-03-27

### Added

#### Core Blockchain (`dina-core`)
- Foundation types: `Address` ([u8; 32], SHA-256 of Ed25519 pubkey), `Hash` ([u8; 32]), `Sig64` newtype for 64-byte signatures
- `Transaction` enum with four variants: `Transfer`, `DeployContract`, `CallContract`, `RegisterDevice`
- `Block` and `BlockHeader` with Merkle tree transaction roots
- `Account` type with USDC balance (u64 micro-units), nonce, code hash, and storage root
- `AccountState` in-memory state manager with transfer, credit, debit, and nonce operations
- Ed25519 cryptographic primitives: keypair generation, signing, verification, address derivation
- Merkle tree implementation for block transaction roots
- `DeviceIdentity` and `DeviceAttestation` types for hardware device registration
- `WitnessProof` for hardware-witnessed transactions
- `FeeSchedule` for configurable transaction fee minimums
- Comprehensive error types via `thiserror`
- Genesis block creation (unsigned and signed variants)

#### Consensus (`dina-consensus`)
- TurboBFT consensus engine for 3-7 validators with sub-200ms finality
- Four-phase protocol: Propose, Prevote, Precommit, Commit
- `ConsensusConfig` with configurable validator keys, block time, and timeout
- `ConsensusState` tracking height, round, step, locked block, and locked round
- `Proposal` with Ed25519 signatures and leader verification
- `Vote` (Prevote/Precommit) with signature verification and quorum detection
- `VoteSet` for collecting and deduplicating votes with quorum threshold (`ceil(2n/3)`)
- `CommitCertificate` proving block finality with aggregated precommit signatures
- Round-robin `LeaderSchedule`: `validators[(height + round) % n]`
- `ViewChange` and `ViewChangeCollector` for leader rotation on timeout
- Safety: locked block persists across rounds within same height
- Liveness: timeout-triggered view changes rotate leader

#### Networking (`dina-network`)
- libp2p-based peer-to-peer networking layer
- Transport: TCP with Noise encryption and Yamux multiplexing
- Discovery: mDNS for local networks, Kademlia DHT for wide-area
- Block and transaction propagation via GossipSub
- Peer identity using Ed25519 keypairs
- Message types for consensus, blocks, and transactions

#### Storage (`dina-storage`)
- `DinaDB` persistent storage engine wrapping redb
- 8 storage tables: accounts, blocks, block_hashes, transactions, contract_code, contract_storage, device_registry, state_metadata
- Account read/write with bincode serialization
- Block storage with height-based and hash-based retrieval
- Block hash reverse index for O(1) hash-to-height lookup
- Latest block height tracking in state metadata
- Automatic schema migration system
- In-memory database support for testing via `open_in_memory()`

#### Privacy (`dina-privacy`)
- **Encrypted Memos**: X25519 ECDH key agreement + XChaCha20-Poly1305 AEAD
  - Ephemeral keypair per memo for forward secrecy
  - Symmetric key derived via SHA-256(shared_secret)
  - 24-byte random nonce, authenticated encryption with tamper detection
- **Stealth Addresses**: EIP-5564 adapted for X25519 keys
  - `StealthMetaAddress` with scan and spend public keys
  - One-time address derivation via ECDH + double SHA-256
  - Recipient detection using scan secret
  - Spending key derivation for fund recovery
- **View Keys / Permissions**: Granular key permission system
  - 7 permission types: FullAccess, ViewOnly, TransferOnly, ContractCallOnly, DeviceControl, SessionKey, Custom
  - `PermissionSet` with add, remove, rotate key operations
  - Recursive permission checking with session key expiration
  - `AuthorizedKey` with label, creation time, and last-used tracking

#### WASM Runtime (`dina-wasm`)
- `WasmRuntime` execution engine powered by wasmtime 29
- Fuel-based gas metering (1 gas = 1 micro-USDC)
- Contract deployment: WASM validation, compilation, `__init` call, deterministic address generation
- Contract execution: `__dispatch` method routing with memory allocation
- Host functions: storage read/write, USDC transfer, caller/self address, block context, event emission, SHA-256, Ed25519 verify, cross-contract calls
- `SandboxLimits`: configurable max memory (16 MiB), max gas (10M), max call depth (10)
- `ExecutionResult` with return value, gas used, events, pending transfers, storage overlay
- Gas cost table: storage write (500), Ed25519 verify (300), cross-contract call (1000)

#### RPC Server (`dina-rpc`)
- **JSON-RPC 2.0** server via jsonrpsee on port 8545
  - 14 methods: `dina_sendTransaction`, `dina_getBalance`, `dina_getAccount`, `dina_getBlock`, `dina_getBlockByHash`, `dina_getLatestBlock`, `dina_getTransaction`, `dina_getDevice`, `dina_networkInfo`, `dina_chainId`, `dina_estimateGas`, `dina_gasPrice`, `dina_txPoolStatus`, `dina_pendingTransactions`
- **REST API** server via axum on port 8080
  - 7 endpoints: `GET /health`, `GET /v1/balance/{address}`, `GET /v1/block/latest`, `GET /v1/block/{height}`, `POST /v1/transaction`, `GET /v1/device/{pubkey}`, `GET /v1/peers`
- **WebSocket** subscriptions via tokio broadcast channels
  - 3 topics: `NewBlocks`, `NewTransactions`, `ConsensusUpdates`
  - 256-event buffer per topic
- Gas estimator for all transaction types
- Transaction pool status API
- Rate limiting middleware
- Shared `NodeState` with async RwLock-protected fields

#### Payment Channels (`dina-channels`)
- `PaymentChannel` with bidirectional balance tracking
- Channel lifecycle: Opening, Open, Closing, Closed, Disputed
- Off-chain state updates with Ed25519 dual signatures (~5ms per update)
- Cooperative close: immediate settlement with dual-signed final state
- Unilateral close: challenge period (100 blocks) with state submission
- Dispute resolution: challenge with higher-sequence-number state
- Finalization after challenge period expiry
- Conservation invariant: `balance_a + balance_b == total_locked`
- Deterministic channel ID: `SHA-256(party_a || party_b || timestamp)`

#### BLE Mesh Relay (`dina-relay`)
- `RelayBlob` compact settlement structure (max 200 bytes, BLE + QR compatible)
- Dual Ed25519 signatures (sender + receiver)
- TTL-based expiry (default: 5 minutes) and max hop count (default: 10)
- Relay fee field for incentivizing relay nodes
- `RelayBroadcaster` for BLE advertising
- `RelayScanner` for BLE scanning with company ID filtering
- `RelaySubmitter` for on-chain settlement submission
- `RelayStats` for tracking relay performance
- QR code encoding/decoding for relay blob transfer
- Hop counter with saturation arithmetic

#### Contract SDK (`dina-sdk` + `dina-sdk-macros`)
- Contract authoring SDK for Rust-to-WASM compilation
- `dina_contract` proc-macro for contract state definition
- `dina_impl` proc-macro for method dispatch generation
- `#[init]` attribute for constructor methods
- `#[view]` attribute for read-only methods
- `#[payable]` attribute for methods accepting USDC
- Host function bindings: storage, transfer, caller, events
- `TestRuntime` harness for contract unit testing
- Prelude module for convenient imports

#### MCP Integration (`dina-mcp`)
- MCP tool server for Cognitum Seed devices
- 12 tools: `dina/transfer`, `dina/balance`, `dina/deploy_contract`, `dina/call_contract`, `dina/register_device`, `dina/verify_device`, `dina/channel_open`, `dina/channel_pay`, `dina/channel_close`, `dina/peers`, `dina/block_info`, `dina/network_status`
- JSON Schema definitions for all tool inputs
- `McpToolCall` and `McpToolResult` types with success/failure semantics
- Tool handler routing and device context management

#### DRC Smart Contract Standards (30 standards)
- **ERC Ports (18 standards)**: DRC-1 (Fungible Token/ERC-20), DRC-2 (Device Identity), DRC-4 (Permit/ERC-2612), DRC-5 (Soulbound/ERC-5192), DRC-6 (NFT/ERC-721), DRC-7 (Multi-Token/ERC-1155), DRC-8 (Token-Bound Account/ERC-6551), DRC-9 (Rental/ERC-4907), DRC-10 (Royalties/ERC-2981), DRC-11 (Semi-Fungible/ERC-3525), DRC-12 (Vault/ERC-4626), DRC-13 (Compliant/ERC-3643), DRC-14 (Contract Signature/ERC-1271), DRC-15 (Meta-Transactions/ERC-2771), DRC-16 (Proxy/ERC-1967), DRC-17 (Hooks/ERC-777), DRC-18 (Scriptable/ERC-5169)
- **Novel Standards (12 standards)**: DRC-101 (Agent Wallet), DRC-102 (Capability), DRC-103 (Service Agreement), DRC-104 (Swarm), DRC-105 (Sensor Attestation), DRC-106 (Data Market), DRC-107 (Reputation), DRC-108 (Resource), DRC-109 (Emergency Stop), DRC-110 (Firmware), DRC-111 (Smart Wallet), DRC-112 (View Keys), DRC-113 (Relay)

#### Node Binary (`dina-node`)
- Full validator/full-node binary with CLI argument parsing (clap)
- Ed25519 identity generation and persistence
- Genesis configuration loading (built-in default + custom JSON)
- Genesis block creation with initial account balances
- Mempool with fee-ordered BTreeMap (highest-fee-first), 10K capacity, 1-hour expiry
- Consensus engine integration with mempool feeder
- Block commitment with database persistence
- Periodic mempool maintenance (expired transaction cleanup)
- Graceful shutdown on SIGINT/SIGTERM
- Structured logging via tracing with configurable log levels
- Cross-platform home directory detection (Windows + Unix)

#### CLI Binary (`dina`)
- Command-line wallet and administration tool
- `keygen` -- Generate Ed25519 keypair
- `balance` -- Query account balance
- `status` -- Query network status
- `transfer` -- Send USDC
- `deploy` -- Deploy a WASM contract

#### Infrastructure
- `dina-explorer` crate for block explorer functionality
- `dina-bridge` crate for cross-chain bridging
- `dina-bench` crate for benchmarking with criterion
- `dina-faucet` crate for testnet faucet
- `dina-monitoring` crate for metrics export

#### SDKs
- TypeScript/JavaScript SDK (`sdk/dina-js/`)
- Python SDK (`sdk/dina-py/`)

#### DevOps
- `Dockerfile` for building the node binary
- `Dockerfile.cli` for building the CLI binary
- `docker-compose.yml` for multi-validator testnet
- Default testnet `genesis.json` with 1B USDC faucet

#### Examples
- `hello-world` -- Minimal contract example
- `escrow` -- Two-party escrow contract
- `voting` -- On-chain voting contract
- `subscription` -- Recurring payment subscription
- `marketplace` -- P2P marketplace with listings

#### Documentation
- `CLAUDE.md` -- Developer instructions and project conventions
- `README.md` -- Project overview with architecture diagram and DRC table
- `docs/ARCHITECTURE.md` -- Full system architecture with data flow diagrams
- `docs/API_REFERENCE.md` -- Complete API documentation (JSON-RPC, REST, WS, MCP)
- `docs/DRC_STANDARDS.md` -- All 30 DRC standards with interfaces and interaction map
- `docs/VALIDATOR_GUIDE.md` -- Validator setup, monitoring, and operations guide
- `docs/DEVELOPER_GUIDE.md` -- Developer onboarding, SDK quickstart, contract tutorial
- `docs/WHITEPAPER.md` -- Technical whitepaper with consensus spec and security analysis
- `docs/CHANGELOG.md` -- This changelog

[Unreleased]: https://github.com/lucilla-app/dina_network/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lucilla-app/dina_network/releases/tag/v0.1.0
