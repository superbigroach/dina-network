# Deployed Contracts — Dina Testnet

Chain: `dina-testnet-1`
Last updated: 2026-05-18

## El Tesoro — HNLc (Lempira Digital)

Deployed: 2026-05-18

| Contract | Address | Status | Description |
|----------|---------|--------|-------------|
| HNLc Token (DRC-1) | `0x9c0096dbc2198742d0536d913fc451cc75f865928bc95e26d2381560eca1c980` | LIVE | Lempira Digital — El Tesoro's Honduran lempira stablecoin. symbol=HNLc, decimals=2, mint-on-demand. |
| Base Bridge (Dina side) | `0xe0eebffa10879d77e5925b1471253abbb9aaa49ec3d39c871240974f5e834625` | LIVE | Dina side of the Base Sepolia <-> Dina bridge. Mints/burns HNLc when relayer reports USDC deposits/withdrawals on Base. |
| Base Bridge (Base Sepolia) | `0xd3b5D30EaD83d0Eca3B0Af4f42a2f76F6e4E4A7` | LIVE | `BaseBridge.sol` — locks USDC on Base Sepolia (`lockAndBridge`), releases on withdrawal (`release`). USDC: `0x036CbD53842c5426634e7929541eC2318f3dCF7e`. |

### Treasury Key (HNLc Mint Authority)

| Field | Value |
|-------|-------|
| Treasury public key | `81c0d9d39a2d60047fd3b68972fd02681c4abd864a5cd0f5eb7ab0f8a85ae870` |
| Treasury address (Dina) | `0x372950d6d0448ccf0494efa3724c7a32cc540939cb174ce460290134442e62f0` |
| Private key storage | GCP Secret Manager — `projects/banco-el-tesoro/secrets/dina-treasury-private-key` |

> **Security:** The treasury private key is the HNLc mint authority. It is stored exclusively in GCP Secret Manager and is never committed to this repository.

### HNLc Token Parameters

```
name:           Lempira Digital
symbol:         HNLc
decimals:       2   (centavos — same as physical lempira)
initial_supply: 0   (El Tesoro mints on demand when users fund accounts)
owner:          0x372950d6d0448ccf0494efa3724c7a32cc540939cb174ce460290134442e62f0
```

### Base Bridge Contract Parameters

| Parameter | Value |
|-----------|-------|
| Network | Base Sepolia (chainId 84532) |
| USDC address | `0x036CbD53842c5426634e7929541eC2318f3dCF7e` |
| Relayer | El Tesoro backend service (address in GCP Secret Manager) |
| Contract | `bridges/base-bridge/contracts/BaseBridge.sol` |
| Deploy script | `bridges/base-bridge/scripts/deploy-base-bridge.ts` |

---

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

| Contract | Address / TX Hash | Status | Description |
|----------|---------|--------|-------------|
| Bridged USDC (USDC.e) | `0x4ebb4866...28f7db` | LIVE | Token representing USDC bridged from Base. Only the bridge contract can mint. 1 USDC.e = 1 real USDC locked on Base. |
| Base Bridge (Dina side) — HNLc | `0xe0eebffa10879d77e5925b1471253abbb9aaa49ec3d39c871240974f5e834625` | LIVE | Dina-side bridge for El Tesoro. See El Tesoro section above. |
| Base Bridge (Base Sepolia) — HNLc | `0xd3b5D30EaD83d0Eca3B0Af4f42a2f76F6e4E4A7` | LIVE | Solidity bridge contract. See El Tesoro section above. |
| Relayer Service | *not yet running* | PENDING | Node.js service at `bridges/base-bridge/relayer/`. Watches both chains and relays proofs. |

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
LIVE on testnet:            18 contracts  (+3 El Tesoro: HNLc, Dina bridge, Base bridge)
PENDING (need deployment):  1 (relayer)
NOT ACTIVE (need approval): 6 (CCTP, Wormhole, LayerZero, Axelar, Across, Stargate)
```

## How to Interact

```typescript
import { DinaClient, DinaWallet, TokenContract } from 'dina-js';

const HNLC_ADDRESS = '0x9c0096dbc2198742d0536d913fc451cc75f865928bc95e26d2381560eca1c980';

// Read HNLc balance
const client = new DinaClient('http://35.184.213.248:8080');
const token = TokenContract.token(HNLC_ADDRESS, client);
const balance = await token.balanceOf('0x<your-address>');
console.log('HNLc balance (centavos):', balance.toString());

// Mint (treasury only)
const treasuryWallet = DinaWallet.fromPrivateKey(process.env.TREASURY_PRIVATE_KEY!);
await token.call('mint', { to: '0x<recipient>', amount: 100 }, treasuryWallet);
```

## RPC Endpoints

```
REST:   http://35.184.213.248:8080
Faucet: POST http://35.184.213.248:8080/faucet  { "address": "0x..." }
Portal: https://dina-developer-portal.web.app
```
