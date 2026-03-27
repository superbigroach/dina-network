# Dina Network Bridges

## Bridge Status

| Bridge | Status | Speed | What's Needed |
|--------|--------|-------|---------------|
| **Base ↔ Dina** | **READY TO DEPLOY** | ~5 min | Deploy Solidity to Base Sepolia + start relayer |
| Circle CCTP | NOT ACTIVE | ~15 min | Circle must approve Dina (6-12 months) |
| Wormhole | NOT ACTIVE | ~15 min | Apply at wormhole.com |
| LayerZero | NOT ACTIVE | ~3-10 min | Apply at layerzero.network |
| Axelar | NOT ACTIVE | ~15 min | Apply at axelar.network |
| Across | NOT ACTIVE | ~1-3 min | Apply at across.to |
| Stargate | NOT ACTIVE | ~1-3 min | Requires LayerZero first |

**Only the Base bridge works without third-party approval.** All other bridges have Dina-side contracts ready, but the third-party protocols need to approve and deploy their side.

## Base ↔ Dina Direct Bridge

The only bridge that works right now. No permission needed — we control both sides.

```
   Base (EVM)                           Dina Network (WASM)
  +-----------------+                  +--------------------+
  |  DinaBridge.sol  |  -- relayer -->  |  bridge-base       |
  |  (lock/unlock)   |                 |  (mint/burn)        |
  +-----------------+                  +--------------------+
         ^                                      |
         |              Relayer Service          |
         +---------- (watches both chains) ------+

  Base → Dina: User locks USDC → relayer → bridged-USDC minted on Dina
  Dina → Base: User burns bridged-USDC → relayer → real USDC unlocked on Base
```

| Component | Location | Status |
|-----------|----------|--------|
| Base Solidity contract | `bridges/base-bridge/contracts/DinaBridge.sol` | Ready to deploy |
| Dina WASM contract | `contracts/bridge-base/` | Deployed to testnet |
| Bridged USDC token | `contracts/bridge-usdc/` | Deployed to testnet |
| Relayer service | `bridges/base-bridge/relayer/` | Ready to run |
| Deploy script | `bridges/base-bridge/scripts/deploy.ts` | Ready |

### Deploy Steps

1. Get Base Sepolia ETH: https://www.alchemy.com/faucets/base-sepolia
2. Configure `.env` with deployer private key
3. Deploy: `npx hardhat run scripts/deploy.ts --network base-sepolia`
4. Start relayer: `cd relayer && npm start`
5. Bridge USDC from Base to Dina

See `bridges/base-bridge/README.md` for detailed instructions.

## Future Integrations

All Dina-side contracts are built and deployed. Application guides with requirements and steps are in `bridges/third-party/`:

- `wormhole-application.md` — Guardian support for Dina
- `layerzero-application.md` — Endpoint deployment
- `axelar-application.md` — Gateway deployment
- `across-application.md` — Spoke pool listing
- `stargate-application.md` — Pool listing (requires LayerZero)
- `circle-cctp-application.md` — Native USDC (requires legal entity + audit)

## DinaDEX Swap

Fee-free decentralized exchange. See `bridges/swap/README.md`.
Once tokens are bridged to Dina, anyone can create trading pools and swap.
