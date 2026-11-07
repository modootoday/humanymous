---
id: evidence:root/20261107122437
title: Audit key custody full current regression
status: active
created: 2026-11-07T12:24:37Z
updated: 2026-11-07T12:26:32Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 062260a150dc307e54a4cb638f7c2b4ba8752bff4e9899f314ed181f858f967c, bytes: 5603, store: repo}
---

# Audit key custody full current regression

## What was measured

Root-allocated C, captured source319a5992 (audit Rust14f3fbdd), ordinary-user
Windows/Rust1.99.1, jobs1/incremental0/thread1, exclusive target/validation-audit.
Full audit package all-target locked tests completed exit0 in205.87seconds.
Actual140 passed, failed/filtered/measured0. Exactly5 named ignored helpers are
excluded;2 zero-test binaries are excluded.11 outer harnesses each have exactly
one summary and discovered=passed+ignored. No standalone child markers leaked.

Library81 pass/1ignore; checkpoint parity2; controls21; witness13/1ignore;
backups10/1ignore; custody10/2ignore; Merkle/record/verifier parity1 each. Actual
WAL exclusion/rotation/checkpoint/restart parent, witness lifetime/interruption,
backup crash-reconciliation parent and both custody child-owning parents pass.
Retained backup refusal/sharing-recovery, fresh retained rotation/erasure and
actual stored-authority/control state regressions pass. Crash fixture success is
not proof of power-loss or arbitrary crash-stage coverage.

The captured128 before/after path hashes, including test-support/lib.rs, are
identical. Two new parent_context Rust files appeared during the run and are
absent the actual Audit target test-support depfiles. Broad recursive inventory
zero drift is NOT claimed. After the closing snapshot (12:22:53.673Z), lib.rs
activated the module (12:23:11.607Z) and its hash changed. This140 result does NOT
qualify that later primary-tree dependency change or execute parent_context tests.
Captured before/after/current hashes, depfiles and corrected classification are
attached to private evidence:root/20261107121350. First derived classification
incorrectly called the current module unwired despite observing its activation;
the original attempt is preserved and corrected, with no test rerun or hidden
failure. Session95952 terminal exit0/C returned, no process remains.

## How to read it

Complete audit package regression at the captured source/compiled closure only;
not workspace green, current later dependency activation, protected service/SCM
profile, hosted OS matrix, remote witness independence or Docker product e2e.
No128MiB/RSS, complete S1, bounded operational recovery or production S2/S3 claim.
Focused11 tests overlap this140 run and must not be summed as151 unique tests.
