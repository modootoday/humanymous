---
id: evidence:root/20261107120052
title: Audit key custody frame checkpoint scoped validation
status: active
created: 2026-11-07T12:00:52Z
updated: 2026-11-07T12:01:56Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 3a4721577e1ba76eb742f0731c2d37c8dba6988ae46b522ea843432240ea9928, bytes: 819, store: repo}
---

# Audit key custody frame checkpoint scoped validation

## What was measured

Root-allocated A on ordinary-user x86_64 Windows, Rust/Cargo1.99.1,
jobs1/incremental0/thread1, exclusive target/validation-audit. Sourcec26a5b53
plus explicit preflight depth:usize annotation, with shared signals56a2acdb.
Combined signals/audit all-target locked check and strict Clippy passed.

Four focused harnesses passed26 unique Rust tests, failed/ignored/measured0:
signals wire6 (filtered6), audit framing13 (filtered63), audit checkpoint5
(filtered71), and checkpoint_parity2 (filtered0). Filters are intentional scope,
not missing tests represented as successes. Checkpoint5 includes the existing
witness rollback case, not just four message tests. Library selectors are
disjoint; do not add fixture rows to Rust test counts.

The wire test compares all1112064 valid Unicode scalars in one text against the
prior serde+five replacements oracle and exercises partial writer failure.
Preflight checks92 frozen record and180 checkpoint inputs against independent
owned count traversal; message tests check180 frozen checkpoint bytes and exact
budget boundaries. The unchanged Go parity tests independently verify actual
writer/witness signatures/HMACs and append-only witness state across180 cases.
These fixture rows overlap across tests and are not additional unique tests.

First check failed E0689 on ambiguous numeric depth, with no later command run.
Only explicit usize annotation changed; second fresh admission/snapshot batch
passed. All raw logs and failure are preserved at evidence:root/20261107115851.
Second125-path closure has zero hash and path inventory drift, including all
audit/signals/native-security/test-support files and root manifests/lock/toolchain.
Session40021 terminal exit0; A returned with no active process.

## How to read it

This is scoped component verification, not complete audit regression, native
pilot, protected/hosted profile, actual product e2e or complete S1 qualification.
Preflight remains test-compiled pending the bounded read-only inspector consumer.
No whole-startup128MiB/RSS, authenticated EOF or production S2/S3 claim follows.
Historicalf88 full115 remains exact historical source, not this source's full run.
