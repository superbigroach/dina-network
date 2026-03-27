# =============================================================================
# Dina Network — Terraform Variables
# All tuneable knobs for the deployment live here.
# =============================================================================

# ---- Project & Region -------------------------------------------------------

variable "project_id" {
  description = "GCP project ID where all resources are created"
  type        = string
}

variable "region" {
  description = "Default GCP region for Cloud Run and networking resources"
  type        = string
  default     = "us-central1"
}

variable "validator_regions" {
  description = "Regions for validator VMs — one VM per region for geographic distribution"
  type        = list(string)
  default     = ["us-central1", "europe-west1", "asia-east1"]
}

variable "validator_zones" {
  description = "Zones for validator VMs (must match validator_regions order)"
  type        = list(string)
  default     = ["us-central1-a", "europe-west1-b", "asia-east1-b"]
}

# ---- Compute ----------------------------------------------------------------

variable "validator_machine_type" {
  description = "Machine type for validator VMs"
  type        = string
  default     = "e2-small"
}

variable "validator_disk_size_gb" {
  description = "Persistent disk size for chain data on each validator (GB)"
  type        = number
  default     = 50
}

variable "validator_disk_type" {
  description = "Disk type for validator persistent storage"
  type        = string
  default     = "pd-ssd"
}

# ---- Container Images -------------------------------------------------------

variable "dina_node_image" {
  description = "Container image for the Dina validator/RPC node"
  type        = string
  default     = "gcr.io/PROJECT_ID/dina-node:latest"
}

variable "dina_explorer_image" {
  description = "Container image for the Dina block explorer"
  type        = string
  default     = "gcr.io/PROJECT_ID/dina-explorer:latest"
}

variable "dina_faucet_image" {
  description = "Container image for the testnet faucet"
  type        = string
  default     = "gcr.io/PROJECT_ID/dina-faucet:latest"
}

# ---- Network ----------------------------------------------------------------

variable "chain_id" {
  description = "Chain identifier for the Dina network"
  type        = string
  default     = "dina-testnet-1"
}

variable "p2p_port" {
  description = "libp2p listening port on validator VMs"
  type        = number
  default     = 9944
}

variable "rpc_port" {
  description = "JSON-RPC port"
  type        = number
  default     = 8545
}

variable "rest_port" {
  description = "REST/health-check port"
  type        = number
  default     = 8080
}

# ---- Cloud Run --------------------------------------------------------------

variable "rpc_min_instances" {
  description = "Minimum Cloud Run instances for the RPC service"
  type        = number
  default     = 1
}

variable "rpc_max_instances" {
  description = "Maximum Cloud Run instances for the RPC service"
  type        = number
  default     = 10
}

variable "rpc_cpu" {
  description = "CPU allocation per RPC Cloud Run instance"
  type        = string
  default     = "1"
}

variable "rpc_memory" {
  description = "Memory allocation per RPC Cloud Run instance"
  type        = string
  default     = "512Mi"
}

variable "explorer_min_instances" {
  description = "Minimum Cloud Run instances for the explorer"
  type        = number
  default     = 0
}

variable "explorer_max_instances" {
  description = "Maximum Cloud Run instances for the explorer"
  type        = number
  default     = 5
}

variable "faucet_max_instances" {
  description = "Maximum Cloud Run instances for the faucet"
  type        = number
  default     = 2
}

# ---- DNS / Domain -----------------------------------------------------------

variable "domain" {
  description = "Base domain for services (e.g. dina.network)"
  type        = string
  default     = "dina.network"
}

# ---- Monitoring -------------------------------------------------------------

variable "alert_email" {
  description = "Email address for monitoring alert notifications"
  type        = string
  default     = "ops@dina.network"
}

variable "alert_slack_channel" {
  description = "Slack webhook URL for alert notifications (optional)"
  type        = string
  default     = ""
}

# ---- Labels -----------------------------------------------------------------

variable "environment" {
  description = "Environment label (testnet, mainnet)"
  type        = string
  default     = "testnet"
}

variable "labels" {
  description = "Common labels applied to all resources"
  type        = map(string)
  default = {
    project     = "dina-network"
    managed_by  = "terraform"
  }
}
