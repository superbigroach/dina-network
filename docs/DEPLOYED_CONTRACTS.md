# Deployed Contracts — Dina Testnet

Chain: `dina-testnet-1`
Deployed: 2026-03-27

## Core Token Standards

| Contract | TX Hash | Description |
|----------|---------|-------------|
| DRC-1 Token | `0x8864c4f1...e45acc` | Fungible token (ERC-20 equivalent) |
| DRC-6 NFT | `0x041194f6...02c4f3` | Non-fungible token (ERC-721 equivalent) |
| DRC-7 Multi-Token | `0xade3f96b...f27b58` | Multi-token (ERC-1155 equivalent) |

## DeFi

| Contract | TX Hash | Description |
|----------|---------|-------------|
| DinaDEX Swap | `0x27b54254...f8356d` | AMM DEX (0% fees, Uniswap V2 style) |
| Yield Vault | `0x979f3ed5...371376` | ERC-4626 yield vault |
| Lending Pool | `0xc51f318f...20516e` | Aave-style lending with interest rates |

## Wallet Infrastructure

| Contract | TX Hash | Description |
|----------|---------|-------------|
| DRC-63 Parallel Wallet | `0x4e5d6ca2...e2f8c6` | Auto-scaling parallel transaction wallets |

## Bridge Infrastructure

| Contract | TX Hash | Description |
|----------|---------|-------------|
| Bridged USDC | `0x4ebb4866...28f7db` | Circle Bridged USDC Standard token |
| Base Bridge | `0x20ca0668...5ea0fe` | Base ↔ Dina lock/mint bridge |
| CCTP Bridge | `0xaba71d31...f265e6` | Circle CCTP MessageTransmitter |

## Developer Infrastructure

| Contract | TX Hash | Description |
|----------|---------|-------------|
| Upgradeable Proxy | `0xb251eeb3...3cc88e` | Deploy once, upgrade code anytime (timelock protected) |
| Multicall | `0xa0fa1c1d...aad157` | Batch multiple contract calls in 1 transaction |
| Timelock | `0xd3fbd4a7...11ade5` | Governance delay on critical operations |
| Contract Factory | `0x8fea9c1c...1d7180` | Deploy contracts from registered templates |
| Event Indexer | `0x7ad0df7d...84d3a2` | On-chain event log for querying |

## How to Interact

```typescript
import { DinaClient, DinaWallet, DinaContract } from 'dina-js';

const client = new DinaClient('http://35.184.213.248:8545');
const wallet = DinaWallet.generate();

// Call any deployed contract
await client.callContract(wallet, {
  contract: '0x...contract_address...',
  method: 'method_name',
  args: { ... },
});
```

## RPC Endpoints

```
REST: http://35.184.213.248:8080
RPC:  http://35.184.213.248:8545
```
