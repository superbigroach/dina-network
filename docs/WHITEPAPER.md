# Dina Network: A Machine-Native Blockchain for Agent and Robotics Payments

## Abstract

Dina Network is a Rust-native Layer 1 blockchain designed from the ground up for machine-to-machine payments, AI agent transactions, and IoT commerce. Unlike general-purpose blockchains that treat device interactions as an afterthought, Dina makes hardware identity, offline payments, and autonomous agent wallets first-class primitives. The protocol achieves sub-200ms finality through TurboBFT consensus with 3-7 validators, uses USDC as gas (eliminating native token volatility), and executes WASM smart contracts at 5-25x the speed of EVM/Solidity. A four-layer privacy architecture provides encrypted memos, stealth addresses, view keys, and zero-knowledge proofs. Payment channels enable offline device-to-device transfers at approximately 5ms latency, while a BLE mesh relay network propagates settlements across disconnected environments. Thirty DRC smart contract standards -- 18 ERC ports and 12 novel standards for agents, robots, and privacy -- provide a comprehensive toolkit for building the machine economy.

## 1. Introduction: Why Machines Need Their Own Blockchain

### The Problem

The machine economy is arriving faster than blockchain infrastructure can support it. AI agents are making autonomous purchasing decisions. Fleets of robots coordinate to complete physical tasks. IoT sensors generate data that has real economic value. These systems need to transact, and they need to do so at machine speed.

Existing blockchains fail these use cases in several ways:

1. **Latency**: Ethereum's 12-second block time and 12-minute practical finality is unacceptable for robotic actuators that need sub-second confirmation.

2. **Cost unpredictability**: Native token volatility means the cost of a robot's fuel purchase can fluctuate 20% between the decision to buy and the transaction confirmation.

3. **No hardware identity**: Blockchains treat all accounts identically. There is no way to distinguish a verified hardware device from a software impersonator, yet this distinction is critical for IoT data authenticity.

4. **No offline capability**: Factory floors, warehouses, and agricultural environments frequently lack reliable internet. Machines in these environments cannot use blockchain-based payments.

5. **Agent wallet limitations**: AI agents need wallets with spending caps, allowlists, and time-limited session keys. Standard EOA accounts offer none of these controls.

### The Solution

Dina Network addresses each of these gaps:

- **Sub-200ms finality** through TurboBFT with a small, trusted validator set
- **USDC-as-gas** for cost-predictable transactions (no native token)
- **Hardware identity** via Ed25519 device attestation (same keys as Cognitum Seed hardware)
- **Offline payments** via bidirectional payment channels (~5ms device-to-device)
- **Agent wallets** with programmable spending policies (DRC-101)
- **BLE mesh relay** for settlement propagation in disconnected environments

## 2. Design Principles

### 2.1 Stablecoin-Native

Dina uses USDC (6 decimal places) as the sole unit of value. There is no native token. Transaction fees, contract payments, staking (planned), and all economic activity is denominated in USDC. This eliminates gas token volatility and simplifies machine-to-machine pricing.

### 2.2 Hardware-First Identity

Every address is an Ed25519 public key hash, chosen because Ed25519 is the native key format of:
- Cognitum Seed hardware devices
- SSH infrastructure
- Solana and other high-performance chains
- FIDO2/WebAuthn

This means hardware devices generate blockchain-compatible keys in their secure element without additional cryptographic adapters.

### 2.3 Offline-First Architecture

The protocol is designed for environments where internet connectivity is intermittent:
- Payment channels allow unlimited off-chain transactions between paired devices
- BLE mesh relay propagates settlements passively through nearby devices
- QR codes provide an alternative channel for settlement data transfer

### 2.4 Minimal Trusted Set

Rather than pursuing maximum decentralization (which increases latency), Dina optimizes for a small set of known, trusted validators (3-7). This is appropriate for its target market -- enterprise IoT, fleet management, factory automation -- where participants are known entities.

## 3. Consensus: TurboBFT

### 3.1 Overview

TurboBFT is a pipelined Byzantine Fault Tolerant consensus protocol based on Tendermint/HotStuff principles, optimized for small validator sets (3-7 nodes). It achieves sub-200ms finality by eliminating the overhead of large-scale gossip and committee selection.

### 3.2 Protocol Phases

The consensus round proceeds through four phases:

**Phase 1: Propose**
The round leader is selected deterministically: `validators[(height + round) % n]`. The leader collects pending transactions from the mempool (ordered by fee, highest first), constructs a block, signs it, and broadcasts a `Proposal` message.

**Phase 2: Prevote**
Each validator verifies the proposal (leader identity, signature, block validity) and broadcasts a signed `Prevote` message containing the block hash. If a validator is locked on a block from a previous round, it only prevotes for the locked block (safety property).

**Phase 3: Precommit**
When a validator observes 2/3+ prevotes for a block hash (quorum), it locks on that block and broadcasts a signed `Precommit` message.

**Phase 4: Commit**
When a validator observes 2/3+ precommits for a block hash, the block is committed. A `CommitCertificate` is constructed containing all quorum precommit signatures, providing cryptographic proof of finality. The height advances and the next round begins.

### 3.3 View Changes

If a round times out (default: 10 seconds), validators broadcast `ViewChange` messages requesting advancement to a new round. When 2/3+ view change messages are received for the same target round, the round advances and a new leader takes over.

View changes preserve the safety lock: if a validator was locked on a block in a previous round, it carries that lock forward. This prevents conflicting blocks from being committed at the same height.

### 3.4 Safety and Liveness

**Safety**: Two conflicting blocks cannot both receive 2/3+ precommits at the same height. The locking mechanism ensures that once a block receives a prevote quorum, conflicting proposals in the same or later rounds are rejected.

**Liveness**: As long as 2/3+ validators are honest and connected, the protocol makes progress. If the leader is faulty, the timeout triggers a view change to a new leader within one timeout period.

### 3.5 Quorum Threshold

The quorum size is `ceil(2n/3)`, computed as `(n * 2 + 2) / 3` in integer arithmetic:

| Validators (n) | Quorum | Fault Tolerance (f) |
|----------------|--------|---------------------|
| 3 | 2 | 1 |
| 4 | 3 | 1 |
| 5 | 4 | 1 |
| 7 | 5 | 2 |

### 3.6 Commit Certificate

A `CommitCertificate` contains:
- Block height
- Block hash
- Vector of precommit `Vote` messages (each with voter pubkey and Ed25519 signature)

Verification checks:
1. All votes are `Precommit` type
2. All votes reference the same block hash
3. All signatures are valid
4. No duplicate voters
5. Vote count >= quorum threshold

## 4. Execution: DinaWASM Smart Contracts

### 4.1 Runtime

Contracts are written in Rust, compiled to `wasm32-unknown-unknown`, and executed by wasmtime (version 29). The runtime provides:

- **Fuel-based gas metering**: Every WASM instruction costs fuel. Execution halts when fuel is exhausted.
- **Deterministic execution**: Same inputs always produce same outputs across all validators.
- **Sandboxing**: Contracts run in isolated memory spaces with configurable limits (default: 16 MiB).
- **Cross-contract calls**: Contracts can invoke other contracts with a 1000-gas overhead.

### 4.2 Contract Entry Points

Every contract exports three functions:
- `__alloc(size: i32) -> i32` -- Memory allocator for the host to write arguments
- `__init(args_ptr: i32, args_len: i32)` -- Constructor, called once at deployment
- `__dispatch(method_ptr: i32, method_len: i32, args_ptr: i32, args_len: i32) -> i64` -- Method router

The `__dispatch` return value is a packed i64: high 32 bits = pointer to result data, low 32 bits = length of result data.

### 4.3 Gas Cost Model

Gas costs are deterministic and consistent across all validators:

| Operation | Cost (gas) | Rationale |
|-----------|-----------|-----------|
| WASM instruction | 1 | Base cost per opcode |
| Memory read (host) | 5 | Read from WASM linear memory |
| Memory write (host) | 5 | Write to WASM linear memory |
| Storage read | 100 | Disk I/O for persistent state |
| Storage write | 500 | Disk I/O + write-ahead logging |
| USDC transfer | 200 | Balance mutation + nonce update |
| Cross-contract call | 1000 | New WASM instance + context switch |
| SHA-256 hash | 50 | CPU-bound cryptographic operation |
| Ed25519 verify | 300 | Signature verification |
| Event emission | 100 | Log serialization + indexing |

Since 1 gas = 1 micro-USDC, the maximum gas per call (10,000,000) costs 10 USDC. Typical transfers cost well under $0.01.

### 4.4 Contract Address Derivation

Contract addresses are deterministic: `address = SHA-256(deployer_address || nonce)`. The deployer's nonce increments with each deployment, ensuring unique addresses without global coordination.

## 5. Privacy: Four-Layer Model

### 5.1 Layer 1: Encrypted Memos

Every transaction can carry an encrypted memo visible only to the recipient.

**Cryptographic scheme**: ECIES with X25519 key agreement and XChaCha20-Poly1305 AEAD.

**Protocol**:
1. Sender generates ephemeral X25519 keypair `(r, R)`
2. Sender computes `shared_secret = ECDH(r, recipient_pubkey)`
3. Sender derives `sym_key = SHA-256(shared_secret)`
4. Sender generates random 24-byte nonce
5. Sender encrypts: `ciphertext = XChaCha20Poly1305(sym_key, nonce, plaintext)`
6. Sender publishes `(R, nonce, ciphertext)` alongside the transaction

**Properties**: Forward secrecy (new ephemeral key per memo), authenticated encryption (tampering is detected), compact (no public key infrastructure beyond X25519).

### 5.2 Layer 2: Stealth Addresses

Adapted from EIP-5564. Recipients publish a stealth meta-address `(scan_pubkey, spend_pubkey)`. Senders derive one-time addresses that only the recipient can detect and spend from.

**Derivation**: `address = SHA-256(SHA-256(ECDH(ephemeral, scan_pubkey) || spend_pubkey))`

**Detection**: Recipients scan the chain with their scan secret, computing the expected address for each transaction's ephemeral pubkey. If it matches, the payment is theirs.

**Spending**: `spending_key = SHA-256(ECDH(scan_secret, ephemeral_pubkey) || spend_secret)`

### 5.3 Layer 3: View Keys

A granular permission system where account owners grant specific access levels to authorized keys:

- **FullAccess**: Unrestricted
- **ViewOnly**: Read-only access to balances and history
- **TransferOnly**: Can send transfers within limits (max amount, recipient allowlist)
- **ContractCallOnly**: Can call specific contracts and methods
- **DeviceControl**: Can issue commands to specific IoT devices
- **SessionKey**: Time-limited access with nested permission restrictions
- **Custom**: Freeform capabilities list

Session keys support nesting, allowing time-limited versions of any other permission type.

### 5.4 Layer 4: Zero-Knowledge Proofs (Planned)

The mainnet roadmap includes ZK proofs for private transfers where amounts and participants are hidden from validators while preserving verifiability. This will be implemented using a PLONK-based system adapted for Dina's Ed25519 key structure.

## 6. Payment Channels: Offline Device Transactions

### 6.1 Design

Bidirectional payment channels allow two parties to exchange unlimited off-chain payments after a single on-chain lock-up transaction. Updates take approximately 5ms (device-local signing and state exchange), compared to 200ms+ for on-chain transactions.

### 6.2 State Model

Channel state consists of:
- Channel ID: `SHA-256(party_a || party_b || timestamp)`
- Two balances that must sum to `total_locked` (invariant)
- Monotonically increasing sequence number
- Both parties' Ed25519 signatures

### 6.3 Settlement

**Cooperative close**: Both parties sign the final state. Immediate on-chain settlement with no challenge period.

**Unilateral close**: One party submits a signed state. A 100-block challenge period begins during which the counter-party can submit a newer state (higher sequence number).

**Dispute resolution**: If a newer state is submitted during the challenge period, it replaces the closing state and the channel enters `Disputed` status. After the challenge period expires without further challenges, the latest state is finalized.

### 6.4 Invariants

1. `balance_a + balance_b == total_locked` at all times
2. Both parties must sign every state update
3. Challenge states must have strictly higher sequence numbers
4. Only channel parties can initiate close or challenge

## 7. Mesh Relay: BLE Settlement Network

### 7.1 Concept

The mesh relay network enables payment settlement propagation in environments without internet connectivity, inspired by Apple's Find My network. Any device running a Dina relay client (e.g., phones with the Lucilla app) passively relays settlement blobs via Bluetooth Low Energy (BLE) advertising.

### 7.2 Relay Blob

The `RelayBlob` is a compact data structure (max 200 bytes, BLE + QR compatible) containing:
- Sender and receiver addresses
- Settlement amount and sequence number
- TTL (default: 5 minutes) and max hops (default: 10)
- Relay fee offered to each relay node
- Dual Ed25519 signatures (sender + receiver)
- Hop counter (incremented by each relay)

### 7.3 Relay Incentives

Relay nodes earn micro-fees (configurable per blob) for each successful relay. The DRC-113 standard governs relay fee accounting and reputation:
- Relay nodes register on-chain with their capacity
- Successful relay proofs are submitted by validators
- Fees accumulate and can be claimed periodically

### 7.4 Anti-Spam

- Blobs have a TTL and are dropped after expiry
- Max hop count prevents infinite propagation
- Signature verification prevents forged blobs
- Relay nodes can rate-limit based on sender reputation (DRC-107)

## 8. Token Economics: USDC-as-Gas Model

### 8.1 No Native Token

Dina intentionally has no native token. This is a deliberate design choice:

- **Predictable costs**: Machine operators can budget in USD terms
- **No speculation**: The chain's utility is not entangled with token price movements
- **Regulatory simplicity**: USDC is a regulated stablecoin; no securities concerns
- **Lower barrier**: No need to acquire a new token to use the network

### 8.2 Fee Structure

Transaction fees are paid in USDC micro-units (1 USDC = 1,000,000 micro-USDC):

| Transaction Type | Minimum Fee |
|------------------|------------|
| Transfer | ~200 micro-USDC ($0.0002) |
| Contract call | ~1,000-10,000 micro-USDC (varies by gas) |
| Contract deploy | ~5,000-100,000 micro-USDC (varies by WASM size) |
| Device registration | ~1,000 micro-USDC |

### 8.3 Validator Revenue

Block proposers receive all transaction fees in the blocks they produce. With 43,200 blocks/day and an average of 10 transactions at $0.001 each, a single validator earns approximately $432/day at full utilization.

### 8.4 USDC Bridging (Planned)

USDC enters the Dina Network via a bridging mechanism. Initial implementation will use a federated bridge operated by the validator set. Future versions will integrate with Circle's Cross-Chain Transfer Protocol (CCTP) for trustless bridging from Ethereum, Base, and Solana.

## 9. Security Analysis

### 9.1 Consensus Security

- **Byzantine fault tolerance**: TurboBFT tolerates up to `f = floor((n-1)/3)` faulty validators
- **Finality guarantee**: Once a `CommitCertificate` is produced, the block is irreversible
- **Double-signing detection**: Validators that sign conflicting messages at the same height are detected and slashed
- **View change safety**: Locks persist across rounds, preventing conflicting commits

### 9.2 Cryptographic Security

- **Ed25519**: 128-bit security level, well-studied and constant-time
- **X25519**: Elliptic curve Diffie-Hellman with 128-bit security
- **XChaCha20-Poly1305**: 256-bit key, 192-bit nonce (no nonce reuse risk), authenticated
- **SHA-256**: 128-bit collision resistance, 256-bit preimage resistance

### 9.3 Smart Contract Security

- **Sandboxing**: WASM contracts run in isolated memory with configurable limits
- **Gas metering**: Fuel-based execution prevents infinite loops
- **No re-entrancy**: Synchronous execution model; cross-contract calls complete before returning
- **Deterministic execution**: No access to randomness, timestamps, or external I/O from within contracts

### 9.4 Payment Channel Security

- **Dual signatures**: Both parties must sign every state update
- **Challenge period**: 100-block window for dispute resolution
- **Sequence ordering**: Only strictly higher sequence numbers are accepted as challenges
- **Conservation**: Balance sum invariant is verified at every state transition

## 10. Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Block time | 2 seconds | Configurable per chain |
| Finality | < 200ms | With 3 validators on low-latency links |
| Transaction throughput | 1,000-5,000 TPS | Depends on transaction complexity |
| WASM execution | 5-25x faster than EVM | wasmtime JIT compilation |
| Payment channel update | ~5ms | Local device-to-device |
| BLE relay propagation | ~1 second/hop | Depends on device density |
| Storage write | < 1ms | redb on NVMe SSD |
| State read | < 0.1ms | redb B-tree lookup |

## 11. Comparison to Existing Solutions

| Feature | Dina | Ethereum | Solana | Cosmos |
|---------|------|----------|--------|--------|
| Finality | <200ms | ~12 min | ~400ms | ~6s |
| Gas token | USDC | ETH | SOL | ATOM |
| Smart contracts | WASM | EVM | eBPF/WASM | CosmWASM |
| Hardware identity | Native (DRC-2) | None | None | None |
| Offline payments | Payment channels | L2 only | None | None |
| BLE mesh relay | Native | None | None | None |
| Privacy | 4-layer | None (base) | None | None |
| Agent wallets | DRC-101 | ERC-4337 | None | None |
| Validator count | 3-7 | ~900K | ~2K | Varies |
| Target use case | M2M payments | General | DeFi/NFT | Interchain |

## 12. Roadmap

### Phase 1: Testnet (Current)

- Core blockchain implementation (all 14 crates)
- TurboBFT consensus engine
- WASM smart contract runtime with gas metering
- 30 DRC standard reference implementations
- Payment channels with cooperative and unilateral close
- BLE mesh relay SDK
- JSON-RPC, REST API, and WebSocket server
- MCP tool integration (12 tools)
- TypeScript and Python SDKs
- Docker-based multi-validator testnet

### Phase 2: Hardening (Q3 2026)

- Persistent storage optimization and compaction
- P2P network hardening and NAT traversal
- Light client implementation
- Block explorer web application
- Testnet faucet
- Prometheus metrics and Grafana dashboards
- Comprehensive security audit

### Phase 3: Mainnet Preparation (Q4 2026)

- Staking and slashing implementation
- USDC bridge (federated, then CCTP)
- Zero-knowledge proof integration (Layer 4 privacy)
- Validator onboarding program
- Economic simulation and fee optimization
- Formal verification of consensus protocol

### Phase 4: Mainnet Launch (Q1 2027)

- Genesis ceremony with 5-7 validators
- USDC bridge activation
- Public developer documentation and tutorials
- SDK stable releases (v1.0)
- Community governance framework

### Phase 5: Ecosystem Growth (2027+)

- DRC standard extensions from community proposals
- Cross-chain interoperability (IBC, CCTP)
- Hardware wallet support (Ledger, Trezor)
- Mobile SDK for Flutter/React Native
- Enterprise integrations (fleet management, supply chain)

## 13. Conclusion

Dina Network fills a specific gap in the blockchain landscape: a purpose-built infrastructure for machines that transact. By combining sub-200ms BFT finality, stablecoin-denominated fees, hardware-native identity, offline payment channels, and a BLE mesh relay network, Dina provides the complete payment infrastructure that AI agents, IoT devices, and robotic systems need to participate in the economy.

The choice to use USDC rather than a native token removes speculative dynamics and aligns the chain's value proposition with its utility. The small validator set trades maximum decentralization for the latency properties that machine-to-machine commerce demands. The four-layer privacy architecture ensures that device transactions can be both private (for competitive commercial reasons) and auditable (for regulatory compliance).

Dina is not trying to be a general-purpose blockchain. It is a specialized payment rail for the machine economy -- and that focus is what makes it viable where general-purpose chains are not.

## References

1. Castro, M., and Liskov, B. "Practical Byzantine Fault Tolerance." OSDI, 1999.
2. Yin, M., et al. "HotStuff: BFT Consensus with Linearity and Responsiveness." PODC, 2019.
3. Buchman, E., Kwon, J., and Milosevic, Z. "The latest gossip on BFT consensus." arXiv:1807.04938, 2018.
4. EIP-5564: "Stealth Addresses." Ethereum Improvement Proposals.
5. Circle. "USDC: A Fully-Collateralized US Dollar Stablecoin." 2018.
6. Bernstein, D.J., et al. "Ed25519: High-speed high-security signatures." 2012.
7. Poon, J., and Dryja, T. "The Bitcoin Lightning Network." 2016.
