#!/bin/bash
# =============================================================================
# Dina Network — Complete Testnet Setup Wizard
# All-in-one script that builds, configures, and starts a local testnet.
#
# Usage:  ./scripts/setup-testnet.sh
#
# Starts:
#   - 3 validator nodes (TurboBFT consensus)
#   - 1 RPC node (JSON-RPC + REST)
#   - 1 Faucet server (HTTP faucet API)
#   - 1 Explorer backend (block explorer API)
#
# Press Ctrl+C to cleanly shut down everything.
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
KEYS_DIR="$PROJECT_DIR/keys"
DATA_DIR="$PROJECT_DIR/.local-testnet"
GENESIS_FILE="$PROJECT_DIR/genesis.json"
CHAIN_ID="${CHAIN_ID:-dina-testnet-1}"
NUM_VALIDATORS=3
RUST_LOG="${RUST_LOG:-info,dina_consensus=debug}"

# Binary paths
DINA_CLI="$PROJECT_DIR/target/release/dina"
DINA_NODE="$PROJECT_DIR/target/release/dina-node"

# Port assignments
BASE_P2P_PORT=9944
BASE_RPC_PORT=8545
BASE_REST_PORT=8080
FAUCET_PORT=3000
EXPLORER_PORT=3001

# Track PIDs for cleanup
PIDS=()
SERVICES=()

# ---------------------------------------------------------------------------
# Colors and output helpers
# ---------------------------------------------------------------------------

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

banner() {
    echo ""
    echo -e "${CYAN}${BOLD}"
    echo "  ============================================"
    echo "    Dina Network — Testnet Setup Wizard"
    echo "  ============================================"
    echo -e "${NC}"
    echo "  Chain ID:    $CHAIN_ID"
    echo "  Validators:  $NUM_VALIDATORS"
    echo "  Project:     $PROJECT_DIR"
    echo ""
}

step() {
    echo -e "${BLUE}${BOLD}[STEP]${NC} $1"
}

ok() {
    echo -e "${GREEN}  [OK]${NC} $1"
}

warn() {
    echo -e "${YELLOW}  [WARN]${NC} $1"
}

fail() {
    echo -e "${RED}  [FAIL]${NC} $1"
    exit 1
}

info() {
    echo -e "  ${CYAN}-->  ${NC}$1"
}

# ---------------------------------------------------------------------------
# Cleanup on exit
# ---------------------------------------------------------------------------

cleanup() {
    echo ""
    echo -e "${YELLOW}${BOLD}Shutting down testnet...${NC}"

    for i in "${!PIDS[@]}"; do
        local pid="${PIDS[$i]}"
        local name="${SERVICES[$i]:-unknown}"
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
            echo "  Stopped $name (PID $pid)"
        fi
    done

    # Wait briefly for processes to exit
    sleep 1

    # Force-kill any stragglers
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill -9 "$pid" 2>/dev/null || true
        fi
    done

    echo ""
    echo -e "${GREEN}Testnet stopped cleanly.${NC}"
}

trap cleanup EXIT INT TERM

# ---------------------------------------------------------------------------
# Step 1: Check Prerequisites
# ---------------------------------------------------------------------------

check_prerequisites() {
    step "Checking prerequisites..."

    # Rust
    if command -v rustc &>/dev/null; then
        local rust_version
        rust_version=$(rustc --version | awk '{print $2}')
        ok "Rust $rust_version"
    else
        fail "Rust not found. Install from https://rustup.rs"
    fi

    # Cargo
    if command -v cargo &>/dev/null; then
        ok "Cargo available"
    else
        fail "Cargo not found. Install from https://rustup.rs"
    fi

    # WASM target
    if rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
        ok "wasm32-unknown-unknown target installed"
    else
        warn "wasm32-unknown-unknown target not installed, installing..."
        rustup target add wasm32-unknown-unknown
        ok "wasm32-unknown-unknown target installed"
    fi

    # curl (for health checks)
    if command -v curl &>/dev/null; then
        ok "curl available"
    else
        warn "curl not found — health checks will be skipped"
    fi

    echo ""
}

# ---------------------------------------------------------------------------
# Step 2: Build Binaries
# ---------------------------------------------------------------------------

build_binaries() {
    step "Building binaries (release mode)..."

    cd "$PROJECT_DIR"

    if [ -f "$DINA_NODE" ] && [ -f "$DINA_CLI" ]; then
        local node_age
        node_age=$(( $(date +%s) - $(stat -c %Y "$DINA_NODE" 2>/dev/null || stat -f %m "$DINA_NODE" 2>/dev/null || echo 0) ))
        if [ "$node_age" -lt 3600 ]; then
            ok "Binaries are recent (built ${node_age}s ago) — skipping rebuild"
            info "To force rebuild, delete target/release/ and re-run"
            echo ""
            return 0
        fi
    fi

    info "This may take a few minutes on first run..."
    cargo build --release --bin dina-node --bin dina 2>&1 | tail -5

    if [ -f "$DINA_NODE" ] && [ -f "$DINA_CLI" ]; then
        ok "dina-node built: $DINA_NODE"
        ok "dina CLI built:  $DINA_CLI"
    else
        fail "Build failed — binaries not found"
    fi

    echo ""
}

# ---------------------------------------------------------------------------
# Step 3: Generate Validator Keys + Faucet Key
# ---------------------------------------------------------------------------

generate_keys() {
    step "Generating validator keys..."

    local need_keys=false
    for i in $(seq 1 $NUM_VALIDATORS); do
        if [ ! -f "$KEYS_DIR/validator-$i/node_key" ]; then
            need_keys=true
            break
        fi
    done
    if [ ! -f "$KEYS_DIR/faucet/faucet_key" ]; then
        need_keys=true
    fi

    if ! $need_keys; then
        ok "All keys already exist in $KEYS_DIR"
        echo ""
        return 0
    fi

    # Use the existing generate-keys.sh script if available
    if [ -f "$SCRIPT_DIR/generate-keys.sh" ]; then
        bash "$SCRIPT_DIR/generate-keys.sh"
    else
        # Inline key generation using the dina CLI
        for i in $(seq 1 $NUM_VALIDATORS); do
            local key_dir="$KEYS_DIR/validator-$i"
            mkdir -p "$key_dir"
            if [ ! -f "$key_dir/node_key" ]; then
                "$DINA_CLI" keygen --output "$key_dir/node_key"
                ok "Validator $i key generated"
            else
                ok "Validator $i key exists"
            fi
        done

        # Faucet key
        mkdir -p "$KEYS_DIR/faucet"
        if [ ! -f "$KEYS_DIR/faucet/faucet_key" ]; then
            "$DINA_CLI" keygen --output "$KEYS_DIR/faucet/faucet_key"
            ok "Faucet key generated"
        else
            ok "Faucet key exists"
        fi
    fi

    echo ""
}

# ---------------------------------------------------------------------------
# Step 4: Create Genesis
# ---------------------------------------------------------------------------

create_genesis() {
    step "Creating genesis configuration..."

    # If genesis already has validators, skip
    if [ -f "$GENESIS_FILE" ]; then
        local validator_count
        validator_count=$(grep -c '"pubkey"' "$GENESIS_FILE" 2>/dev/null || echo 0)
        if [ "$validator_count" -ge "$NUM_VALIDATORS" ]; then
            ok "Genesis already configured with $validator_count validators"
            echo ""
            return 0
        fi
    fi

    # Collect validator public keys
    local validator_pubkeys=()
    for i in $(seq 1 $NUM_VALIDATORS); do
        local pub_file="$KEYS_DIR/validator-$i/node_key.pub"
        if [ ! -f "$pub_file" ]; then
            fail "Validator $i public key not found at $pub_file"
        fi
        local pubkey
        pubkey=$(xxd -p -c 64 "$pub_file" 2>/dev/null || od -An -tx1 "$pub_file" | tr -d ' \n')
        validator_pubkeys+=("$pubkey")
        info "Validator $i: ${pubkey:0:16}..."
    done

    # Faucet public key
    local faucet_pub_file="$KEYS_DIR/faucet/faucet_key.pub"
    local faucet_pubkey
    if [ -f "$faucet_pub_file" ]; then
        faucet_pubkey=$(xxd -p -c 64 "$faucet_pub_file" 2>/dev/null || od -An -tx1 "$faucet_pub_file" | tr -d ' \n')
    else
        faucet_pubkey="0000000000000000000000000000000000000000000000000000000000000001"
    fi
    info "Faucet:      ${faucet_pubkey:0:16}..."

    # Build validators JSON
    local validators_json="["
    for i in $(seq 0 $(( NUM_VALIDATORS - 1 ))); do
        if [ $i -gt 0 ]; then
            validators_json+=","
        fi
        validators_json+="
    {
      \"pubkey\": \"${validator_pubkeys[$i]}\",
      \"stake\": 1000000000000,
      \"label\": \"Validator $((i + 1))\"
    }"
    done
    validators_json+="
  ]"

    cat > "$GENESIS_FILE" <<EOF
{
  "chain_id": "$CHAIN_ID",
  "timestamp": $(date +%s),
  "validators": $validators_json,
  "initial_accounts": [
    {
      "address": "$faucet_pubkey",
      "balance": 1000000000000000,
      "label": "Testnet Faucet (1 Billion USDC)"
    }
  ]
}
EOF

    ok "Genesis written to $GENESIS_FILE"
    echo ""
}

# ---------------------------------------------------------------------------
# Step 5: Start Validator Nodes
# ---------------------------------------------------------------------------

start_node() {
    local name=$1
    local data=$2
    local p2p_port=$3
    local rpc_port=$4
    local rest_port=$5
    local is_validator=$6
    local key_file=${7:-""}
    local bootstrap_port=${8:-""}

    mkdir -p "$data"

    local args=(
        "--data-dir=$data"
        "--listen=/ip4/127.0.0.1/tcp/$p2p_port"
        "--rpc-port=$rpc_port"
        "--rest-port=$rest_port"
        "--chain-id=$CHAIN_ID"
        "--genesis=$GENESIS_FILE"
    )

    if [ "$is_validator" = "true" ]; then
        args+=("--validator" "--validator-key=$key_file")
    fi

    if [ -n "$bootstrap_port" ]; then
        args+=("--bootstrap=/ip4/127.0.0.1/tcp/$bootstrap_port")
    fi

    local log_file="$DATA_DIR/${name}.log"

    RUST_LOG="$RUST_LOG" "$DINA_NODE" "${args[@]}" \
        > "$log_file" 2>&1 &

    local pid=$!
    PIDS+=("$pid")
    SERVICES+=("$name")

    info "$name started (PID=$pid, P2P=:$p2p_port, RPC=:$rpc_port, REST=:$rest_port)"
}

start_validators() {
    step "Starting validator nodes..."

    # Clean data directory
    rm -rf "$DATA_DIR"
    mkdir -p "$DATA_DIR"

    # Validator 1 (seed node)
    start_node "validator-1" \
        "$DATA_DIR/validator-1" \
        $((BASE_P2P_PORT)) \
        $((BASE_RPC_PORT + 10)) \
        $((BASE_REST_PORT + 10)) \
        "true" \
        "$KEYS_DIR/validator-1/node_key"

    # Give the seed node time to bind
    sleep 2

    # Validator 2
    start_node "validator-2" \
        "$DATA_DIR/validator-2" \
        $((BASE_P2P_PORT + 1)) \
        $((BASE_RPC_PORT + 20)) \
        $((BASE_REST_PORT + 20)) \
        "true" \
        "$KEYS_DIR/validator-2/node_key" \
        "$BASE_P2P_PORT"

    # Validator 3
    start_node "validator-3" \
        "$DATA_DIR/validator-3" \
        $((BASE_P2P_PORT + 2)) \
        $((BASE_RPC_PORT + 30)) \
        $((BASE_REST_PORT + 30)) \
        "true" \
        "$KEYS_DIR/validator-3/node_key" \
        "$BASE_P2P_PORT"

    ok "3 validators started"
    echo ""
}

# ---------------------------------------------------------------------------
# Step 6: Start RPC Node
# ---------------------------------------------------------------------------

start_rpc_node() {
    step "Starting RPC node..."

    start_node "rpc-node" \
        "$DATA_DIR/rpc-node" \
        $((BASE_P2P_PORT + 3)) \
        $BASE_RPC_PORT \
        $BASE_REST_PORT \
        "false" \
        "" \
        "$BASE_P2P_PORT"

    ok "RPC node started (JSON-RPC=:$BASE_RPC_PORT, REST=:$BASE_REST_PORT)"
    echo ""
}

# ---------------------------------------------------------------------------
# Step 7: Start Faucet Server
# ---------------------------------------------------------------------------

start_faucet() {
    step "Starting faucet server..."

    local faucet_key="$KEYS_DIR/faucet/faucet_key"
    local faucet_log="$DATA_DIR/faucet.log"

    if [ ! -f "$faucet_key" ]; then
        warn "Faucet key not found — faucet will not be started"
        return 0
    fi

    # The faucet server is typically part of the dina-node binary or a standalone
    # If dina-node has a --faucet flag, use that. Otherwise we start it separately.
    # For now, we start the faucet as part of the RPC node's REST API.
    # The faucet endpoints are mounted at /faucet/* on the RPC node's REST port.

    # If there is a standalone faucet binary:
    local faucet_bin="$PROJECT_DIR/target/release/dina-faucet"
    if [ -f "$faucet_bin" ]; then
        "$faucet_bin" \
            --listen "0.0.0.0:$FAUCET_PORT" \
            --rpc-url "http://127.0.0.1:$BASE_RPC_PORT" \
            --faucet-key "$faucet_key" \
            --drip-amount 100000000 \
            --cooldown 60 \
            > "$faucet_log" 2>&1 &

        local pid=$!
        PIDS+=("$pid")
        SERVICES+=("faucet")
        ok "Faucet server started on port $FAUCET_PORT (PID=$pid)"
    else
        # Faucet is served as part of the RPC node REST API at /faucet/*
        ok "Faucet available at http://localhost:$BASE_REST_PORT/faucet/"
        info "Endpoints: POST /faucet/request, GET /faucet/status/:addr, GET /faucet/stats"
    fi

    echo ""
}

# ---------------------------------------------------------------------------
# Step 8: Start Explorer Backend
# ---------------------------------------------------------------------------

start_explorer() {
    step "Starting block explorer backend..."

    local explorer_bin="$PROJECT_DIR/target/release/dina-explorer"
    local explorer_log="$DATA_DIR/explorer.log"

    if [ -f "$explorer_bin" ]; then
        "$explorer_bin" \
            --listen "0.0.0.0:$EXPLORER_PORT" \
            --rpc-url "http://127.0.0.1:$BASE_RPC_PORT" \
            > "$explorer_log" 2>&1 &

        local pid=$!
        PIDS+=("$pid")
        SERVICES+=("explorer")
        ok "Explorer backend started on port $EXPLORER_PORT (PID=$pid)"
    else
        # Explorer API may be served as part of the RPC node
        ok "Explorer API available at http://localhost:$BASE_REST_PORT/api/"
        info "Endpoints: /api/blocks, /api/transactions/:hash, /api/accounts/:addr"
    fi

    echo ""
}

# ---------------------------------------------------------------------------
# Step 9: Wait for Health Checks
# ---------------------------------------------------------------------------

wait_for_health() {
    step "Waiting for nodes to become healthy..."

    local rpc_url="http://localhost:$BASE_REST_PORT/health"
    local retries=30

    for i in $(seq 1 $retries); do
        if curl -sf "$rpc_url" &>/dev/null; then
            ok "RPC node healthy after ${i}s"
            echo ""
            return 0
        fi
        sleep 1
    done

    warn "RPC node did not become healthy within ${retries}s"
    warn "The testnet may still be starting up. Check logs at $DATA_DIR/*.log"
    echo ""
}

# ---------------------------------------------------------------------------
# Step 10: Print Connection Info
# ---------------------------------------------------------------------------

print_info() {
    echo -e "${GREEN}${BOLD}"
    echo "  ============================================"
    echo "    Testnet Running!"
    echo "  ============================================"
    echo -e "${NC}"

    echo "  Validators:"
    for i in $(seq 1 $NUM_VALIDATORS); do
        local p2p=$((BASE_P2P_PORT + i - 1))
        local rpc=$((BASE_RPC_PORT + i * 10))
        local rest=$((BASE_REST_PORT + i * 10))
        echo "    [$i] P2P=:$p2p  RPC=:$rpc  REST=:$rest"
    done
    echo ""

    echo "  RPC Node:"
    echo "    JSON-RPC:  http://localhost:$BASE_RPC_PORT"
    echo "    REST API:  http://localhost:$BASE_REST_PORT"
    echo ""

    echo "  Faucet:"
    if [ -f "$PROJECT_DIR/target/release/dina-faucet" ]; then
        echo "    HTTP:      http://localhost:$FAUCET_PORT"
    else
        echo "    HTTP:      http://localhost:$BASE_REST_PORT/faucet/"
    fi
    echo ""

    echo "  Explorer:"
    if [ -f "$PROJECT_DIR/target/release/dina-explorer" ]; then
        echo "    API:       http://localhost:$EXPLORER_PORT"
    else
        echo "    API:       http://localhost:$BASE_REST_PORT/api/"
    fi
    echo ""

    echo "  Logs:        $DATA_DIR/*.log"
    echo "  Keys:        $KEYS_DIR/"
    echo "  Genesis:     $GENESIS_FILE"
    echo ""

    echo "  Example CLI commands:"
    echo "    $DINA_CLI --rpc-url http://localhost:$BASE_RPC_PORT status"
    echo "    $DINA_CLI --rpc-url http://localhost:$BASE_RPC_PORT balance <ADDRESS>"
    echo "    $DINA_CLI --rpc-url http://localhost:$BASE_RPC_PORT transfer <TO> <AMOUNT>"
    echo ""

    echo -e "  ${YELLOW}Press Ctrl+C to stop the testnet.${NC}"
    echo ""
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
    banner
    check_prerequisites
    build_binaries
    generate_keys
    create_genesis
    start_validators
    start_rpc_node
    start_faucet
    start_explorer
    wait_for_health
    print_info

    # Keep the script alive — wait for any child process to exit
    wait
}

main "$@"
