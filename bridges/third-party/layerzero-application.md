# LayerZero Bridge Integration Application

## Application URL
- Build: https://layerzero.network/build
- Docs: https://docs.layerzero.network
- Discord: https://discord.gg/layerzero

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

### Our Bridge Contract
- Source: `contracts/bridge-layerzero/` (if exists, otherwise reference `contracts/bridge-wormhole/` as template)

## What We Need From LayerZero
1. **Endpoint deployment** on Dina Network
2. **DVN (Decentralized Verifier Network)** support for Dina
3. **Chain registration** with LayerZero Endpoint ID
4. **OApp (Omnichain Application)** framework support

## Technical Requirements (From LayerZero Docs)
- Chain must have deterministic finality (TurboBFT: 100ms)
- Must deploy LayerZero Endpoint contract on-chain
- Must support Ultra Light Node (ULN) message verification
- DVN nodes need RPC access to read on-chain state
- Must implement SendLib and ReceiveLib for message encoding

## Integration Architecture
```
Dina Network                    LayerZero                    Target Chain
[User Tx] -->
  [OApp Contract] -->
    [LZ Endpoint] -->           [DVN Verification] -->
                                [Executor Delivery] -->      [LZ Endpoint] -->
                                                               [OApp Receive]
```

## Assets to Bridge
- USDC (via OFT standard)
- ETH (wrapped, via OFT)
- Any DRC-1 token (custom OFT adapters)

## Timeline Estimate
- Application submission: Week 1
- Technical review: Weeks 2-3
- Endpoint deployment: Weeks 3-5
- DVN integration: Weeks 5-7
- Testnet bridge live: Week 7-8
- Mainnet: TBD
