# Deployed Contracts — Dina Testnet

Chain: `dina-testnet-1`
Deployed: 2026-03-27

## Core Token Standards

| Contract | TX Hash | Status | Description |
|----------|---------|--------|-------------|
| DRC-1 Token | `0x8864c4f1...e45acc` | LIVE | Fungible token (ERC-20 equivalent) |
| DRC-6 NFT | `0x041194f6...02c4f3` | LIVE | Non-fungible token (ERC-721 equivalent) |
| DRC-7 Multi-Token | `0xade3f96b...f27b58` | LIVE | Multi-token (ERC-1155 equivalent) |

## DeFi

| Contract | TX Hash | Status | Description |
|----------|---------|--------|-------------|
| DinaDEX Swap | `0x27b54254...f8356d` | LIVE | AMM DEX, 0% fees, 100ms swaps |
| Yield Vault | `0x979f3ed5...371376` | LIVE | ERC-4626 yield vault |
| Lending Pool | `0xc51f318f...20516e` | LIVE | Aave-style lending with interest rates |

## Wallet Infrastructure

| Contract | TX Hash | Status | Description |
|----------|---------|--------|-------------|
| DRC-63 Parallel Wallet | `0x4e5d6ca2...e2f8c6` | LIVE | Auto-scaling parallel transaction wallets |

## Bridge Infrastructure

### Working Now (no third-party approval needed)

| Contract | TX Hash | Status | Description |
|----------|---------|--------|-------------|
| Bridged USDC (USDC.e) | `0x4ebb4866...28f7db` | LIVE | Token representing USDC bridged from Base. Only the bridge contract can mint. 1 USDC.e = 1 real USDC locked on Base. |
| Base Bridge (Dina side) | `0x20ca0668...5ea0fe` | LIVE | Dina side of the Base ↔ Dina bridge. Mints/burns Bridged USDC when relayer reports deposits/withdrawals. |
| Base Bridge (Base side) | *not yet deployed* | PENDING | Solidity contract at `bridges/base-bridge/contracts/DinaBridge.sol`. Needs Base Sepolia ETH to deploy. |
| Relayer Service | *not yet running* | PENDING | Node.js service at `bridges/base-bridge/relayer/`. Watches both chains and relays proofs. |

**To complete the Base bridge:** Deploy DinaBridge.sol to Base Sepolia + start relayer. See `bridges/base-bridge/README.md`.

### Future Integrations (require third-party approval)

| Contract | TX Hash | Status | What's Needed |
|----------|---------|--------|---------------|
| CCTP Bridge | `0xaba71d31...f265e6` | NOT ACTIVE | Dina-side contract is deployed but Circle hasn't approved Dina. Their attestation service doesn't know about us. Becomes active when Circle integrates Dina (6-12 months). |
| Wormhole | *contract ready* | NOT ACTIVE | Apply at wormhole.com. Need Guardian support for Dina chain. See `bridges/third-party/wormhole-application.md`. |
| LayerZero | *contract ready* | NOT ACTIVE | Apply at layerzero.network. Need endpoint deployment. See `bridges/third-party/layerzero-application.md`. |
| Axelar | *contract ready* | NOT ACTIVE | Apply at axelar.network. Need gateway deployment. See `bridges/third-party/axelar-application.md`. |
| Across | *contract ready* | NOT ACTIVE | Apply at across.to. Need spoke pool listing. See `bridges/third-party/across-application.md`. |
| Stargate | *contract ready* | NOT ACTIVE | Requires LayerZero first. See `bridges/third-party/stargate-application.md`. |

## Developer Infrastructure

| Contract | TX Hash | Status | Description |
|----------|---------|--------|-------------|
| Upgradeable Proxy | `0xb251eeb3...3cc88e` | LIVE | Deploy once, upgrade code anytime (timelock protected) |
| Multicall | `0xa0fa1c1d...aad157` | LIVE | Batch multiple contract calls in 1 transaction |
| Timelock | `0xd3fbd4a7...11ade5` | LIVE | Governance delay on critical operations |
| Contract Factory | `0x8fea9c1c...1d7180` | LIVE | Deploy contracts from registered templates |
| Event Indexer | `0x7ad0df7d...84d3a2` | LIVE | On-chain event log for querying |

## Summary

```
LIVE on testnet:           15 contracts
PENDING (need deployment): 2 (Base bridge Solidity + relayer)
NOT ACTIVE (need approval): 6 (CCTP, Wormhole, LayerZero, Axelar, Across, Stargate)
```

## How to Interact

```typescript
import { DinaClient, DinaWallet } from 'dina-js';

// 1. Create wallet
const wallet = DinaWallet.generate();

// 2. Get testnet USDC
await fetch('http://35.184.213.248:8080/faucet/' + wallet.address, { method: 'POST' });

// 3. Call any contract
const client = new DinaClient('http://35.184.213.248:8545');
await client.callContract(wallet, {
  contract: '0x...contract_address...',
  method: 'method_name',
  args: { ... },
});
```

## RPC Endpoints

```
REST:   http://35.184.213.248:8080
RPC:    http://35.184.213.248:8545
Faucet: POST http://35.184.213.248:8080/faucet/{address}
Portal: https://dina-developer-portal.web.app
```
