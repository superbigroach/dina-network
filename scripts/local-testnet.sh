#!/bin/bash
# =============================================================================
# Dina Network — Local Testnet (without Docker)
# Starts 3 validator nodes + 1 RPC node as background processes.
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
KEYS_DIR="$PROJECT_DIR/keys"
DATA_DIR="$PROJECT_DIR/.local-testnet"
DINA_NODE="${DINA_NODE:-$PROJECT_DIR/target/release/dina-node}"
CHAIN_ID="${CHAIN_ID:-dina-local-testnet}"
GENESIS_FILE="$PROJECT_DIR/genesis.json"

# Base ports — each validator offsets from these
BASE_P2P_PORT=9944
BASE_RPC_PORT=8545
BASE_REST_PORT=8080

PIDS=()

# ---------------------------------------------------------------------------
# Cleanup on exit
# ---------------------------------------------------------------------------
cleanup() {
    echo ""
    echo "Shutting down local testnet..."

    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
            echo "  Stopped process $pid"
        fi
    done

    echo "Local testnet stopped."
}

trap cleanup EXIT INT TERM

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
log() { echo "[$(date '+%H:%M:%S')] $*"; }

wait_for_port() {
    local PORT=$1
    local NAME=$2
    local RETRIES=30

    for i in $(seq 1 $RETRIES); do
        if curl -sf "http://localhost:${PORT}/health" &>/dev/null; then
            return 0
        fi
        sleep 1
    done

    echo "Warning: $NAME on port $PORT did not become healthy within ${RETRIES}s"
    return 1
}

# ---------------------------------------------------------------------------
# Prerequisites
# ---------------------------------------------------------------------------
check_prerequisites() {
    if [ ! -f "$DINA_NODE" ]; then
        log "dina-node binary not found at $DINA_NODE"
        log "Building in release mode..."
        cd "$PROJECT_DIR"
        cargo build --release --bin dina-node
        log "Build complete."
    fi
}

# ---------------------------------------------------------------------------
# Generate keys if they don't exist
# ---------------------------------------------------------------------------
ensure_keys() {
    local NEED_KEYS=false
    for i in 1 2 3; do
        if [ ! -f "$KEYS_DIR/validator-$i/node_key" ]; then
            NEED_KEYS=true
            break
        fi
    done

    if $NEED_KEYS; then
        log "Validator keys not found — generating..."
        bash "$SCRIPT_DIR/generate-keys.sh"
    else
        log "Validator keys found."
    fi
}

# ---------------------------------------------------------------------------
# Start a node
# ---------------------------------------------------------------------------
start_node() {
    local NAME=$1
    local DATA=$2
    local P2P_PORT=$3
    local RPC_PORT=$4
    local REST_PORT=$5
    local IS_VALIDATOR=$6
    local KEY_FILE=${7:-""}
    local BOOTSTRAP_PORT=${8:-""}

    mkdir -p "$DATA"

    local ARGS=(
        "--data-dir=$DATA"
        "--listen=/ip4/127.0.0.1/tcp/$P2P_PORT"
        "--rpc-port=$RPC_PORT"
        "--rest-port=$REST_PORT"
        "--chain-id=$CHAIN_ID"
    )

    if [ "$IS_VALIDATOR" = "true" ]; then
        ARGS+=("--validator" "--validator-key=$KEY_FILE")
    fi

    if [ -n "$BOOTSTRAP_PORT" ]; then
        ARGS+=("--bootstrap=/ip4/127.0.0.1/tcp/$BOOTSTRAP_PORT")
    fi

    local LOG_FILE="$DATA_DIR/${NAME}.log"

    log "Starting $NAME (P2P=$P2P_PORT, RPC=$RPC_PORT, REST=$REST_PORT)..."

    RUST_LOG="${RUST_LOG:-info}" "$DINA_NODE" "${ARGS[@]}" \
        > "$LOG_FILE" 2>&1 &

    local PID=$!
    PIDS+=("$PID")

    log "  $NAME started (PID=$PID, log=$LOG_FILE)"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    echo "============================================"
    echo "  Dina Network — Local Testnet"
    echo "============================================"
    echo ""

    check_prerequisites
    ensure_keys

    # Create data directories
    rm -rf "$DATA_DIR"
    mkdir -p "$DATA_DIR"

    # Copy genesis file into data dirs
    log "Using genesis: $GENESIS_FILE"
    echo ""

    # Start Validator 1 (seed node)
    start_node "validator-1" \
        "$DATA_DIR/validator-1" \
        $((BASE_P2P_PORT)) \
        $((BASE_RPC_PORT + 10)) \
        $((BASE_REST_PORT + 10)) \
        "true" \
        "$KEYS_DIR/validator-1/node_key"

    # Give validator-1 a moment to bind its port
    sleep 2

    # Start Validator 2
    start_node "validator-2" \
        "$DATA_DIR/validator-2" \
        $((BASE_P2P_PORT + 1)) \
        $((BASE_RPC_PORT + 20)) \
        $((BASE_REST_PORT + 20)) \
        "true" \
        "$KEYS_DIR/validator-2/node_key" \
        "$BASE_P2P_PORT"

    # Start Validator 3
    start_node "validator-3" \
        "$DATA_DIR/validator-3" \
        $((BASE_P2P_PORT + 2)) \
        $((BASE_RPC_PORT + 30)) \
        $((BASE_REST_PORT + 30)) \
        "true" \
        "$KEYS_DIR/validator-3/node_key" \
        "$BASE_P2P_PORT"

    # Start RPC Node (non-validator)
    start_node "rpc-node" \
        "$DATA_DIR/rpc-node" \
        $((BASE_P2P_PORT + 3)) \
        $BASE_RPC_PORT \
        $BASE_REST_PORT \
        "false" \
        "" \
        "$BASE_P2P_PORT"

    echo ""
    echo "============================================"
    echo "  Local Testnet Running"
    echo "============================================"
    echo ""
    echo "  Validators:"
    echo "    [1] P2P=:$((BASE_P2P_PORT))     RPC=:$((BASE_RPC_PORT + 10))  REST=:$((BASE_REST_PORT + 10))"
    echo "    [2] P2P=:$((BASE_P2P_PORT + 1))  RPC=:$((BASE_RPC_PORT + 20))  REST=:$((BASE_REST_PORT + 20))"
    echo "    [3] P2P=:$((BASE_P2P_PORT + 2))  RPC=:$((BASE_RPC_PORT + 30))  REST=:$((BASE_REST_PORT + 30))"
    echo ""
    echo "  RPC Node:"
    echo "    JSON-RPC: http://localhost:$BASE_RPC_PORT"
    echo "    REST API: http://localhost:$BASE_REST_PORT"
    echo ""
    echo "  Logs: $DATA_DIR/*.log"
    echo ""
    echo "  Example commands:"
    echo "    dina --rpc-url http://localhost:$BASE_RPC_PORT status"
    echo "    dina --rpc-url http://localhost:$BASE_RPC_PORT balance <address>"
    echo ""
    echo "  Press Ctrl+C to stop all nodes."
    echo ""

    # Wait for any child to exit (keeps script alive for the trap)
    wait
}

main "$@"
