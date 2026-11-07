---
id: plan:root/20261107042614
title: Native init CLI naming and manifest implementation
status: active
created: 2026-11-07T04:26:14Z
updated: 2026-11-07T04:35:44Z
rel:
  governs: [crates/native-cli/**]
  depends_on: [sot:root/native-embedding, sot:root/rust-migration]
---

# Native init CLI naming and manifest implementation

## Goal

Implement humanymous-native-cli with humanymous-native init. Deterministic dry-run
planning plus explicit no-clobber Windows manifest creation; no SDK/runtime,
framework adapter, foreign template, scoring or sandbox-policy implementation.

## Steps

- [x] User accepted package/binary/subcommand naming and requested implementation.
- [x] Claim CLI/canon lane; shared workspace membership/lock remain root-owned.
- [x] Define SDK-15..18 before code: one manifest, dry-run default, unavailable
  capabilities and pinned publication/containment/cleanup semantics.
- [x] Reflect independent blue/red/SRE/privacy/evaluator critique before finishing.
- [x] Implement pure argument/schema responsibilities and isolated file publication.
- [x] Coordinate root workspace registration with exact Cargo manifest.
- [x] Test parser negatives, deterministic honest manifest, dry-run zero writes,
  real write/no-overwrite, junction rejection, concurrency and failure cleanup.
- [x] Run fmt, warning-denied Clippy, library and actual CLI process tests.
- [x] Record evidence, exact platform limits and attributed scoped commit/handover.

## Record

Framework SDK/runtime, all Node-API/.node loading and packaged Electron tests,
Tauri/winit/.NET/Qt/Flutter, multi-file scaffolding, Unix writes, power-loss and
hostile same-identity content mutation remain unverified/outside this slice.
Rust-only, default-deny and sandbox-required policy declarations remain binding.


## Implemented result and evidence

Evidence:root/20261107042614: Windows x86_64 unit11 + actual CLI process6 pass,
zero failed/ignored; warning-denied Clippy, fmt and locked release build pass.
Release pilot ran6 framework dry-runs without workspace files; explicit write
created exactly humanymous-native.json; repeated write returned2 with original
hash unchanged; swallowed-option conflict returned2 with an empty workspace.
The native manifest remains qualified=false and adapter not_implemented.

Root owns/integrates Cargo workspace20 and lock. This lane changes only native-cli,
SDK canon/plans and scoped evidence. SDK runtime/adapter creation and Rust scaffold
emission remain later slices; no backend, third-party probe, score or sandbox
relaxation. Same-user hostile entry replacement and power-loss durability are
outside the creation guarantee. Windows network/SUBST mapping and all non-Windows
execution remain unverified; no cross-platform readiness is claimed.

Next: implement core/FFI lifecycle, real Electron Node-API artifact/consumer and
framework capability tests before enabling any generated integration. Future
safe multi-file Rust scaffolding must not import imaginary packages or emit
foreign script/template code. Read AGENTS.md and native-embedding/rust-migration
SoTs first; no prior chat is required. Current CLI syntax:

`cargo run --locked -p humanymous-native-cli -- init --framework electron --dry-run`

`humanymous-native init --framework electron --write --workspace C:\my-app`

The second command requires an existing trusted local Windows workspace, refuses
an existing destination and only publishes declarative JSON. It does not install
SDK artifacts, configure the external host or activate automation blocking.
