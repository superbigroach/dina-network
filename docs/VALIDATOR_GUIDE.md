# Validator Guide

## Overview

Validators are the core of the Dina Network. They participate in TurboBFT consensus to propose and commit blocks, execute transactions, and maintain the blockchain state. The current protocol supports 3-7 validators with sub-200ms finality.

## Hardware Requirements

### Minimum (Testnet)

| Component | Requirement |
|-----------|-------------|
| CPU | 2 cores, x86_64 or aarch64 |
| RAM | 4 GB |
| Storage | 50 GB SSD |
| Network | 10 Mbps, stable uptime |
| OS | Linux (Ubuntu 22.04+), macOS, or Windows 11 |

### Recommended (Mainnet)

| Component | Requirement |
|-----------|-------------|
| CPU | 4+ cores, modern x86_64 (AVX2 support) |
| RAM | 16 GB |
| Storage | 500 GB NVMe SSD |
| Network | 100 Mbps with static IP, 99.9% uptime |
| OS | Ubuntu 22.04 LTS or Debian 12 |

### Storage Growth Estimate

With an average of 50 transactions per block and 2-second block times:

- ~43,200 blocks/day
- ~25 MB/day of block data (empty blocks are small)
- ~500 MB/day at full capacity
- ~180 GB/year at full capacity

redb provides built-in compaction, so actual disk usage will be lower than raw block data.

## Software Setup

### Prerequisites

1. **Rust toolchain** (1.70 or later):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

2. **WASM target** (required for smart contract execution):
```bash
rustup target add wasm32-unknown-unknown
```

3. **Windows only**: MinGW-w64 with `dlltool` on PATH:
```powershell
# Install via MSYS2
pacman -S mingw-w64-x86_64-toolchain
```

4. **System dependencies** (Linux):
```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev
```

### Build from Source

```bash
git clone https://github.com/lucilla-app/dina_network.git
cd dina_network
cargo build --release --bin dina-node
cargo build --release --bin dina
```

The binaries will be at:
- `target/release/dina-node` -- validator/full-node binary
- `target/release/dina` -- CLI tool

### Verify the Build

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## Key Generation

### Generate a Validator Key

Every validator needs an Ed25519 keypair. Generate one using the CLI:

```bash
./target/release/dina keygen
```

This outputs:
- **Public key** (hex, 32 bytes): Your validator identity
- **Address** (hex, 32 bytes): SHA-256 of the public key, used as your on-chain address
- **Secret key file**: Saved to the specified path

Alternatively, the node will auto-generate a key on first run if none exists.

### Key File Format

The key file is a raw 32-byte Ed25519 secret key (no encoding, no headers). Store it securely:

```bash
# Set restrictive permissions (Linux/macOS)
chmod 600 ~/.dina/node_key
chmod 600 ~/.dina/validator_key
```

### Backup Your Key

Your validator key is your identity on the network. Losing it means losing your validator position and any associated staking rewards.

```bash
# Create an encrypted backup
gpg --symmetric --cipher-algo AES256 ~/.dina/validator_key
# Store the .gpg file in a secure offline location
```

## Genesis Participation

### Testnet Genesis

For testnet, the default genesis configuration is built in. Simply start the node with `--validator`:

```bash
./target/release/dina-node --validator
```

### Custom Genesis

To participate in a new chain genesis:

1. **Share your public key** with the genesis coordinator.

2. **Receive the genesis file** (`genesis.json`) with all validator public keys and initial accounts:

```json
{
  "chain_id": "dina-mainnet-1",
  "timestamp": 1700000000,
  "validators": [
    "aabbcc...64hex...validator1_pubkey",
    "ddeeff...64hex...validator2_pubkey",
    "112233...64hex...validator3_pubkey"
  ],
  "initial_accounts": [
    {
      "address": "0000...0001",
      "balance": 1000000000000000,
      "label": "Foundation Treasury"
    }
  ]
}
```

3. **Start with the genesis file**:

```bash
./target/release/dina-node \
  --validator \
  --validator-key ~/.dina/validator_key \
  --genesis genesis.json \
  --chain-id dina-mainnet-1 \
  --data-dir ~/.dina
```

### Genesis Block

The genesis block is created automatically from the genesis configuration on first startup. It has:
- Block number: 0
- Parent hash: all zeros
- No transactions
- Initial account balances as specified in genesis.json

## Starting the Node

### Validator Mode

```bash
./target/release/dina-node \
  --validator \
  --validator-key ~/.dina/validator_key \
  --data-dir ~/.dina \
  --chain-id dina-testnet-1 \
  --rpc-port 8545 \
  --rest-port 8080 \
  --listen /ip4/0.0.0.0/tcp/9944 \
  --log-level info
```

### Full Node Mode (Non-Validator)

```bash
./target/release/dina-node \
  --data-dir ~/.dina \
  --chain-id dina-testnet-1 \
  --bootstrap /ip4/validator1.example.com/tcp/9944/p2p/12D3KooW...
```

### CLI Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--data-dir` | `~/.dina` | Data directory for chain storage and keys |
| `--listen` | `/ip4/0.0.0.0/tcp/9944` | P2P listen address (multiaddr) |
| `--rpc-port` | `8545` | JSON-RPC server port |
| `--rest-port` | `8080` | REST API server port |
| `--validator` | false | Enable validator mode (participate in consensus) |
| `--validator-key` | (none) | Path to validator key file (uses node key if omitted) |
| `--bootstrap` | (none) | Bootstrap peer multiaddresses (repeatable) |
| `--chain-id` | `dina-testnet-1` | Chain identifier |
| `--genesis` | (built-in) | Path to genesis configuration JSON |
| `--log-level` | `info` | Log level: trace, debug, info, warn, error |

### Systemd Service (Linux)

Create `/etc/systemd/system/dina-node.service`:

```ini
[Unit]
Description=Dina Network Validator Node
After=network.target

[Service]
Type=simple
User=dina
Group=dina
ExecStart=/usr/local/bin/dina-node \
  --validator \
  --validator-key /home/dina/.dina/validator_key \
  --data-dir /home/dina/.dina \
  --chain-id dina-testnet-1 \
  --log-level info
Restart=always
RestartSec=5
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable dina-node
sudo systemctl start dina-node
sudo journalctl -u dina-node -f
```

## Monitoring

### Log Output

The node uses the `tracing` framework with structured JSON output. Key log messages:

```
INFO Starting Dina Network node version=0.1.0 chain_id=dina-testnet-1
INFO Node identity ready address=0xabab...abab
INFO Database opened path=/home/dina/.dina/chain.redb
INFO Genesis block stored at height 0 hash=0xdead...beef
INFO RPC servers started jsonrpc=127.0.0.1:8545 rest=0.0.0.0:8080
INFO Starting consensus engine as validator address=0xabab...abab
INFO Dina node fully started and ready listen=/ip4/0.0.0.0/tcp/9944
```

### Health Check

```bash
curl http://localhost:8080/health
# {"status":"ok","height":12345,"peers":5}
```

### Block Height

```bash
curl http://localhost:8080/v1/block/latest
```

### JSON-RPC Status

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"dina_networkInfo","params":[],"id":1}'
```

### Mempool Status

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"dina_txPoolStatus","params":[],"id":1}'
```

### Prometheus Metrics (Planned)

The `dina-monitoring` crate will expose Prometheus-compatible metrics at `/metrics`:

- `dina_block_height` -- current block height
- `dina_peer_count` -- connected peers
- `dina_tx_pool_size` -- mempool transaction count
- `dina_consensus_round` -- current consensus round
- `dina_block_time_ms` -- time between blocks
- `dina_gas_used_per_block` -- gas consumed per block

## Upgrading

### Rolling Upgrade

For minor version upgrades that do not change the consensus protocol:

1. Build the new version:
```bash
cd dina_network
git pull
cargo build --release --bin dina-node
```

2. Stop the node:
```bash
sudo systemctl stop dina-node
```

3. Replace the binary:
```bash
sudo cp target/release/dina-node /usr/local/bin/dina-node
```

4. Restart:
```bash
sudo systemctl start dina-node
```

### Hard Fork Upgrade

For consensus-breaking changes, all validators must upgrade simultaneously:

1. Coordinate upgrade height with other validators
2. Build and test the new version
3. At the agreed-upon height, stop all validators
4. Replace binaries on all validators
5. Restart all validators within the timeout window

### Database Migrations

The `dina-storage` crate includes an automatic migration system. When the node starts, it checks the schema version in `STATE_METADATA` and runs any pending migrations. Migrations are idempotent and safe to run multiple times.

## Staking and Rewards

### Fee Distribution

Validators earn fees from the transactions they include in blocks. The fee model is:

- **Base fee**: Each transaction type has a minimum fee (set by the `FeeSchedule`)
- **Priority fee**: Users can pay higher fees for faster inclusion
- **Block proposer**: The validator who proposes a committed block receives all transaction fees in that block

### Staking (Planned for Mainnet)

The mainnet will introduce staking requirements:

- **Minimum stake**: TBD USDC locked in a staking contract
- **Unstaking period**: 7-day unbonding period
- **Reward distribution**: Proportional to stake weight

### Slashing Conditions

Validators can be penalized (slashed) for misbehavior:

| Violation | Penalty | Description |
|-----------|---------|-------------|
| Double signing | 5% stake | Signing two different blocks at the same height |
| Extended downtime | 0.1% stake/day | Missing more than 100 consecutive blocks |
| Invalid proposals | Warning | Proposing blocks with invalid transactions |
| Censorship | Manual review | Consistently excluding valid transactions |

### Double Signing Detection

The consensus engine detects double signing by checking:
- Same validator signs two different proposals for the same (height, round)
- Same validator casts two conflicting prevotes or precommits

Evidence of double signing is included in a future block, and the offending validator's stake is slashed.

## Common Issues

### Node fails to start: "failed to open database"

**Cause**: The data directory does not exist or has incorrect permissions.

**Fix**:
```bash
mkdir -p ~/.dina
chmod 755 ~/.dina
```

### Node fails to start: "node key file has invalid length"

**Cause**: The node key file is corrupted or was created by a different key format.

**Fix**: Delete the key file and let the node generate a new one:
```bash
rm ~/.dina/node_key
# Restart the node
```

### Consensus is not progressing

**Cause**: Fewer than 2/3 of validators are online, so quorum cannot be reached.

**Fix**:
- Check that all validators are running and connected
- Verify bootstrap addresses are correct
- Check firewall rules allow port 9944 (P2P) inbound

### Round timeouts and view changes

**Cause**: The leader for the current round is slow or unreachable. The consensus protocol automatically rotates to the next leader after the timeout (default: 10 seconds).

**Fix**: This is normal behavior. If it happens frequently:
- Check network latency between validators
- Increase `timeout_ms` if latency is consistently high
- Ensure the slow validator has adequate resources

### High memory usage

**Cause**: Large mempool or many open WebSocket connections.

**Fix**:
- The mempool is capped at 10,000 transactions by default
- Reduce WebSocket subscriber limits
- Increase system RAM

### Database corruption

**Cause**: Power loss or crash during a write operation.

**Fix**: redb uses write-ahead logging and is crash-safe by design. If corruption occurs:
```bash
# Backup current data
cp -r ~/.dina ~/.dina.bak

# Re-sync from peers
rm ~/.dina/chain.redb
# Restart the node -- it will re-sync from genesis
```

### Port conflicts

**Cause**: Another service is using port 8545, 8080, or 9944.

**Fix**: Use the `--rpc-port`, `--rest-port`, and `--listen` flags to change ports:
```bash
./dina-node --validator --rpc-port 9545 --rest-port 9080 --listen /ip4/0.0.0.0/tcp/9945
```

### Windows: "dlltool not found"

**Cause**: MinGW-w64 is not installed or not on PATH.

**Fix**: Install MinGW-w64 via MSYS2 and ensure `dlltool.exe` is on your PATH:
```powershell
$env:PATH += ";C:\msys64\mingw64\bin"
```

### Clock drift

**Cause**: Validators need synchronized clocks for timeout calculations.

**Fix**: Enable NTP synchronization:
```bash
sudo timedatectl set-ntp true
```

## Security Best Practices

1. **Key management**: Store validator keys on encrypted volumes. Never share secret keys.
2. **Firewall**: Only expose port 9944 (P2P) publicly. Keep RPC ports (8545, 8080) behind a reverse proxy or firewall.
3. **Updates**: Stay on the latest release. Subscribe to security advisories.
4. **Monitoring**: Set up alerts for missed blocks, high round numbers, and unusual mempool growth.
5. **Backups**: Regularly back up your key files and database.
6. **Isolation**: Run the validator in a dedicated user account with minimal privileges.
7. **Sentry nodes**: For mainnet, consider running sentry nodes that shield the validator from direct public connections.
