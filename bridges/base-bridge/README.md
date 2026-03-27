# Base <-> Dina Bridge

Lock USDC on Base, mint bridged-USDC on Dina Network. No third-party approval required.

## Components

| Component | Description |
|-----------|-------------|
| `contracts/DinaBridge.sol` | Solidity contract deployed on Base. Locks USDC on deposit, unlocks on withdrawal. |
| `contracts/MockUSDC.sol` | Test-only mock ERC20 with 6 decimals. |
| `relayer/index.ts` | Node.js service that watches both chains and relays bridge operations. |
| Dina contract (`contracts/bridge-base/`) | Rust contract on Dina that mints/burns bridged-USDC. |

## Quick Start

### 1. Install dependencies

```bash
# Contracts (from bridges/base-bridge/)
npm install

# Relayer (from bridges/base-bridge/relayer/)
cd relayer && npm install
```

### 2. Run tests

```bash
npx hardhat test
```

### 3. Deploy to Base Sepolia

```bash
# Copy and fill in environment variables
cp .env.example .env
# Edit .env with your deployer key, USDC address, relayer address

# Deploy
npx hardhat run scripts/deploy.ts --network baseSepolia

# Verify on Basescan
npx hardhat verify --network baseSepolia <BRIDGE_ADDRESS> <USDC_ADDRESS> <RELAYER_ADDRESS>
```

### 4. Start the relayer

```bash
cd relayer
cp .env.example .env
# Edit .env with bridge addresses and relayer private key

npm run dev
```

## Contract Addresses

### Base Sepolia (Testnet)

| Contract | Address |
|----------|---------|
| USDC     | `0x036CbD53842c5426634e7929541eC2318f3dCF7e` (Circle testnet USDC) |
| DinaBridge | Deploy with `npm run deploy:sepolia` |

### Base Mainnet

| Contract | Address |
|----------|---------|
| USDC     | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` |
| DinaBridge | Deploy with `npm run deploy:mainnet` |

## Bridge Flow

### Base -> Dina (Deposit)

1. User approves USDC spending: `usdc.approve(bridgeAddress, amount)`
2. User calls `bridge.deposit(amount, dinaRecipient)`
3. Contract locks USDC and emits `Deposited` event
4. Relayer detects the event, waits for confirmations
5. Relayer computes SHA-256 proof and calls `claim` on Dina bridge contract
6. Bridged-USDC is minted to the user's Dina address

### Dina -> Base (Withdrawal)

1. User calls `withdraw(amount, baseRecipient, timestamp)` on Dina bridge contract
2. Bridged-USDC is burned, a `PendingWithdrawal` is created
3. Relayer polls Dina for pending withdrawals
4. Relayer signs an EIP-191 proof: `sign(keccak256(recipient, amount, withdrawalId, chainId))`
5. Relayer submits `bridge.withdraw(recipient, amount, withdrawalId, signature)` on Base
6. Contract verifies signature and releases USDC to the recipient
7. Relayer calls `mark_withdrawal_processed` on Dina

## Security

- **Trusted relayer model**: A single relayer signs all withdrawal proofs. In production, upgrade to a multi-sig or threshold signature scheme.
- **Daily limit**: 10M USDC/day rolling cap (configurable by owner).
- **Min/max per tx**: 1 USDC minimum, 1M USDC maximum (configurable).
- **Replay protection**: Each deposit nonce and withdrawal ID is tracked to prevent double-processing.
- **Emergency pause**: Owner can pause the bridge and rescue funds.
- **Signature malleability**: The contract enforces EIP-2 lower-s values.
- **SafeERC20**: All token transfers use OpenZeppelin's SafeERC20 to handle non-standard ERC20 implementations.

## Fees

The Dina-side bridge contract charges a configurable fee in basis points (default 0.1%). The Base-side contract does not charge fees -- fees are only collected on the Dina side during `claim` and `withdraw`.

## Upgrading to Production

1. Replace the single relayer with a multi-sig (e.g., Gnosis Safe) or a decentralised relayer set with threshold BLS signatures.
2. Add a challenge/dispute period for large withdrawals.
3. Implement light-client verification of Base block headers on Dina (or vice versa) to remove trust assumptions.
4. Add monitoring and alerting for the relayer service.
5. Deploy the relayer as a redundant service (e.g., Cloud Run with multiple replicas).
