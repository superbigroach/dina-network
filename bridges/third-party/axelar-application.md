# Axelar Bridge Integration Application

## Application URL
- Ecosystem: https://axelar.network/ecosystem
- Docs: https://docs.axelar.dev
- Discord: https://discord.gg/axelar

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

## What We Need From Axelar
1. **Gateway contract** deployment on Dina Network
2. **Validator set** monitoring Dina for cross-chain calls
3. **Chain registration** in Axelar's supported chains
4. **GMP (General Message Passing)** support for arbitrary cross-chain calls
5. **ITS (Interchain Token Service)** integration for token bridging

## Technical Requirements (From Axelar Docs)
- Chain must have deterministic finality (TurboBFT: 100ms)
- Must deploy Axelar Gateway contract on-chain
- Must implement Gas Service contract for cross-chain gas payment
- Axelar relayers need RPC access
- Must support event-based message passing

## Integration Architecture
```
Dina Network                    Axelar                       Target Chain
[User Tx] -->
  [Gateway Contract] -->
    [Emit ContractCall] -->     [Axelar Validators] -->
                                [Confirm & Route] -->
                                [Relay to Gateway] -->       [Gateway] -->
                                                               [Execute on Dest]
```

## Assets to Bridge
- USDC (via Axelar wrapped axlUSDC)
- ETH (via axlETH)
- Any ITS-registered token

## Axelar Advantage for Dina
- Axelar supports CosmWasm chains natively (similar to Dina's WASM VM)
- GMP enables full cross-chain smart contract calls, not just token transfers
- Axelar's ITS allows Dina tokens to be natively multichain

## Timeline Estimate
- Application submission: Week 1
- Technical review: Weeks 2-4
- Gateway deployment: Weeks 4-6
- Validator integration: Weeks 6-8
- Testnet bridge live: Week 8-10
- Mainnet: TBD
