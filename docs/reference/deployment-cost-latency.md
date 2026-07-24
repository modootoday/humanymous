---
title: Deployment cost, latency & operational footprint
description: "Deployment cost of humanymous Gate: it adds four inline per-request work terms and no published benchmark — a sizing framework to measure in monitor mode."
keywords: ["humanymous Gate deployment cost","latency and operational footprint","reference implementation","inline reverse proxy","per-request latency","monitor mode measurement","ingress and resource bounds","single-node model","TLS termination cost","audit hash-chain write","capacity planning SRE","no published benchmark"]
---

# Deployment cost, latency & operational footprint

**Diátaxis quadrant:** Reference. **Audience:** SREs and platform/infrastructure owners sizing the buy — deciding what capacity, failure budget, and operational surface a humanymous Gate deployment adds.

This page is honest about one thing up front: this repository is a **reference implementation, not a production-hardened build**, and it ships **no published benchmark**. There are no reference latency, throughput, RAM, or CPU numbers to quote, and this page does not invent any. What it gives you instead is the shape of the cost — what work Gate adds per request, the ingress and resource bounds that are actually defined in code, the failure-domain reality of putting a TLS-terminating reverse proxy in the request path, and the single-node model — so you can plan a measurement rather than trust a marketing figure.

humanymous Gate ("Gate" after first mention) is the reverse-proxy enforcement layer: it terminates TLS, streams the detection bundle into HTML, scores layers L1–L7 inline, enforces the verdict at the edge, and writes every decision to a tamper-evident audit log — sitting in front of an origin app it fronts but does not control.

For the reference-vs-production boundary, see [Production-ready vs reference (prod-delta)](./production-vs-reference.md). To capture your own numbers, see [Observability & SIEM](../how-to/observability-siem.md) and [Self-validation & red-team](../how-to/self-validation-red-team.md). For the request-path impact on your app's behavior, see [Will this break my app?](../explanation/will-this-break-my-app.md).

> **Important:** Every specific figure a vendor or a benchmark gives you is meaningless until it is reproduced on your hardware, with your traffic mix, at your policy strictness. Treat this page as a sizing framework, and measure in monitor mode before you commit capacity.

---

## What adds per-request cost

Gate is inline. Every request that passes through it does more work than a plain reverse proxy would. The added cost is qualitative here — the reference publishes no per-item timing — but these are the four sources of per-request work you are buying:

- **TLS termination.** Gate terminates TLS at the edge (minimum TLS 1.2). That is the standard handshake-plus-symmetric-crypto cost of any terminating proxy, and it is also what makes the L5 network/protocol signals (JA3/JA4-family TLS fingerprint, HTTP/2 fingerprint, header order) observable at all.
- **Streaming HTML injection.** For HTML responses, Gate streams the detection bundle into the page in a single pass — add-only and idempotent. This adds parsing/injection work on the response path for HTML content types (non-HTML responses are not rewritten).
- **Inline L1–L7 scoring.** Gate aggregates the L1–L6 signals into a 0–100 risk score and then applies the hard rules (L7), producing an ALLOW / CHALLENGE / DENY verdict **inline, before the verdict takes effect**. This is synchronous decision work on the request path, not an out-of-band analysis.
- **The audit hash-chain write.** Every decision is emitted to the tamper-evident audit sink **before it takes effect**: an append-only hash chain with a per-record HMAC, plus an Ed25519 Signed Tree Head checkpoint every 32 records and an independent local witness co-sign. That is durable write work in the critical path of each decision.

Qualitatively, the added per-request latency is the sum of those four work terms:

$$t_{\text{Gate}} \;=\; t_{\text{TLS}} + t_{\text{inject}} + t_{\text{score}} + t_{\text{audit}}$$

A valid, fingerprint-bound verdict trust token short-circuits re-scoring on the ALLOW fast path, so repeat traffic from an already-verified session costs less than a first-time request — the scoring term drops out:

$$t_{\text{Gate}}^{\text{fast}} \;\approx\; t_{\text{TLS}} + t_{\text{inject}} + t_{\text{audit}}$$

The symbols are placeholders, not measured values — the reference publishes none. How much each term costs, and how much less the fast path costs, is something to measure, not a number this page will supply.

> **Confirmed in source:** The authoritative durable sink (the local write-ahead log) writes synchronously per record: `WALSink.AppendRecord` marshals the sealed record, appends it under a single-writer mutex, flushes the buffer, and calls `fsync` before returning — so "sealed" means "on disk" before the enforcement side effect is released. It is not batched or buffered-before-ack; there is one fsync per record, and that fsync is the durability boundary backing the "emitted before the decision takes effect" ordering guarantee. (Segment rotation and checkpoint writes are fsync'd as well.)

---

## Ingress and resource bounds that ARE defined

These limits are set in the reference and apply to **both listeners** (the `:8444` edge and the `:8445` admin listener). They are the guardrails the process enforces regardless of tuning — useful for capacity planning, timeout budgeting, and understanding what a slow or oversized client will hit.

| Bound | Value | What it governs |
|-------|-------|-----------------|
| `ReadHeaderTimeout` | 8s | Max time to read request headers before the connection is cut. |
| `ReadTimeout` | 30s | Max time to read the full request. |
| `WriteTimeout` | 30s | Max time to write the response. |
| `IdleTimeout` | 60s | Max keep-alive idle time before the connection is closed. |
| `MaxHeaderBytes` | 1 MiB | Cap on total request header size. |
| HTTP/2 `MaxConcurrentStreams` | 100 | Max concurrent streams per HTTP/2 connection. |
| HTTP/2 `MaxReadFrameSize` | 1 MiB | Cap on a single HTTP/2 frame. |
| HTTP/2 `MaxUploadBufferPerConnection` | 1 MiB | Per-connection upload buffer ceiling. |
| TLS minimum version | 1.2 | Handshakes below TLS 1.2 are rejected. |

These bounds also form part of Gate's defense against slow-client and protocol-abuse pressure (they cap how long a connection can hold resources and how large a frame or header block can be). They are limits, not a performance profile — they tell you the ceiling behavior, not the per-request cost.

Separately, the control-plane endpoints (`/collect`, `/session`) carry a per-IP rate limiter: a default 10s window with a soft threshold of 60 and a hard threshold of 120 (a `429` on the hard threshold). That protects the beacon path; it is a rate control, not a throughput figure. The hard threshold sets a per-IP ceiling on the beacon path of

$$R_{\text{hard}} = \frac{120\ \text{requests}}{10\ \text{s}} = 12\ \text{req/s per IP}$$

which is a defensive cap, not a capacity target for the edge as a whole.

---

## Failure-domain reality

Putting Gate in the request path is a deliberate architectural change, and an SRE should size it as one.

- **You are adding a new failure domain.** A TLS-terminating reverse proxy in front of your origin is now on the critical path for every request it fronts. If Gate is unavailable, the traffic it fronts is affected — this is true of any inline proxy and is the central availability trade-off. The origin still runs, but clients reach it through Gate.
- **Fail-open vs fail-closed is asymmetric by design.** When a verdict is Unknown, Gate **fails closed** (blocks) if the route is `strict` **or** the method is unsafe (POST/PUT/PATCH/DELETE), and **fails open** (passes) only for safe GET/HEAD on non-`strict` routes. The fail-open case is a documented, accepted residual risk, metered by fingerprint/subnet rate controls. For your capacity model this means: under partial-failure or overload conditions, mutating and strict traffic is denied rather than degraded, while safe reads may still pass.
- **The kill switch is fleet-wide.** The kill switch demotes hard-rule enforcement to monitor mode across the whole fleet — detection stops and traffic flows, while manual bans still enforce. It is a dual-control action (committed by a distinct Approver). Size your incident response around the fact that this lever is all-or-nothing, not per-route.

> **Warning:** The kill switch acts **fleet-wide**, not on a single node or route. Engaging it demotes hard-rule enforcement to monitor everywhere at once. Treat it as an availability-recovery lever of last resort, and rehearse the dual-control approval path before you need it.

For how these failure modes translate into observable app behavior, see [Will this break my app?](../explanation/will-this-break-my-app.md).

---

## The single-in-process-node model

The reference runs as a **single in-process node**. This is the most important sizing constraint and the one most likely to surprise someone planning for horizontal scale.

- **Verdict store and bans are in-process.** Verdict trust tokens and IP/fingerprint bans live in memory on one Gate instance. They are not shared across instances in the reference.
- **There is no shared/HA state in the reference.** A shared store (for example Redis) for fleet-consistent verdicts and bans, high availability, and horizontal scale is **prod-delta** — a production responsibility, not something the reference binary provides.
- **Sizing implication.** With a single node, capacity is one process's capacity, and there is no built-in failover. Running more than one instance without externalized state means each node scores and bans independently, so a ban or a verdict on one node is not visible on another. Plan the shared-state layer as part of any multi-node deployment.

See [Production-ready vs reference (prod-delta)](./production-vs-reference.md) for the full list of what a production deployment must add, including the shared fleet-state layer.

---

## Measure your own numbers — in monitor mode

Because there is no published benchmark, the only trustworthy numbers are the ones you produce on your own hardware. The reference gives you the tools to do that safely.

1. **Deploy in monitor mode first.** In monitor (shadow) mode, Gate scores and records but does not enforce — so you can measure the added request-path cost against real traffic without denying anyone. See the global `-monitor` flag and the `monitor` preset in [CLI, config & policy reference](./cli-config-policy.md).
2. **Read the cost off the audit stream and KPIs.** The reference's observability surface is the audit stream (`GET /audit` with filters and a cursor), the Integrity view/endpoint, and the Overview KPIs in the Ledger — there is **no** Prometheus `/metrics` endpoint and **no** health/readiness probe. Plan to derive latency and throughput from your own load generator plus these signals, and to ship the audit stream to your metrics/SIEM stack. See [Observability & SIEM](../how-to/observability-siem.md).
3. **Load-test with the red-team harness.** The bundled red-team harness and simulators exercise your **own** deployment end-to-end (`cmd/redteam`, `test/redteam/*.mjs`, the e2e runner). Use them against your target to observe behavior under load. See [Self-validation & red-team](../how-to/self-validation-red-team.md).

> **Note:** Any latency or throughput figure sourced from the reference — including anything measured on the maintainers' hardware — is **reference-measured**. It is a starting hypothesis, not a capacity guarantee. Re-measure on your hardware, with your policy strictness and traffic mix, before you size on it.

> **TODO(verify):** Reference-measured baseline latency/throughput/RAM/CPU figures on a stated hardware profile. None are published in the ground truth; this page states none. If the operator produces them, record the hardware, traffic mix, and policy preset alongside each figure and label them "reference-measured — measure on your hardware."

---

## Related pages

- [Production-ready vs reference (prod-delta)](./production-vs-reference.md) — the full reference-vs-production boundary, including shared fleet state and HA.
- [Observability & SIEM](../how-to/observability-siem.md) — what to measure with and where the signals come from.
- [Self-validation & red-team](../how-to/self-validation-red-team.md) — load-test and validate your own deployment.
- [Will this break my app?](../explanation/will-this-break-my-app.md) — how the failure modes above show up in app behavior.
- [CLI, config & policy reference](./cli-config-policy.md) — flags, presets, and the monitor mode you measure in.
