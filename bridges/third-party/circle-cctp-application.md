# Circle CCTP (Cross-Chain Transfer Protocol) Application

## Application URL
- Partnerships: https://www.circle.com/partnerships
- CCTP Docs: https://developers.circle.com/stablecoins/cctp-getting-started
- Developer Portal: https://developers.circle.com

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

## What We Need From Circle
1. **Native USDC** deployment on Dina Network (not bridged/wrapped)
2. **CCTP MessageTransmitter** contract deployment
3. **CCTP TokenMessenger** contract deployment
4. **Attestation service** support for Dina chain
5. **Domain registration** in CCTP protocol

## Prerequisites (Circle Requirements)
This is the most demanding integration. Circle requires:

1. **Legal entity** — Incorporated company with clear jurisdiction
2. **Security audit** — Smart contracts audited by a reputable firm (Trail of Bits, OpenZeppelin, etc.)
3. **6+ months mainnet operation** — Proven chain stability and uptime
4. **Compliance review** — KYC/AML framework, sanctions screening capability
5. **Insurance/reserves** — May require collateral or insurance coverage
6. **Technical standards** — Must meet Circle's technical integration requirements

## Technical Requirements (From CCTP Docs)
- Chain must have deterministic finality (TurboBFT: 100ms)
- Must deploy MessageTransmitter contract for cross-chain attestations
- Must deploy TokenMessenger contract for burn-and-mint USDC flow
- Circle attestation service needs RPC access to verify burn events
- Must implement CCTP V2 signature scheme (7-parameter format)
- Must support domain-based message routing

## Integration Architecture
```
Dina Network                    Circle Attestation           Target Chain
[User burns USDC] -->
  [TokenMessenger.depositForBurn] -->
    [MessageTransmitter.sendMessage] -->
                                [Attestation Service] -->
                                [Sign attestation] -->       [receiveMessage] -->
                                                               [Mint native USDC]
```

## CCTP vs Wrapped USDC
- **CCTP (native USDC)**: Burn on source, mint on destination. No liquidity pools needed. 1:1 always.
- **Wrapped USDC**: Lock on source, mint wrapped on destination. Requires liquidity. Can depeg.
- **We want**: Native USDC via CCTP (the gold standard)

## Why Circle Should Support Dina
- 100ms finality means near-instant USDC transfers
- WASM VM is secure and auditable
- Lucilla app already uses USDC on Base via Circle Modular Wallets
- Existing relationship with Circle (see Lucilla project)
- DinaDEX provides 0% fee swaps — great UX for USDC holders

## Timeline Estimate (Longer Due to Requirements)
- Initial outreach: Month 1
- Legal/compliance review: Months 2-4
- Security audit completion: Months 3-5
- Technical integration: Months 5-7
- Testnet CCTP: Month 7-8
- Mainnet native USDC: Month 8-10 (after 6 months mainnet)
- Full timeline: 8-12 months realistically
