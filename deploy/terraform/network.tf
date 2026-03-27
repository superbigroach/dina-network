# =============================================================================
# Dina Network — VPC, Subnets, and Firewall Rules
# =============================================================================

# ---- VPC --------------------------------------------------------------------

resource "google_compute_network" "dina" {
  name                    = "dina-network"
  auto_create_subnetworks = false
  description             = "VPC for Dina blockchain validators and services"
}

# ---- Subnets (one per validator region) -------------------------------------

resource "google_compute_subnetwork" "validator" {
  count = length(var.validator_regions)

  name          = "dina-validator-${var.validator_regions[count.index]}"
  region        = var.validator_regions[count.index]
  network       = google_compute_network.dina.id
  ip_cidr_range = "10.${count.index + 1}.0.0/24"

  log_config {
    aggregation_interval = "INTERVAL_5_SEC"
    flow_sampling        = 0.5
    metadata             = "INCLUDE_ALL_METADATA"
  }
}

# ---- Firewall: allow P2P traffic between validators ------------------------

resource "google_compute_firewall" "p2p_internal" {
  name    = "dina-allow-p2p-internal"
  network = google_compute_network.dina.id

  description = "Allow libp2p traffic between all Dina validators"

  allow {
    protocol = "tcp"
    ports    = [tostring(var.p2p_port)]
  }

  # Only between instances tagged as validators
  source_tags = ["dina-validator"]
  target_tags = ["dina-validator"]
}

# ---- Firewall: allow P2P from external (for RPC nodes to sync) -------------

resource "google_compute_firewall" "p2p_external" {
  name    = "dina-allow-p2p-external"
  network = google_compute_network.dina.id

  description = "Allow inbound P2P connections from any IP (public network participation)"

  allow {
    protocol = "tcp"
    ports    = [tostring(var.p2p_port)]
  }

  source_ranges = ["0.0.0.0/0"]
  target_tags   = ["dina-validator"]
}

# ---- Firewall: allow health checks from GCP load balancer probes -----------

resource "google_compute_firewall" "health_check" {
  name    = "dina-allow-health-check"
  network = google_compute_network.dina.id

  description = "Allow GCP health-check probes to reach the REST health endpoint"

  allow {
    protocol = "tcp"
    ports    = [tostring(var.rest_port)]
  }

  # GCP health-check probe source ranges
  source_ranges = [
    "35.191.0.0/16",
    "130.211.0.0/22",
  ]

  target_tags = ["dina-validator"]
}

# ---- Firewall: allow SSH via IAP (no public SSH) ---------------------------

resource "google_compute_firewall" "iap_ssh" {
  name    = "dina-allow-iap-ssh"
  network = google_compute_network.dina.id

  description = "Allow SSH only through Identity-Aware Proxy — no direct public SSH"

  allow {
    protocol = "tcp"
    ports    = ["22"]
  }

  # IAP forwarding IP range
  source_ranges = ["35.235.240.0/20"]
  target_tags   = ["dina-validator"]
}

# ---- Firewall: deny all other ingress (implicit, but explicit for clarity) --

resource "google_compute_firewall" "deny_all_ingress" {
  name    = "dina-deny-all-ingress"
  network = google_compute_network.dina.id

  description = "Default deny all ingress — only explicitly allowed ports are open"
  priority    = 65534

  deny {
    protocol = "all"
  }

  source_ranges = ["0.0.0.0/0"]
}

# ---- Cloud Router + NAT (validators need outbound but not inbound IPs) -----

resource "google_compute_router" "dina" {
  name    = "dina-router"
  region  = var.region
  network = google_compute_network.dina.id
}

resource "google_compute_router_nat" "dina" {
  name   = "dina-nat"
  router = google_compute_router.dina.name
  region = var.region

  nat_ip_allocate_option             = "AUTO_ONLY"
  source_subnetwork_ip_ranges_to_nat = "ALL_SUBNETWORKS_ALL_IP_RANGES"

  log_config {
    enable = true
    filter = "ERRORS_ONLY"
  }
}
