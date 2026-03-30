# Zero-Fee Model: How Dina Eliminates Transaction Fees

## The Core Insight

Dina is a USDC-native chain with **truly zero transaction fees**. No gas fees.
No micro-fees. No hidden costs. Every transaction costs exactly $0.00.

```
Traditional chain:  Users pay gas fees → Validators earn fees
Dina:               Users pay nothing → Dina Inc. funds validators
```

Gas is still **tracked internally** (to prevent infinite loops and abuse in smart
contracts), but nobody pays for it. The gas meter is a safety mechanism, not a
billing system.

## How It Works

### Zero Fees, Full Gas Metering

Every smart contract execution runs through a `GasMeter` that counts operations:

| Operation | Gas Cost | USDC Cost |
|-----------|----------|-----------|
| WASM instruction | 1 gas | $0.00 |
| Storage read | 100 gas | $0.00 |
| Storage write | 500 gas | $0.00 |
| USDC transfer | 200 gas | $0.00 |
| Cross-contract call | 1,000 gas | $0.00 |
| SHA-256 hash | 50 gas | $0.00 |
| Ed25519 verify | 300 gas | $0.00 |

The gas price is **zero**. Gas is consumed during execution but costs nothing.
If a contract hits the gas limit (e.g., infinite loop), execution halts with
"out of gas" -- protecting the network without charging the user.

This is the breakthrough: **gas metering for safety, zero gas pricing for users.**

### Users Keep 100% of Their Yield

When users hold USDC on Dina, they earn yield (~4.5% APY) from the underlying
US Treasury-backed reserves. **Users keep all of their yield.** Dina does not
take a cut.

```
User holds 10,000 USDC on Dina
→ Earns 4.5% APY = $450/year
→ All $450 goes to the user
→ Dina takes $0
```

### How Validators Get Paid

Dina operates 21 validators that verify transactions, produce blocks, and
maintain consensus. Validator infrastructure costs are an **operational expense
funded by Dina Inc.**:

| Expense | Monthly Cost |
|---------|-------------|
| 21 validators (e2-medium GCE) | ~$1,575 |
| Bandwidth and monitoring | ~$500 |
| DevOps and maintenance | ~$500 |
| **Total** | **~$2,575/month** |

This is funded through:
1. **Raised capital** -- seed/Series A funding covers early operations
2. **Enterprise revenue** -- paid tiers for guaranteed throughput
3. **Wallet activations** -- $1 one-time activation per wallet

The cost is remarkably low. Running a global payment network that processes
100K+ TPS costs less than $3K/month in infrastructure.

## Spam Prevention Without Fees

Without fees, how do we prevent spam? Three mechanisms:

### A. Rate Limiting Per Account

Every account gets a transaction quota based on its USDC balance:

```
Rate allocation = (account_balance / total_TVL) * network_capacity
```

A wallet holding $10,000 on a network with $1B TVL and 100K TPS capacity gets:
`($10,000 / $1B) * 100,000 = 1 TPS` sustained throughput.

### B. Proof of Balance

To transact, you must hold a minimum USDC balance ($1). This makes Sybil
attacks expensive -- creating 1 million spam accounts requires holding $1M in
USDC.

### C. Priority Lanes (Optional Paid Tier)

| Tier | Cost | Guarantee |
|------|------|-----------|
| Free | $0 | Best-effort, rate-limited by balance |
| Priority | $99/mo | Guaranteed 100 TPS, never throttled |
| Enterprise | $999/mo | Guaranteed 10K TPS, dedicated lane |

## Why This Works

### The Flywheel

```
Zero fees attract users
→ Users bring USDC
→ Users earn yield (keeps them on the network)
→ Enterprise customers pay for priority
→ Revenue funds validators + growth
→ More users
→ Repeat
```

### Comparison to Other Chains

| Chain | Fee Model | User Experience |
|-------|-----------|-----------------|
| Ethereum | $1-50 per tx (gas in ETH) | Expensive, volatile |
| Solana | $0.001 per tx (gas in SOL) | Cheap but need SOL |
| Base | $0.01 per tx (gas in ETH) | Need ETH to start |
| **Dina** | **$0.00 per tx (no gas token)** | **Free, no token needed** |

Every other chain requires users to hold a volatile gas token before they can
do anything. Dina requires nothing -- just USDC.

### Why No Other Blockchain Can Copy This

Token-based blockchains are structurally unable to go zero-fee because:
1. Their validators are paid in the native token via inflation + fees
2. Removing fees removes validator incentives
3. They have no alternative revenue source

Dina can be zero-fee because validators are paid by Dina Inc., not by users.
The network doesn't need fee revenue to function.

## Implementation Status

| Component | Status |
|-----------|--------|
| Zero-fee transactions (fee=0) | Implemented |
| Gas metering (GasMeter) | Implemented |
| 100ms block time | Implemented |
| Ed25519 signed transactions | Implemented |
| Rate limiting by balance | Planned |
| $1 minimum balance | Planned |
| Enterprise priority tiers | Planned |

## Summary

Dina is the first blockchain where transactions are truly free. Gas is metered
for safety but costs nothing. Users keep 100% of their USDC yield. Validators
are funded as an operational expense, not by extracting value from users.

This is not a temporary subsidy or a loss leader. It's the permanent economic
model of the network.
