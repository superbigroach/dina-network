#!/bin/bash
# =============================================================================
# Dina Network — Docker Testnet Setup
# Builds Docker images and starts a complete testnet using Docker Compose,
# including validators, RPC node, faucet, and block explorer.
#
# Usage:  ./scripts/setup-testnet-docker.sh [--with-explorer] [--with-faucet-ui]
#
# Options:
#   --with-explorer      Also start the block explorer container
#   --with-faucet-ui     Also start the faucet web UI container
#   --rebuild             Force rebuild of Docker images
#   --detach              Run in detached mode (background)
#   --clean               Remove volumes and start fresh
#
# Press Ctrl+C to stop (unless --detach is used).
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
KEYS_DIR="$PROJECT_DIR/keys"
COMPOSE_FILE="$PROJECT_DIR/docker-compose.yml"
COMPOSE_OVERRIDE="$PROJECT_DIR/docker-compose.testnet.yml"

# Parse options
WITH_EXPLORER=false
WITH_FAUCET_UI=false
FORCE_REBUILD=false
DETACH=false
CLEAN=false

for arg in "$@"; do
    case "$arg" in
        --with-explorer)   WITH_EXPLORER=true ;;
        --with-faucet-ui)  WITH_FAUCET_UI=true ;;
        --rebuild)         FORCE_REBUILD=true ;;
        --detach)          DETACH=true ;;
        --clean)           CLEAN=true ;;
        -h|--help)
            echo "Usage: $0 [--with-explorer] [--with-faucet-ui] [--rebuild] [--detach] [--clean]"
            exit 0
            ;;
        *)
            echo "Unknown option: $arg"
            exit 1
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Colors
# ---------------------------------------------------------------------------

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

step() { echo -e "${BLUE}${BOLD}[STEP]${NC} $1"; }
ok()   { echo -e "${GREEN}  [OK]${NC} $1"; }
warn() { echo -e "${YELLOW}  [WARN]${NC} $1"; }
fail() { echo -e "${RED}  [FAIL]${NC} $1"; exit 1; }
info() { echo -e "  ${CYAN}-->  ${NC}$1"; }

# ---------------------------------------------------------------------------
# Banner
# ---------------------------------------------------------------------------

echo ""
echo -e "${CYAN}${BOLD}"
echo "  ============================================"
echo "    Dina Network — Docker Testnet Setup"
echo "  ============================================"
echo -e "${NC}"
echo "  Project:     $PROJECT_DIR"
echo "  Explorer:    $WITH_EXPLORER"
echo "  Faucet UI:   $WITH_FAUCET_UI"
echo ""

# ---------------------------------------------------------------------------
# Step 1: Check Prerequisites
# ---------------------------------------------------------------------------

step "Checking prerequisites..."

if command -v docker &>/dev/null; then
    docker_version=$(docker --version | awk '{print $3}' | tr -d ',')
    ok "Docker $docker_version"
else
    fail "Docker not found. Install from https://docs.docker.com/get-docker/"
fi

# Check for docker compose (v2) or docker-compose (v1)
if docker compose version &>/dev/null 2>&1; then
    COMPOSE_CMD="docker compose"
    ok "Docker Compose v2"
elif command -v docker-compose &>/dev/null; then
    COMPOSE_CMD="docker-compose"
    ok "Docker Compose v1"
else
    fail "Docker Compose not found"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 2: Generate Keys (if needed)
# ---------------------------------------------------------------------------

step "Checking validator keys..."

need_keys=false
for i in 1 2 3; do
    if [ ! -f "$KEYS_DIR/validator-$i/node_key" ]; then
        need_keys=true
        break
    fi
done

if $need_keys; then
    info "Keys not found — generating via generate-keys.sh..."
    info "Note: This requires the dina CLI to be built locally."

    # Try to generate keys using the script
    if [ -f "$SCRIPT_DIR/generate-keys.sh" ]; then
        # First ensure the CLI binary exists
        if [ ! -f "$PROJECT_DIR/target/release/dina" ]; then
            info "Building dina CLI for key generation..."
            cd "$PROJECT_DIR"
            cargo build --release --bin dina 2>&1 | tail -3
        fi
        export PATH="$PROJECT_DIR/target/release:$PATH"
        bash "$SCRIPT_DIR/generate-keys.sh"
    else
        fail "generate-keys.sh not found and keys are missing"
    fi
else
    ok "All validator keys present"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 3: Generate docker-compose override for extras
# ---------------------------------------------------------------------------

step "Generating Docker Compose configuration..."

# Build the override file with optional services
cat > "$COMPOSE_OVERRIDE" <<'YAML_HEAD'
# Auto-generated by setup-testnet-docker.sh — do not edit manually.
# This file extends docker-compose.yml with optional services.

services:
YAML_HEAD

if $WITH_FAUCET_UI; then
    cat >> "$COMPOSE_OVERRIDE" <<'YAML_FAUCET'
  # -------------------------------------------------------------------------
  # Faucet Web UI (static nginx container)
  # -------------------------------------------------------------------------
  faucet-ui:
    build:
      context: ./faucet-app
      dockerfile: Dockerfile
    container_name: dina-faucet-ui
    hostname: faucet-ui
    networks:
      - dina-net
    ports:
      - "3000:80"
    depends_on:
      rpc-node:
        condition: service_healthy
    restart: unless-stopped
    labels:
      - "dina.service=faucet-ui"

YAML_FAUCET
    ok "Faucet UI service added (port 3000)"
fi

if $WITH_EXPLORER; then
    cat >> "$COMPOSE_OVERRIDE" <<'YAML_EXPLORER'
  # -------------------------------------------------------------------------
  # Block Explorer Backend
  # -------------------------------------------------------------------------
  explorer:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: dina-explorer
    hostname: explorer
    command:
      - "--data-dir=/data"
      - "--listen=/ip4/0.0.0.0/tcp/9944"
      - "--rpc-port=8545"
      - "--rest-port=8080"
      - "--chain-id=dina-local-testnet"
      - "--bootstrap=/ip4/validator-1/tcp/9944"
      - "--explorer"
      - "--explorer-port=3001"
    volumes:
      - explorer-data:/data
    networks:
      - dina-net
    environment:
      - RUST_LOG=info,dina_explorer=debug
      - DINA_CHAIN_ID=dina-local-testnet
    ports:
      - "3001:3001"
    depends_on:
      rpc-node:
        condition: service_healthy
    restart: unless-stopped
    labels:
      - "dina.service=explorer"

YAML_EXPLORER
    ok "Explorer service added (port 3001)"
fi

# If neither extra service was added, add a placeholder so the file is valid
if ! $WITH_FAUCET_UI && ! $WITH_EXPLORER; then
    # docker compose requires at least one service definition to be valid
    # We use an empty override by removing the file and not passing it
    rm -f "$COMPOSE_OVERRIDE"
    ok "Base configuration only (no extras)"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 4: Clean (optional)
# ---------------------------------------------------------------------------

if $CLEAN; then
    step "Cleaning old volumes and containers..."
    $COMPOSE_CMD -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || true
    if [ -f "$COMPOSE_OVERRIDE" ]; then
        $COMPOSE_CMD -f "$COMPOSE_FILE" -f "$COMPOSE_OVERRIDE" down -v --remove-orphans 2>/dev/null || true
    fi
    ok "Old data removed"
    echo ""
fi

# ---------------------------------------------------------------------------
# Step 5: Build Docker Images
# ---------------------------------------------------------------------------

step "Building Docker images..."

COMPOSE_FILES="-f $COMPOSE_FILE"
if [ -f "$COMPOSE_OVERRIDE" ]; then
    COMPOSE_FILES="$COMPOSE_FILES -f $COMPOSE_OVERRIDE"
fi

BUILD_ARGS=""
if $FORCE_REBUILD; then
    BUILD_ARGS="--no-cache"
fi

$COMPOSE_CMD $COMPOSE_FILES build $BUILD_ARGS 2>&1 | tail -10

ok "Docker images built"
echo ""

# ---------------------------------------------------------------------------
# Step 6: Start Services
# ---------------------------------------------------------------------------

step "Starting testnet services..."

UP_ARGS=""
if $DETACH; then
    UP_ARGS="-d"
fi

$COMPOSE_CMD $COMPOSE_FILES up $UP_ARGS --remove-orphans &
COMPOSE_PID=$!

# In detached mode, wait for health checks
if $DETACH; then
    wait $COMPOSE_PID 2>/dev/null || true

    step "Waiting for health checks..."
    local retries=60
    for i in $(seq 1 $retries); do
        if curl -sf "http://localhost:8080/health" &>/dev/null; then
            ok "RPC node healthy after ${i}s"
            break
        fi
        if [ $i -eq $retries ]; then
            warn "RPC node did not become healthy within ${retries}s"
        fi
        sleep 1
    done
fi

echo ""

# ---------------------------------------------------------------------------
# Step 7: Print Connection Info
# ---------------------------------------------------------------------------

echo -e "${GREEN}${BOLD}"
echo "  ============================================"
echo "    Docker Testnet Running!"
echo "  ============================================"
echo -e "${NC}"

echo "  Validators: 3 (dina-validator-1, -2, -3)"
echo ""
echo "  RPC Node:"
echo "    JSON-RPC:   http://localhost:8545"
echo "    REST API:   http://localhost:8080"
echo "    Health:     http://localhost:8080/health"
echo ""
echo "  Faucet API:   http://localhost:8080/faucet/"

if $WITH_FAUCET_UI; then
    echo "  Faucet UI:    http://localhost:3000"
fi

if $WITH_EXPLORER; then
    echo "  Explorer:     http://localhost:3001"
fi

echo ""
echo "  Management:"
echo "    Logs:       $COMPOSE_CMD $COMPOSE_FILES logs -f"
echo "    Stop:       $COMPOSE_CMD $COMPOSE_FILES down"
echo "    Clean:      $COMPOSE_CMD $COMPOSE_FILES down -v"
echo ""

if ! $DETACH; then
    echo -e "  ${YELLOW}Press Ctrl+C to stop the testnet.${NC}"
    echo ""
    # Wait for docker compose to exit
    wait $COMPOSE_PID 2>/dev/null || true
fi
