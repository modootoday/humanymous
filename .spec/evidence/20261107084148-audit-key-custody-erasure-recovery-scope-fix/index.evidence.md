---
id: evidence:root/20261107084148
title: Audit key custody erasure recovery scope fix
status: active
created: 2026-11-07T08:41:48Z
updated: 2026-11-07T08:41:48Z
rel:
  governs: [crates/audit/src/keys]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody erasure recovery scope fix

## What was measured

Static inspection found recover_erasure_pending did not receive trusted logical
scope, while its validate_update path checked only subject commitment. The fix
requires an explicit trusted installation/node Scope and checks the immutable
ErasurePermit against it before I/O. Ordinary Vault authorization reuses the
same predicate. One native regression was authored with two independent signed
scope policies, exact-byte refusal checks and successful correct-scope recovery.
Formatting and scoped public Specfold checks passed. No compilation or native
execution occurred for this later source; existing8132117 remains unchanged.

## How to read it

This is source evidence and a static counterexample, not a reproduced production
attack or hosted pass. The stronger signature changes a low-level native API;
only two existing own Windows test callsites were found and updated. Runtime and
authority owners were notified; callers must obtain scope from trusted config.
There is no sealed-format/KDF/scoring change, new ignored helper, workflow
dispatch or push. Previous Ubuntu16/11 passes apply only to8132117/20b57d2 and
cannot establish this fix's regression result or full-product qualification.
