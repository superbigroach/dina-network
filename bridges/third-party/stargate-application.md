# Stargate Bridge Integration Application

## Application URL
- Website: https://stargate.finance
- Docs: https://stargateprotocol.gitbook.io
- Discord: https://discord.gg/stargate

## What to Include in Application

### Chain Information
- **Chain Name**: Dina Network
- **Chain ID**: dina-testnet-1
- **Consensus**: TurboBFT, 3-7 validators, 100ms finality
- **VM**: WASM (Rust smart contracts)
- **Native Token**: DINA

### RPC Endpoints (All 3 Validators)
- Validator 1: `https://rpc1.dina.network`
- Validator 2: `https://rpc2.dina.network`
- Validator 3: `https://rpc3.dina.network`

### Block Explorer
- `https://dina-developer-portal.web.app/explorer`

### GitHub
- `https://github.com/superbigroach/dina-network`

## What We Need From Stargate
1. **Pool listing** for Dina Network on Stargate V2
2. **LayerZero endpoint** (Stargate V2 uses LayerZero — may need LayerZero first)
3. **OFT (Omnichain Fungible Token)** integration
4. **Unified liquidity pool** access for USDC/ETH

## Technical Requirements (From Stargate Docs)
- Stargate V2 is built on LayerZero (apply to LayerZero first)
- Must deploy Stargate Pool contracts on-chain
- Must support OFT standard for cross-chain token transfers
- Chain must have deterministic finality (TurboBFT: 100ms)
- Must implement messaging via LayerZero Endpoint

## Integration Architecture
```
Dina Network                    Stargate (via LayerZero)     Target Chain
[User Swap/Bridge] -->
  [Stargate Pool] -->
    [LZ Endpoint] -->           [DVN Verification] -->
                                [Cross-chain Msg] -->        [Stargate Pool] -->
                                                               [User Receives]
```

## Assets to Bridge
- USDC (primary — Stargate is the main stablecoin bridge)
- ETH (via OFT)
- USDT

## Stargate Advantage for Dina
- Stargate provides unified liquidity: no fragmented wrapped tokens
- V2 uses LayerZero for messaging (apply to both)
- High volume protocol means deep liquidity for Dina users
- Taxi/Bus model allows cost-optimized bridging

## Dependency
- LayerZero integration should be completed first (Stargate V2 requires it)
- Apply to LayerZero and Stargate in parallel, but deploy LayerZero endpoint first

## Timeline Estimate
- Application submission: Week 1 (parallel with LayerZero)
- LayerZero endpoint required first: Weeks 3-7
- Stargate pool deployment: Weeks 7-9
- Testnet bridge live: Week 9-11
- Mainnet: TBD
