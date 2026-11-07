---
id: evidence:root/20261107094725
title: Audit key custody retained erasure bridge source
status: active
created: 2026-11-07T09:47:25Z
updated: 2026-11-07T09:47:25Z
rel:
  governs: [crates/audit/src/keys/custody]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody retained erasure bridge source

## What was measured

Authored retained-permit normal erasure and explicit pending recovery over the
existing scoped vault/custody transaction. Normal application rejects divergent
vault snapshot or pending state before shredding. Recovery authenticates existing
committed state, requires fresh exact-scope/action/subject/time capability and
preserves existing unchanged-identity and exact-tombstone validation.
One actual-filesystem regression was authored for wrong scope, consumed nonce
reuse, persisted tombstone, wrong-subject pending refusal and fresh exact recovery
with all writer/HMAC/witness identity preserved. Formatting/document checks only.

## How to read it

No Cargo/native process/Docker validation ran under the benchmark quiet window.
These are authored source and tests, not passing runtime evidence. The service
must serialize vault access, witness audit intent before apply and fail closed
after post-shred persistence error. Completion is only this sealed generation;
backup retirement, streaming sinks, unknown-copy removal and product integration
remain separate requirements. No approval reissue, generic effect replay,
automatic retry or new key provisioning was added.
