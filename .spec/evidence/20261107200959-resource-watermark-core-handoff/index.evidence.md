---
id: evidence:root/20261107200959
title: Exact watermark input handoff for accepted core consolidation
status: active
created: 2026-11-07T20:09:59Z
updated: 2026-11-07T20:11:47Z
rel:
  depends_on: [sot:root/resource-watermark, sot:root/structural-development, plan:root/20261107135500, evidence:root/20261107144843]
files:
  - {path: handoff.json, sha256: 3a8f2ba82cac7dc9cee9fe56c2b08429acd953fa3c227835d549e8a03a587986, bytes: 6814, store: repo}
---

# Root-only structural handoff

The human-confirmed eight-package baseline absorbs this context into
`crates/sdk/humanymous-core/src/watermark`. Root owns the atomic seven-context
integration, workspace/lock/import changes and branch synchronization. This owner
has not moved/renamed any path, changed a manifest or created a compatibility
shim/worktree/branch. The title and accepted destination are not a parallel write
grant on the new core package. Current claim is only the own spec handoff scope.

## Frozen source and ownership

Exact source2eb2069a7851324b90311d54ffe663d8eb9f75cf contains18 package files,
including16Rust files. All18 current bytes match the previously recorded corrected
source hashes. No package dirty files, active owned process handle, target lease
or pending native cleanup exists. C29 receipt9ba8e308c2d3f3accb41b0de07a8ade0fb527e19
records native Windows29pure units plus fmt/all-target check/strict Clippy passing.
First27f1Clippy101/tests NOT_RUN stays preserved separately. These historical
receipts remain tied to their original source and paths after consolidation.

handoff.json enumerates every source path, byte count, SHA256 and intended root
destination. src/lib.rs becomes the context mod.rs; other Rust-relative paths
remain under watermark. The leaf Cargo.toml is retired only by root after graph
integration; its dependency responsibilities are absorbed, not dropped. The
historical INTEGRATION.md needs an explicit root archival placement, not inclusion
as an executable module or a new permanent helper package.

## Visibility, API and dependency hazards

- Current crate-root Error/frame imports must resolve within the new watermark
  context. Do not bind them to an unrelated core Error/frame or add aliases at
  the whole core root merely to preserve old unqualified imports.
- frame.rs currently uses pub(crate) for Limits input/output, crc32, Budget,
  Candidates and scan. These were private to the leaf package; after merging,
  use the watermark context boundary (for example pub(super) where appropriate)
  so unrelated core siblings cannot bypass admission or mint recovery candidates.
- Ledger.inner and Record fields remain private. State/Slot and their fields,
  Reservation::new and state methods remain ledger-context restricted. Do not
  widen these to permit sibling construction, reuse IDs, forge live records or
  bypass TTL/cancellation/poison behavior. Lookup is still privileged unverified
  record data, never successful trace or operator authorization by itself.
- base32 remains codec-private. Public payload/ledger/limits/low-level APIs require
  deliberate facade curation; package publication does not make internal helpers
  permanent SDK commitments. No extra compatibility package is proposed.
- There is no unsafe code in this context. Preserve core forbid(unsafe_code).
  Dependencies are existing hmac0.12.1,sha20.10.9,subtle2.6.1,zeroize1.9.1 only.
  The context uses std synchronization/collections/time, not native/browser/HTTP
  providers; do not claim no_std or inherited WASM-target qualification.

No include!, include_bytes!, include_str!, path attributes, CARGO_MANIFEST_DIR or
CARGO_BIN_EXE dependency was found. Unit data is inline Rust byte/string data;
child test modules follow relative paths. Preserve child tests during lib.rs to
mod.rs transition. The only current Cargo/Rust name/path matches found are the
root workspace member and leaf manifest; no Rust consumer imports were found.
Future runtime/HTTP/lab consumers remain an explicit new contract, not a guessed
cross-package rename. Shared graph ownership remains root's.

## Required post-consolidation gates

Fmt first, selected core watermark29units plus all-target check/strict Clippy under
the actual consolidated lock/features. Preserve all failed/ignored/filtered counts
and the negative frame/ambiguity/NUL/ID/TTL/poison/security assertions. Verify that
unrelated core siblings cannot access frame helpers or ledger custody constructors.
Root additionally owns cross-context type/API ambiguity, core unsafe/feature
boundaries, complete workspace and applicable collector/WASM integration checks.
Old29passes cannot qualify the new package graph or moved source automatically.

Retained incomplete scope: standards-validated WASM codec, PNGmetadata/LSB/alpha/
memory/cancellation, validated high-level apply/recover/trace, full original
format tests and actual resource/file/browser/HTTP/Docker product gates. No row
is retired or full migration completion inferred from this handoff. Once root
freezes/moves the slice, this owner must stop old-path writing and await the exact
new path grant before resuming implementation. No compiler/workload was run here.
