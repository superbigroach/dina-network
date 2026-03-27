# Dina Network API Reference

## Overview

Dina Network exposes three API interfaces:

1. **JSON-RPC 2.0** on port 8545 (default) -- primary programmatic interface
2. **REST API** on port 8080 (default) -- HTTP-friendly queries and submissions
3. **WebSocket** -- real-time subscriptions for blocks, transactions, and consensus

Additionally, the **MCP (Model Context Protocol)** server exposes 12 tools for Cognitum Seed devices.

## JSON-RPC Methods

All JSON-RPC methods use the `dina_` namespace prefix. The server is built on jsonrpsee and conforms to JSON-RPC 2.0.

Default endpoint: `http://127.0.0.1:8545`

### dina_sendTransaction

Submit a signed transaction to the mempool.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `tx_hex` | string | Yes | Hex-encoded signed transaction (with optional `0x` prefix) |

**Returns:** `string` -- the transaction hash (hex-encoded SHA-256)

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "dina_sendTransaction",
  "params": ["0x7b22547261..."],
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "a1b2c3d4e5f6...64-char-hex-hash",
  "id": 1
}
```

---

### dina_getBalance

Get the USDC balance of an address in micro-units.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `address` | string | Yes | 64-character hex address (with optional `0x` prefix) |

**Returns:** `u64` -- balance in micro-USDC (1 USDC = 1,000,000)

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "dina_getBalance",
  "params": ["0xabab...abab"],
  "id": 2
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": 1000000000,
  "id": 2
}
```

---

### dina_getAccount

Get full account information including balance, nonce, and contract status.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `address` | string | Yes | 64-character hex address |

**Returns:**
```json
{
  "address": "0xabab...abab",
  "balance": 1000000000,
  "nonce": 42,
  "has_code": false
}
```

**Errors:**
- `-32001`: Account not found

---

### dina_getBlock

Get a block by its height (block number).

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `height` | u64 | Yes | Block height (0 = genesis) |

**Returns:**
```json
{
  "hash": "deadbeef...64hex",
  "block_number": 100,
  "parent_hash": "cafebabe...64hex",
  "state_root": "11223344...64hex",
  "transactions_root": "aabbccdd...64hex",
  "timestamp": 1700000000,
  "proposer": "validator-address-hex",
  "transaction_count": 5,
  "transactions": ["tx-hash-1", "tx-hash-2", "..."]
}
```

**Errors:**
- `-32001`: Block not found

---

### dina_getBlockByHash

Get a block by its hash.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `hash` | string | Yes | 64-character hex block hash |

**Returns:** Same as `dina_getBlock`

**Errors:**
- `-32602`: Invalid hash format
- `-32001`: Block not found

---

### dina_getLatestBlock

Get the most recent committed block.

**Parameters:** None

**Returns:** Same as `dina_getBlock`

**Errors:**
- `-32603`: No blocks in chain (should not occur after genesis)

---

### dina_getTransaction

Get a transaction by its hash.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `hash` | string | Yes | 64-character hex transaction hash |

**Returns:**
```json
{
  "hash": "tx-hash-hex",
  "sender": "sender-address-hex",
  "nonce": 5,
  "fee": 1000,
  "tx_type": "Transfer",
  "block_number": 42
}
```

`tx_type` is one of: `Transfer`, `DeployContract`, `CallContract`, `RegisterDevice`

`block_number` is `null` if the transaction is still pending in the mempool.

**Errors:**
- `-32001`: Transaction not found

---

### dina_getDevice

Get a registered device by its public key.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pubkey` | string | Yes | Hex-encoded Ed25519 public key (32 bytes) |

**Returns:**
```json
{
  "address": "device-address-hex",
  "name": "Cognitum Seed v1",
  "device_type": "CognitumSeed",
  "owner": "owner-address-hex",
  "active": true,
  "registered_at": 1700000000
}
```

**Errors:**
- `-32001`: Device not found

---

### dina_networkInfo

Get current network status.

**Parameters:** None

**Returns:**
```json
{
  "chain_id": "dina-testnet-1",
  "block_height": 12345,
  "peer_count": 5,
  "version": "0.1.0",
  "protocol_version": 1
}
```

---

### dina_chainId

Get the chain identifier string.

**Parameters:** None

**Returns:** `string` -- e.g., `"dina-testnet-1"`

---

### dina_estimateGas

Estimate the gas cost for a transaction before submitting it.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `tx_type` | string | Yes | One of: `transfer`, `contract_call`, `deploy`, `device_registration`, `batch` |
| `params` | object | Yes | Type-specific parameters (see below) |

**Type-specific params:**

For `transfer`:
```json
{ "amount": 1000000, "memo_size": 64 }
```

For `contract_call`:
```json
{ "method": "transfer", "args_size": 128 }
```

For `deploy`:
```json
{ "wasm_size": 50000 }
```

For `device_registration`:
```json
{}
```

For `batch`:
```json
{ "tx_count": 10 }
```

**Returns:**
```json
{
  "gas_estimate": 5000,
  "fee_estimate": 5000,
  "breakdown": { ... }
}
```

**Errors:**
- `-32602`: Unknown `tx_type`

---

### dina_gasPrice

Get current gas price information.

**Parameters:** None

**Returns:**
```json
{
  "base_fee": 1,
  "priority_fee": 0,
  "gas_per_usdc": 1000000
}
```

---

### dina_txPoolStatus

Get the transaction pool (mempool) status.

**Parameters:** None

**Returns:**
```json
{
  "pending": 42,
  "queued": 0,
  "total_value": 5000000
}
```

---

### dina_pendingTransactions

Get pending transactions from the mempool.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `limit` | usize | Yes | Maximum number of transactions to return (capped at 1000) |

**Returns:** Array of `TransactionInfo` objects (same schema as `dina_getTransaction`, but with `block_number: null`).

---

## REST API Endpoints

Default base URL: `http://0.0.0.0:8080`

### GET /health

Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "height": 12345,
  "peers": 5
}
```

---

### GET /v1/balance/{address}

Get the USDC balance of an address.

**Path parameters:**
- `address` -- 64-character hex address

**Response (200):**
```json
{
  "address": "abab...abab",
  "balance": 1000000000
}
```

**Response (400):**
```json
{
  "error": "invalid address: ..."
}
```

---

### GET /v1/block/latest

Get the latest committed block.

**Response (200):** BlockInfo JSON object

**Response (500):**
```json
{
  "error": "no blocks in chain"
}
```

---

### GET /v1/block/{height}

Get a block by its height.

**Path parameters:**
- `height` -- Block number (u64)

**Response (200):** BlockInfo JSON object

**Response (404):**
```json
{
  "error": "block 999 not found"
}
```

---

### POST /v1/transaction

Submit a signed transaction.

**Request body:**
```json
{
  "tx_hex": "0x7b22547261..."
}
```

**Response (200):**
```json
{
  "tx_hash": "a1b2c3d4..."
}
```

**Response (400):**
```json
{
  "error": "invalid hex: ..."
}
```

---

### GET /v1/device/{pubkey}

Get a registered device by its public key.

**Path parameters:**
- `pubkey` -- Hex-encoded Ed25519 public key

**Response (200):** DeviceInfo JSON object

**Response (404):**
```json
{
  "error": "device not found"
}
```

---

### GET /v1/peers

Get connected peer information.

**Response:**
```json
{
  "peer_count": 5,
  "peers": []
}
```

---

## WebSocket Subscriptions

The WebSocket server supports real-time event subscriptions on three topics.

### Subscription Topics

| Topic | Description | Payload |
|-------|-------------|---------|
| `NewBlocks` | Emitted when a new block is committed | BlockInfo JSON |
| `NewTransactions` | Emitted when a new transaction enters the mempool | TransactionInfo JSON |
| `ConsensusUpdates` | Emitted on consensus state changes | Consensus state JSON |

### Connection

Connect to `ws://127.0.0.1:8545/ws` (shares port with JSON-RPC).

### Event Format

```json
{
  "topic": "NewBlocks",
  "payload": "{ ... serialized BlockInfo ... }"
}
```

### Buffer

Each topic has a broadcast channel buffer of 256 events. Slow consumers may miss events if they fall behind.

---

## MCP Tools

The Dina MCP server exposes 12 tools in the `dina/` namespace for Cognitum Seed devices. All tools accept JSON input and return `McpToolResult`.

### Tool Result Format

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

On failure:
```json
{
  "success": false,
  "data": {},
  "error": "description of what went wrong"
}
```

---

### dina/transfer

Send USDC to an address.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `to` | string | Yes | Recipient address (0x-prefixed hex, 32 bytes) |
| `amount` | integer | Yes | Amount in micro-USDC (minimum: 1) |
| `memo` | string | No | Hex-encoded memo bytes |
| `fee` | integer | No | Transaction fee in micro-USDC |

---

### dina/balance

Check the USDC balance of an address.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `address` | string | No | Address to query (omit for device's own address) |

---

### dina/deploy_contract

Deploy a WASM smart contract.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `wasm_bytecode` | string | Yes | Hex-encoded WASM bytecode |
| `init_args` | string | No | Hex-encoded initialization arguments |
| `fee` | integer | No | Transaction fee in micro-USDC |

---

### dina/call_contract

Call a method on a deployed smart contract.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `contract` | string | Yes | Contract address |
| `method` | string | Yes | Method name to call |
| `args` | string | No | Hex-encoded method arguments |
| `usdc_attached` | integer | No | Micro-USDC to attach to the call |
| `fee` | integer | No | Transaction fee in micro-USDC |

---

### dina/register_device

Register a Cognitum device on-chain.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `device_pubkey` | string | Yes | Device Ed25519 public key (hex, 32 bytes) |
| `owner` | string | Yes | Owner address |
| `firmware_hash` | string | Yes | SHA-256 of device firmware (hex, 32 bytes) |
| `witness_root` | string | No | Merkle root of witness history |
| `attestation_signature` | string | Yes | Ed25519 attestation signature (hex, 64 bytes) |
| `fee` | integer | No | Transaction fee |

---

### dina/verify_device

Verify a device's attestation against its on-chain identity.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `device_id` | string | Yes | Device ID (hex, 32 bytes) |
| `attestation_pubkey` | string | No | Public key for verification |
| `firmware_hash` | string | No | Expected firmware hash |
| `witness_root` | string | No | Expected witness root |
| `attestation_signature` | string | No | Attestation signature to verify |

---

### dina/channel_open

Open a bidirectional payment channel.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `counterparty` | string | Yes | Counterparty address |
| `deposit` | integer | Yes | Micro-USDC to lock in the channel (minimum: 1) |
| `fee` | integer | No | Transaction fee |

---

### dina/channel_pay

Make an off-chain payment through an open channel.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `channel_id` | string | Yes | Channel identifier (hex, 32 bytes) |
| `amount` | integer | Yes | Micro-USDC to pay (minimum: 1) |

---

### dina/channel_close

Close a payment channel and settle on-chain.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `channel_id` | string | Yes | Channel identifier (hex, 32 bytes) |
| `fee` | integer | No | Settlement transaction fee |

---

### dina/peers

List discovered peers on the network.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `limit` | integer | No | Max peers to return (1-1000) |

---

### dina/block_info

Get information about a specific block.

**Input:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `block_number` | integer | No | Block height (omit for latest) |
| `block_hash` | string | No | Block hash (overrides block_number) |

---

### dina/network_status

Get current network status.

**Input:** No parameters required.

---

## Error Codes

### JSON-RPC Standard Errors

| Code | Name | Description |
|------|------|-------------|
| `-32700` | Parse error | Invalid JSON was received |
| `-32600` | Invalid Request | JSON is not a valid request object |
| `-32601` | Method not found | Method does not exist |
| `-32602` | Invalid params | Invalid method parameters |
| `-32603` | Internal error | Internal server error |

### Dina-Specific Errors

| Code | Name | Description |
|------|------|-------------|
| `-32001` | Not found | Requested resource (account, block, tx, device) not found |
| `-32002` | Insufficient balance | Sender does not have enough USDC |
| `-32003` | Invalid signature | Transaction signature verification failed |
| `-32004` | Invalid nonce | Transaction nonce does not match account nonce |
| `-32005` | Contract error | WASM contract execution failed |
| `-32006` | Out of gas | Transaction ran out of gas during execution |
| `-32007` | Mempool full | Transaction pool is at capacity |

---

## Rate Limiting

The RPC server includes a rate limiting middleware. Current limits for testnet:

| Endpoint Type | Limit |
|--------------|-------|
| Read queries | No limit (testnet) |
| Transaction submissions | 100/minute per IP |
| WebSocket subscriptions | 10 connections per IP |

Mainnet will implement stricter rate limits with API key authentication.

---

## Authentication

### Testnet

No authentication is required. All endpoints are open.

### Mainnet (Planned)

- API keys for high-volume access
- Ed25519 signature-based authentication for MCP tools
- Rate limiting tied to staked USDC amount
