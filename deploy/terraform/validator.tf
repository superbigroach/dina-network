# =============================================================================
# Dina Network — Validator VM Configuration
#
# Creates 3 Compute Engine VMs, each in a different region, running the
# dina-node Docker container in --validator mode.
#
# Each VM:
#   - Pulls its validator key from Secret Manager at boot
#   - Stores chain data on a persistent SSD
#   - Auto-restarts on failure via the restart policy
#   - Reports health via the REST /health endpoint
# =============================================================================

# ---- Secrets: one per validator for its Ed25519 signing key -----------------

resource "google_secret_manager_secret" "validator_key" {
  count = 3

  secret_id = "dina-validator-key-${count.index}"

  labels = merge(var.labels, {
    validator_index = tostring(count.index)
    environment     = var.environment
  })

  replication {
    auto {}
  }

  depends_on = [google_project_service.apis["secretmanager.googleapis.com"]]
}

# Placeholder version — operators must upload the real key material via:
#   gcloud secrets versions add dina-validator-key-0 --data-file=./keys/validator-0/node_key
resource "google_secret_manager_secret_version" "validator_key_placeholder" {
  count = 3

  secret      = google_secret_manager_secret.validator_key[count.index].id
  secret_data = "REPLACE_WITH_REAL_VALIDATOR_KEY_${count.index}"

  lifecycle {
    # Prevent Terraform from overwriting keys that operators have uploaded
    ignore_changes = [secret_data]
  }
}

# ---- Persistent disks for chain data ---------------------------------------

resource "google_compute_disk" "validator_data" {
  count = 3

  name  = "dina-validator-data-${count.index}"
  zone  = var.validator_zones[count.index]
  type  = var.validator_disk_type
  size  = var.validator_disk_size_gb

  labels = merge(var.labels, {
    validator_index = tostring(count.index)
    purpose         = "chain-data"
  })
}

# ---- Validator VMs ----------------------------------------------------------

resource "google_compute_instance" "validator" {
  count = 3

  name         = "dina-validator-${count.index}"
  machine_type = var.validator_machine_type
  zone         = var.validator_zones[count.index]

  tags = ["dina-validator"]

  labels = merge(var.labels, {
    role            = "validator"
    validator_index = tostring(count.index)
    environment     = var.environment
  })

  # Boot disk — Container-Optimized OS for running Docker natively
  boot_disk {
    initialize_params {
      image = "projects/cos-cloud/global/images/family/cos-stable"
      size  = 10
      type  = "pd-balanced"
    }
  }

  # Attach the persistent chain-data disk
  attached_disk {
    source      = google_compute_disk.validator_data[count.index].self_link
    device_name = "chain-data"
    mode        = "READ_WRITE"
  }

  network_interface {
    subnetwork = google_compute_subnetwork.validator[count.index].id

    # Ephemeral external IP for P2P connectivity
    access_config {
      # GCP assigns a public IP automatically
    }
  }

  service_account {
    email  = google_service_account.validator.email
    scopes = ["cloud-platform"]
  }

  # ---- Startup script: mount data disk, fetch key, run container -----------
  metadata = {
    # cos-cloud uses cloud-init; we use a startup script for Docker
    startup-script = templatefile("${path.module}/templates/validator-startup.sh.tpl", {
      validator_index   = count.index
      chain_id          = var.chain_id
      p2p_port          = var.p2p_port
      rpc_port          = var.rpc_port
      rest_port         = var.rest_port
      node_image        = var.dina_node_image
      project_id        = var.project_id
      # Validator 0 is the seed — others bootstrap from it
      bootstrap_addr    = count.index == 0 ? "" : "/ip4/${google_compute_instance.validator[0].network_interface[0].access_config[0].nat_ip}/tcp/${var.p2p_port}"
    })

    # Enable the logging agent
    google-logging-enabled = "true"
  }

  # Auto-restart if the VM crashes; do not preempt (validators must be reliable)
  scheduling {
    automatic_restart   = true
    on_host_maintenance = "MIGRATE"
    preemptible         = false
  }

  # Prevent Terraform from recreating VMs just because the image updated
  lifecycle {
    ignore_changes = [
      boot_disk[0].initialize_params[0].image,
    ]
  }

  depends_on = [
    google_project_service.apis["compute.googleapis.com"],
    google_secret_manager_secret_version.validator_key_placeholder,
  ]
}

# ---- Health check for validator VMs ----------------------------------------

resource "google_compute_health_check" "validator" {
  name                = "dina-validator-health"
  check_interval_sec  = 30
  timeout_sec         = 5
  healthy_threshold   = 2
  unhealthy_threshold = 3

  http_health_check {
    port         = var.rest_port
    request_path = "/health"
  }
}
