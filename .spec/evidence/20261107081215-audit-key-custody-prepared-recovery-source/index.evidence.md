---
id: evidence:root/20261107081215
title: Audit key custody prepared recovery source
status: active
created: 2026-11-07T08:12:15Z
updated: 2026-11-07T08:12:15Z
rel:
  governs: [crates/audit/src/keys/custody]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody prepared recovery source

## What was measured

Source implementation of an immutable authenticated pending-rotation descriptor
and retained-only exact-ciphertext promotion, reusing the existing custody read,
identity verification and fsync/rename/poison implementation. Two hosted-only
actual-file tests cover exact promotion, wrong passwords, vault changes and
pending ciphertext substitution. Formatting/public Specfold checks passed.
No new compilation or execution occurred; explicit slot allocation is pending.

## How to read it

This is source evidence, not crash/runtime/product qualification. Tests stage
actual pending files then explicitly reopen custody; they do not yet simulate a
process dying mid-write. Unknown completed rename outcomes, external actor
rights, power loss and complete product effect handling still need direct proof.
No new raw ignored helper or workflow dispatch exists. Prior c6a6492 green is
historical and cannot qualify these changed custody/storage sources.
