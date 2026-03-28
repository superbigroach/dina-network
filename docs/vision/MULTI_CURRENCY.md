# Multi-Currency System — Zero-Fee Global FX

## The Vision

Every currency in the world as a stablecoin on Dina. Convert between any pair
instantly, for free, at the real mid-market rate. Every currency earns yield
from its home country's government bonds.

This replaces:
- Western Union ($10-25 fee + 5% FX markup)
- Bank wires ($25-50 + 1-3% spread)
- Wise ($3-7 + 0.5% markup)
- PayPal international (3.5% + FX spread)
- Visa cross-border (1-3% foreign transaction fee)

With: $0 fee, 0% markup, 100ms settlement, yield on every currency.

---

## Currency List (Phase 1 — Top 20)

| Symbol | Currency | Backing | Yield Source | Est. APY |
|--------|----------|---------|-------------|---------|
| USDC | US Dollar | Circle (T-bills) | US Treasury | 4.5% |
| EURC | Euro | EU gov bonds | ECB rate | 3.5% |
| GBPC | British Pound | UK gilts | BoE rate | 4.0% |
| CADC | Canadian Dollar | CA T-bills | BoC rate | 3.8% |
| JPYC | Japanese Yen | JGB | BoJ rate | 0.5% |
| CHFC | Swiss Franc | Swiss gov bonds | SNB rate | 1.5% |
| AUDC | Australian Dollar | AU gov bonds | RBA rate | 4.0% |
| CNHC | Chinese Yuan (offshore) | CNH instruments | PBoC rate | 2.5% |
| INRC | Indian Rupee | India gov bonds | RBI rate | 6.5% |
| KRWC | South Korean Won | KR gov bonds | BoK rate | 3.2% |
| MXNC | Mexican Peso | MX gov bonds | Banxico rate | 10.5% |
| BRSC | Brazilian Real | BR gov bonds | BCB rate | 12.5% |
| SGDC | Singapore Dollar | SG gov bonds | MAS rate | 3.2% |
| HKDC | Hong Kong Dollar | HK gov bonds | HKMA rate | 4.3% |
| SEKG | Swedish Krona | SE gov bonds | Riksbank rate | 3.0% |
| NOKC | Norwegian Krone | NO gov bonds | Norges rate | 3.5% |
| NZDC | New Zealand Dollar | NZ gov bonds | RBNZ rate | 4.5% |
| ZARC | South African Rand | ZA gov bonds | SARB rate | 8.0% |
| TRYL | Turkish Lira | TR gov bonds | TCMB rate | 45.0% |
| NGNS | Nigerian Naira | NG gov bonds | CBN rate | 18.0% |

Phase 2: 50 currencies. Phase 3: 155+ currencies (every ISO 4217 code).

---

## How Currency Stablecoins Work

Each Dina currency stablecoin is backed 1:1:

```
1 EURC on Dina = €1 held in a regulated European bank account
1 GBPC on Dina = £1 held in a regulated UK bank account
1 MXNC on Dina = MX$1 held in a regulated Mexican bank account

The backing currency is invested in that country's government bonds.
The yield flows to holders of the stablecoin.
```

### Minting and Redemption

```
MINTING (real currency → stablecoin):
  User sends €1,000 to Dina's EU bank partner
  → Dina mints 1,000 EURC on-chain
  → User receives 1,000 EURC in their wallet
  → €1,000 is invested in EU government bonds

REDEMPTION (stablecoin → real currency):
  User burns 1,000 EURC on-chain
  → Dina's EU bank partner sends €1,000 to user's bank
  → €1,000 worth of bonds are sold
  → Settlement: 1-2 business days to bank (instant on-chain)
```

---

## How Zero-Fee FX Conversion Works

### On-Chain Currency Swap

```
User has 1,000 USDC and wants 930 EURC:

1. User taps: "Swap $1,000 → EUR"
2. Oracle reports: EUR/USD = 0.93
3. On-chain AMM (DinaDEX) executes:
   Input:  1,000 USDC
   Output: 930 EURC (at oracle rate, zero fee)
4. User now has 930 EURC earning 3.5% APY
5. Time: 100ms
6. Fee: $0
```

### Why No Fee Works for FX

Traditional FX charges fees because:
1. Banks need to cover operational costs
2. Market makers need profit for providing liquidity
3. Settlement takes days and has counterparty risk

Dina eliminates all three:
1. Operations funded by yield on company treasury
2. Liquidity providers earn yield on deposits (no fees needed)
3. Settlement is instant and atomic (no counterparty risk)

```
LIQUIDITY PROVIDER ECONOMICS:

LP deposits $500K USDC + €465K EURC into USDC/EURC pool
Total value: ~$1M

Traditional DEX LP income:
  0.3% fee × daily volume → maybe 5-20% APY
  But: impermanent loss risk, fee income varies wildly

Dina LP income:
  4.5% APY on the USDC portion ($500K × 4.5% = $22,500)
  3.5% APY on the EURC portion (€465K × 3.5% = €16,275)
  Total: ~$40,000/year guaranteed from yield alone
  No fee income needed. No impermanent loss on stable pairs.

  EUR/USD barely moves (±5% per year), so IL is minimal.
  Yield income dominates. LPs are happy. Users pay zero.
```

### Cross-Currency Payments

```
Sebastian in Canada sends ₹50,000 to Priya in India:

1. Sebastian taps Send → ₹50,000 → Priya's phone number
2. App shows: "Send ₹50,000 (≈ C$820 at market rate)"
3. Sebastian confirms with FaceID
4. On-chain:
   a. C$820 CADC swaps to → $595 USDC (via CADC/USDC pool)
   b. $595 USDC swaps to → ₹50,000 INRC (via USDC/INRC pool)
   c. All in one atomic transaction
5. Priya receives ₹50,000 INRC in 100ms
6. Priya earns 6.5% APY on her rupees

Fee: $0
Time: 100ms (not 3-5 business days)
FX rate: Real-time mid-market (no markup)

Western Union would charge: C$45 + 3% FX markup = ~C$70 in costs
Dina: C$0
```

---

## Yield by Country — Why This Matters

### Developing Countries Get the Most

```
A user in Turkey holding 10,000 TRYL (Turkish Lira stablecoin):
  Yield: 45% APY = 4,500 TRYL/year

  Turkish bank savings account: 30-40% (if you're lucky)
  Turkish inflation: ~50%

  Dina gives them government bond rates without needing
  a brokerage account or minimum balances.

A user in Nigeria holding 1,000,000 NGN (Naira stablecoin):
  Yield: 18% APY = 180,000 NGN/year

  Nigerian bank savings: 3-5%

  Dina gives them 4-6x what their bank pays.

A user in Brazil holding 5,000 BRL (Real stablecoin):
  Yield: 12.5% APY = 625 BRL/year

  Most Brazilians earn <5% in savings accounts.
```

### Why People Would Switch

```
Brazilian user today:
  Bank savings: 5% APY
  Fees: monthly account fee, transfer fees, FX markup
  International transfers: 3-5 days, $25+ fee

Brazilian user on Dina:
  BRSC yield: 12.5% APY (2.5x better)
  Fees: $1/month (everything included)
  International transfers: 100ms, $0 fee

  Plus they can hold USD, EUR, GBP simultaneously
  and earn yield on all of them.
```

---

## Regulatory Approach

### Per-Currency Licensing

Each stablecoin requires regulatory compliance in its home jurisdiction:

| Currency | Jurisdiction | License Needed | Estimated Cost |
|----------|-------------|----------------|---------------|
| USDC | USA | Already Circle-licensed | $0 (use Circle) |
| EURC | EU | EMI license (MiCA) | €500K-€1M |
| GBPC | UK | FCA e-money license | £500K |
| CADC | Canada | MSB registration | C$50K |
| MXNC | Mexico | CNBV fintech license | MX$1M |
| INRC | India | RBI approval | Complex |
| NGNS | Nigeria | CBN approval | Complex |

### Phased Rollout Strategy

```
Phase 1 (Launch):     USDC only (Circle handles everything)
Phase 2 (Month 6):    Add EURC, GBPC, CADC (established regulatory frameworks)
Phase 3 (Month 12):   Add JPYC, AUDC, CHFC, SGDC (developed markets)
Phase 4 (Month 18):   Add MXNC, BRSC, INRC, ZARC (emerging markets)
Phase 5 (Month 24+):  Remaining currencies as licenses are obtained
```

---

## Technical Implementation

### On-Chain

Each currency is a DRC-1 token contract:
```
EURC: DRC-1 token, admin = Dina Inc., mint/burn restricted to licensed minter
GBPC: DRC-1 token, admin = Dina Inc., same pattern
...
```

FX swaps via DinaDEX pools:
```
Pool: USDC/EURC    (most liquid, primary routing pair)
Pool: USDC/GBPC
Pool: USDC/JPYC
Pool: EURC/GBPC    (direct EUR↔GBP without USDC hop)
Pool: USDC/MXNC
...
```

Cross-currency routing:
```
CADC → INRC route:
  Best path: CADC → USDC → INRC (2 hops via USDC)
  Or if enough volume: CADC → INRC direct pool
  Router picks cheapest path automatically (zero fee either way)
```

### Oracle Integration

Real-time FX rates from:
- Chainlink price feeds (primary)
- Pyth Network (secondary)
- Custom aggregator that averages multiple sources

Prices update every block (100ms). Stale price protection:
if oracle data is >5 seconds old, swap is rejected.
