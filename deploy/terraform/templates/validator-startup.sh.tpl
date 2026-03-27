#!/bin/bash
# =============================================================================
# Dina Network — Validator VM Startup Script
# Runs on Container-Optimized OS at boot.
#
# This script:
#   1. Formats and mounts the persistent chain-data disk
#   2. Fetches the validator signing key from Secret Manager
#   3. Pulls and starts the dina-node Docker container
#   4. Configures auto-restart via Docker's restart policy
# =============================================================================

set -euo pipefail

VALIDATOR_INDEX="${validator_index}"
CHAIN_ID="${chain_id}"
P2P_PORT="${p2p_port}"
RPC_PORT="${rpc_port}"
REST_PORT="${rest_port}"
NODE_IMAGE="${node_image}"
PROJECT_ID="${project_id}"
BOOTSTRAP_ADDR="${bootstrap_addr}"

DATA_DIR="/mnt/chain-data"
KEY_DIR="/mnt/keys"

echo "[dina-startup] Validator $VALIDATOR_INDEX starting up..."

# ---- Step 1: Mount the persistent chain-data disk --------------------------

DEVICE="/dev/disk/by-id/google-chain-data"

# Format only if not already formatted (first boot)
if ! blkid "$DEVICE" &>/dev/null; then
  echo "[dina-startup] Formatting chain-data disk..."
  mkfs.ext4 -F "$DEVICE"
fi

mkdir -p "$DATA_DIR"
mount -o discard,defaults "$DEVICE" "$DATA_DIR"
chmod 755 "$DATA_DIR"

echo "[dina-startup] Chain data disk mounted at $DATA_DIR"

# ---- Step 2: Fetch validator key from Secret Manager -----------------------

mkdir -p "$KEY_DIR"

# Use the metadata server to get an access token for the service account
ACCESS_TOKEN=$(curl -sf -H "Metadata-Flavor: Google" \
  "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token" \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])")

SECRET_NAME="dina-validator-key-$VALIDATOR_INDEX"
SECRET_URL="https://secretmanager.googleapis.com/v1/projects/$PROJECT_ID/secrets/$SECRET_NAME/versions/latest:access"

echo "[dina-startup] Fetching validator key from Secret Manager..."
curl -sf -H "Authorization: Bearer $ACCESS_TOKEN" "$SECRET_URL" \
  | python3 -c "import sys,json,base64; print(base64.b64decode(json.load(sys.stdin)['payload']['data']).decode())" \
  > "$KEY_DIR/node_key"

chmod 600 "$KEY_DIR/node_key"
echo "[dina-startup] Validator key written to $KEY_DIR/node_key"

# ---- Step 3: Build the dina-node Docker run command ------------------------

# Authenticate Docker with GCR
docker-credential-gcr configure-docker --registries gcr.io 2>/dev/null || true

echo "[dina-startup] Pulling image $NODE_IMAGE..."
docker pull "$NODE_IMAGE"

# Stop any existing container from a previous boot
docker rm -f dina-validator 2>/dev/null || true

DOCKER_ARGS=(
  "--name" "dina-validator"
  "--restart" "always"
  "--network" "host"
  "-v" "$DATA_DIR:/data"
  "-v" "$KEY_DIR:/keys:ro"
  "-e" "RUST_LOG=info,dina_consensus=debug,dina_network=debug"
  "-e" "DINA_CHAIN_ID=$CHAIN_ID"
  "-e" "DINA_VALIDATOR_INDEX=$VALIDATOR_INDEX"
)

NODE_ARGS=(
  "--data-dir" "/data"
  "--listen" "/ip4/0.0.0.0/tcp/$P2P_PORT"
  "--rpc-port" "$RPC_PORT"
  "--rest-port" "$REST_PORT"
  "--validator"
  "--validator-key" "/keys/node_key"
  "--chain-id" "$CHAIN_ID"
)

# Add bootstrap address if this is not the seed validator
if [ -n "$BOOTSTRAP_ADDR" ]; then
  NODE_ARGS+=("--bootstrap" "$BOOTSTRAP_ADDR")
fi

echo "[dina-startup] Starting dina-node container..."
docker run -d "$${DOCKER_ARGS[@]}" "$NODE_IMAGE" "$${NODE_ARGS[@]}"

echo "[dina-startup] Validator $VALIDATOR_INDEX is running."

# ---- Step 4: Wait for health and log success --------------------------------

for i in $(seq 1 30); do
  if curl -sf "http://localhost:$REST_PORT/health" >/dev/null 2>&1; then
    echo "[dina-startup] Health check passed after $${i}s"
    break
  fi
  sleep 1
done

echo "[dina-startup] Startup complete."
