# CLAUDE.md

## Project Overview

Dina Network is a Rust-native Layer 1 blockchain for machine-to-machine payments, AI agent transactions, and IoT/robotics commerce. It uses USDC-as-gas (no native token), Ed25519 cryptography, TurboBFT consensus, and WASM smart contracts.

## Build Commands

```bash
cargo check                       # Type check all crates
cargo build                       # Build entire workspace
cargo test --workspace            # Run all tests
cargo clippy --workspace -- -D warnings  # Lint all crates
cargo build --bin dina-node       # Build node binary only
cargo build --bin dina            # Build CLI binary only
cargo build -p drc1-token --target wasm32-unknown-unknown  # Build a single contract to WASM
```

## Architecture

- **Workspace**: 11 library crates in `crates/`, 2 binaries in `node/` and `cli/`, 30 contract crates in `contracts/`
- **Type authority**: All core types flow from `dina-core` (Address, Hash, Transaction, Block, Account)
- **Contract pattern**: `dispatch(state, method, args, caller)` -- all contracts use this entry point
- **Contract target**: Contracts compile to WASM via `crate-type = ["cdylib"]` with `wasm32-unknown-unknown`

## Critical Rules

### Types
- `Address` is `[u8; 32]` everywhere (SHA-256 of Ed25519 public key)
- `Hash` is `[u8; 32]` everywhere (SHA-256 of content)
- Use `Sig64` newtype for `[u8; 64]` signature fields (provides serde compatibility via `serde-big-array`)
- USDC amounts are `u64` in **micro-units** (1 USDC = 1,000,000 micro-USDC)

### Rust Edition and Toolchain
- Edition **2021** (not 2024)
- MinGW toolchain on Windows (not MSVC) -- `dlltool` must be on PATH
- WASM target: `wasm32-unknown-unknown` (install via `rustup target add wasm32-unknown-unknown`)

### Code Patterns
- Never use `unwrap()` in library crates -- return `Result` with descriptive errors
- All public APIs must have doc comments
- Smart contracts must be `no_std` compatible
- Use `thiserror` for error types in library crates, `anyhow` in binaries only

### Testing
- Every crate must have unit tests
- Integration tests go in `tests/` at workspace root
- Contract tests use the SDK's `TestRuntime` harness
- Run `cargo test --workspace` before submitting any PR

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ed25519-dalek` | 2.x | All Ed25519 signing and verification |
| `libp2p` | 0.54 | Peer-to-peer networking |
| `wasmtime` | 29 | WASM contract execution runtime |
| `redb` | 2.x | Embedded key-value storage |
| `axum` | 0.8 | HTTP/REST API framework |
| `jsonrpsee` | 0.25 | JSON-RPC 2.0 server |
| `serde` | 1.x | Serialization (JSON, bincode) |
| `serde-big-array` | 0.5 | Serde support for `[u8; 64]` signatures |
| `sha2` | 0.10 | SHA-256 hashing |
| `tokio` | 1.x | Async runtime |

## Project Layout

```
crates/
  dina-core/         Foundation types, crypto, genesis config
  dina-consensus/    TurboBFT consensus engine
  dina-network/      libp2p networking layer
  dina-storage/      redb persistent storage
  dina-privacy/      Stealth addresses, view keys, encrypted memos, ZK
  dina-rpc/          JSON-RPC + REST server
  dina-wasm/         WASM contract runtime
  dina-sdk/          Contract authoring SDK
  dina-sdk-macros/   Proc-macros for contract dispatch
  dina-channels/     Offline payment channels
  dina-relay/        BLE mesh relay protocol
node/                Validator/full-node binary (dina-node)
cli/                 CLI wallet and admin tool (dina)
contracts/           30 DRC standard reference implementations
genesis.json         Testnet genesis configuration
```

## DRC Contract Standards

There are 30 DRC standards: DRC-1 through DRC-18 (ERC ports, skipping DRC-3) and DRC-101 through DRC-113 (novel agent/robot/privacy standards). Each contract crate follows the naming convention `drc{N}-{name}` and compiles to a WASM module.

## Common Tasks

### Add a new DRC contract
1. Create `contracts/drc{N}-{name}/` with `Cargo.toml` and `src/lib.rs`
2. Set `crate-type = ["cdylib"]` and depend on `dina-sdk`
3. Implement `dispatch(state, method, args, caller) -> Result<Vec<u8>>`
4. Add the crate to the workspace `members` in root `Cargo.toml`
5. Write tests using `dina-sdk::TestRuntime`

### Run a local testnet
```bash
cargo run --bin dina-node -- --validator --data-dir ./data --genesis genesis.json
```

### Generate a keypair
```bash
cargo run --bin dina -- keygen
```
