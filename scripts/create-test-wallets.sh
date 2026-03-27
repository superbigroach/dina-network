#!/bin/bash
# =============================================================================
# Dina Network — Test Wallet Generator
# Creates 10 named test wallets for development, requests faucet USDC for each.
#
# Wallets:
#   Users:   Alice, Bob, Charlie, Dave, Eve
#   Devices: Robot1, Robot2, Robot3
#   System:  Faucet, Treasury
#
# Usage:  ./scripts/create-test-wallets.sh [--rpc-url URL] [--faucet-url URL]
#
# Output: keys/test-wallets/<name>/
#           - private_key     (32-byte Ed25519 secret key)
#           - private_key.pub (32-byte Ed25519 public key)
#           - address.txt     (hex-encoded address)
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
WALLETS_DIR="$PROJECT_DIR/keys/test-wallets"
DINA_CLI="${DINA_CLI:-$PROJECT_DIR/target/release/dina}"
RPC_URL="${RPC_URL:-http://localhost:8545}"
FAUCET_URL="${FAUCET_URL:-http://localhost:8080/faucet}"

# Parse command-line args
for arg in "$@"; do
    case "$arg" in
        --rpc-url=*)    RPC_URL="${arg#*=}" ;;
        --faucet-url=*) FAUCET_URL="${arg#*=}" ;;
        -h|--help)
            echo "Usage: $0 [--rpc-url=URL] [--faucet-url=URL]"
            echo ""
            echo "Creates 10 test wallets and funds them via the faucet."
            echo ""
            echo "Options:"
            echo "  --rpc-url=URL      RPC endpoint (default: http://localhost:8545)"
            echo "  --faucet-url=URL   Faucet endpoint (default: http://localhost:8080/faucet)"
            exit 0
            ;;
    esac
done

# Wallet definitions: name, type
WALLETS=(
    "alice:user"
    "bob:user"
    "charlie:user"
    "dave:user"
    "eve:user"
    "robot1:device"
    "robot2:device"
    "robot3:device"
    "faucet:system"
    "treasury:system"
)

# ---------------------------------------------------------------------------
# Colors
# ---------------------------------------------------------------------------

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

# ---------------------------------------------------------------------------
# Banner
# ---------------------------------------------------------------------------

echo ""
echo -e "${CYAN}${BOLD}"
echo "  ============================================"
echo "    Dina Network — Test Wallet Generator"
echo "  ============================================"
echo -e "${NC}"
echo "  Output:     $WALLETS_DIR"
echo "  RPC URL:    $RPC_URL"
echo "  Faucet URL: $FAUCET_URL"
echo "  Wallets:    ${#WALLETS[@]}"
echo ""

# ---------------------------------------------------------------------------
# Prerequisites
# ---------------------------------------------------------------------------

if [ ! -f "$DINA_CLI" ]; then
    echo -e "${YELLOW}[WARN]${NC} dina CLI not found at $DINA_CLI"
    echo "       Building..."
    cd "$PROJECT_DIR"
    cargo build --release --bin dina 2>&1 | tail -3
    if [ ! -f "$DINA_CLI" ]; then
        echo -e "${RED}[FAIL]${NC} Build failed"
        exit 1
    fi
fi

# ---------------------------------------------------------------------------
# Generate Wallets
# ---------------------------------------------------------------------------

echo -e "${BLUE}${BOLD}[STEP]${NC} Generating wallets..."
echo ""

mkdir -p "$WALLETS_DIR"

# Arrays to collect results for the summary table
WALLET_NAMES=()
WALLET_TYPES=()
WALLET_ADDRESSES=()
WALLET_STATUSES=()
WALLET_BALANCES=()

for entry in "${WALLETS[@]}"; do
    IFS=':' read -r name type <<< "$entry"

    wallet_dir="$WALLETS_DIR/$name"
    key_file="$wallet_dir/private_key"
    address_file="$wallet_dir/address.txt"

    mkdir -p "$wallet_dir"

    # Generate key if it doesn't exist
    if [ -f "$key_file" ]; then
        echo -e "  ${DIM}[$name]${NC} Key already exists -- skipping generation"
    else
        "$DINA_CLI" keygen --output "$key_file" 2>/dev/null
        echo -e "  ${GREEN}[$name]${NC} Key generated"
    fi

    # Read the public key / address
    local_address=""
    if [ -f "${key_file}.pub" ]; then
        local_address=$(xxd -p -c 64 "${key_file}.pub" 2>/dev/null || od -An -tx1 "${key_file}.pub" | tr -d ' \n')
    fi

    # Save address to text file for easy reference
    if [ -n "$local_address" ]; then
        echo "$local_address" > "$address_file"
    fi

    WALLET_NAMES+=("$name")
    WALLET_TYPES+=("$type")
    WALLET_ADDRESSES+=("$local_address")
done

echo ""

# ---------------------------------------------------------------------------
# Fund Wallets via Faucet
# ---------------------------------------------------------------------------

echo -e "${BLUE}${BOLD}[STEP]${NC} Requesting faucet USDC for each wallet..."
echo ""

# Check if faucet is reachable
faucet_available=true
if ! curl -sf "${FAUCET_URL}/stats" &>/dev/null; then
    echo -e "  ${YELLOW}[WARN]${NC} Faucet not reachable at $FAUCET_URL"
    echo -e "  ${YELLOW}      ${NC} Start the testnet first: ./scripts/setup-testnet.sh"
    echo -e "  ${YELLOW}      ${NC} Skipping funding step."
    faucet_available=false
fi

for i in "${!WALLET_NAMES[@]}"; do
    name="${WALLET_NAMES[$i]}"
    address="${WALLET_ADDRESSES[$i]}"

    if [ -z "$address" ]; then
        WALLET_STATUSES+=("no-key")
        WALLET_BALANCES+=("--")
        continue
    fi

    if ! $faucet_available; then
        WALLET_STATUSES+=("skipped")
        WALLET_BALANCES+=("--")
        continue
    fi

    # Request funds from faucet
    response=$(curl -sf -X POST "${FAUCET_URL}/request" \
        -H "Content-Type: application/json" \
        -d "{\"address\": \"$address\"}" 2>/dev/null || echo '{"success":false,"error":"connection failed"}')

    success=$(echo "$response" | grep -o '"success":\s*true' || true)

    if [ -n "$success" ]; then
        amount=$(echo "$response" | grep -o '"amount_display":"[^"]*"' | cut -d'"' -f4)
        WALLET_STATUSES+=("funded")
        WALLET_BALANCES+=("${amount:-100 USDC}")
        echo -e "  ${GREEN}[$name]${NC} Funded: ${amount:-100 USDC}"
    else
        error=$(echo "$response" | grep -o '"error":"[^"]*"' | cut -d'"' -f4)
        if echo "$error" | grep -qi "cooldown\|wait\|rate"; then
            WALLET_STATUSES+=("cooldown")
            WALLET_BALANCES+=("(pending)")
            echo -e "  ${YELLOW}[$name]${NC} Rate limited -- try again later"
        else
            WALLET_STATUSES+=("error")
            WALLET_BALANCES+=("--")
            echo -e "  ${RED}[$name]${NC} Failed: ${error:-unknown error}"
        fi
    fi

    # Brief pause between requests to avoid flooding the faucet
    sleep 1
done

echo ""

# ---------------------------------------------------------------------------
# Summary Table
# ---------------------------------------------------------------------------

echo -e "${GREEN}${BOLD}"
echo "  ============================================"
echo "    Test Wallets Summary"
echo "  ============================================"
echo -e "${NC}"

# Table header
printf "  ${BOLD}%-12s %-8s %-20s %-10s %-15s${NC}\n" \
    "NAME" "TYPE" "ADDRESS" "STATUS" "BALANCE"
printf "  %-12s %-8s %-20s %-10s %-15s\n" \
    "------------" "--------" "--------------------" "----------" "---------------"

for i in "${!WALLET_NAMES[@]}"; do
    name="${WALLET_NAMES[$i]}"
    type="${WALLET_TYPES[$i]}"
    address="${WALLET_ADDRESSES[$i]}"
    status="${WALLET_STATUSES[$i]:-unknown}"
    balance="${WALLET_BALANCES[$i]:---}"

    # Truncate address for display
    if [ -n "$address" ] && [ ${#address} -gt 16 ]; then
        addr_display="${address:0:8}...${address: -8}"
    else
        addr_display="${address:-(none)}"
    fi

    # Color the status
    case "$status" in
        funded)   status_display="${GREEN}funded${NC}" ;;
        cooldown) status_display="${YELLOW}cooldown${NC}" ;;
        error)    status_display="${RED}error${NC}" ;;
        skipped)  status_display="${DIM}skipped${NC}" ;;
        *)        status_display="${DIM}$status${NC}" ;;
    esac

    printf "  %-12s %-8s %-20s " "$name" "$type" "$addr_display"
    printf "%-10b %-15s\n" "$status_display" "$balance"
done

echo ""
echo "  Key files:   $WALLETS_DIR/<name>/private_key"
echo "  Addresses:   $WALLETS_DIR/<name>/address.txt"
echo ""

# ---------------------------------------------------------------------------
# Generate .env-style file for easy import
# ---------------------------------------------------------------------------

ENV_FILE="$WALLETS_DIR/test-wallets.env"

{
    echo "# Dina Network Test Wallets"
    echo "# Generated on $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
    echo "#"
    echo "# Usage: source keys/test-wallets/test-wallets.env"
    echo ""
    for i in "${!WALLET_NAMES[@]}"; do
        name="${WALLET_NAMES[$i]}"
        address="${WALLET_ADDRESSES[$i]}"
        upper_name=$(echo "$name" | tr '[:lower:]' '[:upper:]')
        echo "DINA_${upper_name}_ADDRESS=$address"
    done
} > "$ENV_FILE"

echo -e "  ${CYAN}Env file written: $ENV_FILE${NC}"
echo "  Source it:       source $ENV_FILE"
echo ""

# ---------------------------------------------------------------------------
# Generate JSON file for SDK usage
# ---------------------------------------------------------------------------

JSON_FILE="$WALLETS_DIR/test-wallets.json"

{
    echo "{"
    echo "  \"generated\": \"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\","
    echo "  \"chain_id\": \"dina-testnet-1\","
    echo "  \"rpc_url\": \"$RPC_URL\","
    echo "  \"wallets\": ["
    for i in "${!WALLET_NAMES[@]}"; do
        name="${WALLET_NAMES[$i]}"
        type="${WALLET_TYPES[$i]}"
        address="${WALLET_ADDRESSES[$i]}"
        comma=""
        if [ $i -lt $(( ${#WALLET_NAMES[@]} - 1 )) ]; then
            comma=","
        fi
        echo "    {"
        echo "      \"name\": \"$name\","
        echo "      \"type\": \"$type\","
        echo "      \"address\": \"$address\","
        echo "      \"key_file\": \"keys/test-wallets/$name/private_key\""
        echo "    }${comma}"
    done
    echo "  ]"
    echo "}"
} > "$JSON_FILE"

echo -e "  ${CYAN}JSON file written: $JSON_FILE${NC}"
echo ""
echo -e "${GREEN}Done!${NC} Use these wallets for local development and testing."
echo ""
