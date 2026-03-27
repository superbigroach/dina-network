# DinaDEX — Fee-Free Decentralized Exchange

The Dina Network's built-in AMM swap. Zero fees. 100ms swaps.

## Contract
The DinaDEX contract is at: contracts/dex-swap/

## How It Works
- Constant product AMM (x * y = k), same math as Uniswap V2
- Zero trading fees (configurable, but default 0%)
- 100ms swap execution (vs 12s on Ethereum)
- Multi-hop routing (ETH -> USDC -> SOL in one tx)
- Any DRC-1 token can be listed

## Why Zero Fees?
Most DEXes charge 0.3% per swap. DinaDEX charges 0%.
Revenue comes from the network itself, not from taxing users.
LPs earn from token price appreciation and can set their own fees later.

## Tokens Available (after bridge integration)
- USDC (native)
- Bridged ETH (via Base bridge)
- Bridged SOL (via Wormhole)
- Bridged BTC (via Wormhole)
- Any DRC-1 token deployed on Dina
