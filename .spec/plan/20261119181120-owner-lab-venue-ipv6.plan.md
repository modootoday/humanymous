---
id: plan:root/20261119181120
title: Lane I owner decisions #152-1 cloud-container venue and #152-2 IPv6 stack absence
status: active
created: 2026-11-19T18:11:20Z
updated: 2026-11-19T18:11:20Z
rel:
  governs: [crates/defender/src/lab/isolation.rs, crates/defender/src/lab/process.rs, crates/attacker/src/verification.rs, test/lab/runner/src/docker.rs, test/lab/runner/src/docker/affinity.rs, test/lab/runner/src/orchestration.rs, test/lab/runner/src/orchestration/qualification.rs, test/lab/runner/src/product_e2e.rs, test/lab/runner/src/product_e2e/ledger.rs, scripts/ci/delivery/src/pilot.rs, .agents/rules/96-docker-engine-affinity.md]
  depends_on: [sot:root/container-test-environments, sot:root/rust-wargame]
---

# Lane I owner decisions #152-1 and #152-2

## Goal

Implement the two lab decisions the owner delegated on issue #75 (comment
7017558784), raised by PR #152. Branch `claude/owner-lab-venue-ipv6`, based on
`origin/claude/lab-product-e2e-truth` (PR #152 head 3317810f) because both
change the same lab files.

- #152-1: an explicitly named cloud-container venue for the lab engine
  preflight, bound to an operator-supplied engine ID, recorded in the receipt
  with `authenticated_provenance: false`, and never able to make
  `full_product_qualified` true. Canon delta in
  `.spec/sot/container-test-environments.sot.md`; rule 96 matches it.
- #152-2: a directly observed absent IPv6 stack counts as no IPv6 routes and
  is recorded as `stack_absent`; every other read error stays fail-closed.
  Canon delta in `.spec/sot/rust-wargame.sot.md`.

## Steps

- [x] `isolation::ipv6` pure judgement over the route table read and the
  `if_inet6` and `/proc/sys/net/ipv6` observations; `Ipv6Evidence` recorded
  in `ProcessReceipt.ipv6`; the liveness recheck refuses changed evidence.
- [x] Tests: absent stack accepted and recorded; permission, invalid-data and
  other errors, malformed lines, non-loopback routes and mixed observations
  refused; IPv4 checks unchanged; the verifier rejects a missing or unknown
  `ipv6` value.
- [x] `docker::Venue` with `--venue cloud-container --engine-id ID` on lab
  `run` and `engine-preflight`; explicit `--host` socket; refusals on Windows,
  hosted Linux and engine overrides; ID binding in the preflight.
- [x] `Qualification::non_qualifying_venue` on the lab `run` and the
  `product-e2e` ledger; `engine_venue` in the lab report.
- [x] `product-e2e --engine-id ID` optional binding.
- [x] `humanymous-delivery pilot docker --venue cloud-container --engine-id ID`
  forwards the flags, forces `full` false and `validate` refuses the venue.
  `merge_gate.rs` is not touched.
- [x] SoT deltas, rule 96, attacker INTEGRATION, reviews.
- [ ] Docker product-e2e run in the cloud container with the new venue; the
  receipt facts go in the PR description, raw receipt stays private.

## Out of scope

Human-gated `scripts/ci/delivery/src/merge_gate.rs`; PR #153's readiness
change; the 7 unsupported attack-catalog entries.
