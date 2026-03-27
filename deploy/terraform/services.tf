# =============================================================================
# Dina Network — Cloud Run Services
#
# Stateless services that scale automatically:
#   - RPC: public JSON-RPC endpoint for wallets and dApps
#   - Explorer: block explorer web UI + API
#   - Faucet: testnet token dispenser (testnet-only)
# =============================================================================

# ---- RPC Service ------------------------------------------------------------
# The RPC node connects to the validator P2P network, syncs the chain, and
# exposes JSON-RPC (port 8545) for external consumers.

resource "google_cloud_run_v2_service" "rpc" {
  name     = "dina-rpc"
  location = var.region

  labels = merge(var.labels, {
    role        = "rpc"
    environment = var.environment
  })

  template {
    scaling {
      min_instance_count = var.rpc_min_instances
      max_instance_count = var.rpc_max_instances
    }

    containers {
      image = var.dina_node_image

      args = [
        "--data-dir", "/data",
        "--listen", "/ip4/0.0.0.0/tcp/${var.p2p_port}",
        "--rpc-port", tostring(var.rpc_port),
        "--rest-port", tostring(var.rest_port),
        "--chain-id", var.chain_id,
        # Bootstrap from validator-0's public IP
        "--bootstrap", "/ip4/${google_compute_instance.validator[0].network_interface[0].access_config[0].nat_ip}/tcp/${var.p2p_port}",
      ]

      ports {
        container_port = var.rpc_port
        name           = "h2c"
      }

      resources {
        limits = {
          cpu    = var.rpc_cpu
          memory = var.rpc_memory
        }
      }

      env {
        name  = "RUST_LOG"
        value = "info,dina_rpc=debug"
      }

      env {
        name  = "DINA_CHAIN_ID"
        value = var.chain_id
      }

      startup_probe {
        http_get {
          path = "/health"
          port = var.rest_port
        }
        initial_delay_seconds = 10
        period_seconds        = 5
        failure_threshold     = 10
      }

      liveness_probe {
        http_get {
          path = "/health"
          port = var.rest_port
        }
        period_seconds    = 30
        failure_threshold = 3
      }
    }

    # 5-minute timeout — RPC requests should be fast, but initial sync takes time
    timeout = "300s"
  }

  # Allow unauthenticated access — this is a public RPC endpoint
  lifecycle {
    ignore_changes = [
      template[0].containers[0].image,
    ]
  }

  depends_on = [google_project_service.apis["run.googleapis.com"]]
}

# Make RPC publicly accessible (no auth required for blockchain RPC)
resource "google_cloud_run_v2_service_iam_member" "rpc_public" {
  project  = var.project_id
  location = var.region
  name     = google_cloud_run_v2_service.rpc.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}

# ---- Explorer Service ------------------------------------------------------
# Block explorer: shows blocks, transactions, accounts, and contract state.

resource "google_cloud_run_v2_service" "explorer" {
  name     = "dina-explorer"
  location = var.region

  labels = merge(var.labels, {
    role        = "explorer"
    environment = var.environment
  })

  template {
    scaling {
      min_instance_count = var.explorer_min_instances
      max_instance_count = var.explorer_max_instances
    }

    containers {
      image = var.dina_explorer_image

      ports {
        container_port = 3000
        name           = "http1"
      }

      resources {
        limits = {
          cpu    = "1"
          memory = "512Mi"
        }
      }

      env {
        name  = "DINA_RPC_URL"
        value = google_cloud_run_v2_service.rpc.uri
      }

      env {
        name  = "DINA_CHAIN_ID"
        value = var.chain_id
      }

      env {
        name  = "DINA_EXPLORER_PORT"
        value = "3000"
      }

      startup_probe {
        http_get {
          path = "/health"
          port = 3000
        }
        initial_delay_seconds = 5
        period_seconds        = 3
        failure_threshold     = 10
      }

      liveness_probe {
        http_get {
          path = "/health"
          port = 3000
        }
        period_seconds    = 30
        failure_threshold = 3
      }
    }

    timeout = "60s"
  }

  depends_on = [google_project_service.apis["run.googleapis.com"]]
}

resource "google_cloud_run_v2_service_iam_member" "explorer_public" {
  project  = var.project_id
  location = var.region
  name     = google_cloud_run_v2_service.explorer.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}

# ---- Faucet Service (testnet only) -----------------------------------------
# Dispenses test USDC tokens. Disabled on mainnet via count conditional.

resource "google_cloud_run_v2_service" "faucet" {
  count = var.environment == "testnet" ? 1 : 0

  name     = "dina-faucet"
  location = var.region

  labels = merge(var.labels, {
    role        = "faucet"
    environment = var.environment
  })

  template {
    scaling {
      min_instance_count = 0
      max_instance_count = var.faucet_max_instances
    }

    containers {
      image = var.dina_faucet_image

      ports {
        container_port = 3001
        name           = "http1"
      }

      resources {
        limits = {
          cpu    = "0.5"
          memory = "256Mi"
        }
      }

      env {
        name  = "DINA_RPC_URL"
        value = google_cloud_run_v2_service.rpc.uri
      }

      env {
        name  = "DINA_CHAIN_ID"
        value = var.chain_id
      }

      env {
        name  = "FAUCET_AMOUNT"
        value = "1000000"  # 1 USDC (6 decimals)
      }

      env {
        name  = "FAUCET_COOLDOWN_SECONDS"
        value = "3600"  # 1 hour between requests per IP
      }

      # Faucet private key stored in Secret Manager
      env {
        name = "FAUCET_PRIVATE_KEY"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.faucet_key[0].id
            version = "latest"
          }
        }
      }

      startup_probe {
        http_get {
          path = "/health"
          port = 3001
        }
        initial_delay_seconds = 5
        period_seconds        = 3
        failure_threshold     = 5
      }
    }

    timeout = "30s"
  }

  depends_on = [google_project_service.apis["run.googleapis.com"]]
}

# Faucet key secret — only created for testnet
resource "google_secret_manager_secret" "faucet_key" {
  count = var.environment == "testnet" ? 1 : 0

  secret_id = "dina-faucet-private-key"

  labels = merge(var.labels, {
    role        = "faucet"
    environment = var.environment
  })

  replication {
    auto {}
  }

  depends_on = [google_project_service.apis["secretmanager.googleapis.com"]]
}

resource "google_secret_manager_secret_version" "faucet_key_placeholder" {
  count = var.environment == "testnet" ? 1 : 0

  secret      = google_secret_manager_secret.faucet_key[0].id
  secret_data = "REPLACE_WITH_FAUCET_PRIVATE_KEY"

  lifecycle {
    ignore_changes = [secret_data]
  }
}

resource "google_cloud_run_v2_service_iam_member" "faucet_public" {
  count = var.environment == "testnet" ? 1 : 0

  project  = var.project_id
  location = var.region
  name     = google_cloud_run_v2_service.faucet[0].name
  role     = "roles/run.invoker"
  member   = "allUsers"
}

# =============================================================================
# Global Load Balancer for RPC
#
# Routes rpc.dina.network -> Cloud Run RPC service with SSL termination.
# =============================================================================

resource "google_compute_region_network_endpoint_group" "rpc_neg" {
  name                  = "dina-rpc-neg"
  region                = var.region
  network_endpoint_type = "SERVERLESS"

  cloud_run {
    service = google_cloud_run_v2_service.rpc.name
  }
}

resource "google_compute_backend_service" "rpc" {
  name        = "dina-rpc-backend"
  protocol    = "HTTPS"
  timeout_sec = 30

  backend {
    group = google_compute_region_network_endpoint_group.rpc_neg.id
  }

  log_config {
    enable      = true
    sample_rate = 0.1
  }
}

resource "google_compute_url_map" "rpc" {
  name            = "dina-rpc-url-map"
  default_service = google_compute_backend_service.rpc.id
}

# Managed SSL certificate for rpc.dina.network
resource "google_compute_managed_ssl_certificate" "rpc" {
  name = "dina-rpc-ssl"

  managed {
    domains = ["rpc.${var.domain}"]
  }
}

resource "google_compute_target_https_proxy" "rpc" {
  name             = "dina-rpc-https-proxy"
  url_map          = google_compute_url_map.rpc.id
  ssl_certificates = [google_compute_managed_ssl_certificate.rpc.id]
}

resource "google_compute_global_address" "rpc" {
  name = "dina-rpc-ip"
}

resource "google_compute_global_forwarding_rule" "rpc" {
  name       = "dina-rpc-forwarding"
  target     = google_compute_target_https_proxy.rpc.id
  ip_address = google_compute_global_address.rpc.address
  port_range = "443"
}
