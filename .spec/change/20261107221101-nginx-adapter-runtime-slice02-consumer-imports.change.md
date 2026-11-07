---
id: change:root/20261107221101
title: nginx adapter runtime slice02 consumer imports
status: active
created: 2026-11-07T22:11:01Z
updated: 2026-11-07T22:11:01Z
rel:
  depends_on: [sot:root/nginx-adapter, sot:root/rust-migration, sot:root/structural-development]
---

# nginx adapter runtime slice02 consumer imports

## Change

Root granted RUNTIME-SLICE-02 nginx Rust consumer edits. Apply the existing
humanymous_runtime prefix transformation first (zero existing references), then
replace15 authority/audit references across request.rs, native.rs, pilot/mod.rs
and pilot/authority.rs with humanymous_runtime::authority/audit. Formatting only
reorders the affected use declarations. No aliases, compatibility packages,
scoring, behavior, visibility or fixture bytes changed.

The pre-existing dirty nginx Cargo.toml belongs to root. This owner changed no
manifest/lock/features, shared runtime file or path. CORE-SLICE-01 frozen fixture
references remain on their already mapped core destinations.

## Migration

Source-ready only: fmt, diff and old-reference/cascade scans pass. Compiler,
Clippy and CI/runtime are NOT_RUN until root allocates checks against a coherent
workspace. Root owns all context feature/default-feature decisions and the
combined graph. No current source qualification is inherited from8132117.
Exact before/after file hashes are attached in the nginx evidence bundle.
