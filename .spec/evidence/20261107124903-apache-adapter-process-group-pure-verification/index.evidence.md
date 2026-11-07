---
id: evidence:root/20261107124903
title: Apache adapter process group pure verification
status: active
created: 2026-11-07T12:49:03Z
updated: 2026-11-07T12:49:23Z
rel:
  governs: [crates/apache-adapter]
files:
  - {path: result.json, sha256: b402fc75c73a5db8a9e7d8ad092120476593244b0403be068f9b68492c75f1ec, bytes: 1171, store: repo}
---

# Apache adapter process group pure verification

## What was measured

Root allocated actual REA-Ubuntu Linux Rust1.99.1 pure verification of immutable
b400094480483d9abb8328234f9de12520b71b65, offline/locked/jobs1/incremental0.
Safe12/native13 tests passed with zero failures, ignored or filtered. Both
all-target checks and both warnings-denied Clippy checks passed. Full extracted
source inventories before/after matched. Admission80.40percent/31.73GiBfree;
after82.61percent/30.39GiBfree. The allocation was returned.

## How to read it

The source isolates foreground master and CLI helper groups at birth and checks
actual kernel ownership immediately after pidfd capture, before wait/reap. This
allocation compiles that path but does not execute it. No Apache/CLI process,
pidfd signal, selected-httpd ABI build or Docker ran. Historical35/10and33/12
remain failed; runtime group identities there remain unknown. Qualification
still requires root-reviewed actual hosted isolated CLI and unchanged45cases.
