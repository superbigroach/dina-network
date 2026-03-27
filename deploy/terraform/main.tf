# =============================================================================
# Dina Network — Terraform Main Configuration
#
# Provisions the full Dina blockchain infrastructure on GCP:
#   - 3 Compute Engine VMs for validators (geographically distributed)
#   - Cloud Run services for RPC, Explorer, and Faucet
#   - VPC with firewall rules for P2P and health-check traffic
#   - Secret Manager for validator keys
#   - Cloud Monitoring alerts
#   - Global load balancer for the public RPC endpoint
# =============================================================================

terraform {
  required_version = ">= 1.5"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
    google-beta = {
      source  = "hashicorp/google-beta"
      version = "~> 5.0"
    }
  }

  # Store state in a GCS bucket — create this bucket manually before init.
  backend "gcs" {
    bucket = "dina-network-tfstate"
    prefix = "terraform/state"
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

provider "google-beta" {
  project = var.project_id
  region  = var.region
}

# ---- Enable required GCP APIs -----------------------------------------------

resource "google_project_service" "apis" {
  for_each = toset([
    "compute.googleapis.com",
    "run.googleapis.com",
    "secretmanager.googleapis.com",
    "monitoring.googleapis.com",
    "logging.googleapis.com",
    "containerregistry.googleapis.com",
    "artifactregistry.googleapis.com",
  ])

  project = var.project_id
  service = each.key

  disable_dependent_services = false
  disable_on_destroy         = false
}

# ---- Service account for validators -----------------------------------------

resource "google_service_account" "validator" {
  account_id   = "dina-validator"
  display_name = "Dina Validator Node"
  description  = "Service account for Dina validator VMs — reads secrets, writes logs"
}

# Allow the validator SA to read its own key secrets
resource "google_project_iam_member" "validator_secret_accessor" {
  project = var.project_id
  role    = "roles/secretmanager.secretAccessor"
  member  = "serviceAccount:${google_service_account.validator.email}"
}

# Allow the validator SA to write logs and metrics
resource "google_project_iam_member" "validator_log_writer" {
  project = var.project_id
  role    = "roles/logging.logWriter"
  member  = "serviceAccount:${google_service_account.validator.email}"
}

resource "google_project_iam_member" "validator_metric_writer" {
  project = var.project_id
  role    = "roles/monitoring.metricWriter"
  member  = "serviceAccount:${google_service_account.validator.email}"
}
