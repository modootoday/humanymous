---
id: plan:root/20261107143100
title: Remaining core responsibility implementation and qualification
status: active
created: 2026-11-07T14:31:00Z
updated: 2026-11-07T14:31:00Z
rel:
  depends_on: [sot:root/program-roadmap, sot:root/rust-migration]
---

# Core responsibility review and execution contract

Root delegated a read-only review of the 23 rows currently routed to
root-integration in the legacy owner-routing inventory. This review found four
rows with concrete Rust functionality and nineteen without equivalent behavior
in the inspected tree. This is a source-review result, not runtime parity or a
change to the authoritative 314-row shipping registry.

| Context | Rows | Existing implementation and next owner action |
| --- | --- | --- |
| Behavior | 2 | Existing shared feature extractor and 68-vector fixture; root reconciles actual source-specific receipts rather than duplicating extraction |
| Proof of work | 2 | Existing challenge work generator/target verifier and108-vector fixture; root distinguishes pure parity from actual issuance/verification integration |
| Abuse | 2 | Fingerprint sliding-window soft/hard levels, state-preserving configure and bounded GC/reset remain missing; design must preserve frozen observation semantics |
| Anomaly | 2 | Per-key median/MAD history, score-before-append, warmup/floor, bounded eviction and TTL/reset remain missing |
| Correlation | 4 | Capture owner prepares network-context design for fingerprint/subnet/session and velocity/fanout producers plus shared-coordinator degraded fallback |
| Pass | 3 | Root reviews exact deterministic generator/TTL/offset/instance contract before pure challenge-module implementation; HMAC attestation alone is not the puzzle protocol |
| Report | 4 | Wargame owner prepares bounded legacy aggregation, escaped deterministic HTML and a thin Rust report command in its existing report context |
| Traffic guard | 4 | Capture owner prepares bounded64-record session history, TLS-address backfill and exact consistency producers; registry IDs are not producer implementations |

No producer or state primitive may introduce different weights, thresholds,
hard-rule predicates or verdict bands. Existing disabled extension permutation
and header-order predicates stay disabled; traffic JA4 resumption tolerance and
rotation semantics need exact source/test preservation. Missing shared-provider
evidence is explicit degraded/unavailable state, never healthy required-signal
readiness. Inputs use caller-supplied time and finite budgets where appropriate;
new dependencies/manifests require root graph review.

Each owner first records exact baseline rows, Rust paths, compatibility deltas,
bounds and tests in its owned plan. Source preparation is not execution approval.
Units, actual process/file pilots and Docker composition remain separate gates.
Only root may merge reviewed runtime-parity or approved retirement dispositions
into the baseline shipping inventory. None is promoted by this plan.

Historical behavior/PoW evidence candidates include the a0b6dfc2 six-native
checkpoint and the8132117 Windows feature fixture. They must be inspected for
their exact source and outcome; an enclosing failed/incomplete workspace run
cannot qualify the current workspace or replace final actual integration.
