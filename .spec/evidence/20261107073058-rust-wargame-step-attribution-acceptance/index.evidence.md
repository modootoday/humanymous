---
id: evidence:root/20261107073058
title: Rust wargame step attribution acceptance
status: active
created: 2026-11-07T07:30:58Z
updated: 2026-11-07T07:32:14Z
rel:
  governs: [crates/attacker]
  depends_on: [sot:root/rust-wargame]
files:
  - {path: summary.json, sha256: 191465a80ece94908be721cbcf0a1a012085d399c9eb95fc98b72b8afeaaf5c0, bytes: 454, store: repo}
---

# Rust wargame step attribution acceptance

## What was measured

First-attempt Windows x86_64 Rust1.99.1 checks:26pure passed/0failed/0ignored
(attacker library15, CLI1, verifier CLI1, defender9); scoped all-target Clippy
with -Dwarnings passed. Two zero-test doctest harnesses are separate. Build times
12.15s/2.73s. Root allocated Cargo1: locked/j1/incremental0/testthreads1.

Source75d7c82 is included in observed HEAD7cda20c5660707b06e905aa6fb289cbab44e9f32.
Owned attacker/defender and shared Cargo/lock were clean; other dependency source
was dirty, including audit storage. Initial status is retained privately, but no
immutable full dependency graph was captured. This does not qualify a clean
checkout, a later dependency graph or actual hosted execution.

## How to read it

Step relabelling/reordering/extra observations are rejected; failed prefixes stay
failed ledgers. No application process/socket/Docker/browser/human/AT was run.
Slot returned immediately. Fresh hosted integration, original-byte strict receipt
and7capability/participant categories remain open. Raw logs and source metadata:
evidence:root/20261107073022.
