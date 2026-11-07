---
id: evidence:root/20261107114310
title: Audit key custody Merkle S1 scoped validation
status: active
created: 2026-11-07T11:43:10Z
updated: 2026-11-07T11:43:36Z
rel:
  governs: [crates/audit/src/merkle]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 9e645104bf3f5499132d1b04f40698dc7b0bf18d1d332a1ef7354f5fd135911a, bytes: 1114, store: repo}
---

# Audit key custody Merkle S1 scoped validation

## What was measured

Root-allocated A, ordinary-user Windows, jobs1/incremental0/thread1 and exclusive
target/validation-audit. Candidate9c5f4e7c plus test-only Clippy range-expression
correction passed locked all-target check and Clippy with warnings denied.
Merkle library selector8 passed/55 filtered; immutable Go parity1 passed with
no filters. Total9 Rust tests, zero failed/ignored/measured. The unchanged Go
oracle covers66 cases, sizes0..65,2145 inclusion paths and2080 consistency paths
with exact ordered proof-byte equality. Six new primitive tests include synthetic
maximum-u64 counters/descriptors without allocating huge histories.

First attempt passed check but failed Clippy manual_range_contains before tests;
failure retained, corrected expression and fresh second snapshots recorded.
Second119-path before/after inventory has one changed documentation path,
crates/test-support/INTEGRATION.md (two comparison side indicators). Remaining118
paths are identical, including all Rust/manifest/fixture inputs. No source or
manifest reference to that document was found; actual test-support Cargo depfiles
list only Rust modules and Cargo.toml. Broad119-path zero drift is NOT claimed.
Raw originals, admissions, snapshots, commands/timings, logs, oracle coverage and
depfiles: evidence:root/20261107114044. Session20220 terminal exit0; A batch ended.

## How to read it

Pure Merkle qualification only. No full audit suite, native process pilot,
protected profile, hosted matrix, product e2e or measured128MiB/RSS guarantee.
Legacy Tree still borrows full leaves and memoizes proof subtrees. Fixed proof
capacity follows the RFC path bound; it does not alter acceptance policy.
Historical115-pass full suite remains tied to sourcef88e76f4.
