---
id: change:root/20261107201356
title: nginx adapter core slice01 consumer imports
status: active
created: 2026-11-07T20:13:56Z
updated: 2026-11-07T20:13:56Z
rel:
  depends_on: [sot:root/nginx-adapter, sot:root/rust-migration, sot:root/structural-development]
---

# nginx adapter core slice01 consumer imports

## Change

Root assigned CORE-SLICE-01 consumer Rust edits in current nginx source paths.
Replace five imports across request.rs, native.rs, pilot/mod.rs and
pilot/authority.rs: humanymous_integrity becomes humanymous_core::integrity,
humanymous_signals becomes humanymous_core::signals. No other old context
references remain in this crate's Rust files. No behavior/scoring/visibility,
manifest/lock or path moves are part of this slice.

Update three frozen fixture include paths to root's preparation file-map
destinations under crates/sdk/humanymous-core/tests/scoring/fixtures/frozen.
The original JSON bytes and hashes remain unchanged. Root must preserve those
destinations or update this exact consumer when finalizing the relocation.

## Migration

Rustfmt and diff checks passed. No compiler, Clippy or full CI was started:
root explicitly requires the new workspace to be coherent first. The recorded
source/hash handoff is source-ready only, not a runtime or product result.
Root owns Cargo manifests/locks, seven-context relocation and the graph commit.
After coherence, run selected locked library/package/pilot checks and feature-
separate native/hosted qualification through allocated execution resources.
Historical exact8132117 receipts do not qualify the changed imports/graph.
