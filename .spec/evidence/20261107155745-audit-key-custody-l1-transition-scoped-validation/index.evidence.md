---
id: evidence:root/20261107155745
title: Audit key custody L1 transition scoped validation
status: active
created: 2026-11-07T15:57:45Z
updated: 2026-11-07T16:02:48Z
rel:
  governs: [crates/audit/src/verification/scan/recovery_transition.rs]
  depends_on: [plan:root/20261107151758]
files:
  - {path: facts.json, sha256: 4686b3983a0800d84248c41f91436ad917c6d6b36ec7d3fc3221ea6aa39b89c7, bytes: 7985, store: repo}
---

# Audit key custody L1 transition scoped validation

## What was measured

Exact immutable8cbc0397e14511b883299a865b29ff700b6510cb, ordinary native GNU
Rust1.99.1/Linux x86_64/UID1000, exclusive existing warm Audit target. Root grant
was limited to FIVE private L1 recovery-transition invariant tests plus scoped
fmt/check/strict Clippy. Jobs1/incremental0/locked/offline/test-thread1; no install,
fetch, new target, actual custody/repair/Ready, Docker or SDK execution.

Fmt FIRST0/0.180s, package all-target check0/2.945s, strict Clippy0/3.435s.
Five individually exact harness invocations each reported1 passed/0 failed/
0 ignored/0 measured/114 filtered, with exactly one outer summary and selected
test name actually executed. Tests cover immutable read-only preserved-tail
preconditions, zero/u64MAX/anchor-ahead, segment rotation limit, control-only
bootstrap decisions and original Control cancellation versus replacement/reset.
They are local pure plans, not real authentication/owner/I/O capabilities.

Actual session36565 terminal0, target process observation no match (pgrep1), A
returned before read-only closing hashes. Accepted COMPONENT input comparison:
177 source/fixture/manifest/lock/toolchain inputs (7,434,998bytes),67 resolved GNU
registry packages/3181 files,73 selected Rust executables/host/target library
files all matched hashes AND inventory. Source scope includes entire audit/
signals/test-support packages and all workspace Cargo manifests/config inputs;
it is not a comparison of every2707 full-archive file.44 Cargo/formatter/clippy/
credential candidates remained absent; ambient build-config HOME hash unchanged.
Explicit child argv binds qualified tools/offline/target/jobs. Three formatter
driver/tool hashes taken BEFORE fmt also matched closing selected tools.
External OS/system linker remains outside selected Rust tool hash coverage.

Independent X-owner Windows-local review additionally checked all48 private
attachment SHA256/lengths, five exact-name footers, seven terminal check/Clippy/
test records plus fmt-first, and semantic before/after inventories. It streamed
the canonical tar without extraction: archive SHA matched and all177 selected
member SHA256/length plus computed Git blob SHA1 matched actual8cbc tree.
Manifest JSON ordering differs; equality refers to semantic per-file records,
not identical before/after JSON serialization. No WSL/UNC/compiler was used in
that review. Acceptance is private L1/component only; no native authority.

Preparation timings are observations, not an achieved speedup: materialize1.310s,
registry hash-before26.829s, selected tool hash-before18.912s. The proposed30s
preparation target was NOT achieved. Selection reuses declared paths only; all
content/config/registry/tool/environment hashes were refreshed. An earlier
scratch-selector regex-capture error was corrected before preparation, with the
failed partial artifact retained separately and never used as input closure.

## How to read it

Facts attached; exact raw logs/argv/counts/input snapshots locally retained as
`evidence:root/20261107154715`. Full archive SHA256 is attached, with large tar
retained privately instead of violating Specfold4MiB per-file attachment cap.
No source correction was needed in this actual compiler/test tranche.

Broader seven noncapacity tests and explicitly ignored native/control/capacity
execution were NOT RUN. Four capacity cases and facade source compile in GNU
all-target check/library build, but no facade behavior/RSS/FD/native proof is
claimed from compilation. Historical five subprocess helpers were not executed.
Actual-type borrow compile-fail, post-auth expiry/no-write, native ownership/
durability, full audit/workspace, product/hosted and original full goal remain
incomplete. Previous3d325 actual-native qualification remains historical exact
source. No production open_native promotion or erased ambiguity/authority claim.

All WSL compiler and closure handles finished before user-requested REA-Ubuntu
shutdown handoff; A/WSL use released. Remaining evidence work used Windows files,
with no new WSL workload or implicit goal pause.
