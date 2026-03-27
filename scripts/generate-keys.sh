#!/bin/bash
# =============================================================================
# Dina Network — Validator Key Generation
# Generates Ed25519 keypairs for 3 validators and a faucet account,
# then produces a genesis.json with all validator pubkeys.
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
KEYS_DIR="$PROJECT_DIR/keys"
GENESIS_FILE="$PROJECT_DIR/genesis.json"
CHAIN_ID="${CHAIN_ID:-dina-local-testnet}"
NUM_VALIDATORS=3

# Check if dina CLI is available
if ! command -v dina &>/dev/null; then
    echo "Error: 'dina' CLI not found in PATH."
    echo "Build it first:  cargo build --release --bin dina"
    echo "Then add target/release/ to your PATH, or run:"
    echo "  export PATH=\"\$PATH:$PROJECT_DIR/target/release\""
    exit 1
fi

echo "============================================"
echo "  Dina Network — Key Generation"
echo "============================================"
echo ""

# Create key directories
for i in $(seq 1 $NUM_VALIDATORS); do
    mkdir -p "$KEYS_DIR/validator-$i"
done
mkdir -p "$KEYS_DIR/faucet"

# Generate validator keys
VALIDATOR_PUBKEYS=()
VALIDATOR_ADDRESSES=()

for i in $(seq 1 $NUM_VALIDATORS); do
    KEY_FILE="$KEYS_DIR/validator-$i/node_key"

    if [ -f "$KEY_FILE" ]; then
        echo "[Validator $i] Key already exists at $KEY_FILE — skipping."
        # Read the existing public key
        PUBKEY_HEX=$(xxd -p -c 64 "${KEY_FILE}.pub" 2>/dev/null || echo "")
        if [ -z "$PUBKEY_HEX" ]; then
            echo "  Warning: public key file missing, regenerating..."
            rm -f "$KEY_FILE"
        fi
    fi

    if [ ! -f "$KEY_FILE" ]; then
        echo "[Validator $i] Generating Ed25519 keypair..."
        OUTPUT=$(dina keygen --output "$KEY_FILE" 2>&1)
        echo "  $OUTPUT"
    fi

    # Read the 32-byte public key and convert to hex
    PUBKEY_HEX=$(xxd -p -c 64 "${KEY_FILE}.pub")
    VALIDATOR_PUBKEYS+=("$PUBKEY_HEX")

    echo "  Public key: 0x${PUBKEY_HEX}"
    echo ""
done

# Generate faucet key
FAUCET_KEY="$KEYS_DIR/faucet/faucet_key"
if [ ! -f "$FAUCET_KEY" ]; then
    echo "[Faucet] Generating faucet keypair..."
    OUTPUT=$(dina keygen --output "$FAUCET_KEY" 2>&1)
    echo "  $OUTPUT"
else
    echo "[Faucet] Key already exists at $FAUCET_KEY — skipping."
fi
FAUCET_PUBKEY=$(xxd -p -c 64 "${FAUCET_KEY}.pub")
echo "  Public key: 0x${FAUCET_PUBKEY}"
echo ""

# Build genesis.json
echo "Generating genesis.json..."

# Build the validators JSON array
VALIDATORS_JSON="["
for i in $(seq 0 $(( NUM_VALIDATORS - 1 ))); do
    if [ $i -gt 0 ]; then
        VALIDATORS_JSON+=","
    fi
    VALIDATORS_JSON+=$(cat <<ENTRY

    {
      "pubkey": "${VALIDATOR_PUBKEYS[$i]}",
      "stake": 1000000000000,
      "label": "Validator $((i + 1))"
    }
ENTRY
    )
done
VALIDATORS_JSON+=$'\n  ]'

cat > "$GENESIS_FILE" <<EOF
{
  "chain_id": "$CHAIN_ID",
  "timestamp": $(date +%s),
  "validators": $VALIDATORS_JSON,
  "initial_accounts": [
    {
      "address": "$FAUCET_PUBKEY",
      "balance": 1000000000000000,
      "label": "Testnet Faucet (1 Billion USDC)"
    }
  ]
}
EOF

echo ""
echo "============================================"
echo "  Key Generation Complete"
echo "============================================"
echo ""
echo "Keys directory:  $KEYS_DIR"
echo "Genesis file:    $GENESIS_FILE"
echo ""
echo "Validators:"
for i in $(seq 1 $NUM_VALIDATORS); do
    echo "  [$i] $KEYS_DIR/validator-$i/node_key"
done
echo ""
echo "Faucet: $KEYS_DIR/faucet/faucet_key"
echo ""
echo "Next steps:"
echo "  1. Start the local testnet:  ./scripts/local-testnet.sh"
echo "  2. Or use Docker Compose:    docker compose up --build"
