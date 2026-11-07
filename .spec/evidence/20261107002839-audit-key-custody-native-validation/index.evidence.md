---
id: evidence:root/20261107002839
title: Audit key custody native validation
status: active
created: 2026-11-07T00:28:39Z
updated: 2026-11-07T00:28:39Z
files:
  - {path: facts.json, sha256: 9eb4ba8dcd732e15d335f1f84d916dda299e11dc33ee6bf1870975f2b618667f, bytes: 2499, store: repo}
rel:
  governs: [crates/audit/src/keys/**]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody native validation

## What was measured

Rust1.99.1 actual workspace audit suite passed on Windows49/Linux47; custody
scenarios10/8. Two test helper entrypoints are ignored in the main process and
explicitly exercised by subprocess tests; they are excluded from pass totals.
Both workspace Clippy all-targets with denied warnings passed. Linux used the
recorded Rust1.99.1 bookworm image with source mounted read-only. Windows used
the actual NTFS filesystem, strict owner DACLs, real junctions and rename-sharing
failure. Unix covered private modes, symlink/hardlink refusal and local fsync.

facts.json contains only sanitized facts, source hashes and private execution
hashes. Raw logs and direct-source harness configuration are registered under
private evidence:root/20261107002501; compiler caches are not evidence outputs.

## How to read it

This establishes a bounded native filesystem slice. It is not Docker whole-product
e2e, power-loss testing, remote filesystem qualification or deployment readiness.
Windows namespace durability remains unconfirmed; current-user/private-directory
trust does not defend against same-owner or privileged replacement. Production
erasure authority, permit-aware pending erasure recovery, independent witness,
rollback/backup handling and runtime/service wiring remain explicit gates.
