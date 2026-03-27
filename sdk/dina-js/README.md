# dina-js

TypeScript/JavaScript SDK for [Dina Network](https://github.com/superbigroach/dina-network) — the fastest blockchain with swarm wallets, parallel execution, and 82 DRC smart contract standards.

## Install

```bash
npm install dina-js
```

## Quick Start

```typescript
import { DinaWallet, DinaClient } from 'dina-js';

// Create a wallet
const wallet = DinaWallet.generate();
console.log('Address:', wallet.address);

// Connect to testnet
const client = new DinaClient('http://35.184.213.248:8545');

// Check balance
const balance = await client.getBalance(wallet.address);

// Send USDC
const txHash = await client.transfer(wallet, {
  to: '0x...',
  amount: 1_000_000n, // 1 USDC (6 decimals)
});

// Wait for confirmation (100ms finality)
const receipt = await client.waitForTransaction(txHash);
```

## Features

- **Wallet**: Generate, import from private key or mnemonic, sign/verify
- **Client**: Full JSON-RPC client with WebSocket subscriptions
- **Contracts**: Deploy and interact with WASM smart contracts
- **Token (DRC-1)**: ERC-20 equivalent — balanceOf, transfer, approve
- **Agent Wallet (DRC-101)**: AI agent wallets with spending limits
- **Payment Channels**: Off-chain micro-payments with 5ms latency

## API

### DinaWallet

```typescript
DinaWallet.generate()                    // New random wallet
DinaWallet.fromPrivateKey(key)           // From 32-byte key or hex string
DinaWallet.fromMnemonic('word1 word2..') // From BIP-39 mnemonic
wallet.address                           // 0x... address
wallet.publicKey                         // Uint8Array
wallet.sign(message)                     // Ed25519 signature
wallet.verify(message, signature)        // Verify signature
wallet.exportPrivateKey()                // Hex string (handle with care)
```

### DinaClient

```typescript
const client = new DinaClient('http://rpc-url:8545');

// Queries
await client.getBalance(address)         // bigint (micro-USDC)
await client.getAccount(address)         // { address, balance, nonce }
await client.getBlock(height)            // Block data
await client.getLatestBlock()            // Latest block
await client.getTransaction(hash)        // Transaction receipt
await client.getNetworkInfo()            // Chain info

// Transactions
await client.transfer(wallet, { to, amount, memo? })
await client.deployContract(wallet, { wasmBytes, initArgs })
await client.callContract(wallet, { contract, method, args, usdcAttached? })
await client.waitForTransaction(hash, timeout?)

// WebSocket subscriptions
const unsub = client.onNewBlock((block) => { ... })
const unsub = client.onTransaction(address, (tx) => { ... })
client.disconnect()
```

### Contracts

```typescript
import { DinaContract, TokenContract, AgentWalletContract } from 'dina-js';

// DRC-1 Token
const token = DinaContract.token(tokenAddress, client);
await token.balanceOf(address)
await token.transfer(wallet, to, amount)
await token.approve(wallet, spender, amount)

// DRC-101 Agent Wallet
const agent = DinaContract.agentWallet(agentAddress, client);
await agent.executeTransfer(wallet, to, amount)
await agent.spendingStats()
await agent.emergencyStop(wallet)
```

### Payment Channels

```typescript
import { PaymentChannel } from 'dina-js';

const channel = new PaymentChannel(wallet, client);
const channelId = await channel.open(counterparty, 100_000_000n); // 100 USDC
const signedState = await channel.pay(channelId, 1_000_000n);     // 1 USDC (instant, off-chain)
await channel.close(channelId);                                    // Settle on-chain
```

## Works With

- Node.js 18+
- Next.js / React
- React Native
- Deno / Bun
- Any JavaScript runtime with `fetch` and `crypto`

## Testnet

```
RPC: http://35.184.213.248:8545
REST: http://35.184.213.248:8080
Chain ID: dina-testnet-1
```

## License

MIT
