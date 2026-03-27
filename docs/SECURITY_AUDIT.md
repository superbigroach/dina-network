# Dina Network Security Audit

**Auditor:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-27
**Scope:** Core node, RPC layer, smart contracts (dex-swap, defi-lending, defi-vault, bridge-usdc, bridge-base, drc16-proxy, drc63-swarm-wallet)
**Commit:** HEAD of `dev` branch

---

## Executive Summary

The Dina Network codebase is well-structured with good Rust idioms and reasonable overflow protection in the account layer. However, the audit uncovered **3 critical**, **5 high**, **8 medium**, and **6 low** severity findings. The most severe issue is that **Ed25519 transaction signatures are never verified** in either the RPC submission path or the block execution path, meaning anyone can forge transactions for any address.

| Severity | Count |
|----------|-------|
| CRITICAL | 3     |
| HIGH     | 5     |
| MEDIUM   | 8     |
| LOW      | 6     |

**Total findings: 22**

---

## CRITICAL Findings

### C-1: Transaction Signatures Are Never Verified

```
SEVERITY: CRITICAL
FILE: crates/dina-rpc/src/rest.rs:153-196, crates/dina-rpc/src/jsonrpc.rs:264-288, node/src/chain_state.rs:90-138, crates/dina-core/src/executor.rs:300-349
LINE: Multiple
ISSUE: Ed25519 signatures on transactions are never verified anywhere in the transaction lifecycle. The REST handler (submit_transaction_handler), JSON-RPC handler (send_transaction), ChainState::execute_transaction, and BlockExecutor::execute_transaction all accept transactions without calling verify_signature(). The validate_transaction() method on BlockExecutor only checks that signature bytes are non-zero (line 394: `if sig_bytes == [0u8; 64]`), and this method is never called in the transaction submission or block execution path.
IMPACT: Any attacker can craft a transaction with an arbitrary `from` address and a non-zero garbage signature to steal funds from any account. This is a total compromise of all account security.
FIX: Add signature verification at the RPC submission layer. This requires mapping sender addresses to public keys (e.g., via an on-chain registry or requiring the public key in the submission payload). At minimum, the mempool or block executor must call `tx.verify_signature()` with the sender's known public key before accepting the transaction. The current code comment at executor.rs:391 says "Real signature verification happens at the mempool layer" but no such verification exists anywhere.
```

### C-2: Faucet Mints Unlimited USDC Into Real Account State

```
SEVERITY: CRITICAL
FILE: crates/dina-rpc/src/rest.rs:238-305
LINE: 274-293
ISSUE: The faucet endpoint credits 1,000 USDC (1_000_000_000 micro-USDC) directly to the account state AND injects a faucet transaction from the zero address with no signature validation. The rate limiter only tracks by address (10-minute cooldown per address), but an attacker can generate unlimited addresses (no cost since signatures are not verified) to drain unlimited USDC. Combined with C-1, an attacker can faucet to address A, then forge a transfer from A to their real address.
IMPACT: Infinite money creation on testnet. If this code runs on mainnet with real USDC bridged, the faucet should not exist or must be removed.
FIX: (1) Remove the faucet entirely for any production deployment. (2) For testnet, add a global rate limit (total USDC minted per hour), not just per-address. (3) Add IP-based rate limiting. (4) The faucet rate_limit HashMap grows unboundedly in memory -- add eviction.
```

### C-3: ChainState Does Not Verify Nonces or Signatures

```
SEVERITY: CRITICAL
FILE: node/src/chain_state.rs:90-138
LINE: 90-103
ISSUE: ChainState::execute_transaction skips nonce validation entirely. It deducts fees and increments nonce for non-coinbase transactions, but never checks that the transaction's nonce matches the account's expected nonce. This means the same transaction can be replayed repeatedly (replay attack), and transactions can be submitted in any order.
IMPACT: Replay attacks -- any observed transaction can be re-submitted indefinitely to drain the sender's account. Transaction ordering guarantees are broken.
FIX: Before executing, verify `tx.nonce() == account.nonce`. The BlockExecutor.execute_body() does check nonces (line 422), but ChainState.execute_transaction() does not, and it is ChainState that the block production loop in main.rs uses (line 530).
```

---

## HIGH Findings

### H-1: Block Executor Validates Signatures With Non-Zero Check Only

```
SEVERITY: HIGH
FILE: crates/dina-core/src/executor.rs:384-398
LINE: 393-396
ISSUE: BlockExecutor::validate_transaction() checks `sig_bytes == [0u8; 64]` as its only "signature validation." Any 64-byte value that is not all zeros passes. The code comments acknowledge this: "Real signature verification happens at the mempool layer where the public key is available" -- but no such verification exists anywhere.
IMPACT: Combined with C-1, this means the only barrier to forging transactions is using non-zero bytes in the signature field.
FIX: Implement real Ed25519 verification. Either require senders to register their public keys on-chain (so the executor can look them up), or modify the transaction submission API to include the public key and verify address == SHA-256(pubkey).
```

### H-2: CORS Allows Any Origin on REST API

```
SEVERITY: HIGH
FILE: crates/dina-rpc/src/rest.rs:328-331
LINE: 328-331
ISSUE: The REST API uses `CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)`, allowing any website to make requests to the node's REST API including the faucet and transaction submission endpoints.
IMPACT: Any malicious website visited by a node operator could submit transactions or abuse the faucet via the user's browser. On a validator node, this could be used for denial-of-service attacks by flooding the mempool.
FIX: Restrict CORS origins to known frontends. For testnet, at minimum remove the faucet from the open CORS policy. For production, CORS should be restrictive or API authentication should be required.
```

### H-3: Bridge Proof is Recomputable by Relayer (Trusted Relayer = Single Point of Compromise)

```
SEVERITY: HIGH
FILE: contracts/bridge-base/src/lib.rs:152-196
LINE: 171-180
ISSUE: The bridge claim proof is SHA-256(base_tx_hash || amount || recipient || relayer). The relayer can compute this proof for any parameters since all inputs are known to the relayer. There is no external signature or Merkle proof from Base. A compromised relayer can mint arbitrary USDC on Dina by fabricating base_tx_hashes that do not correspond to real Base transactions.
IMPACT: A compromised relayer can mint unlimited bridged USDC, draining all locked USDC on the Base side when users try to withdraw.
FIX: (1) The proof should include a cryptographic signature from a trusted party (or threshold of parties) that actually observed the Base chain transaction. (2) Implement light client verification of Base block headers. (3) At minimum, add multi-sig relayer support so no single key compromise can forge proofs. The code comments acknowledge this is a simplified model.
```

### H-4: DEX swap_route Passes min_output=0 for Intermediate Hops

```
SEVERITY: HIGH
FILE: contracts/dex-swap/src/lib.rs:494-529
LINE: 510
ISSUE: In swap_route(), each intermediate hop calls `self.swap_exact_in(pool_id, trader, &current_token, current_amount, 0)` with `min_output=0`. Only the final output is checked against the user's `min_output`. This means intermediate pool states can be manipulated between hops by a frontrunner.
IMPACT: A sandwich attacker can manipulate an intermediate pool's price before the multi-hop swap, extracting value from the trader on intermediate hops. The final slippage check only protects the overall output but not per-hop fairness.
FIX: Calculate expected intermediate outputs and pass reasonable min_output values for each hop, or perform the multi-hop atomically with a single k-invariant check across all pools.
```

### H-5: Bridged USDC Mint Has No Total Supply Cap

```
SEVERITY: HIGH
FILE: contracts/bridge-usdc/src/lib.rs:155-165
LINE: 163
ISSUE: The mint function (`balance + amount`) has no overflow check. While u64 overflow is unlikely in practice, there is no supply cap to ensure minted amount corresponds to locked amount on Base. The bridge and token contracts are separate with no on-chain link verifying `total_minted <= total_locked_on_base`.
IMPACT: If the bridge contract or relayer is compromised, unlimited USDC.e can be minted with no on-chain constraint.
FIX: Add checked arithmetic for balance addition. Consider adding a supply cap that can only be increased by the bridge contract, or link the token contract to the bridge contract so minting is bounded by verified deposits.
```

---

## MEDIUM Findings

### M-1: Node Key File Written Without Restricted Permissions

```
SEVERITY: MEDIUM
FILE: node/src/main.rs:133-136
LINE: 135
ISSUE: The node's Ed25519 private key is written to `<data_dir>/node_key` using `std::fs::write()` with default permissions. On Unix systems this may be world-readable depending on umask. On Windows, ACLs depend on the parent directory.
IMPACT: Other users on a shared machine can read the validator's private key, allowing them to forge blocks or steal the validator identity.
FIX: Set file permissions to 0600 (owner read/write only) on Unix after writing. Use appropriate ACL settings on Windows.
```

### M-2: Unbounded Transaction Pool Growth in RPC Layer

```
SEVERITY: MEDIUM
FILE: crates/dina-rpc/src/jsonrpc.rs:280-284, crates/dina-rpc/src/rest.rs:181-188
LINE: 280-284
ISSUE: The RPC layer's tx_pool (Vec<Transaction>) has no size limit. The real Mempool has a 10,000 transaction cap, but transactions are first pushed to the RPC tx_pool unboundedly before the bridge task drains them to the real Mempool every 100ms. An attacker can submit millions of transactions faster than the bridge drains them.
IMPACT: Memory exhaustion denial-of-service. The node will run out of memory and crash.
FIX: Add a capacity check to the RPC tx_pool before pushing transactions. Reject submissions when the pool exceeds a threshold (e.g., 20,000).
```

### M-3: Faucet Rate-Limit HashMap Grows Unboundedly

```
SEVERITY: MEDIUM
FILE: crates/dina-rpc/src/rest.rs:31, 254-268
LINE: 31
ISSUE: The faucet_rate_limit HashMap stores every address that has ever requested faucet funds. Old entries are never evicted. Over time, this will consume significant memory.
IMPACT: Slow memory leak / denial-of-service vector. An attacker can generate millions of unique addresses and hit the faucet once each, growing the HashMap indefinitely.
FIX: Add periodic cleanup of entries older than FAUCET_COOLDOWN_SECS, or use an LRU cache with a fixed capacity.
```

### M-4: Lending Pool Has No Collateral Requirement

```
SEVERITY: MEDIUM
FILE: contracts/defi-lending/src/lib.rs:257-292
LINE: 257
ISSUE: The borrow function has no collateral requirement. The code comment says "For testnet simplicity, no collateral required." Any address can borrow the entire pool with zero collateral.
IMPACT: On testnet, any user can drain all supplied funds by borrowing without posting collateral, making the lending pool insolvent immediately.
FIX: Implement a collateral factor system before any mainnet deployment. Require borrowers to have supply positions worth at least (borrow_amount / collateral_factor) to back their loans.
```

### M-5: DRC-16 Proxy execute_upgrade Has No Caller Check

```
SEVERITY: MEDIUM
FILE: contracts/drc16-proxy/src/lib.rs:125-149
LINE: 125
ISSUE: The execute_upgrade() function does not verify that the caller is the admin. It only checks that the timelock has expired. Anyone can execute a pending upgrade once the timelock expires.
IMPACT: While the propose_upgrade() is admin-only, a malicious actor could front-run the admin's execution or execute an upgrade the admin intended to cancel but hadn't yet.
FIX: Add `assert!(caller == self.admin, "DRC16: only admin can execute upgrade")` to execute_upgrade(). The dispatch function also does not pass the caller to execute_upgrade.
```

### M-6: Vault Susceptible to Inflation Attack (First Depositor)

```
SEVERITY: MEDIUM
FILE: contracts/defi-vault/src/lib.rs:73-77, 103-125
LINE: 73-77
ISSUE: The vault uses a 1:1 share ratio for the first deposit (line 75: `return assets`). An attacker can: (1) deposit 1 wei to get 1 share, (2) transfer a large amount of tokens directly to the vault (via harvest/add_yield if they are the owner, or through external means), inflating the share price. Subsequent depositors get zero shares due to integer division rounding down.
IMPACT: Classic ERC-4626 inflation/donation attack. The first depositor can steal funds from all subsequent depositors through share price manipulation.
FIX: Implement a virtual offset (e.g., add a constant like 1e6 to both total_assets and total_shares in the conversion formula), or mint a small amount of shares to a dead address on first deposit, or enforce a minimum initial deposit.
```

### M-7: Bridge Withdrawal Can Underflow total_minted

```
SEVERITY: MEDIUM
FILE: contracts/bridge-base/src/lib.rs:206-239
LINE: 233
ISSUE: `self.total_minted -= amount` in withdraw() can underflow if total_minted is less than the withdrawal amount. This could happen if accounting gets out of sync (e.g., due to direct token burns not going through the bridge). Rust in release mode wraps on underflow rather than panicking.
IMPACT: total_minted wraps to a very large number, breaking all bridge accounting.
FIX: Use `self.total_minted = self.total_minted.saturating_sub(amount)` or add an explicit check that `self.total_minted >= amount`.
```

### M-8: Block Production Uses Unverified Transactions

```
SEVERITY: MEDIUM
FILE: node/src/main.rs:496-530
LINE: 497-506
ISSUE: The block production loop in main.rs collects transactions from the mempool and builds blocks without any validation. It calls BlockExecutor::execute_block() which does not verify signatures (see C-1). The mempool's add_transaction() also performs no validation -- it only checks for duplicates and fee ordering. Combined with C-1, this means forged transactions are routinely included in blocks.
IMPACT: Forged transactions are included in committed blocks.
FIX: Add transaction validation (signature verification, nonce check, balance check) either at mempool insertion time or before block building.
```

---

## LOW Findings

### L-1: DEX Pool Creation Has No Access Control

```
SEVERITY: LOW
FILE: contracts/dex-swap/src/lib.rs:139-171
LINE: 139
ISSUE: Any caller can create pools for any token pair. There is no whitelist or owner-only restriction on pool creation.
IMPACT: An attacker can create pools with fake/misleading token names, potentially confusing users. Pool creation spam could bloat contract state.
FIX: Add an owner-only or whitelist check for pool creation, or charge a creation fee.
```

### L-2: DRC-16 Proxy Uses Weak Hash Function

```
SEVERITY: LOW
FILE: contracts/drc16-proxy/src/lib.rs:188-194
LINE: 188-194
ISSUE: compute_hash() uses a djb2 variant (non-cryptographic hash) to compute the implementation hash. This hash is used to identify contract versions in the upgrade history.
IMPACT: An attacker could craft a malicious contract with the same djb2 hash as a legitimate one, making upgrade history misleading. However, the upgrade still requires admin approval.
FIX: Use SHA-256 or another cryptographic hash for implementation code hashing.
```

### L-3: Block Height Cast From usize to u64

```
SEVERITY: LOW
FILE: crates/dina-rpc/src/rest.rs:89, crates/dina-rpc/src/jsonrpc.rs:261, rest.rs:133
LINE: 89, 133
ISSUE: Block height is computed as `blocks.len().saturating_sub(1) as u64` and block access uses `height as usize`. On 32-bit systems, usize is 32 bits while u64 block heights could exceed usize::MAX. Additionally, `height as usize` at rest.rs:133 could silently truncate on 32-bit platforms.
IMPACT: Incorrect block retrieval on 32-bit platforms after 4 billion blocks (practically unlikely but a correctness issue).
FIX: Use `usize::try_from(height)` with proper error handling.
```

### L-4: Lending Pool Interest Calculation Precision Loss

```
SEVERITY: LOW
FILE: contracts/defi-lending/src/lib.rs:156-165
LINE: 156-165
ISSUE: The interest factor calculation performs integer division in the middle of the computation: `borrow_rate_bps * elapsed * INDEX_PRECISION / (BPS * SECONDS_PER_YEAR)`. For very short time periods (small elapsed), this can round to zero, causing interest to not accrue for short blocks.
IMPACT: Minor interest under-collection for short time periods. Over many blocks this could compound to a small but measurable discrepancy.
FIX: Reorder operations to maximize precision: multiply all numerators first, then divide.
```

### L-5: Swarm Wallet create_authority Does Not Validate Owner

```
SEVERITY: LOW
FILE: contracts/drc63-swarm-wallet/src/lib.rs:99-125
LINE: 99
ISSUE: create_authority() accepts any `owner` string parameter. The caller does not need to be the owner being set. This means anyone can create an authority assigned to someone else.
IMPACT: Low impact since only the owner can operate on the authority. However, it could confuse users if authorities are created in their name without consent, and the dispatch function at line 501 passes `a.owner` from args rather than using the actual caller identity.
FIX: Either set `owner = caller` always, or validate that the caller is authorized to create authorities for the specified owner.
```

### L-6: RPC and REST Do Not Validate Transaction Before Indexing

```
SEVERITY: LOW
FILE: crates/dina-rpc/src/jsonrpc.rs:274-284, crates/dina-rpc/src/rest.rs:181-188
LINE: 274-284
ISSUE: Both RPC endpoints index the transaction in tx_index and push it to tx_pool without any validation (not even basic deserialization checks beyond JSON parsing). Malformed transactions (e.g., with invalid addresses) are stored and forwarded.
IMPACT: The tx_index grows with invalid entries that will never be included in blocks. This wastes memory and could confuse RPC clients querying transaction status.
FIX: Call validate_transaction() or at minimum check basic structural validity before indexing.
```

---

## Informational Notes

### I-1: Dual State Management Creates Consistency Risk

The node maintains two separate account states: `ChainState.accounts` (used by block production) and `NodeState.accounts` (used by RPC). These are synced after each block (main.rs:586-589 replaces the entire RPC account state), but there is a brief window where RPC queries may return stale data. This is acceptable for a blockchain node but should be documented.

### I-2: No Transaction Size Limits

There are no limits on `wasm_bytecode` size in DeployContract or `args` size in CallContract. A single transaction could contain megabytes of WASM bytecode. The gas system exists but the mempool does not enforce gas limits on incoming transactions. Consider adding a max transaction size check at the RPC layer.

### I-3: No Chain ID in Transaction Signing

The transaction signing payload does not include the chain_id. If the same keys are used on different Dina networks (e.g., testnet and mainnet), transactions could be replayed across chains.

### I-4: Consensus Not Audited

This audit focused on the node binary's single-validator mode and smart contracts. The `dina-consensus` crate (TurboBFT) was not in scope but appears to have signature verification in its vote/proposal handling.

---

## Recommendations by Priority

### Immediate (Before Any Real Value on Chain)

1. **Implement transaction signature verification** (C-1, H-1) -- This is the single most important fix. Without it, all accounts are compromised.
2. **Add nonce verification in ChainState** (C-3) -- Required to prevent replay attacks.
3. **Remove or secure the faucet** (C-2) -- Remove entirely for production; add global rate limits for testnet.

### Before Testnet with Real Bridge Funds

4. **Upgrade bridge proof verification** (H-3) -- Move from trusted relayer to multi-sig or light client proofs.
5. **Add collateral to lending pool** (M-4) -- Currently trivially drainable.
6. **Fix vault inflation attack** (M-6) -- Implement virtual offset in share calculation.
7. **Bound RPC transaction pool** (M-2) -- Prevent memory exhaustion DoS.

### Before Mainnet

8. **Restrict CORS** (H-2) -- Production APIs should not allow any origin.
9. **Add chain_id to transaction signing** (I-3) -- Prevent cross-chain replay.
10. **Add transaction size limits** (I-2) -- Prevent DoS via oversized transactions.
11. **Set proper key file permissions** (M-1) -- Protect validator keys.
12. **Fix bridge withdrawal underflow** (M-7) -- Use checked/saturating arithmetic.

---

## Files Audited

| File | Lines | Status |
|------|-------|--------|
| `crates/dina-core/src/executor.rs` | ~600 | Audited |
| `crates/dina-core/src/account.rs` | 222 | Audited |
| `crates/dina-core/src/transaction.rs` | 366 | Audited |
| `crates/dina-rpc/src/rest.rs` | 333 | Audited |
| `crates/dina-rpc/src/jsonrpc.rs` | 525 | Audited |
| `node/src/chain_state.rs` | 291 | Audited |
| `node/src/main.rs` | 685 | Audited |
| `node/src/mempool.rs` | 363 | Audited |
| `contracts/dex-swap/src/lib.rs` | 943 | Audited |
| `contracts/defi-lending/src/lib.rs` | 758 | Audited |
| `contracts/defi-vault/src/lib.rs` | 603 | Audited |
| `contracts/bridge-usdc/src/lib.rs` | 632 | Audited |
| `contracts/bridge-base/src/lib.rs` | 594 | Audited |
| `contracts/drc16-proxy/src/lib.rs` | 585 | Audited |
| `contracts/drc63-swarm-wallet/src/lib.rs` | ~550 | Audited |

---

*This audit was conducted through static code review only. No dynamic testing, fuzzing, or formal verification was performed. Findings should be verified by the development team and addressed before handling real financial value.*
