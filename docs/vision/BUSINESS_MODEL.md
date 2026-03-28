# Dina Network Business Model

## The Model: Yield-Funded Operations, Zero User Fees

The company raises capital, invests it in US Treasury bills, and uses the yield
to permanently fund all operations. Users pay $1 once for wallet access. Zero
transaction fees. Users earn 100% of the yield on their own USDC.

## Operating Costs (Annual)

| Expense | Year 1 (7 val) | Year 2 (13 val) | Year 3 (21 val) |
|---------|---------------:|----------------:|----------------:|
| 3 engineers × $150K | $450,000 | $450,000 | $450,000 |
| Validators | $252,000 | $468,000 | $756,000 |
| Cloud/infra | $50,000 | $75,000 | $100,000 |
| Legal/compliance | $50,000 | $75,000 | $100,000 |
| Marketing | $200,000 | $300,000 | $400,000 |
| **Total** | **$1,002,000** | **$1,368,000** | **$1,806,000** |

## The Raise: How Much Covers Everything

At 4.5% T-bill yield, to cover $1,806,000/year (Year 3 peak costs) from yield
alone:

```
$1,806,000 / 0.045 = $40,133,333
```

Round up: **$42M endowment covers all operating costs forever from yield alone.**

But you don't need to cover Year 3 costs from day one. The phased approach:

### Raise $8M (Seed + Series A combined)

```
Allocation:
  $5,000,000 → Treasury (T-bills at 4.5% = $225,000/year yield)
  $3,000,000 → Operating cash (runway buffer)

Year 1:
  Yield income:     $225,000
  Wallet fees:      $76,000  (76K users × $1)
  Total income:     $301,000
  Total costs:      $1,002,000
  Net burn:         $701,000
  Cash used:        $701,000 from operating reserves
  Remaining cash:   $2,299,000

Year 2:
  Yield income:     $225,000
  Wallet fees:      $424,000 (424K new users × $1)
  Enterprise subs:  $200,000
  Total income:     $849,000
  Total costs:      $1,368,000
  Net burn:         $519,000
  Cash used:        $519,000
  Remaining cash:   $1,780,000

Year 3:
  Yield income:     $225,000
  Wallet fees:      $1,500,000 (1.5M new users × $1)
  Enterprise subs:  $500,000
  Total income:     $2,225,000
  Total costs:      $1,806,000
  NET PROFIT:       $419,000 ← self-sustaining
  Remaining cash:   $2,199,000 (growing)

Year 4:
  Yield income:     $225,000
  Wallet fees:      $3,000,000 (3M new users)
  Enterprise subs:  $1,000,000
  Total income:     $4,225,000
  Total costs:      $2,200,000 (hire more, scale)
  NET PROFIT:       $2,025,000
```

At Year 3, the company is profitable. The $5M treasury stays invested
permanently, generating $225K/year forever. The operating cash was never fully
depleted. Wallet fee revenue scales with adoption and eventually dominates.

### Why $8M Is Enough

| Metric | Value |
|--------|-------|
| Total raise | $8,000,000 |
| Treasury (permanent) | $5,000,000 |
| Operating runway | $3,000,000 |
| Monthly burn (Year 1) | $83,500 |
| Months of runway | 36 (cash) + infinite (yield) |
| Breakeven | ~Month 28 |
| Users at breakeven | ~1,400,000 |
| Profitable | Year 3 |
| Marketing budget | $200K-$400K/year (included) |

### Alternative: Raise $15M For Full Safety

```
$15M raised:
  $10M → Treasury ($450,000/year yield)
  $5M  → Operating cash

Year 1 burn covered 45% by yield alone ($450K of $1M).
Cash runway: 5+ years even with zero users.
Breakeven at just 550,000 users regardless of timeline.
Full marketing budget from day one.
Never stresses about runway.
```

This gives room for aggressive marketing, slower-than-expected adoption, and
the ability to keep transaction fees at zero from day one without worrying
about revenue timing.

---

## The Wallet System

### What $1 Gets You

Every user pays $1 once (in USDC) to activate their Dina account. This creates:

```
YOUR DINA ACCOUNT ($1 one-time activation)
│
├── 🏦 Smart Contract Account (SCA) ×3
│   ├── SCA-1: Main account (daily spending)
│   ├── SCA-2: Savings account (long-term hold, earns yield)
│   └── SCA-3: Backup account (emergency reserve)
│   │
│   Each SCA has:
│   ├── Its own password (set by you)
│   ├── Its own balance
│   ├── Social recovery (3 trusted contacts)
│   ├── Biometric auth (FaceID/fingerprint)
│   └── Time-locked large withdrawals (24hr delay on amounts >$1,000)
│
├── 🤖 Agent Wallet ×3
│   ├── Agent-1: Shopping agent (limit: $500/day)
│   ├── Agent-2: Subscription manager (limit: $100/day)
│   └── Agent-3: Trading bot (limit: $2,000/day)
│   │
│   Each Agent Wallet has:
│   ├── Its own password (separate from SCA)
│   ├── Spending limits (set by you)
│   ├── Revocable by any SCA (instant kill switch)
│   └── Cannot access SCA funds directly
│
└── ⚡ Parallel Wallet ×3
    ├── Parallel-1: Business payments (high throughput)
    ├── Parallel-2: Streaming payments (salary/subscriptions)
    └── Parallel-3: API transactions (developer use)
    │
    Each Parallel Wallet has:
    ├── Its own password (separate from SCA and agents)
    ├── Its own nonce (enables true simultaneous transactions)
    ├── Revocable by any SCA
    └── Configurable spending limits
```

Total: 3 SCAs + 3 Agent Wallets + 3 Parallel Wallets = **9 wallets per user**.
Each with its own password. Each independently controllable.

### Why 3 of Each

```
3 SCAs give you:
  - Separation of concerns (spending / savings / emergency)
  - If one SCA key is compromised, move funds to another in seconds
  - The compromised SCA can be reset while others keep working
  - Like having 3 bank accounts at different banks

3 Agent Wallets give you:
  - Different AI agents for different tasks
  - Different spending limits per agent
  - If one agent is compromised, revoke just that one
  - Other agents keep working normally

3 Parallel Wallets give you:
  - 3 independent transaction streams simultaneously
  - Each has its own nonce, so they don't block each other
  - Business can run 3 payment processors at once
  - If one key leaks, kill it, others unaffected
```

### Key Compromise Recovery

```
SCENARIO: Your Agent-2 password is stolen

Step 1: You notice unauthorized transactions from Agent-2

Step 2: From ANY of your 3 SCAs, send one command:
        revoke_wallet(agent_2)
        → Agent-2 is instantly frozen. No more transactions.
        → Attacker locked out immediately.

Step 3: Any remaining funds in Agent-2 auto-return to SCA-1
        (built into the smart contract — on revocation, sweep funds home)

Step 4: Generate fresh Agent-2 with new keys and new password
        → Free (just a transaction, zero fees)

Step 5: Fund new Agent-2 from SCA-1 with whatever limit you want

Total time: ~30 seconds
Total cost: $0
Funds at risk: only what was in Agent-2 (you set the limit)
Your SCAs: never touched, never at risk
```

```
SCENARIO: Your SCA-1 password is stolen (worst case)

Step 1: Attacker tries to drain SCA-1
        → Large withdrawal triggers 24hr time-lock
        → You get a notification

Step 2: From SCA-2 or SCA-3, emergency freeze SCA-1:
        freeze_wallet(sca_1)
        → SCA-1 frozen. Time-locked withdrawal cancelled.

Step 3: Social recovery on SCA-1:
        → 2 of 3 trusted contacts approve a key reset
        → New password set on SCA-1

Step 4: Unfreeze SCA-1 from SCA-2 or SCA-3

Total time: ~24 hours (social recovery)
Funds lost: $0 (time-lock prevented the theft)
```

```
SCENARIO: ALL keys compromised (phone stolen, everything hacked)

Step 1: Contact 2 of 3 trusted recovery contacts

Step 2: They each submit a recovery transaction
        → After 2/3 contacts confirm, a 48hr recovery window opens

Step 3: All wallets freeze automatically

Step 4: You verify identity (biometric + ID document)

Step 5: New device, new keys, all funds intact

This is the nuclear option. Slow (48hrs) but funds are always safe.
```

### Password Architecture

Every wallet has its own independent password:

```
SCA-1:      "correct-horse-battery-staple"     ← you remember this
SCA-2:      "different-password-entirely"        ← different
SCA-3:      "third-unique-passphrase-here"       ← different again
Agent-1:    "agent-shopping-pass-2024"           ← different
Agent-2:    "agent-subs-another-pass"            ← different
Agent-3:    "trading-bot-secure-word"            ← different
Parallel-1: "business-parallel-key"              ← different
Parallel-2: "streaming-pay-passkey"              ← different
Parallel-3: "api-access-passphrase"              ← different

Compromising one password exposes ONE wallet.
The other 8 are completely unaffected.
Each password encrypts that wallet's private key locally.
Passwords never leave your device.
```

On top of passwords, each wallet can optionally have:
- Biometric (FaceID/fingerprint) as second factor
- Hardware key (YubiKey) as second factor
- Passkey (device-bound, phishing-resistant)

---

## Transaction Speed — No Limits For Normal Users

### The Problem With Rate Limiting

Rate limiting says "you can only send X transactions per second." This feels
restrictive. A better approach for Dina:

### The Solution: Generous Defaults + Burst Capacity

Instead of strict per-second rate limiting, use a **token bucket** system:

```
Every account gets a transaction BUCKET that refills over time.

Bucket size: 100 transactions
Refill rate: 1 transaction per second (86,400 per day)

This means:
  - You can send 100 transactions INSTANTLY (burst)
  - Then 1 per second sustained
  - If you wait 2 minutes, you have 120 in your bucket again
```

For a normal user this feels unlimited:

```
NORMAL USER DAY:

8:00 AM  - Buy coffee (1 tx)              Bucket: 99 remaining
12:00 PM - Pay for lunch (1 tx)            Bucket: 100 (refilled)
2:00 PM  - Send money to friend (1 tx)     Bucket: 100 (refilled)
6:00 PM  - Buy groceries (1 tx)            Bucket: 100 (refilled)
8:00 PM  - Pay subscription (1 tx)         Bucket: 100 (refilled)

Used: 5 transactions. Bucket never went below 95.
This person will NEVER feel rate limited.
```

For a business doing lots of transactions:

```
BUSINESS (e-commerce):

Process 500 orders in one batch:
  First 100: instant (bucket burst)
  Next 400: 1 per second = 6.7 minutes
  Total: ~7 minutes for 500 transactions

With 3 Parallel Wallets (each has its own bucket):
  Parallel-1: 100 instant + sustained
  Parallel-2: 100 instant + sustained
  Parallel-3: 100 instant + sustained
  = 300 instant + 3/second sustained
  500 transactions in ~1.1 minutes
```

For power users and businesses that need more, the bucket scales with balance:

```
BALANCE-SCALED BUCKETS:

Balance          Bucket Size    Refill Rate     Daily Capacity
$1-$99           100 tx         1/sec           86,400
$100-$999        500 tx         5/sec           432,000
$1,000-$9,999    2,000 tx       20/sec          1,728,000
$10,000-$99,999  10,000 tx      100/sec         8,640,000
$100,000+        50,000 tx      500/sec         43,200,000
```

With 3 Parallel Wallets, multiply all numbers by 3.

A business holding $100K with 3 parallel wallets:
= 150,000 burst + 1,500/sec sustained = 129,600,000 per day

That's more than Visa's global daily transaction volume.

### Spam Prevention Without Restricting Users

The bucket system prevents spam naturally:

```
ATTACKER with $1 balance:
  Bucket: 100 transactions, refills 1/sec
  Can send 100 spam tx instantly, then 1/sec
  = 86,500 spam tx per day maximum from one account

  Network capacity: 100,000 TPS = 8,640,000,000 tx/day
  One attacker: 86,500 / 8,640,000,000 = 0.000001% of capacity

  Negligible. Not a threat.

ATTACKER with 1,000 accounts ($1 each = $1,000 total):
  = 86,500,000 tx/day
  = 0.001% of capacity
  Still negligible.

ATTACKER with 1,000,000 accounts ($1M total):
  = 86,500,000,000 tx/day
  Exceeds network capacity.
  BUT: attacker has $1M locked on the network
       generating yield that pays for infrastructure.
  AND: at this scale, the network can flag and throttle
       coordinated spam patterns (same behavior, same timing).
```

To actually overwhelm the network, an attacker needs to lock up so much USDC
that the yield from their deposit funds the defense against their own attack.

### Every Transaction Is Still 100ms

The bucket controls HOW MANY you can send. Every transaction that gets through
still confirms in 100ms. The speed never changes. Whether you send 1 or 1,000,
each one confirms in 100ms.

```
Transaction 1: sent 12:00:00.000 → confirmed 12:00:00.100
Transaction 2: sent 12:00:00.001 → confirmed 12:00:00.101
Transaction 3: sent 12:00:00.002 → confirmed 12:00:00.102
...all 100ms, no exceptions
```

---

## Summary

```
RAISE:        $8M ($5M treasury + $3M operating)
TEAM:         3 engineers + founder
VALIDATORS:   7 → 13 → 21 (phased with growth)
REVENUE:      $225K/year yield + $1/user wallet fees + enterprise subs
BREAKEVEN:    Month 28, ~1.4M users
PROFITABLE:   Year 3

USER GETS FOR $1:
  3 Smart Contract Accounts (recoverable, yield-earning)
  3 Agent Wallets (AI spending, limited, revocable)
  3 Parallel Wallets (high-throughput, independent nonces)
  Each with its own password
  Zero transaction fees forever
  100% USDC yield (4.5% APY)
  100ms finality on every transaction
  Social recovery if all keys lost

TRANSACTION LIMITS:
  100 burst + 1/sec sustained (minimum, $1 balance)
  Scales to 50,000 burst + 500/sec ($100K+ balance)
  ×3 with Parallel Wallets
  Normal users never hit any limit

SECURITY:
  9 independent wallets, 9 independent passwords
  Compromise one → other 8 unaffected
  SCAs protect each other (cross-freeze)
  Time-locked large withdrawals
  Social recovery (2-of-3 trusted contacts)
  Key reset costs $0 (free transaction)
```
