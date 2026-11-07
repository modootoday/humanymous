---
id: evidence:root/20261107102348
title: Audit key custody full Windows validation observations
status: active
created: 2026-11-07T10:23:48Z
updated: 2026-11-07T10:28:32Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 2b9e55797d949ceeefa22c7f715a9da937722c7aff7f9cf71908434d02d2b347, bytes: 4973, store: repo}
---

# Audit key custody full Windows validation observations

## What was measured

Root-allocated A ran the complete current audit package on ordinary-user Windows,
Administrator role disabled, Rust1.99.1, jobs1/incremental0/testthreads1 and an
exclusive target/validation-audit. Admission80.11% commit/free33.34GiB.
All-targets locked check passed2.99s; warning-denying all-targets Clippy passed3.10s.
The locked all-targets test command completed exit0 with110 accepted case passes,
zero failures/filtered/measured, rawignored5 excluded and two zero-test binaries
excluded. Eleven outer harnesses each emitted one terminal summary.

| Harness | Passed | Raw ignored |
|---|---:|---:|
| Audit library | 51 | 1 |
| Checkpoint parity | 2 | 0 |
| Control operations | 21 | 0 |
| Durable witness | 13 | 1 |
| Key backups | 10 | 1 |
| Key custody | 10 | 2 |
| Merkle, record and verification parity | 3 | 0 |

Exact five helper identities and all required real parents were reconciled;
discovered=passed+rawignored in every harness. No private marker or nested child
summary leaked to outer output. WAL helper is now ignored rather than a no-op
pass. New retained backup identity/scope/loss/corruption/rollback/expiry cases,
Windows unlink-sharing fault, fresh reconciliation after old expiry and explicit
already-unlinked disk-state fixture actually passed. No crash/power-loss
observation is inferred from the fixture. Existing actual native retainer/witness
children, sealed recovery, permissions, erasure, local sink and parity cases pass.

Recursive112-file closure before/after has zero drift, including own source/tests,
fixtures, shared local dependencies and root manifests/lock/toolchain. Own code
checkpoint174e5f21 contains prior aa655e7b/d7045fee changes. Original commands,
timestamps, source manifests and logs remain private in evidence102137; public
facts.json binds raw log and source-manifest hashes. All handles terminal/A returned.

## How to read it

This is complete audit-package Cargo qualification on ordinary-user Windows,
not full workspace, genuine hosted OS matrix, strict native-actor executable/
discovery receipt or product Docker e2e. The library includes pure and filesystem
cases;110 is not a pure-unit count or110 externally exercised product flows.
Ignored5 and zero-test bins never count as passes. Historical101/100 MSRV and
44-case component observations remain separate exact-source evidence.
No SCM/account/network/Docker operation occurred. Independent retainer account/
storage provenance, production audit-before-apply and final pin acknowledgement,
remote sinks, streaming recovery and signing-identity transition remain open.
Metadata derivation attempt1 rejected singular 'running1 test'; attempt2 corrected
that parser only, preserved the first error and did not rerun tests.
