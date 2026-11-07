---
id: evidence:root/20261107120401
title: Rust wargame terminal accounting pure acceptance
status: active
created: 2026-11-07T12:04:00Z
updated: 2026-11-07T12:05:33Z
rel:
  governs: [crates/attacker]
  depends_on: [sot:root/rust-wargame]
files:
  - {path: summary.json, sha256: cf4d49f992532cfcc58d0fe56ec76df3cac6a65e13ec6fbc806fc98c4a0b43d7, bytes: 842, store: repo}
---

# Rust wargame terminal accounting pure acceptance

## What was measured

Root-assigned A, Windows x86_64:28selected pure tests passed/0failed/0ignored/
0filtered, first attempt. Attacker library17, attacker CLI1, verifier CLI1,
defender9. Two zero-test doctest harnesses are separate. Scoped attacker/defender
all-target Clippy with -Dwarnings passed. Build18.67s/Clippy4.63s. Admission
commit81.2%, free physical31.9GiB; jobs1/incremental0/testthreads1/locked.
No application runtime, Docker, external target or actual evaluator was launched.
A was returned immediately after source capture.

Owned attacker/defender bytes match284d24cf. The actual dependency graph was at
HEAD648d407a6350f0e82f9a72fef44959caadc2ac6a before and
a281ffa51a90e9e19391c4aff4de82933856325a afterward. It already contained newer
audit checkpoint/signing/storage-framing changes, so this is not a claim that
the complete dependency graph equals commit284d24cf. Actual all-crate/root-build
metadata bytes were copied and matched before hashes. Before/after path lists
match. Two raw hashes changed only for capture/INTEGRATION.md and
dotnet-adapter/INTEGRATION.md; Rust/manifests/root lock remained unchanged.
Private120155 retains source ZIP, ordered indices, drift and both command logs.

## How to read it

The two added tests prove rejection of attempted-as-skipped, failed-transport-as-
evaluated, post-failure steps and missing/blank error or false response-verdict
claims, while retaining an honest failed prefix. The fixture is a synthetic failed
ledger, not a mocked successful runtime. The26-case catalog and7unsupported
human/AT/producer obligations remain unchanged. This is pure validation only;
actual product validation and the readiness repair's runtime effect remain open.
Existing hosted receipt hashes/results are not rewritten by this new source.
