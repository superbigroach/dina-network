#!/bin/bash
# =============================================================================
# Dina Network — GCP Deployment
# Deploys 3 validator nodes to Compute Engine + 1 RPC node to Cloud Run.
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration — override via environment variables
# ---------------------------------------------------------------------------
PROJECT_ID="${GCP_PROJECT_ID:-dina-network}"
IMAGE_NAME="${DINA_IMAGE:-gcr.io/${PROJECT_ID}/dina-node}"
IMAGE_TAG="${DINA_IMAGE_TAG:-latest}"
CHAIN_ID="${CHAIN_ID:-dina-testnet-1}"
MACHINE_TYPE="${MACHINE_TYPE:-e2-small}"
DISK_SIZE="${DISK_SIZE:-50}"

# Validator regions (spread across 3 regions for fault tolerance)
REGION_1="${REGION_1:-us-central1}"
REGION_2="${REGION_2:-europe-west1}"
REGION_3="${REGION_3:-asia-east1}"
ZONE_1="${REGION_1}-b"
ZONE_2="${REGION_2}-b"
ZONE_3="${REGION_3}-b"

# Cloud Run region for the RPC node
RPC_REGION="${RPC_REGION:-us-central1}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
KEYS_DIR="$PROJECT_DIR/keys"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
log() { echo "[$(date '+%H:%M:%S')] $*"; }

check_prerequisites() {
    log "Checking prerequisites..."

    if ! command -v gcloud &>/dev/null; then
        echo "Error: gcloud CLI not found. Install it from https://cloud.google.com/sdk"
        exit 1
    fi

    # Verify project
    ACTIVE_PROJECT=$(gcloud config get-value project 2>/dev/null || true)
    if [ "$ACTIVE_PROJECT" != "$PROJECT_ID" ]; then
        log "Setting active project to $PROJECT_ID"
        gcloud config set project "$PROJECT_ID"
    fi

    # Verify keys exist
    for i in 1 2 3; do
        if [ ! -f "$KEYS_DIR/validator-$i/node_key" ]; then
            echo "Error: Validator $i key not found at $KEYS_DIR/validator-$i/node_key"
            echo "Run ./scripts/generate-keys.sh first."
            exit 1
        fi
    done

    log "Prerequisites OK."
}

# ---------------------------------------------------------------------------
# Step 1: Build and push Docker image
# ---------------------------------------------------------------------------
build_and_push_image() {
    log "Building Docker image: ${IMAGE_NAME}:${IMAGE_TAG}"
    cd "$PROJECT_DIR"

    docker build -t "${IMAGE_NAME}:${IMAGE_TAG}" -f Dockerfile .

    log "Pushing image to Google Container Registry..."
    gcloud auth configure-docker gcr.io --quiet
    docker push "${IMAGE_NAME}:${IMAGE_TAG}"

    log "Image pushed: ${IMAGE_NAME}:${IMAGE_TAG}"
}

# ---------------------------------------------------------------------------
# Step 2: Store validator keys in Secret Manager
# ---------------------------------------------------------------------------
store_secrets() {
    log "Storing validator keys in Secret Manager..."

    for i in 1 2 3; do
        SECRET_NAME="dina-validator-${i}-key"

        # Create the secret if it doesn't exist
        if ! gcloud secrets describe "$SECRET_NAME" --project="$PROJECT_ID" &>/dev/null; then
            gcloud secrets create "$SECRET_NAME" \
                --project="$PROJECT_ID" \
                --replication-policy="automatic"
            log "  Created secret: $SECRET_NAME"
        fi

        # Add a new version with the key data
        gcloud secrets versions add "$SECRET_NAME" \
            --project="$PROJECT_ID" \
            --data-file="$KEYS_DIR/validator-$i/node_key"
        log "  Updated secret: $SECRET_NAME"
    done

    log "Secrets stored."
}

# ---------------------------------------------------------------------------
# Step 3: Create firewall rules
# ---------------------------------------------------------------------------
create_firewall_rules() {
    log "Creating firewall rules..."

    # P2P port (9944) — allow between validators
    if ! gcloud compute firewall-rules describe dina-p2p --project="$PROJECT_ID" &>/dev/null; then
        gcloud compute firewall-rules create dina-p2p \
            --project="$PROJECT_ID" \
            --network=default \
            --action=allow \
            --direction=ingress \
            --rules=tcp:9944 \
            --target-tags=dina-validator \
            --source-tags=dina-validator \
            --description="Dina Network P2P traffic between validators"
        log "  Created firewall rule: dina-p2p"
    else
        log "  Firewall rule dina-p2p already exists — skipping."
    fi

    # Allow health checks from GCP load balancers
    if ! gcloud compute firewall-rules describe dina-health-check --project="$PROJECT_ID" &>/dev/null; then
        gcloud compute firewall-rules create dina-health-check \
            --project="$PROJECT_ID" \
            --network=default \
            --action=allow \
            --direction=ingress \
            --rules=tcp:8080 \
            --target-tags=dina-validator \
            --source-ranges=35.191.0.0/16,130.211.0.0/22 \
            --description="Health checks for Dina validator nodes"
        log "  Created firewall rule: dina-health-check"
    else
        log "  Firewall rule dina-health-check already exists — skipping."
    fi

    log "Firewall rules configured."
}

# ---------------------------------------------------------------------------
# Step 4: Deploy validator VMs
# ---------------------------------------------------------------------------
deploy_validator() {
    local INDEX=$1
    local ZONE=$2
    local VM_NAME="dina-validator-${INDEX}"
    local SECRET_NAME="dina-validator-${INDEX}-key"

    log "Deploying $VM_NAME in $ZONE..."

    # Determine bootstrap peers (validator-1 is the seed for all others)
    local BOOTSTRAP_FLAG=""
    if [ "$INDEX" -gt 1 ]; then
        # Get validator-1's internal IP
        V1_IP=$(gcloud compute instances describe dina-validator-1 \
            --zone="$ZONE_1" \
            --project="$PROJECT_ID" \
            --format='value(networkInterfaces[0].networkIP)' 2>/dev/null || echo "")
        if [ -n "$V1_IP" ]; then
            BOOTSTRAP_FLAG="--bootstrap=/ip4/${V1_IP}/tcp/9944"
        fi
    fi

    # Create the VM with a startup script that pulls the key from Secret Manager
    gcloud compute instances create-with-container "$VM_NAME" \
        --project="$PROJECT_ID" \
        --zone="$ZONE" \
        --machine-type="$MACHINE_TYPE" \
        --boot-disk-size="${DISK_SIZE}GB" \
        --tags=dina-validator \
        --container-image="${IMAGE_NAME}:${IMAGE_TAG}" \
        --container-arg="--data-dir=/data" \
        --container-arg="--listen=/ip4/0.0.0.0/tcp/9944" \
        --container-arg="--rpc-port=8545" \
        --container-arg="--rest-port=8080" \
        --container-arg="--validator" \
        --container-arg="--validator-key=/secrets/node_key" \
        --container-arg="--chain-id=${CHAIN_ID}" \
        ${BOOTSTRAP_FLAG:+--container-arg="$BOOTSTRAP_FLAG"} \
        --container-mount-host-path=mount-path=/data,host-path=/mnt/disks/dina-data \
        --scopes=cloud-platform \
        --metadata=startup-script="#!/bin/bash
mkdir -p /mnt/disks/dina-data /secrets
gcloud secrets versions access latest --secret=${SECRET_NAME} > /secrets/node_key
chmod 600 /secrets/node_key
" \
        --no-address 2>/dev/null || \
    gcloud compute instances update-container "$VM_NAME" \
        --project="$PROJECT_ID" \
        --zone="$ZONE" \
        --container-image="${IMAGE_NAME}:${IMAGE_TAG}"

    log "  $VM_NAME deployed in $ZONE"
}

deploy_validators() {
    log "Deploying 3 validator nodes across regions..."

    deploy_validator 1 "$ZONE_1"
    deploy_validator 2 "$ZONE_2"
    deploy_validator 3 "$ZONE_3"

    log "All validators deployed."
}

# ---------------------------------------------------------------------------
# Step 5: Deploy RPC node on Cloud Run
# ---------------------------------------------------------------------------
deploy_rpc_node() {
    log "Deploying RPC node to Cloud Run in $RPC_REGION..."

    # Get validator-1 external IP for bootstrap
    V1_IP=$(gcloud compute instances describe dina-validator-1 \
        --zone="$ZONE_1" \
        --project="$PROJECT_ID" \
        --format='value(networkInterfaces[0].accessConfigs[0].natIP)' 2>/dev/null || echo "")

    BOOTSTRAP_ARG=""
    if [ -n "$V1_IP" ]; then
        BOOTSTRAP_ARG="--bootstrap=/ip4/${V1_IP}/tcp/9944"
    fi

    gcloud run deploy dina-rpc \
        --project="$PROJECT_ID" \
        --region="$RPC_REGION" \
        --image="${IMAGE_NAME}:${IMAGE_TAG}" \
        --platform=managed \
        --port=8545 \
        --cpu=1 \
        --memory=1Gi \
        --min-instances=1 \
        --max-instances=5 \
        --no-allow-unauthenticated \
        --set-env-vars="RUST_LOG=info,DINA_CHAIN_ID=${CHAIN_ID}" \
        --args="--data-dir=/data,--rpc-port=8545,--rest-port=8080,--chain-id=${CHAIN_ID}${BOOTSTRAP_ARG:+,${BOOTSTRAP_ARG}}"

    RPC_URL=$(gcloud run services describe dina-rpc \
        --project="$PROJECT_ID" \
        --region="$RPC_REGION" \
        --format='value(status.url)' 2>/dev/null || echo "unknown")

    log "RPC node deployed: $RPC_URL"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    echo "============================================"
    echo "  Dina Network — GCP Deployment"
    echo "============================================"
    echo ""
    echo "  Project:    $PROJECT_ID"
    echo "  Chain ID:   $CHAIN_ID"
    echo "  Image:      ${IMAGE_NAME}:${IMAGE_TAG}"
    echo "  Validators: $ZONE_1, $ZONE_2, $ZONE_3"
    echo "  RPC region: $RPC_REGION"
    echo ""

    check_prerequisites
    build_and_push_image
    store_secrets
    create_firewall_rules
    deploy_validators
    deploy_rpc_node

    echo ""
    echo "============================================"
    echo "  Deployment Complete"
    echo "============================================"
    echo ""
    echo "  Validators:"
    for i in 1 2 3; do
        local ZONE_VAR="ZONE_${i}"
        echo "    [$i] dina-validator-$i (${!ZONE_VAR})"
    done
    echo ""
    echo "  RPC Node: dina-rpc (Cloud Run, $RPC_REGION)"
    echo ""
    echo "  Useful commands:"
    echo "    gcloud compute instances list --filter='tags.items=dina-validator'"
    echo "    gcloud run services describe dina-rpc --region=$RPC_REGION"
    echo "    gcloud compute ssh dina-validator-1 --zone=$ZONE_1"
}

main "$@"
