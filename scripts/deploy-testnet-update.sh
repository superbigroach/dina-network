#!/bin/bash
# Dina Network — Testnet Update Deployment
# Builds new node binary and all contracts, then deploys to testnet.
#
# Usage: ./scripts/deploy-testnet-update.sh [--skip-build] [--contracts-only]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
RPC_URL="${DINA_RPC_URL:-http://35.184.213.248:8545}"
REST_URL="${DINA_REST_URL:-http://35.184.213.248:8080}"

SKIP_BUILD=false
CONTRACTS_ONLY=false

for arg in "$@"; do
  case "$arg" in
    --skip-build) SKIP_BUILD=true ;;
    --contracts-only) CONTRACTS_ONLY=true ;;
    --help|-h)
      echo "Usage: $0 [--skip-build] [--contracts-only]"
      echo "  --skip-build      Skip Rust compilation, just deploy"
      echo "  --contracts-only  Only build contracts, skip the node binary"
      exit 0
      ;;
  esac
done

echo "============================================"
echo "  DINA NETWORK — TESTNET UPDATE"
echo "============================================"
echo ""
echo "  RPC:  $RPC_URL"
echo "  REST: $REST_URL"
echo "  Root: $ROOT_DIR"
echo ""

# ── Step 1: Check testnet status ──
echo "1. CHECKING TESTNET STATUS"
echo "────────────────────────────"
if curl -sf "${REST_URL}/health" -o /tmp/dina-health.json 2>/dev/null; then
  python3 -m json.tool /tmp/dina-health.json 2>/dev/null || cat /tmp/dina-health.json
  echo ""
else
  echo "  Testnet unreachable at $REST_URL"
  echo "  (Proceeding with build anyway)"
  echo ""
fi

# ── Step 2: Build node binary ──
if [ "$SKIP_BUILD" = false ] && [ "$CONTRACTS_ONLY" = false ]; then
  echo "2. BUILDING NODE BINARY"
  echo "────────────────────────────"
  cd "$ROOT_DIR"
  echo "  Running: cargo build --release"
  echo ""
  if cargo build --release 2>&1 | tail -10; then
    NODE_BIN="$ROOT_DIR/target/release/dina-node"
    if [ -f "$NODE_BIN" ]; then
      NODE_SIZE=$(ls -lh "$NODE_BIN" | awk '{print $5}')
      echo ""
      echo "  Binary: $NODE_BIN ($NODE_SIZE)"
    else
      # On Windows the extension may differ
      NODE_BIN="$ROOT_DIR/target/release/dina-node.exe"
      if [ -f "$NODE_BIN" ]; then
        NODE_SIZE=$(ls -lh "$NODE_BIN" | awk '{print $5}')
        echo ""
        echo "  Binary: $NODE_BIN ($NODE_SIZE)"
      else
        echo "  WARNING: Binary not found at expected path"
      fi
    fi
  else
    echo "  WARNING: cargo build failed (continuing with contracts)"
  fi
  echo ""
else
  echo "2. SKIPPING NODE BINARY BUILD"
  echo ""
fi

# ── Step 3: Build all contracts to WASM ──
echo "3. BUILDING CONTRACTS TO WASM"
echo "────────────────────────────"

cd "$ROOT_DIR"
CONTRACTS_BUILT=0
CONTRACTS_FAILED=0
CONTRACTS_TOTAL=0

for dir in contracts/*/; do
  if [ -f "$dir/Cargo.toml" ]; then
    CONTRACTS_TOTAL=$((CONTRACTS_TOTAL + 1))
    name=$(basename "$dir")

    if [ "$SKIP_BUILD" = true ]; then
      echo "  [skip] $name"
      continue
    fi

    printf "  %-40s " "$name"
    if cargo build --manifest-path "$dir/Cargo.toml" --target wasm32-unknown-unknown --release 2>/dev/null; then
      echo "OK"
      CONTRACTS_BUILT=$((CONTRACTS_BUILT + 1))
    else
      echo "FAILED"
      CONTRACTS_FAILED=$((CONTRACTS_FAILED + 1))
    fi
  fi
done

echo ""
echo "  Total:   $CONTRACTS_TOTAL contracts"
echo "  Built:   $CONTRACTS_BUILT"
echo "  Failed:  $CONTRACTS_FAILED"
echo "  Skipped: $((CONTRACTS_TOTAL - CONTRACTS_BUILT - CONTRACTS_FAILED))"
echo ""

# ── Step 4: List WASM artifacts ──
echo "4. WASM ARTIFACTS"
echo "────────────────────────────"

WASM_DIR="$ROOT_DIR/target/wasm32-unknown-unknown/release"
if [ -d "$WASM_DIR" ]; then
  WASM_COUNT=0
  WASM_TOTAL_SIZE=0

  for wasm in "$WASM_DIR"/*.wasm; do
    if [ -f "$wasm" ]; then
      WASM_COUNT=$((WASM_COUNT + 1))
      SIZE=$(stat -f%z "$wasm" 2>/dev/null || stat --printf="%s" "$wasm" 2>/dev/null || echo "0")
      SIZE_KB=$((SIZE / 1024))
      WASM_TOTAL_SIZE=$((WASM_TOTAL_SIZE + SIZE))
      printf "  %-50s %6d KB\n" "$(basename "$wasm")" "$SIZE_KB"
    fi
  done

  if [ "$WASM_COUNT" -eq 0 ]; then
    echo "  No .wasm files found"
  else
    echo ""
    TOTAL_KB=$((WASM_TOTAL_SIZE / 1024))
    echo "  $WASM_COUNT artifacts, ${TOTAL_KB} KB total"
  fi
else
  echo "  WASM output directory not found: $WASM_DIR"
  echo "  (This is expected if contracts have not been built yet)"
fi
echo ""

# ── Step 5: Show what changed ──
echo "5. CHANGES SINCE LAST TAG/DEPLOY"
echo "────────────────────────────────"

cd "$ROOT_DIR"

# Find the last testnet deploy tag
LAST_TAG=$(git tag -l 'testnet-*' --sort=-creatordate 2>/dev/null | head -1)

if [ -n "$LAST_TAG" ]; then
  echo "  Last deploy tag: $LAST_TAG"
  echo ""
  echo "  Commits since $LAST_TAG:"
  git log --oneline "$LAST_TAG"..HEAD 2>/dev/null | head -20 | sed 's/^/    /'
  echo ""
  echo "  Files changed:"
  git diff --stat "$LAST_TAG"..HEAD 2>/dev/null | tail -5 | sed 's/^/    /'
else
  echo "  No testnet-* tags found. Showing last 10 commits:"
  git log --oneline -10 2>/dev/null | sed 's/^/    /'
fi
echo ""

# Tag this deploy
DEPLOY_TAG="testnet-$(date +%Y%m%d-%H%M%S)"
echo "  Creating tag: $DEPLOY_TAG"
git tag "$DEPLOY_TAG" 2>/dev/null || echo "  (tag creation failed — read-only or detached HEAD)"
echo ""

# ── Step 6: Validator update instructions ──
echo "6. VALIDATOR UPDATE INSTRUCTIONS"
echo "────────────────────────────────"
echo ""
echo "  ROLLING RESTART (zero downtime)"
echo "  ─────────────────────────────────"
echo "  For each validator VM, one at a time:"
echo ""
echo "    # 1. Copy the new binary"
echo "    gcloud compute scp target/release/dina-node <VM_NAME>:/tmp/dina-node-new"
echo ""
echo "    # 2. SSH in and swap + restart"
echo "    gcloud compute ssh <VM_NAME> -- bash -c '"
echo "      sudo systemctl stop dina-node"
echo "      sudo cp /tmp/dina-node-new /usr/local/bin/dina-node"
echo "      sudo chmod +x /usr/local/bin/dina-node"
echo "      sudo systemctl start dina-node"
echo "    '"
echo ""
echo "    # 3. Wait and verify"
echo "    sleep 10"
echo "    gcloud compute ssh <VM_NAME> -- 'curl -s http://localhost:8080/health'"
echo ""
echo "    # 4. Confirm block production before moving to next validator"
echo "    sleep 5"
echo "    gcloud compute ssh <VM_NAME> -- 'curl -s http://localhost:8080/health | python3 -m json.tool'"
echo ""
echo ""
echo "  CONTRACT DEPLOYMENT"
echo "  ─────────────────────────────────"
echo "  After validators are updated, deploy stablecoins:"
echo ""
echo "    node scripts/deploy-stablecoins.js"
echo ""
echo "  Or with a specific key:"
echo ""
echo "    DINA_DEPLOY_KEY=<hex> node scripts/deploy-stablecoins.js"
echo ""
echo ""
echo "  QUICK STATUS CHECK"
echo "  ─────────────────────────────────"
echo ""
echo "    node scripts/testnet-status.js"
echo ""
echo "============================================"
echo "  BUILD COMPLETE — $(date '+%Y-%m-%d %H:%M:%S')"
echo "============================================"
