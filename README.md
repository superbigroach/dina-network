# Dina Network

A Rust-native Layer 1 blockchain for machine-to-machine payments, AI agent transactions, and IoT/robotics commerce.

## Key Features

- **Sub-200ms BFT finality** -- TurboBFT consensus with 3-7 validators
- **USDC-as-gas** -- no native token, fees paid in stablecoins
- **Ed25519 native** -- same keys as Cognitum Seed hardware, SSH, Solana
- **WASM smart contracts** -- write in Rust, 5-25x faster than EVM/Solidity
- **4-layer privacy** -- encrypted memos, stealth addresses, view keys, ZK proofs
- **Offline payments** -- device-to-device payment channels (~5ms local)
- **Mesh relay** -- BLE-based settlement relay (like Apple Find My network)
- **30 DRC standards** -- 18 ERC ports + 12 novel agent/robot/privacy standards
- **Hardware identity** -- Cognitum Seed Ed25519 device attestation

## Architecture

Dina Network is structured as a Cargo workspace with 11 library crates, 2 binary crates, and 30 smart contract crates. All types flow from `dina-core`, which is the source of truth for `Address`, `Hash`, `Transaction`, `Block`, and `Account`.

```
dina-core          foundation types, crypto primitives, genesis
  |
  +-- dina-consensus    TurboBFT consensus engine
  +-- dina-network      libp2p peer-to-peer layer
  +-- dina-storage      redb persistent storage
  +-- dina-privacy      stealth addresses, view keys, encrypted memos
  +-- dina-channels     offline payment channels
  +-- dina-relay        BLE mesh relay protocol
  +-- dina-wasm         wasmtime contract runtime
  +-- dina-rpc          JSON-RPC + REST API (axum/jsonrpsee)
  +-- dina-sdk          contract authoring SDK
  +-- dina-sdk-macros   proc-macros for contract dispatch
```

## Quick Start

### Prerequisites

- Rust 1.70+ with `wasm32-unknown-unknown` target
- (Windows) MinGW-w64 for GNU toolchain -- `dlltool` must be on PATH

### Install the WASM target

```bash
rustup target add wasm32-unknown-unknown
```

### Build

```bash
cargo build
```

### Run a validator node

```bash
cargo run --bin dina-node -- --validator --data-dir ./data
```

### CLI

```bash
cargo run --bin dina -- keygen
cargo run --bin dina -- balance <ADDRESS>
cargo run --bin dina -- status
```

## Project Structure

```
dina_network/
  crates/
    dina-core/           Core types: Address, Hash, Transaction, Block, Account
    dina-consensus/      TurboBFT consensus (3-7 validators, sub-200ms finality)
    dina-network/        libp2p networking layer (gossipsub, Kademlia DHT)
    dina-storage/        Persistent storage via redb
    dina-privacy/        4-layer privacy (stealth addresses, view keys, ZK)
    dina-rpc/            JSON-RPC and REST API server
    dina-wasm/           WASM smart contract runtime (wasmtime)
    dina-sdk/            SDK for writing contracts in Rust
    dina-sdk-macros/     Proc-macros for contract dispatch boilerplate
    dina-channels/       Offline device-to-device payment channels
    dina-relay/          BLE mesh relay for settlement propagation
  node/
    dina-node            Full validator/full-node binary
  cli/
    dina                 Command-line wallet and admin tool
  contracts/
    drc1-token/          Fungible token (ERC-20 port)
    drc2-device-identity/  Hardware device identity attestation
    ...                  30 DRC standard contracts (see table below)
  genesis.json           Default testnet genesis configuration
```

## DRC Standards

### ERC Ports (DRC 1-18)

| DRC | Name | ERC Equivalent | Status |
|-----|------|----------------|--------|
| DRC-1 | Fungible Token | ERC-20 | Implemented |
| DRC-2 | Device Identity | -- (novel) | Implemented |
| DRC-4 | Permit (Gasless Approval) | ERC-2612 | Implemented |
| DRC-5 | Soulbound Token | ERC-5192 | Implemented |
| DRC-6 | NFT | ERC-721 | Implemented |
| DRC-7 | Multi-Token | ERC-1155 | Implemented |
| DRC-8 | Token-Bound Account | ERC-6551 | Implemented |
| DRC-9 | Rental / Lending | ERC-4907 | Implemented |
| DRC-10 | Royalties | ERC-2981 | Implemented |
| DRC-11 | Semi-Fungible Token | ERC-3525 | Implemented |
| DRC-12 | Vault (Yield) | ERC-4626 | Implemented |
| DRC-13 | Compliant Token | ERC-3643 | Implemented |
| DRC-14 | Contract Signature | ERC-1271 | Implemented |
| DRC-15 | Meta-Transactions | ERC-2771 | Implemented |
| DRC-16 | Proxy (Upgradeable) | ERC-1967 | Implemented |
| DRC-17 | Hooks | ERC-777 hooks | Implemented |
| DRC-18 | Scriptable | ERC-5169 | Implemented |

### Novel Agent / Robot / Privacy Standards (DRC 101-113)

| DRC | Name | Purpose | Status |
|-----|------|---------|--------|
| DRC-101 | Agent Wallet | AI agent-owned wallets with spending policies | Implemented |
| DRC-102 | Capability | Delegated permission tokens for agents | Implemented |
| DRC-103 | Service Agreement | Machine-to-machine SLA contracts | Implemented |
| DRC-104 | Swarm | Multi-agent coordination and task distribution | Implemented |
| DRC-105 | Sensor Attestation | IoT sensor data authenticity proofs | Implemented |
| DRC-106 | Data Market | Buy/sell sensor and AI training data | Implemented |
| DRC-107 | Reputation | On-chain agent/device reputation scoring | Implemented |
| DRC-108 | Resource | Compute/bandwidth/storage resource tokens | Implemented |
| DRC-109 | Emergency Stop | Circuit breaker for autonomous systems | Implemented |
| DRC-110 | Firmware | On-chain firmware registry and verification | Implemented |
| DRC-111 | Smart Wallet | Programmable wallet with session keys | Implemented |
| DRC-112 | View Keys | Selective disclosure for privacy compliance | Implemented |
| DRC-113 | Relay | Mesh relay incentive and routing protocol | Implemented |

## Workspace Crates

### Infrastructure Libraries

| Crate | Description |
|-------|-------------|
| `dina-core` | Foundation types, Ed25519 crypto, genesis, USDC accounting |
| `dina-consensus` | TurboBFT Byzantine fault-tolerant consensus engine |
| `dina-network` | libp2p peer discovery, gossipsub, block propagation |
| `dina-storage` | Persistent block and state storage via redb |
| `dina-privacy` | Stealth addresses, encrypted memos, view keys, ZK proofs |
| `dina-rpc` | JSON-RPC 2.0 and REST API server (axum + jsonrpsee) |
| `dina-wasm` | WASM contract execution runtime (wasmtime) |
| `dina-sdk` | Rust SDK for authoring DRC-compatible smart contracts |
| `dina-sdk-macros` | Proc-macros for contract entry points and dispatch |
| `dina-channels` | Offline payment channels for device-to-device transfers |
| `dina-relay` | BLE mesh relay protocol for offline settlement |

### Binaries

| Binary | Description |
|--------|-------------|
| `dina-node` | Full/validator node -- runs consensus, networking, RPC |
| `dina` (CLI) | Command-line tool for key management, transfers, queries |

### Smart Contracts

All 30 DRC standard reference implementations live in `contracts/`. Each compiles to a standalone WASM module using the `dina-sdk` and `dina-sdk-macros` crates.

## License

MIT

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Ensure all tests pass (`cargo test --workspace`)
4. Ensure no clippy warnings (`cargo clippy --workspace -- -D warnings`)
5. Submit a pull request

All smart contracts must include unit tests and follow the `dispatch(state, method, args, caller)` pattern established by `dina-sdk`.
