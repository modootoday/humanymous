---
id: plan:root/20261107000722
title: User approved Rust 1.99.1 baseline
status: active
created: 2026-11-07T00:07:22Z
updated: 2026-11-07T00:07:22Z
rel:
  depends_on: [sot:root/rust-migration, sot:root/delivery, sot:root/program-roadmap]
---

# Toolchain upgrade

- [x] User explicitly approves core MSRV/toolchain1.99.1.
- [x] Align workspace/docs-tool Cargo, pinned rust-toolchain, workflow generator and native probe.
- [x] Pin official authority pilot compiler image by1.99.1 OCI digest.
- [x] Update active design/roadmap baseline while retaining historical1.88 evidence.
- [x] Run locked workspace/all-target151 tests, warnings-denied Clippy, fmt and eight-library portable WASM.
- [x] Run docs-tool7 tests, CI-policy9 tests and actionlint.
- [x] Run isolated authority Docker pilot14 default cases plus explicit other-UID parent/helper qualification.
- [ ] Record actual outcomes and same-SHA native remote follow-through.

This changes toolchain policy, not scoring, wire formats or runtime capabilities.
Meari/CEF full closure remains unbuilt; old six-tuple results remain SHA-bound.
