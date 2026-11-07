---
id: evidence:root/20261107075922
title: Resumed Go Gate contract pure unit and scoped lint verification
status: active
created: 2026-11-07T07:59:22Z
updated: 2026-11-07T08:03:07Z
rel:
  depends_on: [sot:root/go-adapter, plan:root/20261107072800]
files:
  - {path: summary.json, sha256: 2d06e48497bca2e6ae27b30c212e69c74858860d0d357bc8d70eb59c5b81151f, bytes: 1588, store: repo}
---

# Scope and actual results

Root explicitly assigned B to target/node-native/build with build jobs1 and
incremental0. Resource guard observed34.02GiB free physical and83percent commit.
Rust1.99.1 ran the current three pure Go contract units, all passed with zero
failed/ignored/measured/filtered. Scoped locked all-target Clippy with denied
warnings returned0. B was returned immediately; no Go server, IPC or Docker ran.

The208-file before/after source subset includes owned Go code, enumerated shared
dependency source and manifests plus workspace manifest/lock/toolchain. One
concurrent change occurred in crates/runtime/src/tests/browser.rs. Runtime's
lib.rs gates its tests module with cfg(test); it is a non-test dependency of
these Go units. Own source/manifests/lock/toolchain were unchanged. This remains
a working-tree component observation, not an immutable whole-workspace proof.

Private before/after manifests and raw unit/Clippy logs remain under
.spec.local/root/evidence/20261107075923-go-adapter-resumed-validation.
The public summary preserves exact source hashes and accounting. The earlier
two-test root result belongs to older source307e9cc and is separate.

Actual upstream Go, compiler/container build, runtime/witness isolation, eight
hosted cases and product e2e remain NOT_RUN. No host is release-qualified.
