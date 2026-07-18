# Observability and SIEM integration

**Quadrant:** How-to. **Audience:** SRE and security engineers wiring humanymous Gate into your monitoring and SIEM.

This guide shows you what observability surface the reference implementation of humanymous Gate actually exposes today, how to pull edge decisions out of it with `curl`, and how to bridge that data to a SIEM until the production-only pieces (a metrics endpoint, health probes, native log shipping) are in place.

> **Important:** This repository is a reference implementation, not a production-hardened build. Several observability features an SRE would expect — a Prometheus metrics endpoint, load-balancer health probes, and structured-log SIEM shipping — are not in the reference. This page is candid about that and gives you a working interim path.

## What exists today

The reference Gate gives you three observability surfaces, all on the authenticated admin listener (`-admin-addr`, default `:8445`), never on the public edge:

1. **The audit stream** — `GET /__hmn/admin/audit`, structured JSON records of every edge decision, with server-side filters and a paging cursor. This is your primary machine-readable feed.
2. **The Ledger Overview KPIs** — a human-facing live feed of allow/challenge/deny decisions with rollup counters, at `https://localhost:8445/__hmn/admin/console`. See the [Ledger tour](audit-console-tour.md).
3. **Chain-integrity status** — the Integrity view and `GET /__hmn/admin/integrity`, which verify the tamper-evident audit log live (append-only hash chain + per-record HMAC + Ed25519 Signed Tree Heads) using the public key alone.

Observability today is the audit stream plus these two console views. There is no separate metrics or telemetry pipeline.

## What does NOT exist (do not look for it)

> **Warning:** Do not point a load balancer health check or a Prometheus scraper at Gate. Neither surface exists in the reference, and the closest-looking path is a trap.

- **No Prometheus `/metrics` endpoint.** There is no metrics-exposition surface in the reference build.
- **No `/healthz` or `/readyz` probe.** There is no Gate health or readiness endpoint.
- **The `/health` route is not a Gate health check.** `/health` is an *origin application* path that ships mapped to the `off` preset — meaning Gate does not inject or enforce on it. It is a documented **bypass** of detection, not a liveness signal for Gate itself. Treating it as a health probe would tell you nothing about Gate's own state and would route probe traffic straight through unscored.

If you need a coarse liveness signal in the interim, exercise an authenticated admin endpoint (for example `GET /__hmn/admin/whoami`) and treat a `200` as "the admin listener is up." This is not a substitute for a real readiness probe; see [Production vs. reference](../reference/production-vs-reference.md).

## Authenticate to the admin API

The admin API base is `/__hmn/admin/` on the admin listener. Every request needs a bearer token; the comparison is constant-time, and a missing or invalid token returns `404` (deny-by-default — the admin plane does not advertise itself). Every authenticated access is meta-audited (an `admin.access` record) before anything is served.

Tokens are configured through the `HMN_ADMIN_TOKENS` environment variable (`"auditor:tok,operator:tok,approver:tok,dpo:tok"`); if unset, Gate mints random tokens per boot and prints them at startup. For observability you only need read access, so use the **Auditor** token — Auditor is read-only (`canRead`) and cannot request or approve any change. See [RBAC and separation of duties](../reference/rbac-separation-of-duties.md) for the full role matrix.

Set your token once:

```
export HMN_ADMIN_TOKEN="<your-auditor-token>"
```

## Pull edge decisions from the audit stream

`GET /__hmn/admin/audit` returns edge-decision records as structured JSON. It supports these server-side filters, which you can combine:

- `verdict=` — `allow`, `challenge`, `deny`, or `none`
- `host=` — origin host
- `route=` — matched route
- `rule=` — a hard-rule identifier (for example a specific HR)
- `minRisk=` — minimum risk score (0–100)
- `before=` — the paging cursor

Fetch the most recent deny decisions:

```
curl --silent \
  --header "Authorization: Bearer ${HMN_ADMIN_TOKEN}" \
  "https://localhost:8445/__hmn/admin/audit?verdict=deny"
```

> **Note:** The reference dev certificate is self-signed in memory. Against a dev instance you will need `curl --insecure` (or `-k`) to skip certificate validation. Do not carry that flag into any trusted environment.

### Page through history with the cursor

The response carries a `nextBefore` cursor. To page backward through older records, pass that value as the next request's `before=` parameter; repeat until the cursor is empty (no more records).

```
curl --silent \
  --header "Authorization: Bearer ${HMN_ADMIN_TOKEN}" \
  "https://localhost:8445/__hmn/admin/audit?verdict=deny&before=<nextBefore-from-previous-page>"
```

The response envelope is `{"records":[…],"count":<n>,"nextBefore":<seq>}` — records newest-first; page the next (older) batch with `?before=<nextBefore>`. `limit` defaults to 50, max 500.

Each record's canonical fields for a SIEM mapping: `event_id`, `seq`, `node_id`, `ts` (RFC3339 nanoseconds, UTC), `event_type`, `event_version`, `actor{kind,id_pseudonym}`, `tenant_id`, `session_pseudonym`, `host`, `route_class` (`html|api|upgrade|control|static`), `verdict` (`allow|challenge|deny|none`), `risk_score`, `triggered_rules` (`["HR-…"]`), `top_signals[{id,verdict,conf}]`, `enforcement_action`, `enforcement_mode` (`monitor|shadow|enforce`), `fail_mode` (`none|fail_open|fail_closed|degraded`), `fail_reason`, `latency_us`, `upstream{status,error_class}`, `tls{ja4_pseudonym,h2fp_pseudonym,sni_pseudonym}`, `config_version`, `key_id`, `data_class`, the chain fields `prev_hash`/`record_hash`/`hmac`, and `incident` (the opaque handle, added at read time). Note the subject is a pseudonym (`actor.id_pseudonym` / `session_pseudonym`), never a raw identifier. The full shapes live in the [CLI, config & policy reference](../reference/cli-config-policy.md#request--response-shapes).

### Narrow to what you care about

Combine filters to build focused queries an on-call engineer or a scheduled job can run. For example, high-risk challenges on the checkout route:

```
curl --silent \
  --header "Authorization: Bearer ${HMN_ADMIN_TOKEN}" \
  "https://localhost:8445/__hmn/admin/audit?verdict=challenge&route=/checkout&minRisk=50"
```

Records are pseudonymous: raw identifiers (IP, TLS/HTTP fingerprints, UA, device fingerprint) are stored only as per-subject-key-derived pseudonyms, never raw. Your SIEM will ingest pseudonyms, not source IPs. Re-identification is a separate, dual-controlled action and is out of scope for a monitoring feed.

## Check chain integrity

`GET /__hmn/admin/integrity` runs the audit-log verification live and reports whether the tamper-evident chain is intact — the same logic the Console's Integrity view surfaces. Poll it on a schedule and alert on any failure class.

```
curl --silent \
  --header "Authorization: Bearer ${HMN_ADMIN_TOKEN}" \
  "https://localhost:8445/__hmn/admin/integrity"
```

The verification can report these mismatch classes, any of which should page your security on-call: hash-break, hmac-invalid, seq-gap, linkage-break, checkpoint-mismatch, and node-missing (a suppression alert).

`GET /__hmn/admin/integrity` returns `{"node","ok":<bool>,"class":"<mismatch-class>","records":<n>,"checkpoints":<n>,"witnessed":<bool>,"lastSTH":{"treeSize","root"}}`; on failure it adds `divergentSeq`, `detail`, and (if the witness attestation fails) `witnessFailAt`. Alert on `ok:false` or `witnessed:false`. The mismatch classes are `hash-break`, `hmac-invalid`, `seq-gap`, `linkage-break`, `checkpoint-mismatch`, and `node-missing` — see [Verify the audit log](verify-audit-log.md).

## Bridge to a SIEM in the meantime

Native SIEM shipping is a production concern the reference does not implement (see below). Until then, the supported pattern is **poll-and-forward**:

1. Run a small collector job on an interval.
2. Call `GET /__hmn/admin/audit` with an Auditor token, filtering to what you retain (or leaving filters off to capture all decisions).
3. Follow the `nextBefore` cursor to drain new records since your last run, persisting the cursor between runs so you never miss or double-ship.
4. Map each record's fields to your SIEM schema and forward.

This gives you allow/challenge/deny decisions, risk scores, matched routes, and triggered hard rules in your SIEM without waiting for native shipping. Because the feed is pull-based and the records are pseudonymous, your collector needs only network reach to the admin listener and a read-only token — keep that token scoped to Auditor and rotate it like any other credential.

> **Note:** You are polling, not streaming. Your SIEM's freshness is bounded by your poll interval. Size the interval against your audit volume so a single page-drain keeps up between runs.

## Explicitly production-only (prod-delta)

The following are **not** in the reference build. Do not document or configure them as working; plan for them as production deltas:

- A Prometheus `/metrics` exposition endpoint.
- Health and readiness probes (`/healthz` / `/readyz`) for load balancers and orchestrators.
- Native structured-log SIEM shipping in a standard schema (for example OCSF or CEF).
- Server-sent-events (SSE) live-push for the Console and downstream consumers — the reference Console refreshes and polls rather than receiving pushes.

For the full list and the reasoning behind each, see [Production vs. reference](../reference/production-vs-reference.md).

## Related

- [CLI, config, and policy reference](../reference/cli-config-policy.md) — the `-admin-addr` flag, `HMN_ADMIN_TOKENS`, and other startup levers.
- [Ledger tour](audit-console-tour.md) — the Overview KPIs and Integrity view described here, in the UI.
- [Production vs. reference](../reference/production-vs-reference.md) — the complete prod-delta list.
