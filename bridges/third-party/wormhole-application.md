# Wormhole Bridge Integration Application

## Application URL
- Contact: https://wormhole.com/contact
- Docs: https://docs.wormhole.com
- Discord: https://discord.gg/wormholecrypto

## What to Include in Application

### Chain Information
- **Chain Name**: Dina Network
- **Chain ID**: dina-testnet-1
- **Consensus**: TurboBFT, 3-7 validators, 100ms finality
- **VM**: WASM (Rust smart contracts via CosmWasm-style runtime)
- **Native Token**: DINA

### RPC Endpoints (All 3 Validators)
- Validator 1: `https://rpc1.dina.network`
- Validator 2: `https://rpc2.dina.network`
- Validator 3: `https://rpc3.dina.network`

### Block Explorer
- `https://dina-developer-portal.web.app/explorer`

### GitHub
- `https://github.com/superbigroach/dina-network`

### Our Wormhole Contract
- Source: `contracts/bridge-wormhole/`
- Already deployed on Dina testnet
- Implements Wormhole message parsing and VAA verification

## What We Need From Wormhole
1. **Guardian set** to monitor Dina Network for cross-chain messages
2. **Chain registration** in the Wormhole registry (assign chain ID)
3. **Relayer support** for automatic message delivery
4. **Token bridge** registration for wrapped asset transfers

## Technical Requirements (From Wormhole Docs)
- Chain must have deterministic finality (TurboBFT provides this at 100ms)
- Must implement Core Bridge contract (our `bridge-wormhole` contract)
- Must support VAA (Verified Action Approval) verification
- Guardian nodes need an RPC endpoint to observe on-chain events
- Chain must support event emission for cross-chain messages

## Integration Architecture
```
Dina Network                    Wormhole                     Target Chain
[User Tx] -->
  [bridge-wormhole contract] -->
    [Emit Message] -->          [Guardian Observation] -->
                                [Sign VAA] -->
                                [Relay VAA] -->              [Receive & Execute]
```

## Assets to Bridge
- USDC (priority)
- ETH (wrapped)
- SOL (wrapped)
- BTC (wrapped via Wormhole Portal)

## Timeline Estimate
- Application submission: Week 1
- Technical review: Weeks 2-4
- Guardian integration: Weeks 4-8
- Testnet bridge live: Week 8-10
- Mainnet: TBD (after Dina mainnet launch)
