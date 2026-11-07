---
id: evidence:root/20261107083903
title: Rust wargame hosted strict byte receipt
status: active
created: 2026-11-07T08:39:03Z
updated: 2026-11-07T08:41:34Z
rel:
  governs: [crates/attacker]
  depends_on: [sot:root/rust-wargame]
files:
  - {path: wargame-receipt.json, sha256: 0ae5938c13878bf71db675ea592b7107620689188ef3180cd041c97c7368406a, bytes: 3209, store: repo}
  - {path: run.json, sha256: 0fe6c220c33f952b916e444942d4d8bcc94fa959b6dab1678dfecc6630182a0b, bytes: 206, store: repo}
  - {path: docker-job.json, sha256: 58b5f02129d4088fa115f9102640ba4789107b37d6800539940fd40895c7690c, bytes: 1286, store: repo}
---

# Rust wargame hosted strict byte receipt

## What was measured

Actual run42355168729, attempt1, Docker job128351926068, source
8132117e7bf1445939d7dee9066fcf45ccf0915b. The Docker job failed, but its accepted
public strict-byte receipt was preserved and independently recovered. All26
indices0..25 are present. Recounted totals: scheduled26/evaluated0/errors19/
skipped0/unsupported7/expectation_mismatches0. Non-evaluated expectations are null.
The verifier reports internally_consistent=true, mode=owned_process_network,
scoped_passed=false, full_product_qualified=false. No prior19-case success is reused.

Original-report SHA256:
6af47402f4036613d25e7043a98d1ee4f244b2c000af5cd9a46734fa6f947fcc.
Lab source-manifest SHA256:
eb5eb569b3eec4886592f377f4945fe5478b676fca418ed42ad61705c616a2d4.
Receipt source equals the run head. The exact CI consumer independently hashes
the private original file and lab manifest before accepting this public receipt.
The private original body was not recovered locally; no local byte recomputation
is claimed. Lab assertions35=33passed+2failed; diagnostics name actual-owned-wargame
and execution at wargame_process. Delivery errors1/component-only/full=false,
source_changed_during_execution=false. Other workflow jobs were still running.

Source: [actual Docker job](https://github.com/modootoday/humanymous/actions/runs/42355168729/job/128351926068).
Original captured CLI log is private evidence:root/20261107082419. Extraction
removed timestamp prefixes and one interleaved known delivery-blocked stderr line;
that line remains in the captured log. Public JSON is LF-serialized projection,
not private original-report bytes. wargame-receipt.json is the actual recovered
ledger projection; run.json and docker-job.json preserve its execution identity.

## How to read it

Strict-byte verification and receipt recovery executed, but the runtime wargame
FAILED with19errors before evaluation. This is a valid failure ledger, not human/
AT or product qualification. Earlier startup_failure/zero jobs remains evidence081913.
Immutable813source inspection found that ServiceHost adds readiness metadata absent
from defender's expected configuration-only profile. Whole-object equality must
reject that envelope if reached. Private row reasons are unavailable, so this is
a source-proven blocker, not a directly recovered per-case error diagnosis.
Runtime-owned shared-builder repair and a fresh hosted round remain necessary.
