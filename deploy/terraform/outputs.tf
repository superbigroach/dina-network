# =============================================================================
# Dina Network — Terraform Outputs
#
# Connection info for operators, CI/CD, and downstream Terraform modules.
# =============================================================================

# ---- Validator IPs ----------------------------------------------------------

output "validator_ips" {
  description = "Public IP addresses of validator VMs (for P2P bootstrap and monitoring)"
  value = {
    for i, instance in google_compute_instance.validator :
    "validator-${i}" => instance.network_interface[0].access_config[0].nat_ip
  }
}

output "validator_internal_ips" {
  description = "Internal VPC IP addresses of validator VMs"
  value = {
    for i, instance in google_compute_instance.validator :
    "validator-${i}" => instance.network_interface[0].network_ip
  }
}

output "validator_names" {
  description = "Names and zones of validator VMs (for SSH via gcloud)"
  value = {
    for i, instance in google_compute_instance.validator :
    "validator-${i}" => {
      name = instance.name
      zone = instance.zone
    }
  }
}

# ---- Bootstrap address for joining the network ------------------------------

output "bootstrap_multiaddr" {
  description = "libp2p multiaddress for bootstrapping new nodes from validator-0"
  value       = "/ip4/${google_compute_instance.validator[0].network_interface[0].access_config[0].nat_ip}/tcp/${var.p2p_port}"
}

# ---- Cloud Run Service URLs -------------------------------------------------

output "rpc_url" {
  description = "Cloud Run URL for the JSON-RPC service (auto-generated)"
  value       = google_cloud_run_v2_service.rpc.uri
}

output "rpc_public_url" {
  description = "Public URL for the RPC endpoint behind the load balancer"
  value       = "https://rpc.${var.domain}"
}

output "rpc_load_balancer_ip" {
  description = "Global static IP for the RPC load balancer — point DNS A record here"
  value       = google_compute_global_address.rpc.address
}

output "explorer_url" {
  description = "Cloud Run URL for the block explorer"
  value       = google_cloud_run_v2_service.explorer.uri
}

output "faucet_url" {
  description = "Cloud Run URL for the testnet faucet (empty string if mainnet)"
  value       = var.environment == "testnet" ? google_cloud_run_v2_service.faucet[0].uri : ""
}

# ---- Network info -----------------------------------------------------------

output "vpc_id" {
  description = "VPC network self-link (for peering or additional subnets)"
  value       = google_compute_network.dina.self_link
}

output "chain_id" {
  description = "Chain ID used by this deployment"
  value       = var.chain_id
}

# ---- DNS records to create --------------------------------------------------

output "dns_records" {
  description = "DNS records that must be created in your domain registrar"
  value = {
    "rpc.${var.domain}"      = "A ${google_compute_global_address.rpc.address}"
    "explorer.${var.domain}" = "CNAME to ${google_cloud_run_v2_service.explorer.uri}"
    "faucet.${var.domain}"   = var.environment == "testnet" ? "CNAME to ${google_cloud_run_v2_service.faucet[0].uri}" : "N/A (mainnet)"
  }
}
