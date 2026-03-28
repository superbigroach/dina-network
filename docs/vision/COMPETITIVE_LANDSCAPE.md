# Competitive Landscape: How Every Chain Charges Fees Today

## Current Fee Models Across All Blockchains

Every blockchain in existence charges transaction fees. No exceptions.

### How It Works Everywhere Else

When you send a transaction on ANY chain, you pay a fee in the chain's native token:

| Chain | Native Token | Avg Fee | Fee Currency | Annual Fee Revenue |
|-------|-------------|---------|--------------|-------------------|
| Ethereum | ETH | $0.50-$50 | ETH | ~$2B |
| Solana | SOL | $0.00025 | SOL | ~$400M |
| Avalanche | AVAX | $0.01-$0.10 | AVAX | ~$50M |
| Polygon | MATIC/POL | $0.001-$0.01 | POL | ~$30M |
| Arbitrum | ETH | $0.01-$0.10 | ETH | ~$100M |
| Base | ETH | $0.001-$0.01 | ETH | ~$50M |
| BNB Chain | BNB | $0.03-$0.10 | BNB | ~$200M |
| Tron | TRX | $0.10-$1.00 | TRX (energy) | ~$1B |
| Near | NEAR | $0.001 | NEAR | ~$5M |
| Sui | SUI | $0.001 | SUI | ~$10M |
| Aptos | APT | $0.001 | APT | ~$5M |

### Why They All Charge Fees

1. **Spam prevention**: Without fees, an attacker could flood the network with
   billions of garbage transactions for free, consuming all block space.

2. **Validator compensation**: Validators run expensive hardware. Fees (plus
   token inflation) pay them to keep the network running.

3. **Economic security**: In proof-of-stake chains, the cost of attacking the
   network must exceed the potential profit. Fees contribute to this security
   budget.

4. **Resource pricing**: Block space is finite. Fees act as a market mechanism
   to allocate scarce block space to the highest-value transactions.

### The Problem With Fee-Based Models

- **Users must hold native tokens**: Before you can use Ethereum, you must buy
  ETH. Before Solana, buy SOL. This is friction that kills adoption.

- **Fee volatility**: Ethereum gas can spike 100x during congestion. Users get
  surprise $50 fees on a $10 swap. Unpredictable costs make budgeting impossible
  for businesses.

- **Token speculation conflicts with utility**: ETH's price is driven by
  speculation, not by the network's actual value as infrastructure. This creates
  perverse incentives.

- **Inflation subsidy**: Most chains pay validators primarily through token
  inflation (printing new tokens), not fees. This dilutes holders and is
  unsustainable long-term.

### Dina's Current Fee Model

Dina currently charges a small USDC-denominated fee per transaction:

```
Base fee: 10 micro-USDC ($0.00001) per transaction
```

This is already among the cheapest in the industry, and crucially, it's
denominated in USDC (not a volatile token). Users never need to buy a
separate gas token.

But the vision is to eliminate this fee entirely.

## Why "Cheapest Fees" Is Not Enough

Being the cheapest is a race to the bottom. Solana charges $0.00025. Sui charges
$0.001. Someone will always undercut you.

The winning strategy is not "cheapest fees." It's **zero fees** -- a category
of one. You can't be cheaper than free.

The question is: how do you sustain a network with zero transaction fees without
collapsing?

See [ZERO_FEE_MODEL.md](ZERO_FEE_MODEL.md) for the answer.
