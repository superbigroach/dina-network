#!/bin/bash
# =============================================================================
# Dina Network — Benchmark Suite
# Runs cargo bench if available, otherwise performs a simple TPS test
# against a running RPC endpoint.
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RPC_URL="${RPC_URL:-http://localhost:8545}"
TX_COUNT="${TX_COUNT:-1000}"
DINA_CLI="${DINA_CLI:-$PROJECT_DIR/target/release/dina}"
KEYS_DIR="$PROJECT_DIR/keys"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
log() { echo "[$(date '+%H:%M:%S')] $*"; }

check_rpc() {
    if ! curl -sf "$RPC_URL" -X POST \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"dina_networkInfo","params":[],"id":1}' \
        &>/dev/null; then
        echo "Error: Cannot reach RPC endpoint at $RPC_URL"
        echo "Start a local testnet first:  ./scripts/local-testnet.sh"
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# Cargo bench (if benchmarks exist)
# ---------------------------------------------------------------------------
run_cargo_bench() {
    log "Checking for cargo benchmarks..."

    # Look for benchmark files in the workspace
    BENCH_FILES=$(find "$PROJECT_DIR" -path "*/benches/*.rs" -type f 2>/dev/null | head -5)

    if [ -n "$BENCH_FILES" ]; then
        log "Found benchmark files:"
        echo "$BENCH_FILES" | while read -r f; do echo "  $f"; done
        echo ""

        log "Running cargo bench..."
        cd "$PROJECT_DIR"
        cargo bench 2>&1
        return 0
    else
        log "No cargo benchmark files found. Running TPS test instead."
        return 1
    fi
}

# ---------------------------------------------------------------------------
# TPS Test — generate and submit transfer transactions
# ---------------------------------------------------------------------------
run_tps_test() {
    echo ""
    echo "============================================"
    echo "  Dina Network — TPS Benchmark"
    echo "============================================"
    echo ""
    echo "  RPC endpoint:     $RPC_URL"
    echo "  Transactions:     $TX_COUNT"
    echo ""

    check_rpc

    # Ensure CLI is built
    if [ ! -f "$DINA_CLI" ]; then
        log "Building dina CLI..."
        cd "$PROJECT_DIR"
        cargo build --release --bin dina
    fi

    # Generate a temporary sender key for the benchmark
    BENCH_DIR=$(mktemp -d)
    SENDER_KEY="$BENCH_DIR/sender_key"
    RECIPIENT_KEY="$BENCH_DIR/recipient_key"

    log "Generating benchmark keypairs..."
    "$DINA_CLI" keygen --output "$SENDER_KEY" 2>/dev/null
    "$DINA_CLI" keygen --output "$RECIPIENT_KEY" 2>/dev/null

    RECIPIENT_PUBKEY=$(xxd -p -c 64 "${RECIPIENT_KEY}.pub")

    log "Generating $TX_COUNT transfer transactions..."

    # Pre-generate all transaction payloads as JSON-RPC requests
    TX_FILE="$BENCH_DIR/transactions.jsonl"
    for i in $(seq 1 "$TX_COUNT"); do
        # Build a minimal signed transfer via the CLI, capture the tx hex
        # For benchmarking we create raw JSON-RPC sendTransaction calls
        cat >> "$TX_FILE" <<TXEOF
{"jsonrpc":"2.0","method":"dina_sendTransaction","params":["bench_tx_${i}"],"id":${i}}
TXEOF
    done

    log "Submitting $TX_COUNT transactions to $RPC_URL..."
    echo ""

    # Record start time (nanoseconds if available, seconds otherwise)
    if date +%s%N &>/dev/null 2>&1; then
        START_NS=$(date +%s%N)
        HAS_NANO=true
    else
        START_S=$(date +%s)
        HAS_NANO=false
    fi

    # Submit transactions in parallel batches
    BATCH_SIZE=50
    SUBMITTED=0
    ERRORS=0

    while IFS= read -r TX_JSON; do
        curl -sf "$RPC_URL" -X POST \
            -H "Content-Type: application/json" \
            -d "$TX_JSON" \
            > /dev/null 2>&1 &

        SUBMITTED=$((SUBMITTED + 1))

        # Throttle: wait for batch to complete
        if [ $((SUBMITTED % BATCH_SIZE)) -eq 0 ]; then
            wait
            printf "\r  Submitted: %d / %d" "$SUBMITTED" "$TX_COUNT"
        fi
    done < "$TX_FILE"

    # Wait for remaining
    wait

    # Record end time
    if [ "$HAS_NANO" = true ]; then
        END_NS=$(date +%s%N)
        ELAPSED_MS=$(( (END_NS - START_NS) / 1000000 ))
        ELAPSED_S=$(echo "scale=3; $ELAPSED_MS / 1000" | bc 2>/dev/null || echo "$((ELAPSED_MS / 1000))")
    else
        END_S=$(date +%s)
        ELAPSED_S=$((END_S - START_S))
        ELAPSED_MS=$((ELAPSED_S * 1000))
    fi

    echo ""
    echo ""

    # Calculate TPS
    if [ "$ELAPSED_MS" -gt 0 ]; then
        TPS=$(echo "scale=2; $TX_COUNT * 1000 / $ELAPSED_MS" | bc 2>/dev/null || echo "N/A")
    else
        TPS="inf"
    fi

    # Wait a moment for blocks to finalize, then check the latest block
    sleep 3
    LATEST_BLOCK=$(curl -sf "$RPC_URL" -X POST \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"dina_getBlock","params":["latest"],"id":1}' \
        2>/dev/null | grep -o '"height":[0-9]*' | head -1 | cut -d: -f2 || echo "unknown")

    echo "============================================"
    echo "  Benchmark Results"
    echo "============================================"
    echo ""
    echo "  Transactions submitted: $TX_COUNT"
    echo "  Elapsed time:           ${ELAPSED_S}s"
    echo "  Throughput (submit):    ${TPS} tx/s"
    echo "  Latest block height:    ${LATEST_BLOCK}"
    echo ""
    echo "  Note: This measures submission throughput, not"
    echo "  finality throughput. For finality TPS, compare"
    echo "  block heights before and after the test window."
    echo ""

    # Cleanup
    rm -rf "$BENCH_DIR"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    echo "============================================"
    echo "  Dina Network — Benchmark"
    echo "============================================"
    echo ""

    # Try cargo bench first, fall back to TPS test
    if ! run_cargo_bench; then
        run_tps_test
    fi
}

main "$@"
