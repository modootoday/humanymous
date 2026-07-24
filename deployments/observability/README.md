# Observability wiring for humanymous Gate

Drop-in Prometheus config for scraping Gate's admin-plane metrics endpoint and
alerting on the structural health of its audit chain. These files are the concrete
companions to the [Observability and SIEM integration](../../docs/how-to/observability-siem.md)
how-to — read that guide for the full picture (audit-stream poll-and-forward, the
`?subject=` subject-access export, health probes, and where per-verdict rates come from).

## Files

| File | What it is |
|------|------------|
| `prometheus-scrape.yml` | A `scrape_configs` job for `GET /__hmn/admin/metrics` on the **admin** listener: `scheme: https`, the **Auditor** bearer token (read-only), and a dev `tls_config` (`insecure_skip_verify` for the self-signed admin cert — swap for a real CA / mTLS in production). |
| `alerts.yml` | Prometheus alerting rules. **Structural only — no invented numeric verdict/DENY thresholds.** |

## What the alerts cover

| Alert | Condition | Meaning |
|-------|-----------|---------|
| `HmnGateAuditChainBroken` | `hmn_gate_audit_integrity_ok == 0` | The tamper-evident audit chain no longer verifies end-to-end. Highest severity. |
| `HmnGateAuditWitnessLost` | `hmn_gate_audit_witnessed == 0` | The latest checkpoints lost their independent-witness co-signature. |
| `HmnGateAuditProjectionDropping` | `increase(hmn_gate_audit_projection_dropped_total[10m]) > 0` | A Tier-1/2 projection sink (Redis/ClickHouse) shed records. The WAL is still authoritative; the downstream view is incomplete. |
| `HmnGateKillSwitchEngaged` | `hmn_gate_killswitch == 1` | The fleet-wide kill switch is engaged (enforcement demoted to monitor). |
| `HmnGateScrapeDown` | `up == 0` | Prometheus cannot reach the metrics endpoint — a blackout that hides every rule above. |

Per-verdict rates (allow/challenge/deny) are **not** in `/metrics` and are not alerted
on here — derive them from the audit stream, and tune any rate-based alerting per
deployment. The reference ships no numeric verdict thresholds; see
[on-call quick reference](../../docs/reference/on-call-quick-reference.md).

## Use it

1. Provision the **Auditor** token to a file Prometheus can read (referenced as
   `credentials_file` in `prometheus-scrape.yml`) — never inline the secret.
2. Merge `prometheus-scrape.yml`'s job into your `prometheus.yml` `scrape_configs:`,
   pointing `targets` at your gate's admin `host:port`.
3. Reference `alerts.yml` from `rule_files:` in `prometheus.yml`.
4. In production, drop `insecure_skip_verify` and pin a real CA (`ca_file`), and — if the
   admin plane runs behind mTLS (`gate -admin-mtls-ca`) — add a client `cert_file`/`key_file`.
