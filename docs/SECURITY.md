# Dina Network Security Model

This document describes the security architecture, threat model, and operational security practices for the Dina Network blockchain.

## Threat Model

### What We Protect Against

1. **Double-spend attacks** -- A malicious actor attempts to spend the same USDC balance twice.
2. **Replay attacks** -- Re-submitting a previously valid transaction to drain funds.
3. **Balance overflow/underflow** -- Arithmetic manipulation to create or destroy value.
4. **WASM sandbox escape** -- A malicious smart contract attempts to access host resources outside its sandbox.
5. **Denial-of-service** -- Flooding the network with oversized transactions, spam, or resource-exhausting contract calls.
6. **Byzantine validators** -- Up to f faulty validators in a 3f+1 BFT consensus.
7. **Payment channel fraud** -- Submitting stale channel states to steal funds during unilateral close.
8. **Key compromise** -- An attacker obtains a user's private key.
9. **Front-running** -- A validator reorders transactions within a block for profit.

### What We Do NOT Protect Against

- Compromise of more than 1/3 of validator stake (standard BFT assumption).
- Side-channel attacks on the cryptographic library itself (delegated to `ed25519-dalek`).
- Social engineering attacks against individual users.
- Bugs in the Rust compiler or `wasmtime` runtime (trusted computing base).

## Cryptographic Primitives

| Primitive | Algorithm | Library | Security Level |
|-----------|-----------|---------|----------------|
| Signatures | Ed25519 | `ed25519-dalek` | 128-bit |
| Hashing | SHA-256 | `sha2` | 128-bit (collision) |
| Key exchange | X25519 | `x25519-dalek` | 128-bit |
| Address derivation | SHA-256(pubkey) | `sha2` | 128-bit preimage |
| Merkle trees | SHA-256 | `rs_merkle` | 128-bit |
| Serialization | Bincode (deterministic) | `bincode` | N/A |

### Security Properties

- **Ed25519**: Provides existential unforgeability under chosen-message attack (EUF-CMA). Deterministic signatures prevent nonce-reuse vulnerabilities.
- **SHA-256**: Collision-resistant hash function. Used for address derivation, transaction hashing, state roots, and Merkle tree construction.
- **Address derivation**: `Address = SHA-256(Ed25519_pubkey)`. One-way: knowing an address does not reveal the public key. The public key is only exposed when the account first transacts.

## Transaction Validation Checklist

Every transaction must pass these checks before inclusion in a block:

1. **Signature verification** -- Ed25519 signature is valid for the transaction's signing bytes and the sender's public key.
2. **Nonce check** -- Transaction nonce equals the sender account's current nonce (strictly sequential, no gaps).
3. **Fee bounds** -- Fee is within `[min_transaction_fee, max_transaction_fee]` as defined by `ProtocolLimits`.
4. **Balance sufficiency** -- `balance >= fee + amount` (checked with overflow protection).
5. **Size limits** -- Serialized transaction size is within `max_transaction_size`.
6. **Replay protection** -- Transaction hash has not been seen in the replay protection cache.
7. **Type-specific validation**:
   - **Transfer**: Target is not zero address; no self-transfer; memo within size limit; amount within `max_transfer_amount`.
   - **DeployContract**: WASM bytecode starts with magic number `\0asm`; bytecode within `max_contract_size`.
   - **CallContract**: Method name is alphanumeric + underscore only; args within `max_args_size`.
   - **RegisterDevice**: Device not already registered; attestation signature valid.

## WASM Sandbox Security Model

Smart contracts run inside a `wasmtime` sandbox with the following isolation guarantees:

### Resource Limits

| Resource | Limit | Enforcement |
|----------|-------|-------------|
| Gas (fuel) | Per-call budget | Wasmtime fuel metering |
| Memory | 16 MB max (configurable) | Wasmtime memory limits |
| Storage writes | 1,000 per transaction | Host function enforcement |
| Events emitted | 50 per transaction | Host function enforcement |
| Call depth | 10 levels | Cross-contract call tracking |

### Host Function Security

All host functions (`__host_*`) enforce:

- **Bounds checking**: Every memory read/write validates `ptr + len` does not exceed WASM linear memory.
- **Gas metering**: Every host function call deducts gas before executing.
- **Input validation**: Negative amounts are rejected before casting to `u64`. Zero-amount transfers are rejected.
- **Storage limits**: Write count is tracked and enforced against sandbox limits.
- **No raw system access**: Contracts cannot access filesystem, network, or system calls.

### Isolation Guarantees

- Each contract call gets its own `WasmHostState` instance with isolated storage overlay.
- Storage changes are only committed on successful execution.
- Pending transfers are only applied after the contract call completes successfully.
- A contract cannot read another contract's storage directly (only via cross-contract calls).

## Consensus Attack Resistance

The Dina Network uses a BFT (Byzantine Fault Tolerant) consensus protocol with the following properties:

- **Safety**: The network never finalizes two conflicting blocks, as long as fewer than 1/3 of validators are Byzantine.
- **Liveness**: The network continues producing blocks as long as more than 2/3 of validators are honest and online.
- **Finality**: Blocks are final once committed (no probabilistic finality / reorganizations).
- **Validator set**: Limited to `max_validator_count` (7 on mainnet) with minimum stake requirement (`min_validator_stake` = $10,000 USDC).

### Block Validation

- Block header signature is verified against the proposer's public key.
- Parent hash must match the previous block.
- Transactions root is recomputed and verified against the Merkle root in the header.
- State root is recomputed after execution and verified.
- Timestamp must be non-decreasing.

## Payment Channel Security

Payment channels enable off-chain USDC transfers between two parties with on-chain settlement.

### Invariants

1. **Conservation of funds**: `balance_a + balance_b == total_locked` at all times (enforced with overflow-safe arithmetic).
2. **Monotonic sequence**: State updates have strictly increasing sequence numbers.
3. **Dual signatures**: Both parties must sign every state update.
4. **Challenge period**: Unilateral close triggers a timeout during which the counterparty can submit a newer state.

### Attack Resistance

- **Stale state attack**: If party A submits an old state during unilateral close, party B can challenge with a state that has a higher sequence number.
- **Signature forgery**: State updates require valid Ed25519 signatures from both parties.
- **Balance manipulation**: Conservation-of-funds check prevents creating or destroying value within a channel. Overflow protection prevents wrapping attacks.
- **Non-party interference**: Only channel participants can initiate close or challenge.

## Key Management Best Practices

1. **Never expose private keys** in logs, error messages, or debug output.
2. **Use hardware security modules** (HSMs) for validator signing keys in production.
3. **Rotate keys periodically** and use separate keys for different purposes (signing vs. encryption).
4. **Derive addresses from public keys** -- never store raw private keys alongside addresses.
5. **Use secure random number generation** (`OsRng`) for key generation.

## Smart Contract Security Patterns

### For Contract Developers

1. **Check-effects-interactions**: Validate inputs, update state, then make external calls.
2. **Reentrancy guards**: The sandbox prevents reentrancy at the host level (each call gets isolated state).
3. **Integer overflow**: Use checked arithmetic. The host enforces overflow protection on transfers.
4. **Access control**: Verify `__host_caller()` before privileged operations.
5. **Storage key hygiene**: Use namespaced keys to avoid collisions between logical storage domains.

### Protocol-Level Protections

- Gas metering prevents infinite loops.
- Memory limits prevent allocation bombs.
- Storage write limits prevent state bloat attacks.
- Event emission limits prevent log flooding.

## Known Limitations and Assumptions

1. **Single-chain security**: The Dina Network is a standalone chain. Cross-chain bridge security is out of scope.
2. **Trusted validator set**: The initial validator set is permissioned. Decentralization is a future milestone.
3. **WASM execution is not yet fully connected**: The executor records contract calls as events but does not yet execute WASM bytecode. Full WASM execution is in progress.
4. **No formal verification**: The protocol has not been formally verified. Security relies on testing, code review, and defensive programming.
5. **Clock assumptions**: Block timestamps are proposed by validators and are trusted to be approximately correct.

## Responsible Disclosure Process

If you discover a security vulnerability in the Dina Network:

1. **Do NOT** open a public GitHub issue.
2. Email **security@dina.network** with:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact assessment
   - Your suggested fix (if any)
3. We will acknowledge receipt within 48 hours.
4. We will provide a detailed response within 7 days, including our assessment and timeline for a fix.
5. We ask that you do not publicly disclose the vulnerability until we have released a fix or 90 days have passed, whichever comes first.

## Audit Status

| Component | Status | Date | Notes |
|-----------|--------|------|-------|
| Core types & crypto | Internal review | 2025 | SHA-256 + Ed25519 via audited `ed25519-dalek` crate |
| Transaction validation | Internal review + automated hardening | 2025 | Overflow protection added to all arithmetic paths |
| Account state | Internal review + automated hardening | 2025 | `checked_add`/`saturating_add` on all balance operations |
| Payment channels | Internal review + automated hardening | 2025 | Conservation checks use `checked_add`; sequence monotonicity enforced |
| WASM host functions | Internal review + automated hardening | 2025 | Bounds checking, negative amount rejection, gas metering |
| Consensus | Internal review | 2025 | BFT with 7-validator set |
| External audit | **Not yet performed** | -- | Planned before mainnet launch |

### Hardening Measures Applied

The following security fixes were applied during the automated security audit:

1. **Balance overflow protection** (`account.rs`): `credit()` uses `saturating_add`; `transfer()` checks receiver overflow with `checked_add` before mutating state.
2. **Nonce overflow protection** (`account.rs`): `increment_nonce()` uses `checked_add` to prevent wrapping at `u64::MAX`.
3. **Fee + amount overflow** (`executor.rs`): `validate_transaction()` uses `checked_add` for `fee + amount` to prevent underflow-based balance bypass.
4. **Block fee accumulation** (`executor.rs`): `total_fees` and `total_gas` use `saturating_add` to prevent panic on accumulation overflow.
5. **Negative amount rejection** (`host.rs`): `__host_transfer` rejects `amount <= 0` before casting `i64` to `u64`, preventing a malicious contract from wrapping a negative value to drain its balance.
6. **Channel conservation overflow** (`channel.rs`): All `balance_a + balance_b` conservation checks use `checked_add` to prevent a crafted state from bypassing the invariant.
7. **Channel deposit overflow** (`channel.rs`): `open()` uses `saturating_add` for `deposit_a + deposit_b`.
8. **Protocol limits** (`limits.rs`): Comprehensive size, fee, and structural validation for transactions and blocks.
9. **Replay protection** (`security.rs`): Bounded transaction hash cache to detect duplicate submissions.
10. **Input sanitization** (`security.rs`): Method name validation, memo size checks, zero-address checks, self-transfer prevention.
