# Across Bridge Integration Application

## Application URL
- Ecosystem: https://across.to/ecosystem
- Docs: https://docs.across.to
- Discord: https://discord.gg/across

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

## What We Need From Across
1. **Spoke pool** deployment on Dina Network
2. **Relayer network** support for Dina deposits
3. **Chain registration** in the Across protocol
4. **UMA Optimistic Oracle** integration for settlement

## Technical Requirements (From Across Docs)
- Chain must have deterministic finality (TurboBFT: 100ms)
- Must deploy SpokePool contract on-chain
- Relayers must be able to read/write to Dina via RPC
- Must support ERC-20 style token interface (DRC-1 is compatible)
- Settlement happens on Ethereum mainnet via UMA

## Integration Architecture
```
Dina Network                    Across                       Target Chain
[User Deposit] -->
  [SpokePool] -->               [Relayer Observes] -->
                                [Fill on Destination] -->    [User Receives]
                                [Submit Proof to UMA] -->
                                [Settlement on L1]
```

## Assets to Bridge
- USDC (primary)
- ETH (wrapped)
- WBTC

## Across Advantage for Dina
- Across uses an intent-based model: users get funds on destination chain fast
- Relayer competition means competitive rates
- UMA optimistic oracle provides trustless settlement
- Well-suited for high-speed chains like Dina (100ms finality means fast confirmations)

## Timeline Estimate
- Application submission: Week 1
- Technical review: Weeks 2-4
- SpokePool deployment: Weeks 4-6
- Relayer integration: Weeks 6-8
- Testnet bridge live: Week 8-10
- Mainnet: TBD
