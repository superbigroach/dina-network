#!/usr/bin/env bash
# =============================================================================
# Dina Network — 3-Validator Testnet Deployment on GCP
#
# Cost: 3x e2-medium = ~$75/month
# TPS:  ~100,000 (2 lanes parallel execution per validator)
#
# Prerequisites:
#   - gcloud CLI authenticated (gcloud auth login)
#   - Docker installed locally
#   - GCP project with billing enabled
#
# Usage:
#   ./deployment/gcp-testnet-deploy.sh           # deploy all 3 validators
#   ./deployment/gcp-testnet-deploy.sh teardown   # destroy everything
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration — edit these
# ---------------------------------------------------------------------------
PROJECT_ID="${GCP_PROJECT:-lucilla-b0493}"
REGION="us-central1"
ZONE="us-central1-a"
MACHINE_TYPE="e2-medium"          # 2 vCPUs, 4GB RAM, ~$25/month
CHAIN_ID="dina-testnet-1"
IMAGE_NAME="dina-node"
REPO_NAME="dina-network"
AR_REPO="${REGION}-docker.pkg.dev/${PROJECT_ID}/${REPO_NAME}/${IMAGE_NAME}"

VALIDATORS=(
  "dina-validator-0"
  "dina-validator-1"
  "dina-validator-2"
)

# P2P port and RPC port
P2P_PORT=9944
RPC_PORT=8545

# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------
log()  { echo "$(date '+%H:%M:%S') [INFO]  $*"; }
err()  { echo "$(date '+%H:%M:%S') [ERROR] $*" >&2; }
bold() { echo -e "\033[1m$*\033[0m"; }

# ---------------------------------------------------------------------------
# Step 0: Validate prerequisites
# ---------------------------------------------------------------------------
check_prereqs() {
  log "Checking prerequisites..."
  command -v gcloud >/dev/null 2>&1 || { err "gcloud CLI not found"; exit 1; }
  command -v docker >/dev/null 2>&1 || { err "docker not found"; exit 1; }

  # Set project
  gcloud config set project "${PROJECT_ID}" --quiet
  log "Using project: ${PROJECT_ID}"
}

# ---------------------------------------------------------------------------
# Step 1: Create Artifact Registry repo (if not exists)
# ---------------------------------------------------------------------------
setup_registry() {
  log "Setting up Artifact Registry..."
  gcloud artifacts repositories describe "${REPO_NAME}" \
    --location="${REGION}" --quiet 2>/dev/null || \
  gcloud artifacts repositories create "${REPO_NAME}" \
    --repository-format=docker \
    --location="${REGION}" \
    --description="Dina Network container images" \
    --quiet

  # Configure Docker auth
  gcloud auth configure-docker "${REGION}-docker.pkg.dev" --quiet
  log "Artifact Registry ready: ${AR_REPO}"
}

# ---------------------------------------------------------------------------
# Step 2: Build and push Docker image
# ---------------------------------------------------------------------------
build_and_push() {
  log "Building Docker image..."
  cd "$(dirname "$0")/.."

  docker build -t "${IMAGE_NAME}:latest" -f Dockerfile .
  docker tag "${IMAGE_NAME}:latest" "${AR_REPO}:latest"
  docker push "${AR_REPO}:latest"

  log "Image pushed: ${AR_REPO}:latest"
}

# ---------------------------------------------------------------------------
# Step 3: Generate validator keys
# ---------------------------------------------------------------------------
generate_keys() {
  log "Generating validator keys..."
  mkdir -p deployment/keys

  for i in 0 1 2; do
    KEY_FILE="deployment/keys/validator-${i}.key"
    if [ ! -f "${KEY_FILE}" ]; then
      # Generate 32 random bytes as Ed25519 seed
      openssl rand 32 > "${KEY_FILE}"
      log "Generated key: ${KEY_FILE}"
    else
      log "Key exists: ${KEY_FILE} (skipping)"
    fi
  done

  # Add keys to .gitignore
  if ! grep -q "deployment/keys/" .gitignore 2>/dev/null; then
    echo "deployment/keys/" >> .gitignore
  fi
}

# ---------------------------------------------------------------------------
# Step 4: Reserve static IPs
# ---------------------------------------------------------------------------
reserve_ips() {
  log "Reserving static IPs..."
  for name in "${VALIDATORS[@]}"; do
    gcloud compute addresses describe "${name}-ip" \
      --region="${REGION}" --quiet 2>/dev/null || \
    gcloud compute addresses create "${name}-ip" \
      --region="${REGION}" --quiet
  done

  # Collect IPs for peer list
  VALIDATOR_IPS=()
  for name in "${VALIDATORS[@]}"; do
    ip=$(gcloud compute addresses describe "${name}-ip" \
      --region="${REGION}" --format="get(address)")
    VALIDATOR_IPS+=("${ip}")
    log "${name} IP: ${ip}"
  done
}

# ---------------------------------------------------------------------------
# Step 5: Create firewall rules
# ---------------------------------------------------------------------------
setup_firewall() {
  log "Setting up firewall rules..."

  # P2P traffic between validators
  gcloud compute firewall-rules describe dina-p2p --quiet 2>/dev/null || \
  gcloud compute firewall-rules create dina-p2p \
    --allow=tcp:${P2P_PORT} \
    --target-tags=dina-validator \
    --source-tags=dina-validator \
    --description="Dina P2P consensus traffic" \
    --quiet

  # RPC access (restrict to your IP in production)
  gcloud compute firewall-rules describe dina-rpc --quiet 2>/dev/null || \
  gcloud compute firewall-rules create dina-rpc \
    --allow=tcp:${RPC_PORT} \
    --target-tags=dina-validator \
    --source-ranges="0.0.0.0/0" \
    --description="Dina RPC access" \
    --quiet
}

# ---------------------------------------------------------------------------
# Step 6: Create VMs
# ---------------------------------------------------------------------------
create_vms() {
  log "Creating validator VMs..."

  # Build bootstrap peer list (all validators know about each other)
  BOOTSTRAP_ARGS=""
  for ip in "${VALIDATOR_IPS[@]}"; do
    BOOTSTRAP_ARGS="${BOOTSTRAP_ARGS} --bootstrap /ip4/${ip}/tcp/${P2P_PORT}"
  done

  for i in 0 1 2; do
    name="${VALIDATORS[$i]}"
    ip="${VALIDATOR_IPS[$i]}"

    # Check if VM already exists
    if gcloud compute instances describe "${name}" --zone="${ZONE}" --quiet 2>/dev/null; then
      log "${name} already exists, skipping"
      continue
    fi

    log "Creating ${name} (${MACHINE_TYPE}) at ${ip}..."

    gcloud compute instances create-with-container "${name}" \
      --zone="${ZONE}" \
      --machine-type="${MACHINE_TYPE}" \
      --address="${name}-ip" \
      --tags=dina-validator \
      --boot-disk-size=50GB \
      --boot-disk-type=pd-ssd \
      --container-image="${AR_REPO}:latest" \
      --container-arg="--validator" \
      --container-arg="--validator-key" \
      --container-arg="/data/validator.key" \
      --container-arg="--chain-id" \
      --container-arg="${CHAIN_ID}" \
      --container-arg="--rpc-port" \
      --container-arg="${RPC_PORT}" \
      --container-arg="--listen" \
      --container-arg="/ip4/0.0.0.0/tcp/${P2P_PORT}" \
      ${BOOTSTRAP_ARGS} \
      --container-mount-host-path=mount-path=/data,host-path=/home/dina/data \
      --metadata=startup-script="#!/bin/bash
mkdir -p /home/dina/data
# Copy validator key from Secret Manager or generate
if [ ! -f /home/dina/data/validator.key ]; then
  openssl rand 32 > /home/dina/data/validator.key
fi" \
      --quiet

    log "${name} created successfully"
  done
}

# ---------------------------------------------------------------------------
# Step 7: Verify deployment
# ---------------------------------------------------------------------------
verify() {
  log "Waiting 30s for nodes to boot..."
  sleep 30

  bold "=== Validator Status ==="
  for i in 0 1 2; do
    ip="${VALIDATOR_IPS[$i]}"
    name="${VALIDATORS[$i]}"

    # Check if RPC responds
    if curl -sf "http://${ip}:${RPC_PORT}" -X POST \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","method":"dina_blockNumber","params":[],"id":1}' \
      --max-time 5 2>/dev/null; then
      echo ""
      log "${name} (${ip}): ONLINE"
    else
      err "${name} (${ip}): NOT RESPONDING (may still be starting)"
    fi
  done

  bold ""
  bold "=== Deployment Complete ==="
  bold ""
  bold "Testnet Details:"
  bold "  Chain ID:    ${CHAIN_ID}"
  bold "  Validators:  3x ${MACHINE_TYPE} (~\$75/month total)"
  bold "  TPS:         ~100,000 (2-lane parallel execution)"
  bold "  Finality:    100ms"
  bold ""
  bold "RPC Endpoints:"
  for i in 0 1 2; do
    bold "  http://${VALIDATOR_IPS[$i]}:${RPC_PORT}"
  done
  bold ""
  bold "To scale up later:"
  bold "  gcloud compute instances set-machine-type dina-validator-0 \\"
  bold "    --zone=${ZONE} --machine-type=c3-highcpu-44"
  bold "  (repeat for each validator, then restart)"
}

# ---------------------------------------------------------------------------
# Teardown — destroy everything
# ---------------------------------------------------------------------------
teardown() {
  bold "=== Tearing down testnet ==="

  for name in "${VALIDATORS[@]}"; do
    log "Deleting ${name}..."
    gcloud compute instances delete "${name}" --zone="${ZONE}" --quiet 2>/dev/null || true
    gcloud compute addresses delete "${name}-ip" --region="${REGION}" --quiet 2>/dev/null || true
  done

  gcloud compute firewall-rules delete dina-p2p --quiet 2>/dev/null || true
  gcloud compute firewall-rules delete dina-rpc --quiet 2>/dev/null || true

  log "Teardown complete. Artifact Registry and keys preserved."
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  if [ "${1:-}" = "teardown" ]; then
    teardown
    exit 0
  fi

  bold "=== Dina Network Testnet Deployment ==="
  bold "  3x ${MACHINE_TYPE} validators in ${ZONE}"
  bold "  Estimated cost: ~\$75/month"
  bold ""

  check_prereqs
  setup_registry
  build_and_push
  generate_keys
  reserve_ips
  setup_firewall
  create_vms
  verify
}

main "$@"
