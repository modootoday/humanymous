---
id: evidence:root/20261107080632
title: Audit key custody trusted snapshot source
status: active
created: 2026-11-07T08:06:32Z
updated: 2026-11-07T08:06:32Z
rel:
  governs: [crates/audit/src/storage]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody trusted snapshot source

## What was measured

Authored TrustedSnapshot native atomicity/privacy wrapper over PrivateState and
per-instance bounded reads/writes. Existing owners keep their4MiB default.
Runtime requested this shared owner for authenticated operational-state commits;
runtime still owns typed payloads, revision/CAS, signed WAL binding and apply.
One hosted-only test covers bounds, lifetime lock and exact pending recovery.
Formatting and scoped public Specfold checks passed; compilation and execution
have not run for this later source.

## How to read it

The c6a6492 pure/static results predate this storage delta and do not qualify it.
Native .pending is an interrupted atomic write, distinct from the runtime's typed
candidate lifecycle. Equality-checked promotion is not cryptographic verification
or effect authorization. There is no reset/discard/automatic recovery API.
Hosted actual-file/ACL/crash/process and runtime/product qualification remain
not_run. No owned Cargo slot or process is held.
