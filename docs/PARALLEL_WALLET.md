# DRC-63: Parallel Wallet -- Auto-Scaling Parallel Wallet System

> **Standard:** DRC-63
> **Contract:** `drc63-swarm-wallet` (directory name kept for backwards compatibility)
> **SDK Class:** `ParallelWallet` (from `@dina-network/sdk`)

## What Is a Parallel Wallet?

A Parallel Wallet lets one user control N sub-wallets, each with an independent nonce. This breaks the sequential transaction bottleneck that every other blockchain imposes, enabling truly parallel on-chain transactions from a single account.

```
Traditional (every chain):
  User --> 1 Wallet --> sequential nonce --> 10 tx/s max

Dina Parallel Wallet:
  User --> ParallelAuthority
            |-- SubWallet #0 (nonce: 0) --> tx A --+
            |-- SubWallet #1 (nonce: 0) --> tx B --|
            |-- SubWallet #2 (nonce: 0) --> tx C --|  ALL IN THE SAME
            |-- SubWallet #3 (nonce: 0) --> tx D --|  100ms BLOCK
            |-- ...                                |
            +-- SubWallet #N (nonce: 0) --> tx N --+
```

## Why No Other Chain Has This

### The Nonce Bottleneck

Every blockchain uses a nonce (or equivalent ordering constraint) per account. Transaction #5 literally cannot execute until #4 is confirmed. This caps per-user throughput to 1 transaction per block.

| Chain     | Block Time | Per-User TPS |
|-----------|-----------|-------------|
| Ethereum  | 12s       | 0.08        |
| Base      | 2s        | 0.5         |
| Solana    | 400ms     | 2.5         |
| Sui       | 400ms     | ~10         |
| Aptos     | 400ms     | ~10         |
| Dina (1 wallet) | 100ms | 10       |
| **Dina (100 wallets)** | **100ms** | **1,000** |
| **Dina (1K wallets)** | **100ms** | **10,000** |

### Why Can't Others Copy It?

**Ethereum/Base/Arc (EVM)**
- Nonces are hardcoded into the EVM protocol. You can deploy multiple smart wallets using CREATE2, but each one needs its own EOA or separate deployment.
- Cost: deploying 100 smart wallets = $50-500 in gas.
- On Dina: creating 100 parallel wallets = $0.02.

**Solana**
- No nonces, but account locking serializes transactions touching the same account. You CAN use multiple accounts, but there is no standard for managing them as a group.
- Each account needs SOL for rent exemption (~$13 for 100 accounts).
- On Dina: 100 parallel wallets = $0.02.

**Sui/Aptos**
- Object-based parallelism at the protocol level gives high throughput, but there is no first-class standard for one user to operate a managed fleet of wallets with auto-scaling, rebalancing, and consolidation.
- Dina's DRC-63 is a first-class standard with SDK support.

## The 4 Modes

### Mode 1: Single

One wallet, one transfer. The trivial case.

```
Payments: 1
Wallets used: 1
On-chain txs: 1
Fee: $0.0001
Time: 100ms (1 block)
```

### Mode 2: Batch (DRC-19)

One wallet sends a single batch transaction containing up to 100 recipients.

```
Payments: up to 100
Wallets used: 1
On-chain txs: 1 (batch)
Fee: $0.0001
Time: 100ms (1 block)

Best for: cost optimization with < 100 recipients
```

### Mode 3: Parallel (DRC-63)

N wallets each send one transaction, all in the same block.

```
Payments: up to N (one per wallet)
Wallets used: N
On-chain txs: N
Fee: N x $0.0001
Time: 100ms (1 block)

Best for: speed optimization, low latency
```

### Mode 4: Parallel-Batch (DRC-63 + DRC-19)

N wallets each send one batch transaction, all in the same block. Maximum throughput.

```
Payments: N x 100
Wallets used: N
On-chain txs: N (each a batch of 100)
Fee: N x $0.0001
Time: 100ms (1 block)

Example: 100 wallets x 100 recipients = 10,000 payments in 1 block
Fee: $0.01
Time: 100ms
```

## Cost Breakdown

| Payments  | Mode            | Wallets | On-chain Txs | Fee      | Time   |
|-----------|-----------------|---------|-------------|----------|--------|
| 1         | Single          | 1       | 1           | $0.0001  | 100ms  |
| 10        | Batch           | 1       | 1           | $0.0001  | 100ms  |
| 50        | Batch           | 1       | 1           | $0.0001  | 100ms  |
| 100       | Batch           | 1       | 1           | $0.0001  | 100ms  |
| 100       | Parallel        | 100     | 100         | $0.01    | 100ms  |
| 1,000     | Parallel-Batch  | 10      | 10          | $0.001   | 100ms  |
| 10,000    | Parallel-Batch  | 100     | 100         | $0.01    | 100ms  |
| 100,000   | Parallel-Batch  | 1,000   | 1,000       | $0.10    | 100ms  |
| 1,000,000 | Parallel-Batch  | 10,000  | 10,000      | $1.00    | 100ms  |

**Key takeaway:** 10,000 payments for $0.01 in 100ms. No other chain can do this.

## SDK Usage

### Installation

```typescript
import { DinaClient, DinaWallet, ParallelWallet } from '@dina-network/sdk';
```

### Quick Start

```typescript
const client = new DinaClient('https://rpc.dina.network');
const wallet = DinaWallet.fromPrivateKey(myKey);

// Create a parallel wallet with auto-scaling (up to 100 sub-wallets)
const parallel = ParallelWallet.pro(wallet, client);

// Send a single payment (uses master wallet)
await parallel.transfer(recipientAddress, 1_000_000n); // 1 USDC

// Send 500 payments -- auto-selects parallel-batch mode
const hashes = await parallel.batchTransfer(payments);
```

### Presets

```typescript
// Solo: max 1 wallet, no parallelism (for testing or simple use)
const solo = ParallelWallet.solo(wallet, client);

// Standard: up to 10 sub-wallets (small businesses)
const standard = ParallelWallet.standard(wallet, client);

// Pro: up to 100 sub-wallets (high-throughput applications)
const pro = ParallelWallet.pro(wallet, client);

// Enterprise: up to 10,000 sub-wallets (maximum throughput)
const enterprise = ParallelWallet.enterprise(wallet, client);
```

### Custom Configuration

```typescript
const parallel = new ParallelWallet(wallet, client, {
  maxWallets: 500,
  autoScale: true,
  minBalancePerWallet: 5_000_000n, // 5 USDC minimum per sub-wallet
});
```

### Batch Transfer with Options

```typescript
// Optimize for cost (uses batch mode, fewer on-chain txs)
const hashes = await parallel.batchTransfer(payments, {
  priority: 'cost',
});

// Optimize for speed (uses parallel mode, all in one block)
const hashes = await parallel.batchTransfer(payments, {
  priority: 'speed',
});

// Auto mode (default): batch for < 100, parallel-batch for >= 100
const hashes = await parallel.batchTransfer(payments, {
  priority: 'auto',
});

// Force parallel even for small payment counts
const hashes = await parallel.batchTransfer(payments, {
  parallel: true,
});

// Set a fee budget
const hashes = await parallel.batchTransfer(payments, {
  maxFee: 10_000n, // max $0.01 total fees
});
```

### Wallet Management

```typescript
// Manually create sub-wallets
const addresses = await parallel.createWallets(50);

// Fund all sub-wallets evenly from master wallet
await parallel.fundAll(100_000_000n); // 100 USDC split across all

// Check stats
const stats = await parallel.stats();
console.log(`Active wallets: ${stats.activeWallets}`);
console.log(`Total balance: ${stats.totalBalance}`);
console.log(`Avg balance: ${stats.avgBalance}`);
console.log(`Total transactions: ${stats.totalTransactions}`);

// Consolidate all balances back to master
const consolidateHashes = await parallel.consolidate();
```

### Real-World Example: Payroll for 10,000 Employees

```typescript
const enterprise = ParallelWallet.enterprise(wallet, client);

// Build payment list
const payments = employees.map(emp => ({
  to: emp.walletAddress,
  amount: emp.salaryUSDC,
  memo: `Payroll ${month}`,
}));

// Send all 10,000 payments
// Auto-selects parallel-batch: 100 wallets x 100 recipients each
// Completes in 1 block (100ms), costs $0.01
const hashes = await enterprise.batchTransfer(payments, {
  priority: 'speed',
});

console.log(`Paid ${hashes.length} employees in 100ms`);
```

## On-Chain Contract API Reference

The DRC-63 contract manages `ParallelAuthority` records, each owning a vector of `SubWallet` entries.

### Data Structures

```rust
struct ParallelAuthority {
    owner: String,                    // master wallet address
    sub_wallets: Vec<SubWallet>,      // created sub-wallets
    max_wallets: u64,                 // safety cap (default 1000)
    auto_rebalance: bool,             // auto-distribute USDC
    min_balance_per_wallet: u64,      // minimum USDC per sub-wallet
    total_distributed: u64,           // total USDC distributed
    created_at: u64,                  // creation timestamp
    paused: bool,                     // emergency pause flag
}

struct SubWallet {
    address: String,                  // derived address
    public_key: String,               // Ed25519 public key hex
    balance: u64,                     // current USDC balance
    nonce: u64,                       // transaction count
    active: bool,                     // can send transactions
    created_at: u64,                  // creation timestamp
}

struct ParallelStats {
    active_wallets: u64,
    total_balance: u64,
    avg_balance: u64,
    total_txs: u64,
}
```

### Functions

| Function | Args | Returns | Description |
|----------|------|---------|-------------|
| `create_authority` | `owner, max_wallets` | `authority_id` | Create a new parallel authority |
| `create_wallets` | `authority_id, count` | `[indices]` | Create N sub-wallets (owner only) |
| `auto_scale` | `authority_id, needed_count` | `[indices]` | Create wallets up to needed count |
| `fund_all` | `authority_id, total_amount` | OK | Distribute USDC evenly |
| `fund_wallet` | `authority_id, wallet_index, amount` | OK | Fund a specific sub-wallet |
| `consolidate` | `authority_id` | `total_drained` | Drain all sub-wallets to owner |
| `consolidate_wallet` | `authority_id, wallet_index` | `amount_drained` | Drain one sub-wallet |
| `set_max_wallets` | `authority_id, new_max` | OK | Change safety cap |
| `get_authority` | `authority_id` | `ParallelAuthority` | Read authority state |
| `get_sub_wallet` | `authority_id, index` | `SubWallet` | Read sub-wallet state |
| `get_stats` | `authority_id` | `ParallelStats` | Get aggregate stats |
| `pause` | `authority_id` | OK | Emergency pause |
| `unpause` | `authority_id` | OK | Resume operations |
| `remove_wallet` | `authority_id, index` | `amount_drained` | Deactivate and drain a wallet |

### CLI Examples

```bash
# Create a parallel authority with max 500 wallets
dina contract call $DRC63_ADDR create_authority \
  '{"owner":"dina1_your_address","max_wallets":500,"timestamp":1700000000}'

# Create 100 sub-wallets
dina contract call $DRC63_ADDR create_wallets \
  '{"authority_id":"pa-1","count":100,"timestamp":1700000000}'

# Auto-scale to 200 (creates only what's missing)
dina contract call $DRC63_ADDR auto_scale \
  '{"authority_id":"pa-1","needed_count":200,"timestamp":1700000000}'

# Fund all wallets with 1000 USDC (10 USDC each for 100 wallets)
dina contract call $DRC63_ADDR fund_all \
  '{"authority_id":"pa-1","total_amount":1000000000}'

# Check stats
dina contract call $DRC63_ADDR get_stats '{"authority_id":"pa-1"}'

# Emergency pause
dina contract call $DRC63_ADDR pause '{"authority_id":"pa-1"}'

# Consolidate all funds back to owner
dina contract call $DRC63_ADDR consolidate '{"authority_id":"pa-1"}'
```

## Architecture Diagram

```
+---------------------------------------------------------------+
|                    PARALLEL WALLET SYSTEM                       |
|                                                                |
|  Master Wallet: 0xOWNER (controls everything)                  |
|  Authority ID:  pa-1                                           |
|  Total Balance: 10,000 USDC (split across sub-wallets)         |
|  Mode: Auto (selects optimal strategy per transfer)            |
|                                                                |
|  +----------+ +----------+ +----------+     +----------+      |
|  | Wallet 0 | | Wallet 1 | | Wallet 2 | ... | Wallet N |      |
|  | 100 USDC | | 100 USDC | | 100 USDC |     | 100 USDC |      |
|  | nonce: 3 | | nonce: 7 | | nonce: 1 |     | nonce: 0 |      |
|  +----+-----+ +----+-----+ +----+-----+     +----+-----+      |
|       |             |             |                |            |
|  +----v-----+ +----v-----+ +----v-----+     +----v-----+      |
|  | batch tx | | batch tx | | batch tx |     | batch tx |      |
|  | 100 pays | | 100 pays | | 100 pays |     | 100 pays |      |
|  +----------+ +----------+ +----------+     +----------+      |
|       |             |             |                |            |
|       +-------------+-------------+----------------+            |
|                           |                                    |
|                    ALL IN ONE BLOCK                              |
|                    (100ms, 150ms finality)                       |
+---------------------------------------------------------------+

Strategy Selection:
  payments <= 1    -->  [Single]          1 wallet, 1 tx
  payments < 100   -->  [Batch]           1 wallet, 1 batch tx
  payments >= 100  -->  [Parallel-Batch]  N wallets, N batch txs

  Override with priority:
    'cost'  --> minimize on-chain txs (batch mode)
    'speed' --> minimize latency (parallel mode)
    'auto'  --> default smart selection
```

## When to Use Each Mode

| Scenario | Recommended Mode | Why |
|----------|-----------------|-----|
| Single payment | Single | No overhead |
| Pay 10 people | Batch | 1 tx, cheapest |
| Pay 50 people, time-sensitive | Parallel | All in 1 block |
| Payroll (1,000 employees) | Parallel-Batch | 10 wallets x 100 each |
| Airdrop (10,000 recipients) | Parallel-Batch | 100 wallets x 100 each |
| IoT sensor payments (100K/day) | Parallel-Batch + Auto-Scale | Enterprise preset |
| Real-time trading (50 trades/sec) | Parallel | 50 wallets, instant |
| Micro-payments (millions) | Parallel-Batch + Channels | Off-chain + settlement |

## Comparison to Other Chains

```
                    Per-User TPS     Cost for 10,000 payments
                    ~~~~~~~~~~~~     ~~~~~~~~~~~~~~~~~~~~~~~~
Ethereum            0.08             ~$50-500 (gas)
Base                0.5              ~$5-50 (gas)
Solana              2.5              ~$5 (rent + fees)
Sui                 10               ~$1 (gas)
Aptos               10               ~$1 (gas)
Dina (Parallel)     1,000-10,000     $0.01
```

## Real-World Use Cases

1. **Warehouse with 100 robots** -- each robot gets its own sub-wallet, all 100 transact simultaneously
2. **High-frequency AI trading** -- 50 sub-wallets handling different trading pairs, 500 trades/second
3. **IoT sensor network** -- 1,000 sensors reporting data every second, batched into 1-second settlement
4. **Mass payroll / airdrop** -- 10,000 recipients paid in 100ms for $0.01
5. **Payment channel mesh** -- 500 store locations each with parallel wallets, settling independently
