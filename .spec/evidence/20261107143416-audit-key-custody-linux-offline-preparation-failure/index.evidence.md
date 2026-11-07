---
id: evidence:root/20261107143416
title: Audit key custody Linux offline preparation failure
status: active
created: 2026-11-07T14:34:16Z
updated: 2026-11-07T14:35:54Z
rel:
  governs: [crates/audit]
  depends_on: [plan:root/20261107135332]
files:
  - {path: facts.json, sha256: 224c3003a395d487789dc2d7d707ee4a0e751f890f12b2251007836cba90bd7f, bytes: 1852, store: repo}
---

# Audit key custody Linux offline preparation failure

## What was measured

Exact candidate3d325bdbc2c182d9805688d74fe1087bd8851de2, ordinary-user
REA-Ubuntu/WSL Linux x86_64 preflight. Initial memory admission85% refused;
one authorized fresh observation82%/31490MiB passed. Actual native Rust/Cargo
1.99.1 GNU identities, UID1000, ext-family magic0xef53, private0700 source/
fixture directories, softFD10240/hard1048576 and sufficient disk observed.
Ext-family does not independently prove ext4 or physically local media.

Full-workspace offline Cargo metadata failed before all proposed validation:
missing asn1-rs0.7.2; offline mode refused an HTTP request. Original stderr and
empty metadata output retained privately as `evidence:root/20261107142917`.
PowerShell orchestration terminated1; exact nested Cargo exit number was not
persisted, so it is not reconstructed or reported as an observed101.
No fmt/check/Clippy or any of SIX selected Rust tests ran in this Linux attempt.
No package was downloaded/installed, no compiler restart/retry or shared graph
change occurred. No native healthy/crypto/fault/memory/product result is proved.

Immutable Git archive SHA256:
9d7943b6254b13d06a87038c05d7be56e8ba2a6cb447fb456564bbaa29809143.
All2569 source files before/after match both hashes and inventory; captured count
matches Git tree2569. All16 Cargo configuration/credential candidates were absent
before/after. Registry/tool-file closure capture did not complete because offline
metadata failed; version identities alone are not a complete execution closure.

## How to read it

Subsequent Root-approved package-scoped offline tree retry succeeded and the
unchanged3d325 source completed six selected tests in
[[evidence:root/20261107144415]]. This document and its NOT_RUN facts describe
the preserved initial preparation attempt only, not that later allocation.

This is a preparation failure, not a failed or passing Rust test run. The earlier
Windows33-test Phase-B proof remains exact to its historical source. Source
inspection finds a workspace lock path Runtime -> rcgen -> x509-parser -> asn1-rs;
this motivates a proposed package-scoped offline closure retry, not proof of the
actual resolved GNU audit graph. Root allocation was returned after terminal
failure with no active Cargo/exec handle. Any retry needs an explicit renewed
allocation and retained original failure. Full objective remains incomplete.
