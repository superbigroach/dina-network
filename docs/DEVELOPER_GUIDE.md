# Dina Network — Developer Guide

## Testnet Endpoints

Dina runs regional validators with HTTPS proxies. Your app auto-selects the
fastest endpoint based on user location.

### HTTPS Proxies (for browsers / frontend apps)

| Region | URL | Validator |
|--------|-----|-----------|
| 🇨🇦 Montreal | `https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app` | northamerica-northeast1 |
| 🇺🇸 Iowa | `https://dina-testnet-proxy-jy6qm6s57a-uc.a.run.app` | us-central1 |
| 🇬🇧 London | `https://dina-proxy-eu-jy6qm6s57a-nw.a.run.app` | europe-west2 |

### Direct Validator Access (for backends / servers / bots)

| Region | REST API | RPC | P2P |
|--------|----------|-----|-----|
| 🇨🇦 Montreal | `http://34.118.177.132:8080` | `http://34.118.177.132:8545` | `34.118.177.132:9944` |
| 🇺🇸 Iowa | `http://35.184.213.248:8080` | `http://35.184.213.248:8545` | `35.184.213.248:9944` |
| 🇬🇧 London | `http://35.246.48.82:8080` | `http://35.246.48.82:8545` | `35.246.48.82:9944` |

**When to use which:**
- **Browser apps** -- use HTTPS proxies (browsers block HTTP from HTTPS pages)
- **Backend servers** -- use direct validator HTTP (faster, no proxy overhead)
- **Co-located servers** -- same GCP region as validator = ~5ms + 100ms finality = ~105ms total

### Chain Info

| Parameter | Value |
|-----------|-------|
| Chain ID | `dina-testnet-1` |
| Block time | 100ms |
| Hard finality | 1 block (100ms) |
| Transaction fees | $0.00 (zero) |
| Currency | USDC (6 decimals, 1 USDC = 1,000,000 micro-USDC) |
| Signing | Ed25519 |
| Address format | 64-char hex (SHA-256 of Ed25519 public key) |

---

## REST API Reference

### Health Check

```
GET /health
```

Response:
```json
{"status": "ok", "height": 12345, "peers": 0}
```

### Get Balance

```
GET /v1/balance/{address}
```

Response:
```json
{"address": "8e5aad01e9...", "balance": 10000000000}
```

Balance is in micro-USDC. Divide by 1,000,000 to get USDC.

### Submit Transaction

```
POST /v1/transaction
Content-Type: application/json

{"tx_hex": "<hex-encoded JSON of signed Transaction>"}
```

Response:
```json
{"tx_hash": "0x9659fc7ade82d3e2d883..."}
```

### Get Transaction History

```
GET /v1/transactions/{address}
```

Response:
```json
{
  "transactions": [
    {
      "tx_hash": "0x9659fc7a...",
      "type": "send",
      "from": "0x8e5aad01...",
      "to": "0xbe84130e...",
      "amount": 10000000000,
      "fee": 0,
      "nonce": 1774761321505,
      "block_height": 177316,
      "status": "confirmed"
    }
  ]
}
```

### Get Block

```
GET /v1/block/latest
GET /v1/block/{height}
```

### Faucet (testnet only)

```
POST /faucet/{address}
```

Dispenses 10,000 USDC to the given address. 30-second cooldown per address.

---

## Quick Start Examples

### JavaScript -- Send USDC (browser)

```javascript
// Race regional endpoints to find fastest
const ENDPOINTS = [
  'https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app',
  'https://dina-testnet-proxy-jy6qm6s57a-uc.a.run.app',
  'https://dina-proxy-eu-jy6qm6s57a-nw.a.run.app',
];

async function findFastest() {
  return await Promise.any(
    ENDPOINTS.map(async (ep) => {
      await fetch(`${ep}/health`);
      return ep;
    })
  );
}

// Check balance
const BASE = await findFastest();
const res = await fetch(`${BASE}/v1/balance/${address}`);
const { balance } = await res.json();
console.log(`Balance: ${balance / 1_000_000} USDC`);

// Fund from faucet (testnet)
await fetch(`${BASE}/faucet/${address}`, { method: 'POST' });
```

### JavaScript -- Sign and Send Transaction

```javascript
import * as ed from '@noble/ed25519';
import { sha256, sha512 } from '@noble/hashes/sha2.js';

// Set up Ed25519 (required for @noble/ed25519 v3)
ed.hashes.sha512 = (...msgs) => {
  const h = sha512.create();
  for (const m of msgs) h.update(m);
  return h.digest();
};

// Generate keypair
const privateKey = ed.utils.randomSecretKey();
const publicKey = ed.getPublicKey(privateKey);
const address = toHex(sha256(publicKey));

// Build Transaction::Transfer JSON (matches Rust serde format)
const tx = {
  Transfer: {
    from: Array.from(hexToBytes(fromAddress)),
    to: Array.from(hexToBytes(toAddress)),
    amount: 5000000,     // 5 USDC
    memo: null,
    device_witness: null,
    nonce: Date.now(),
    fee: 0,              // zero fees
    pub_key: Array.from(publicKey),
    signature: Array.from(ed.sign(signingPayload, privateKey)),
  }
};

// Hex-encode the JSON and submit
const txHex = toHex(new TextEncoder().encode(JSON.stringify(tx)));
const result = await fetch(`${BASE}/v1/transaction`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ tx_hex: txHex }),
});
const { tx_hash } = await result.json();
console.log('Confirmed:', tx_hash); // ~100-350ms
```

### Python -- Check Balance

```python
import requests

BASE = "https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app"
address = "8e5aad01e92c7aaa73c035a9db569488860110e5e3a2734730f705d421d330d4"

resp = requests.get(f"{BASE}/v1/balance/{address}")
balance = resp.json()["balance"]
print(f"Balance: ${balance / 1_000_000:.2f} USDC")

# Fund from faucet
requests.post(f"{BASE}/faucet/{address}")
```

### cURL

```bash
# Health check
curl https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app/health

# Get balance
curl https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app/v1/balance/YOUR_ADDRESS

# Fund from faucet
curl -X POST https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app/faucet/YOUR_ADDRESS

# Get latest block
curl https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app/v1/block/latest

# Transaction history
curl https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app/v1/transactions/YOUR_ADDRESS
```

---

## Architecture

```
USER (anywhere in the world)
    |
    | HTTPS (auto-selects nearest)
    |
    |---> Montreal Proxy ---> Montreal Validator (100ms blocks)
    |---> Iowa Proxy     ---> Iowa Validator     (100ms blocks)
    +---> London Proxy   ---> London Validator   (100ms blocks)
```

### Components

**Validator (GCE VM, $24/month each)** -- Runs the blockchain:
- Receives and verifies transactions (Ed25519 signatures)
- Checks nonces and balances
- Executes WASM smart contracts with gas metering
- Produces blocks every 100ms
- Stores chain state

**HTTPS Proxy (Cloud Run, ~$0-2/month each)** -- Browser compatibility:
- Translates HTTPS to HTTP
- Adds CORS headers
- No logic, just forwarding

### Latency by Connection Type

| Connection | Path | Expected |
|---|---|---|
| Browser (same region) | HTTPS Proxy -> Validator | ~120-150ms |
| Browser (cross-region) | HTTPS Proxy -> Validator | ~250-350ms |
| Server (same region) | Direct HTTP -> Validator | ~105ms |
| Server (co-located) | Same data center | ~102ms |

### Why 100ms Finality

21 known validators means minimal consensus rounds. Each block is final
the moment it's produced -- no "soft confirmation" period, no rollback window.

### Zero Fees

Gas is metered (prevents infinite loops) but costs $0.00. The GasMeter runs
on every WASM contract call, counting operations. When the limit is hit,
execution halts. But the gas price is zero -- nobody pays.

Validators are funded by Dina Inc. as an operational expense, not by user fees.

---

## 9-Wallet System

Every user gets 9 independent wallets with their own Ed25519 keypair:

| Wallet | Type | Purpose |
|--------|------|---------|
| Smart 1 | Smart Account | Main wallet |
| Smart 2 | Smart Account | Savings |
| Smart 3 | Smart Account | Backup |
| Agent 1 | Agent Wallet | AI shopping bot |
| Agent 2 | Agent Wallet | Bill payments |
| Agent 3 | Agent Wallet | Custom agent |
| Parallel 1 | Parallel Wallet | Business payments |
| Parallel 2 | Parallel Wallet | Streaming payments |
| Parallel 3 | Parallel Wallet | API access |

All earn 4.5% APY. Users keep 100%.

---

## Live Apps

| App | URL |
|-----|-----|
| Wallet | https://dina-wallet.web.app |
| Explorer | https://dina-explorer.web.app |

---

## Speed Comparison (Hard Finality)

| Chain | Finality | User Sees | Fees |
|-------|----------|-----------|------|
| **Dina** | **100ms** | **~120ms** | **$0.00** |
| Sui | 500ms | ~1s | need SUI token |
| Sei | 400ms | ~1s | need SEI token |
| Aptos | 900ms | ~1.5s | need APT token |
| Solana | 6-12s | ~6s | need SOL token |
| Ethereum | 12 min | ~12 min | $1-50 + need ETH |

---

## Costs

| Component | Count | Monthly |
|-----------|-------|---------|
| Validators (GCE) | 3 regional | $72 |
| Proxies (Cloud Run) | 3 regional | ~$3 |
| **Total** | | **~$75/month** |

21 validators at mainnet: ~$525/month for a global network faster than
any blockchain in existence.
