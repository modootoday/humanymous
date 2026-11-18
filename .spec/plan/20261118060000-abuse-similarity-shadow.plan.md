---
id: plan:root/20261118060000
title: "Abuse-similarity shadow rollout"
status: draft
created: 2026-11-18T06:00:00Z
updated: 2026-11-18T06:00:00Z
rel:
  depends_on: [sot:root/abuse-similarity]
---

# Abuse-similarity shadow rollout

## Goal

Implement sot:root/abuse-similarity up to shadow operation without changing any
served verdict. Each slice lands as its own `claude/<slug>` pull request.

## Slices

| # | Slice | Done when |
|---|---|---|
| S1 | Pure kernel in `humanymous-core::model::similarity`: store, policy, shadow ledger, preview | Scoped `cargo test --locked`, fmt and strict Clippy pass |
| S2 | Offline separation check on Docker catalog sessions versus human controls | Separation and events-to-confidence report recorded; stop on a negative result |
| S3 | Runtime admin API and console view: get, validate, preview and save policy (audited by digest) | Gateway tests; policy save refused outside the bounds |
| S4 | Runtime shadow recording beside the served verdict; stats on the admin console | Docker product case shows the stats and verdicts unchanged |
| S5 | Promotion proposal | Needs-owner item only; no code until the owner lifts the freeze |

## Constraints

- Detection freeze: no weight, threshold, predicate or verdict band changes.
- Admission only from catalog sessions and operator confirmations.
- Permanent bans remain dual-control proposals.
- No raw vectors or subject identifiers are retained by the store.
