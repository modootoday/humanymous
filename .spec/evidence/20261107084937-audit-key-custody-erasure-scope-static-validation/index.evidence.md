---
id: evidence:root/20261107084937
title: Audit key custody erasure scope static validation
status: active
created: 2026-11-07T08:49:37Z
updated: 2026-11-07T08:53:12Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 98c675605fcc18684f7bf45fed19c6ae6d929683a975e16032a83ebe348d7db4, bytes: 1545, store: repo}
  - {path: post-check-lock-drift.json, sha256: 2b384c34f62930916740d233a89ed89986daf78e7eb89c3e0189ec0987f5bbc2, bytes: 582, store: repo}
---

# Audit key custody erasure scope static validation

## What was measured

Root explicitly assigned B/target/validation-agents, jobs1/incremental0/
testthreads1 for a04603e1. All-target cargo check and seven selected pure tests
passed. First all-target Clippy failed with items_after_test_module; the exact
failed log and107-file recursive before/after closure were preserved unchanged.
The existing retention implementation block was moved before its test module,
without suppression or behavioral changes. All-target Clippy attempt2 passed.
Final stable source reran the same seven pure tests (all passed) and compiled/
linked control_operations with --no-run. Repeats do not create14 unique tests.

The recursive closure includes audit/native-security/signals/test-support plus
root Cargo manifest/lock/toolchain. Only the declared retention.rs reorder
changed; its preattempt2 hash matches final. B was returned immediately after
checks. Public facts contain exact selectors/counts and private closure hashes;
original logs/manifests are attached to private evidence:root/20261107084636.

## How to read it

No actual file/ACL/IPC/process/erasure native test executed locally. The newly
authored same-subject foreign-scope regression is compiled, not passed. Prior
8132117 Ubuntu/macOS16/11 observations do not qualify this later source. The
older all-target check occurred before the layout-only reorder; final Clippy
typechecked all targets and final no-run linked the integration target.
After-check documentation/evidence edits do not pretend to be part of compiler
input bytes. No Cargo graph edits, push, workflow dispatch or ignored-gate
relaxation occurred. Full hosted/cross-OS/runtime/product gates remain open.

After B was returned, root Cargo.lock changed in the working tree by three
Python-adapter dependency edges. Audit/dependency Rust sources stayed matched.
The original results remain bound to their tested lock9b96ab7a...; they do not
claim validation of the newer worktree lock65b83031.... The exact before/current
SHA256 values are in a separate attached post-check delta, preserving the
original facts. Root was notified; audit edited no manifest or lock and started
no unallocated revalidation. Later evidence/documentation commits do not imply
whole-HEAD qualification.
