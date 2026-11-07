---
id: evidence:root/20261107111706
title: Audit key custody canonical S1 vector validation
status: active
created: 2026-11-07T11:17:06Z
updated: 2026-11-07T11:18:16Z
rel:
  governs: [crates/audit/src/record]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 87ea9d9573d0308ede9aeb44c1bb5454e76dfe6438f5e3f22febb577d4c3902b, bytes: 1351, store: repo}
---

# Audit key custody canonical S1 vector validation

## What was measured

Allocated local Windows A at frozen sourcef88e76f4, jobs1/incremental0/thread1:
all-targets locked check2.86s and warning-denying Clippy3.12s passed. Canonical
selector6, normalization selector2 and record_parity target1 all passed:9 Rust
tests, zero failed/ignored. Library selectors intentionally filter other cases;
record_parity has zero filtered cases.92 unique frozen Go canonical records and
13464 Unicode15 texts were checked. The same92 canonical fixtures are exercised
by the emitter and parity test, not184 unique cases. Source114 files before/after
has zero drift, including local dependencies and root lock/toolchain.
Original command outputs/manifests remain private in evidence111549.

## How to read it

This proves current canonical/NFC vector and budget/fault-test compatibility,
not complete S1, measured allocation128MiB, production recovery or product e2e.
No new native service/SCM/accounts/network/Docker activity was part of this batch.
Root separately renewed A for a full audit regression; that result is recorded
separately and must not be added to these overlapping9 cases as unique tests.
