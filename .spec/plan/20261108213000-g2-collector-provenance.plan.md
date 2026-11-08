---
id: plan:root/20261108213000-g2-collector-provenance
title: G2 / R03 fresh session-bound collector provenance — design delta
status: active
created: 2026-11-08T21:30:00Z
updated: 2026-11-08T21:30:00Z
rel:
  depends_on:
    - sot:root/runtime-application
    - sot:root/client-collector
    - plan:root/20261107055000
    - plan:root/20261107061820
  governs:
    - crates/sdk/humanymous-runtime/src/gateway/session.rs
    - crates/sdk/humanymous-runtime/src/gateway/application.rs
---

# G2 / R03 fresh collector provenance — plan delta

Brief: Matrix G2 / plan R03 keep `required.collector` at unconditional 422
because collect stores a session-bound `ClientReport` without a server-owned
receipt that separates server-issued epoch binding and receive time/counter from
client-asserted version and guard coverage. This delta specifies that receipt and
the observation-style acceptance join against **live** fields; detection freeze;
no invent product green.

## 1. Problem

`collect` / `collect_browser` can 200/ACK while `required.collector` always 422.
Inventing `has_report => accept` violates SoT (report ingestion ≠ fresh
provenance). Cap=1 `--collector` feeder ≠ G2 closed. Seam package exists without
an acceptance predicate.

## 2. Vocabulary / IDs

| ID | Meaning |
| --- | --- |
| G2 | Session-bound real browser collector product gap |
| R03 | Operational row: fresh collection provenance/coverage |
| FreshCollectorProvenance | Server-owned receipt on the volatile session |
| Partial guard | `guard.reported=false` (any of eval/Function/script uncovered) |
| Complete guard coverage | `guard.reported=true` per existing `Epoch::snapshot` semantics |

## 3. Decision rules

G2-D1. **Server-owned receipt.** On successful collect (after next-token
consumption), write `FreshCollectorProvenance` on the session. Do not treat the
stored `ClientReport` alone as provenance.

G2-D2. **Distinguish fields.** Receipt records server-issued/observed
`session_id`, `consumed_counter`, `received_at_ms` separately from
client-asserted `engine_version` and guard coverage counts/`reported`. Client
fields remain assertions, not execution attestation.

G2-D3. **Epoch binding without new wire.** Server-issued collection epoch binding
for this first profile is `(session_id, consumed_counter at collect)`. No new
ClientReport wire field and no scoring change.

G2-D4. **Acceptance predicate.** `required.collector` is no longer unconditional
422. Authorize only when the live session has a receipt with nonempty
`engine_version` prefixed `rust-wasm-` and `guard.reported == true`. Missing
session, missing receipt, empty/foreign version, or partial guard → 422
pre-consumption (no ACK).

G2-D5. **Partial never completes by signature.** Default/partial collector
snapshots (`reported=false`) still fail `required.collector` after 200 collect —
preserves existing fail-closed pilot/fixture expectations.

G2-D6. **Honesty split.** Observation-gated authorize is allowed once G2-D1–D5
land. Configuration readiness, matrix `required_unavailable`, and `pass_product*`
remain false until actual browser/product binding is measured. Cap=1 pure green ≠
product green. No detection-score edits.

## 4. Wiring

| Surface | Path |
| --- | --- |
| Receipt + lookup | `crates/sdk/humanymous-runtime/src/gateway/session.rs` |
| Capability gate | `.../gateway/application.rs` (`required.collector` like `original_h2`) |
| SoT | `runtime-application.sot.md` Collection; `client-collector.sot.md` join note |
| Plans | this file; checkbox on `20261107055000`; R03 status on `20261107061820` |
| Evidence | `.spec.local/root/evidence/20261108213000-g2-collector-provenance/` |

## 5. Verification

- Cap=1: clippy `-D warnings`; `missing_client_is_challenge_and_missing_planes_are_not_ready`
  still 422 without provenance; new pure unit(s) for partial-still-422 and
  complete-receipt gate pass (capability only; not product).
- Collector runtime_pilot / fixture expected 422 after partial collect: unchanged.
- Hosted browser/product: **not** claimed. `product=0`.

## 6. Critique reflected

- Blue: reuse live `engine_version` / `guard.reported`; no second scoring path.
- Red P0: `has_report` alone refuses; partial signed reports refuse; foreign
  engine_version prefix refuses.
- SRE: pre-consumption 422; no ACK; no retry invention.
- DPO: receipt stores counts/version only; no raw report body persistence beyond
  existing session report slot.
- Evaluator: readiness/product/matrix stay non-green without hosted measurement.

## 7. Residual

Full loader/distribution, cookie/streaming profiles, actual browser product lab
binding, and matrix G2 cell clearance remain open. Detection freeze.
