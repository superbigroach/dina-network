# Dina Wallet App — User Experience & Architecture

## Overview

The Dina Wallet is a web app (and eventually mobile) where users manage their
money. It looks and feels like a banking app. No crypto terminology. No seed
phrases. Login with Google, Apple, or email. $1/month subscription includes
everything.

---

## Onboarding Flow

```
Step 1: User visits app.dina.network
Step 2: "Sign in with Google" (or Apple, email+password)
Step 3: Set a master passkey (FaceID / fingerprint / device PIN)
Step 4: $1 charged (first month subscription)
Step 5: 9 wallets created automatically in background
Step 6: User sees their dashboard with $0.00 balance
Step 7: "Add money" → link bank account, card, or receive USDC
```

Total time: under 60 seconds. No crypto knowledge required.

---

## Dashboard Layout

```
┌─────────────────────────────────────────────────────────┐
│  DINA                                    [Settings] [?] │
│                                                         │
│  Good morning, Sebastian                                │
│                                                         │
│  Total Balance                                          │
│  $12,847.53                                             │
│  +$1.58 today (yield)             ↑ 4.5% APY            │
│                                                         │
│  [Send]  [Receive]  [Add Money]                         │
│                                                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  MY WALLETS                                             │
│                                                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │
│  │ 💳 Main     │ │ 💰 Savings  │ │ 🔒 Backup   │       │
│  │ $2,347.53   │ │ $10,000.00  │ │ $500.00     │       │
│  │ ★ Default   │ │ 4.5% APY    │ │ Emergency   │       │
│  │ [Use]       │ │ [Use]       │ │ [Use]       │       │
│  └─────────────┘ └─────────────┘ └─────────────┘       │
│                                                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │
│  │ 🤖 Agent 1  │ │ 🤖 Agent 2  │ │ 🤖 Agent 3  │       │
│  │ Shopping    │ │ Bills       │ │ Unused      │       │
│  │ $150.00     │ │ $50.00      │ │ $0.00       │       │
│  │ Limit $500  │ │ Limit $200  │ │ [Set up]    │       │
│  └─────────────┘ └─────────────┘ └─────────────┘       │
│                                                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │
│  │ ⚡ Speed 1   │ │ ⚡ Speed 2   │ │ ⚡ Speed 3   │       │
│  │ Business    │ │ Streaming   │ │ Unused      │       │
│  │ $100.00     │ │ $0.00       │ │ $0.00       │       │
│  │ 100/sec     │ │ [Set up]    │ │ [Set up]    │       │
│  └─────────────┘ └─────────────┘ └─────────────┘       │
│                                                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  YIELD EARNINGS                                         │
│  Today: +$1.58  This month: +$47.40  All time: $284.12 │
│  Balance streaming live ● $12,847.53...54...55...       │
│                                                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  RECENT ACTIVITY                                        │
│  ✓ Received $500.00 from mom          2 min ago         │
│  ✓ Agent 1 paid Amazon $34.99         1 hour ago        │
│  ✓ Sent $25.00 to @carlos             3 hours ago       │
│  ✓ Yield earned $1.58                 today             │
│                                                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  CURRENCIES                                             │
│  USD (USDC)  $12,847.53  ★ Primary                      │
│  EUR (EURC)  €0.00       [Add]                          │
│  GBP (GBPC)  £0.00       [Add]                          │
│  CAD (CADC)  C$0.00      [Add]                          │
│  MXN (MXNC)  $0.00       [Add]                          │
│  + 150 more currencies                                  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Wallet Card Details (Tap to expand)

```
┌─────────────────────────────────────────┐
│ 💳 Main Wallet                    ★ Default │
│                                         │
│ Balance: $2,347.53                      │
│ Yield: +$0.29/day (4.5% APY)           │
│                                         │
│ Today: 3 transactions                   │
│ Bucket: 97/100 available               │
│                                         │
│ Security:                               │
│ ✓ Passkey (FaceID)                      │
│ ✓ Password set                          │
│ ✓ 24hr time-lock on amounts >$1,000    │
│ ✓ Recovery contacts: 3 set             │
│                                         │
│ [Send] [Receive] [Move Money] [Settings]│
└─────────────────────────────────────────┘
```

---

## User-Facing Wallet Names

No crypto jargon. Users never see "SCA", "Agent Wallet", or "Parallel Wallet":

| Internal Name | User Sees | Icon | Purpose |
|--------------|-----------|------|---------|
| SCA-1 | Main Wallet | 💳 | Daily spending (default) |
| SCA-2 | Savings | 💰 | Long-term hold, earns yield |
| SCA-3 | Backup | 🔒 | Emergency reserve |
| Agent-1 | Agent 1 (custom name) | 🤖 | AI assistant spending |
| Agent-2 | Agent 2 (custom name) | 🤖 | AI assistant spending |
| Agent-3 | Agent 3 (custom name) | 🤖 | AI assistant spending |
| Parallel-1 | Speed 1 (custom name) | ⚡ | High-throughput |
| Parallel-2 | Speed 2 (custom name) | ⚡ | High-throughput |
| Parallel-3 | Speed 3 (custom name) | ⚡ | High-throughput |

Users can rename any wallet. "Agent 1" becomes "Shopping Bot".
"Speed 1" becomes "Business Payments".

---

## Security Layers

### Layer 1: Google/Apple Auth (Login)

Gets you into the app. Can view balances and history. Cannot send money.

### Layer 2: Passkey (Authorize Transactions)

FaceID, fingerprint, or device PIN. Required for:
- Sending money from any wallet
- Moving money between wallets
- Changing settings

This is what the user set up during onboarding. The passkey controls the
modular wallet (SCA). It's phishing-resistant and device-bound.

### Layer 3: Per-Wallet Password (Extra Security)

Each wallet has its own optional password. If set:

```
To send from Main Wallet:
  Step 1: Passkey (FaceID)         ← proves it's you
  Step 2: Main Wallet password     ← proves you chose this wallet intentionally
  Step 3: Transaction sends        ← 100ms confirmation
```

For Agent Wallets, the password layer works differently:

```
AGENT WALLET SECURITY:

The company (Dina) holds the signing keys for agent wallets.
This is what lets AI agents transact without your passkey each time.

But the user sets a password that acts as an AUTHORIZATION LAYER:

  AI agent wants to buy something for $34.99
  → Dina's system checks: is this within the user's spending limit? YES
  → Dina's system checks: is this agent wallet active? YES
  → Dina signs the transaction with the agent key
  → Transaction confirms in 100ms
  → User sees notification: "Agent 1 paid Amazon $34.99"

  AI agent wants to buy something for $600 (above $500 limit)
  → Dina's system checks: is this within the spending limit? NO
  → User gets a prompt: "Agent 1 wants to spend $600. Approve?"
  → User enters their agent wallet password + passkey to approve
  → OR user denies it

The password is the override/approval mechanism.
Normal transactions within limits don't need it.
Anything unusual requires it.
```

For Parallel (Speed) Wallets:

```
SPEED WALLET SECURITY:

Same as agent wallets — Dina holds signing keys.
User's password is required to:
  - Fund the speed wallet (move money in from SCA)
  - Change the spending limit
  - Revoke/reset the wallet

Normal transactions within limits flow automatically.
Good for: recurring payments, business batch processing, streaming.
```

### Layer 4: Time-Locks (Large Amounts)

```
Amount          Delay        Can Cancel?
< $1,000       Instant      No (already sent)
$1,000-$10K    1 hour       Yes, from any SCA
$10K-$100K     24 hours     Yes, from any SCA
$100K+         48 hours     Yes, from any SCA + recovery contact
```

### Layer 5: Social Recovery (Nuclear Option)

```
If all else fails:
  - 2 of 3 trusted contacts approve
  - 48-hour waiting period
  - Identity verification (biometric + ID)
  - All wallets reset with new keys
  - Funds intact
```

---

## Subscription Model

### $1/Month — Everything Included

```
WHAT $1/MONTH GETS YOU:

✓ 9 wallets (3 Main + 3 Agent + 3 Speed)
✓ Zero transaction fees (unlimited transactions)
✓ 4.5% APY yield on all USDC balances
✓ 100ms transaction finality
✓ FaceID/fingerprint security
✓ Social recovery
✓ AI agent wallets with spending limits
✓ High-speed parallel wallets
✓ All 155+ currency stablecoins
✓ Free currency conversion (zero FX fees)
✓ Real-time yield streaming
✓ Transaction history and analytics

First month free.
Cancel anytime. Funds always withdrawable.
```

### Why $1/Month Instead of $1 Once

Recurring revenue is more predictable and sustainable:

```
ONE-TIME $1:
  1M users = $1M total, ever
  Need constant new users to generate revenue

$1/MONTH:
  1M users = $1M/month = $12M/year
  Revenue grows AND recurs

  Year 1: avg 500K users × $1/mo × 12 = $6M
  Year 2: avg 1.5M users × $1/mo × 12 = $18M
  Year 3: avg 3M users × $1/mo × 12 = $36M
```

This changes the business model significantly:

```
UPDATED FINANCIALS ($1/month model):

COSTS:
  3 engineers × $150K    = $450K
  21 validators × $3K/mo = $756K
  Cloud/infra             = $100K
  Legal                   = $100K
  Marketing               = $400K
  Total:                   $1,806K/year

REVENUE:
  Year 1: 500K avg users × $12/yr = $6,000,000
  Year 2: 1.5M avg users × $12/yr = $18,000,000
  Year 3: 3M avg users × $12/yr   = $36,000,000

  Plus yield on company treasury ($5M at 4.5% = $225K/yr)

Year 1: Revenue $6.2M - Costs $1.8M = PROFIT $4.4M ← profitable YEAR ONE
Year 2: Revenue $18.2M - Costs $2.5M = PROFIT $15.7M
Year 3: Revenue $36.2M - Costs $3.5M = PROFIT $32.7M
```

$1/month makes the company profitable from Year 1 with 500K users. The yield
on the company treasury is just a bonus, not the primary revenue source.

### Annual Option

```
$10/year (save $2)

Encourages long-term commitment.
Still incredibly cheap for zero-fee banking + yield.
```

---

## Multi-Currency System — Zero-Fee FX

### How It Works

Dina issues stablecoins for every major currency, all on-chain:

```
USDC  — US Dollar (native, from Circle)
EURC  — Euro (from Circle, or Dina-issued)
GBPC  — British Pound
CADC  — Canadian Dollar
JPYC  — Japanese Yen
MXNC  — Mexican Peso
BRSC  — Brazilian Real
INRC  — Indian Rupee
NGNS  — Nigerian Naira
PHPC  — Philippine Peso
...155+ currencies
```

Each stablecoin is backed 1:1 by the real currency held in regulated accounts,
just like USDC is backed by dollars.

### Zero-Fee Currency Conversion

```
User has $1,000 USDC and wants to send €900 to someone in Europe:

Step 1: User taps "Send" → enters €900 → picks recipient
Step 2: App shows: "Send €900 (≈ $972.00 at market rate)"
Step 3: User confirms with passkey
Step 4: On-chain: $972 USDC → €900 EURC (AMM swap, zero fee)
Step 5: Recipient receives €900 EURC in 100ms
Step 6: Recipient can hold EURC (earns yield) or withdraw to bank

Fee: $0
Time: 100ms
Rate: Real-time market rate (Chainlink/Pyth oracle)

Compare:
  Western Union: $10-25 fee + 2-5% FX markup
  Bank wire: $25-50 + 1-3% FX spread
  Wise: $3-7 + 0.5-1% markup
  Dina: $0 + 0% markup (true mid-market rate)
```

### How Zero-Fee FX Is Sustainable

The FX swap happens on DinaDEX (the on-chain AMM). Liquidity providers deposit
both sides of currency pairs and earn yield on their deposits. The swap uses
the constant-product formula with ZERO trading fees.

```
Why would anyone provide liquidity if there are no fees?

Because their deposits earn yield.

LP deposits $100K USDC + €93K EURC into the USDC/EURC pool.
That $193K equivalent earns 4.5% APY = $8,685/year.
The LP earns yield without needing any swap fees.

Traditional DEX: LPs earn from fees (0.3% per swap)
Dina DEX: LPs earn from yield (4.5% on deposits)

Yield-funded liquidity is better for LPs AND free for users.
```

### Yield on Every Currency

```
USDC: 4.5% APY (backed by US T-bills)
EURC: 3.5% APY (backed by EU government bonds)
GBPC: 4.0% APY (backed by UK gilts)
CADC: 3.8% APY (backed by Canadian T-bills)
JPYC: 0.5% APY (backed by Japanese government bonds)

Each currency earns yield from its respective country's government bonds.
Users earn yield in whatever currency they hold.
```

### Real-Time Yield Streaming

The user's balance updates every second:

```
12:00:00  Balance: $12,847.530000
12:00:01  Balance: $12,847.530014  (+$0.000014)
12:00:02  Balance: $12,847.530028  (+$0.000014)
12:00:03  Balance: $12,847.530042  (+$0.000014)
...
Every second, 24/7, 365 days.

$12,847 × 4.5% / 31,536,000 seconds = $0.0000183 per second

On the dashboard, users see their balance ticking up in real time.
"Your money is making money right now."
```

This is made possible by DRC-79 (Micro Payment Stream) built into the protocol.
The yield isn't calculated and deposited monthly like a bank — it streams
continuously, every second, visibly.

---

## Tech Stack

```
Frontend:
  Next.js (React) — web app
  React Native — iOS/Android (Phase 2)
  Tailwind CSS — styling

Auth:
  Firebase Auth (Google, Apple, email sign-in)
  WebAuthn / Passkeys (biometric, device-bound)

Wallet Infrastructure:
  Circle Modular Wallets — SCA creation and key management
  Dina SDK (dina-js) — blockchain interaction

Backend:
  Firebase Functions — subscription billing, notifications
  Dina RPC — direct blockchain communication
  Circle APIs — USDC on/off ramp

Security:
  Passkeys (WebAuthn) — phishing-resistant, device-bound
  Per-wallet passwords — encrypted locally, never sent to server
  Time-locks — smart contract enforced on-chain
  Social recovery — 2-of-3 contact verification
```

---

## User Flows

### Send Money

```
1. Tap [Send]
2. Enter amount: $50
3. Enter recipient: phone number, email, or @username
4. Select wallet: Main (default) or tap to change
5. Confirm with FaceID
6. If wallet has password: enter it
7. "Sent! $50 → @carlos" (100ms)
```

### Receive Money

```
1. Tap [Receive]
2. Show QR code or share link
3. Sender scans or clicks
4. Money arrives in default wallet
5. Push notification: "Received $50 from Mom"
```

### Move Money Between Wallets

```
1. Long-press a wallet card
2. Tap "Move Money"
3. Select source and destination
4. Enter amount
5. Confirm with passkey
6. Instant (same account, just internal transfer)
```

### Set Up an Agent Wallet

```
1. Tap Agent 3 (shows "Set up")
2. Name it: "Travel Bot"
3. Set daily limit: $200
4. Set a password for this wallet
5. Connect to your AI assistant (API key or link)
6. Fund it: move $100 from Main
7. Done — your AI can now spend up to $200/day
```

### Key Compromise — Reset a Wallet

```
1. Notice suspicious transaction on Agent 1
2. Tap Agent 1 → Settings → "Emergency Freeze"
3. Confirm with passkey
4. Agent 1 is instantly frozen
5. Remaining funds auto-return to Main wallet
6. Tap "Reset Wallet"
7. New keys generated (free)
8. Set new password
9. Re-fund if desired
```

### Add a New Currency

```
1. Scroll to Currencies section
2. Tap [Add] next to EUR
3. A EURC wallet is created within your Main wallet
4. Convert: "Swap $500 to €465" → tap confirm
5. Now you hold EURC, earning 3.5% APY in euros
6. Can send euros to anyone in 100ms, zero fees
```
