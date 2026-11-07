---
id: evidence:root/20261107121521
title: Audit key custody incremental verifier scoped validation
status: active
created: 2026-11-07T12:15:21Z
updated: 2026-11-07T12:16:19Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 98e24e2b88a13c59640577683990c99b09383501bc90f27c674e8f2c91ac0608, bytes: 674, store: repo}
---

# Audit key custody incremental verifier scoped validation

## What was measured

Exact source14f3fbdd65d3cee099680455d2392e1fa0a7e402, root-allocated A,
ordinary-user x86_64 Windows/Rust1.99.1, jobs1/incremental0/thread1 and exclusive
target/validation-audit. All-target locked check and strict Clippy passed on the
first attempt. No source correction or repeated command was required.

Four focused harnesses passed11 unique Rust tests, failed/ignored/measured0:
verification library7 (75 intentionally filtered), verification_parity1,
record_parity1 and checkpoint_parity2 (all integration filters0). This includes
the six new priority/scope/resource tests. Actual interleaving checks confirm
later record/base errors override early witness errors and preserve successful
prefix Scope while counting later actual checks separately.

Immutable Go840 Report rows match wire fields; incremental reports additionally
match native Scope from sequential verification. The same840 rows are exercised
inside the unit differential and integration parity, not1680 unique cases.
Canonical92 and checkpoint180 vectors also pass. Fixture rows are not additional
Rust tests and overlap previous focused batches; no cross-batch unique sum.

Full recursive128-path source closure and inventory have zero drift before/after
the allocated batch: audit/signals/native-security/test-support files plus root
manifest/lock/toolchain. All originals, admission, toolchain, command arguments,
timings, snapshots and logs are attached to evidence:root/20261107121350.
Session65593 terminal exit0. A returned; no process or further workload held.

## How to read it

Scoped compatibility/differential qualification only, not full audit suite,
native inspector/pilot, protected/hosted profile, product e2e or complete S1.
The interleaving fixture driver is intentionally a full-history test oracle.
The accumulator remains test-compiled; no authenticated EOF/source provenance,
healthy owner or business effect is produced. Phase-A metadata/exact pin and
actual bounded frontier join/native inspection remain unwired. No128MiB/RSS or
production S2/S3 qualification follows. Historical full115 remains tied tof88.
