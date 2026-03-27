# dina-network

Python SDK for [Dina Network](https://github.com/superbigroach/dina-network) — the fastest blockchain with swarm wallets, parallel execution, and 82 DRC smart contract standards.

## Install

```bash
pip install dina-network
```

## Quick Start

```python
from dina import DinaWallet, DinaClient

# Create a wallet
wallet = DinaWallet.generate()
print(f"Address: {wallet.address}")

# Connect to testnet
client = DinaClient("http://35.184.213.248:8545")

# Check balance
balance = client.get_balance(wallet.address)

# Send USDC
tx_hash = client.transfer(wallet, to="0x...", amount=1_000_000)  # 1 USDC

# Wait for confirmation (100ms finality)
receipt = client.wait_for_transaction(tx_hash)
```

## Async Support

```python
from dina import AsyncDinaClient, DinaWallet

wallet = DinaWallet.generate()
client = AsyncDinaClient("http://35.184.213.248:8545")

balance = await client.get_balance(wallet.address)
tx_hash = await client.transfer(wallet, to="0x...", amount=1_000_000)
```

## Testnet

```
RPC: http://35.184.213.248:8545
REST: http://35.184.213.248:8080
Chain ID: dina-testnet-1
```

## License

MIT
