# =============================================================================
# Dina Network — Cloud Monitoring & Alerting
#
# Alert policies that fire when validators are unhealthy, block production
# stalls, or disk space runs low.
# =============================================================================

# ---- Notification channel: email -------------------------------------------

resource "google_monitoring_notification_channel" "email" {
  display_name = "Dina Ops Email"
  type         = "email"

  labels = {
    email_address = var.alert_email
  }
}

# ---- Notification channel: Slack (optional) --------------------------------

resource "google_monitoring_notification_channel" "slack" {
  count = var.alert_slack_channel != "" ? 1 : 0

  display_name = "Dina Ops Slack"
  type         = "slack"

  labels = {
    channel_name = "#dina-ops"
  }

  sensitive_labels {
    auth_token = var.alert_slack_channel
  }
}

locals {
  notification_channels = concat(
    [google_monitoring_notification_channel.email.name],
    var.alert_slack_channel != "" ? [google_monitoring_notification_channel.slack[0].name] : [],
  )
}

# =============================================================================
# Alert 1: Validator VM Down
#
# Fires if any validator VM stops sending uptime metrics for 5 minutes.
# This likely means the VM crashed or the Docker container exited.
# =============================================================================

resource "google_monitoring_alert_policy" "validator_down" {
  display_name = "Dina Validator Down"
  combiner     = "OR"

  conditions {
    display_name = "Validator VM not sending uptime metrics"

    condition_absent {
      filter   = "resource.type = \"gce_instance\" AND metric.type = \"compute.googleapis.com/instance/uptime\" AND metadata.user_labels.role = \"validator\""
      duration = "300s"

      aggregations {
        alignment_period   = "60s"
        per_series_aligner = "ALIGN_MEAN"
      }

      trigger {
        count = 1
      }
    }
  }

  notification_channels = local.notification_channels

  alert_strategy {
    auto_close = "1800s"  # Auto-close after 30 minutes if resolved
  }

  documentation {
    content   = "A Dina validator VM has stopped reporting uptime metrics. Check the VM status in the GCP Console and review logs with: gcloud logging read 'resource.type=\"gce_instance\" AND labels.role=\"validator\"' --limit=50"
    mime_type = "text/markdown"
  }
}

# =============================================================================
# Alert 2: High CPU on Validators
#
# Fires if CPU utilization exceeds 80% for 10+ minutes, which may indicate
# the validator is struggling to keep up with block production.
# =============================================================================

resource "google_monitoring_alert_policy" "validator_high_cpu" {
  display_name = "Dina Validator High CPU"
  combiner     = "OR"

  conditions {
    display_name = "Validator CPU > 80% for 10 minutes"

    condition_threshold {
      filter          = "resource.type = \"gce_instance\" AND metric.type = \"compute.googleapis.com/instance/cpu/utilization\" AND metadata.user_labels.role = \"validator\""
      comparison      = "COMPARISON_GT"
      threshold_value = 0.8
      duration        = "600s"

      aggregations {
        alignment_period   = "60s"
        per_series_aligner = "ALIGN_MEAN"
      }

      trigger {
        count = 1
      }
    }
  }

  notification_channels = local.notification_channels

  documentation {
    content   = "A validator is running above 80% CPU for over 10 minutes. This may cause missed blocks. Consider upgrading the machine type or investigating resource-heavy operations."
    mime_type = "text/markdown"
  }
}

# =============================================================================
# Alert 3: Validator Disk Usage > 85%
#
# Chain data grows over time. Alert before the disk fills and the node crashes.
# =============================================================================

resource "google_monitoring_alert_policy" "validator_disk_full" {
  display_name = "Dina Validator Disk Nearly Full"
  combiner     = "OR"

  conditions {
    display_name = "Disk utilization > 85%"

    condition_threshold {
      filter          = "resource.type = \"gce_instance\" AND metric.type = \"compute.googleapis.com/instance/disk/utilization\" AND metadata.user_labels.role = \"validator\""
      comparison      = "COMPARISON_GT"
      threshold_value = 0.85
      duration        = "300s"

      aggregations {
        alignment_period   = "60s"
        per_series_aligner = "ALIGN_MEAN"
      }

      trigger {
        count = 1
      }
    }
  }

  notification_channels = local.notification_channels

  documentation {
    content   = "A validator's chain-data disk is over 85% full. Resize the disk with: gcloud compute disks resize dina-validator-data-N --size=NEW_SIZE_GB --zone=ZONE"
    mime_type = "text/markdown"
  }
}

# =============================================================================
# Alert 4: Cloud Run RPC Errors > 5%
#
# Fires when the RPC service error rate exceeds 5% over 5 minutes.
# =============================================================================

resource "google_monitoring_alert_policy" "rpc_errors" {
  display_name = "Dina RPC High Error Rate"
  combiner     = "OR"

  conditions {
    display_name = "RPC 5xx error rate > 5%"

    condition_threshold {
      filter          = "resource.type = \"cloud_run_revision\" AND resource.labels.service_name = \"dina-rpc\" AND metric.type = \"run.googleapis.com/request_count\" AND metric.labels.response_code_class = \"5xx\""
      comparison      = "COMPARISON_GT"
      threshold_value = 0.05
      duration        = "300s"

      aggregations {
        alignment_period     = "60s"
        per_series_aligner   = "ALIGN_RATE"
        cross_series_reducer = "REDUCE_SUM"
      }

      trigger {
        count = 1
      }
    }
  }

  notification_channels = local.notification_channels

  documentation {
    content   = "The Dina RPC Cloud Run service is returning server errors at >5%. Check logs: gcloud logging read 'resource.type=\"cloud_run_revision\" AND resource.labels.service_name=\"dina-rpc\" AND severity>=ERROR' --limit=50"
    mime_type = "text/markdown"
  }
}

# =============================================================================
# Alert 5: Cloud Run RPC Latency > 2s (p95)
#
# High latency on the RPC endpoint affects all downstream dApps and wallets.
# =============================================================================

resource "google_monitoring_alert_policy" "rpc_latency" {
  display_name = "Dina RPC High Latency"
  combiner     = "OR"

  conditions {
    display_name = "RPC p95 latency > 2 seconds"

    condition_threshold {
      filter          = "resource.type = \"cloud_run_revision\" AND resource.labels.service_name = \"dina-rpc\" AND metric.type = \"run.googleapis.com/request_latencies\""
      comparison      = "COMPARISON_GT"
      threshold_value = 2000  # milliseconds
      duration        = "300s"

      aggregations {
        alignment_period     = "60s"
        per_series_aligner   = "ALIGN_PERCENTILE_95"
        cross_series_reducer = "REDUCE_MEAN"
      }

      trigger {
        count = 1
      }
    }
  }

  notification_channels = local.notification_channels

  documentation {
    content   = "The RPC endpoint p95 latency is above 2 seconds. Check if the node is syncing or if there is a sudden traffic spike. Consider increasing min instances."
    mime_type = "text/markdown"
  }
}

# =============================================================================
# Uptime Check: RPC endpoint availability
#
# External probe from multiple GCP regions to verify the RPC endpoint responds.
# =============================================================================

resource "google_monitoring_uptime_check_config" "rpc" {
  display_name = "Dina RPC Uptime"
  timeout      = "10s"
  period       = "60s"

  http_check {
    path         = "/health"
    port         = 443
    use_ssl      = true
    validate_ssl = true
  }

  monitored_resource {
    type = "uptime_url"
    labels = {
      project_id = var.project_id
      host       = "rpc.${var.domain}"
    }
  }

  checker_type = "STATIC_IP_CHECKERS"
}
