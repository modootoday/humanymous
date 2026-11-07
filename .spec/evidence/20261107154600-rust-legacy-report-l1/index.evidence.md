---
id: evidence:root/20261107154600
title: Rust legacy report L1 acceptance
status: active
created: 2026-11-07T15:46:00Z
updated: 2026-11-07T15:48:26Z
rel:
  governs: [crates/attacker]
  depends_on: [sot:root/rust-wargame]
files:
  - {path: receipt.json, sha256: 6cfaf6a9b614cab87524f64dc952ce8ba2e9be876d1c52408661ec8e7348629d, bytes: 2314, store: repo}
---

# Rust legacy report L1 acceptance

Windows Rust1.99.1, offline/locked, jobs1, incremental0. Frozen source is commit
e34cee8abb670188cbcdec5965a5574b345b1750 plus eight exact overlays in receipt.json.
The six Rust overlays implement the legacy11-field importer, honest accounting,
bounded escaped HTML, atomic publication source and thin CLI. Two overlays are
the owning specification and plan. This candidate is separate from hosted306eedc8.

Actual selected library footer:8passed/0failed/2ignored/17filtered. Package
all-target check and strict Clippy passed; direct six-file fmt check passed.
The CLI unit test was compiled, not executed. Both ignored native publication
and Windows sharing-denial cases remain NOT_RUN. No HTML artifact was generated.
No actual filesystem durability, replacement, native cleanup or product gate is
qualified by these pure tests.

The archive plus overlay expected set and post-run materialization each contain
2715files, with0difference records. All eight live overlays match their frozen
hashes. This is source-file closure only; registry/tool/config environment closure
was not measured. Source ZIP, exact overlays, checkpoint, closure and terminal
logs are retained in private evidence:root/20261107154601. The target was returned
at terminal execution before proof serialization. No full-product claim follows.
