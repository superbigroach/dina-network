# Dina Network Bridges

## Active Bridges

### Base <-> Dina Direct Bridge (LIVE)

The only bridge that works without third-party approval.
Lock USDC on Base, mint bridged-USDC on Dina Network.

| Component       | Location                          |
|-----------------|-----------------------------------|
| Base contract   | `bridges/base-bridge/contracts/DinaBridge.sol` |
| Dina contract   | `contracts/bridge-base/`          |
| Relayer service | `bridges/base-bridge/relayer/`    |
| Docs            | `dina-developer-portal.web.app/docs/bridges` |

### How to Deploy

1. Deploy `DinaBridge.sol` to Base Sepolia (see `bridges/base-bridge/README.md`)
2. Deploy the `bridge-base` contract to Dina testnet
3. Configure and start the relayer service
4. Bridge USDC from Base to Dina

### Architecture

```
   Base (EVM)                           Dina Network (Rust VM)
  +-----------------+                  +--------------------+
  |  DinaBridge.sol  |  -- relayer -->  |  bridge-base/lib.rs |
  |  (lock/unlock)   |                  |  (mint/burn)        |
  +-----------------+                  +--------------------+
         ^                                      |
         |              Relayer Service          |
         +---------- (watches both chains) ------+

  Base -> Dina: User locks USDC  -> relayer calls claim()  -> bridged-USDC minted
  Dina -> Base: User calls withdraw() -> relayer signs proof -> USDC unlocked on Base
```

## Pending Integrations

See `bridges/third-party/` for application status of:

- Wormhole
- LayerZero
- Axelar
- Across
- Stargate
- Circle CCTP

## DinaDEX Swap

See `bridges/swap/` for the fee-free decentralised exchange.
