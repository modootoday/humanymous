---
id: plan:root/20261107002005
title: Audit key custody native persistence
status: active
created: 2026-11-07T00:20:05Z
updated: 2026-11-07T00:22:32Z
rel:
  governs: [crates/audit/src/keys/**]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody native persistence

## Goal

Implement the accepted native sealed-custody boundary without changing scoring,
wire envelope, KDF, production erasure authority or shared Cargo ownership.

## Steps

1. Record contracts and critique before implementation.
2. Implement safe native permission validation and lifetime exclusive custody.
3. Implement bounded load/create, pending replacement/recovery and sealed vault
   snapshot persistence with honest OS durability reporting.
4. Run real-source native tests and subprocess contention/fault cases; request
   root Cargo integration, then record exact checks and remaining deployment gates.

## Record

2026-11-07: source complete and root-owned Cargo dependencies integrated. Actual
workspace locked audit suite passed on Windows49/Linux47; custody cases10/8,
with two separately invoked subprocess helper entrypoints excluded from totals.
Clippy all-targets with denied warnings passed on both systems. Native tests
cover private ACL/modes, NTFS junction/Unix aliases, restart/snapshot/rotation,
child-process exclusion and interrupted promotion. Windows actual rename-sharing
failure verifies poisoning and authenticated recovery. No power-loss hardware
test or whole-product e2e is claimed. API/dependency details and independent
witness/admin/runtime gates are recorded in crates/audit/KEYS-INTEGRATION.md.
Sanitized source/execution hashes: [[evidence:root/20261107002839]].
Owned documents/bundles pass Specfold. Full check has out-of-lane private
analysis-layout and root evidence-attachment findings; no unrelated paths were
modified to suppress them. Public files are LF with matching Git-filtered bytes.
