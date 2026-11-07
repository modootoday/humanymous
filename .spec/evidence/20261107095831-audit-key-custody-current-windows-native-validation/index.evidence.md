---
id: evidence:root/20261107095831
title: Audit key custody current Windows native validation
status: active
created: 2026-11-07T09:58:31Z
updated: 2026-11-07T10:02:27Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 35d3f91c1501cf586a9abe276a9e3db615bb96b923e61b58a1c72bd06cfc568a, bytes: 1818, store: repo}
---

# Audit key custody current Windows native validation

## What was measured

Root-allocated A, exclusive target/validation-audit, Windows ordinary-user token
(Administrator role disabled), Rust1.99.1, jobs1/incremental0/testthreads1.
Latest audit source c09d6897 and its predecessor test/retained-erasure changes
passed all-targets locked check and warning-denying Clippy. Four reviewed pure
selectors passed10 tests with zero failures/ignored; their filtering is explicit.

After separate native continuation authorization, all three reviewed targets ran
serially: control_operations21passed/0ignored, durable_witness13passed/1ignored,
key_custody10passed/2ignored. Total44 native passes; rawignored3 excluded.
Each emitted exactly one outer summary, exit0/fail0/filtered0/measured0 and
discovered=passed+rawignored. Exact helper names match facts.json; both custody
parents and witness parent passed, with no outer child-marker leakage.

Actual cases include retained intent/absence and expired kill intent, retained
erasure with fresh exact recovery, foreign-scope pending refusal, live ledger
loss/corruption/rollback, independent retainer child restart, witness stalled
startup deadline, wrong-secret stderr, missing checkpoint repair, ambiguous tail
preservation, verified local sink restart/tamper and Windows ACL/reparse/rename
fault handling. Recursive111-file closure before/static-after/native-after has
zero drift, including audit, shared local dependencies, root manifests/lock and
toolchain. Original logs and source manifests remain private in evidence95528.
All handles terminal; A returned. No SCM/accounts/network/Docker were exercised.

## How to read it

facts.json contains sanitized counts, exact source-manifest hash, per-log hashes
and harness footer seconds. This is direct ordinary-user Windows component
qualification, not genuine hosted, complete audit/all-workspace or product e2e.
Metadata reconciliation is not the strict native-actor executable/discovery
receipt. Pure10 and native44 are distinct selected scopes; ignored3 are not passes.
Backup target, remaining library tests, other OS tuples, protected service/account
profiles, remote sinks and actual product integration remain unqualified here.
No old8132117 passes are inherited. Footer times are not CPU/RSS measurements.
