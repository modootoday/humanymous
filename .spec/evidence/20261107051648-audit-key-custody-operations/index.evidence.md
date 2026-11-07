---
id: evidence:root/20261107051648
title: Audit key custody operations
status: active
created: 2026-11-07T05:16:48Z
updated: 2026-11-07T05:30:29Z
rel:
  governs: [crates/audit/**]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 6353564a29b2c27fb2dc09ae2c2fbe10de7ffa5c3d235b1c9934336fa840ca14, bytes: 11118, store: repo}
  - {path: facts-v2.json, sha256: 26d09af328330c737b949846e8342ccd0a383eb84ec1420353cd7ae0ef6275b1, bytes: 11316, store: repo}
---

# Audit key custody operations

## What was measured

Validated working-source manifest was captured before final execution and checked
unchanged afterward. Actual audit all-target suites passed Windows81/Linux77;
the binary zero-test harness and four ignored subprocess entrypoints are not
success counts. Parent tests explicitly execute native children. The final shared
witness client and separate sealed-key child process are included in witness
12Windows/11Linux cases. Policy5, backup8/7, operational control/custody/sink7/5
and frozen compatibility suites are included in the totals.

Raw command outputs, the pre-client78/75 runs, failed Linux locked-resolution
attempt and retry, source-before-final snapshots, environment and chronology are
registered under private evidence:root/20261107051649. Prior failure/tool errors
are not converted into green cases. Shared Cargo files were not modified by this
owner. facts.json contains only source/command/output hashes and sanitized facts.

Current working snapshot and canonical Git source are distinguished: manifests
describe actual working bytes; post-commit blob comparison is recorded privately
at handoff. Full exact-commit remote six-tuple qualification remains root-owned.

## How to read it

Latest validated working-source record is facts-v2.json with final attempt3
outputs. facts.json and attempt2 outputs remain as their distinct prior snapshot;
the later delta corrected stale legacy rustdoc, not runtime logic. The final
attempt3 source manifest remained unchanged through both full native suites,
both Clippy checks and WASM library check. Full repository Specfold still has
out-of-lane generated/private-artifact findings; scoped audit documents/bundles
are checked separately and are not reported as a globally clean repository.

Rust1.99.1 component/native filesystem qualification, including actual process
and local Docker executions, does not imply whole-product e2e. Windows reports
namespace power-loss uncertainty; FileAndDirectory requirements reject weaker
receipts. Whole-directory rollback defense depends on independent signed pin
custody and trusted clock. Separate deployed account/HSM/mount qualification,
authority/runtime wiring, shared startup owner, body-bound candidate rotation,
streaming recovery, remote sinks and physical/unknown-copy erasure remain gates.

Typed contracts and consumer status are canonical in [[sot:root/audit-key-custody]];
remaining ownership/order and acceptance criteria are in
[[plan:root/20261107002005]] and crates/audit/KEYS-INTEGRATION.md.
