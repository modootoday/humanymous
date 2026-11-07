---
id: evidence:root/20261107085126
title: Rust wargame service profile pure acceptance
status: active
created: 2026-11-07T08:51:26Z
updated: 2026-11-07T08:52:31Z
rel:
  governs: [crates/defender]
  depends_on: [sot:root/rust-wargame]
files:
  - {path: summary.json, sha256: ee80f9f2f77e4ac4260b8624b92e23a4b8a58873007dadb9995e75384d6ccf5a, bytes: 736, store: repo}
---

# Rust wargame service profile pure acceptance

## What was measured

Root-assigned A slot, Windows x86_64:27pure checks passed/0failed/0ignored on first
attempt. Attacker/defender26 plus exactly runtime's standalone profile test1;
41other runtime tests were filtered out and not executed. Two zero-test doctest
harnesses are separate. Scoped all-target Clippy for attacker/defender/runtime
passed with -Dwarnings. Compile durations12.34s/4.39s/4.09s. Locked/jobs1/
incremental0/testthreads1; application process/socket tests0. Slot returned.

Consumer743494c1 uses runtime709b0e2's sole full profile builder and preserves
whole-object rejection of changed scope/listener/witness/unknown fields. The
configuration-only profile is now a negative fixture. Root build metadata and
all existing tracked/untracked crate files were copied before checks; ordered
raw-blob lists of snapshot/before/after match byte-for-byte. Ordered path lists
also match. Before HEAD411b69e5ee1510cb93eff70132e1c3d0df509dc6, after
b6175727cc48aab0a178084c37b8e6b2c20e07d1; unrelated Git history moved while this
captured source scope stayed unchanged. Private source/archive/log evidence084733.

## How to read it

This establishes pure shared-profile consistency, not actual child startup or a
successful network round. Actual813receipt4ed65c4 remains19errors/0evaluated.
A fresh coherent hosted candidate is still required for the repair. No old
failure is overwritten and no unsupported producer becomes qualified.
