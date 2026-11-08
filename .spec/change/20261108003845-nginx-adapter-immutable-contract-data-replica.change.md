---
id: change:root/20261108003845
title: nginx adapter immutable contract data replica
status: active
created: 2026-11-08T00:38:45Z
updated: 2026-11-08T00:38:45Z
rel:
  depends_on: [sot:root/nginx-adapter, sot:root/rust-migration, sot:root/structural-development]
---

# nginx adapter immutable contract data replica

## Change

Root accepted exactly one additional immutable DATA replica for the nginx pilot
contract, not an implementation copy or a fixture API. Copy canonical Git blob
9fece6de6bdeca2fb9832606fa6f19afce7d3752 from producer e3e7830 unchanged into
the current nginx package qualification/fixtures/nginx directory. Provenance
pins producer commit/path/blob, SHA256, Apache-2.0 and the future CLI-owned path.
Change only report.rs's two includes to this package-owned data. Preserve all
35 IDs, schema/version, report behavior and old tests. No manifest/dependency,
public API, scoring/runtime, canonical-file or path-move changes.

## Migration

Canonical Git/working/replica byte equality was checked at preparation. SHA256
is32be2e8c190d64fb095a06c1e9d67b86cd02a880bd0d2b43dbe7e3c2f40964b4,
4030 bytes. No automatic pin refresh is allowed. The normal package no longer
reads CLI qualification source; final adapters consolidation moves this owned
replica with nginx. Canonical original still belongs to CLI and will move to
crates/apps/humanymous-cli/qualification/adapters/nginx/pilot-contract.json.

No existing explicit-root package-source equality gate is implemented in the
owned nginx package: its package binary validates built artifacts, not repository
source. Do not extend that binary or create a new public API under this narrow
grant. Root/CLI must enforce a required explicit-root canonical/working/replica/
provenance equality gate before source archive/qualification, rejecting absent
canonical input and mismatch rather than refreshing the digest or skipping.

Formatter/compiler/Clippy/Docker are NOT_RUN under root's explicit hold. Source
plan after allocation: fmt; old library14 tests with report-group negative tests;
package check and scoped Clippy; explicit-root equality negatives (canonical
drift, replica drift, absent root/input); package inclusion and clean extracted
build without CLI tree; then source-bound actual35 host qualification as needed.
No historical813 result qualifies the new replica/package graph.
