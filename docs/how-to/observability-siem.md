---
layout: doc
title: Prepare telemetry export review
permalink: /how-to/observability-siem.html
reviewed: 2026-11-22
---
# Prepare telemetry export review

**Diátaxis quadrant:** How-to. **Audience:** observability engineers.

The Rust export path has not completed qualification. Separate durable
enforcement audit from sampled analytics and define bounded queue, disk, batch
and attribute limits before choosing an exporter.

Count attempts and outcomes without relying on sampled traces. Use bounded route
and reason labels instead of raw IPs, sessions or URLs. Test sink outage, queue
overflow and restart; expose dropped/degraded intervals. Do not promise unlimited
lossless delivery. See [data review](../reference/data-processing-inventory.md).

## Scrape gateway metrics

When the runtime gateway runs with an admin listener, `GET
/_humanymous/admin/metrics` on that listener returns Prometheus text format
(`text/plain; version=0.0.4`). Any configured admin role may read it; send the
operator's bearer token. The public listener never serves it.

The exposition carries:

- `hmn_gate_uptime_seconds`, `hmn_gate_bans_active` and
  `hmn_gate_audit_projection_dropped_total` (always 0: there is no projection
  sink);
- `hmn_gate_killswitch` and `hmn_gate_monitor`;
- the settings series `hmn_gate_settings_overlay_active`,
  `hmn_gate_settings_overlay_pending_age_seconds`,
  `hmn_gate_settings_apply_total{result}` and
  `hmn_gate_settings_store_errors_total`;
- `hmn_gate_config_version{version}`, the signed policy version that audit
  records also carry. It changes when the policy or the process key changes,
  so alert on changes, not on a fixed value.

Audit-chain health (records, checkpoints, integrity, witness), operational
log health and process memory series are not exported yet. Watch audit
integrity through the audit tools until they are.
