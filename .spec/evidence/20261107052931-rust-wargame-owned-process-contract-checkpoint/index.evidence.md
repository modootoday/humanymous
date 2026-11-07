---
id: evidence:root/20261107052931
title: Rust wargame owned-process contract checkpoint
status: active
created: 2026-11-07T05:29:31Z
updated: 2026-11-07T05:31:58Z
rel:
  governs: [crates/attacker]
files:
  - {path: summary.json, sha256: e15da34ac779d1a72a4ff0607b3beab3250ecc57b957bc5b6615fb7f98ea4c1e, bytes: 1142, store: repo}
---

# Rust wargame owned-process contract checkpoint

## What was measured

Current working-source Windows Rust1.99.1 unit checkpoint:19passed/0failed/0ignored,
Clippy with warnings denied. The accepted owned-process APIs and v2 ledger are
implemented; no v2 network case has executed at this checkpoint. Immutable Docker
source/image evidence remains the next Compose-owned stage. Historical v1 engine
simulation evidence does not qualify this new source.

## How to read it

summary.json is a sanitized contract/status summary, not raw execution traces.
No capabilities, session keys, cookies, request bodies or private installation
artifacts are published. Seven required producer/control categories stay
unsupported; human/AT and full-product qualification are false. Working-source
and canonical Git-byte observations remain distinct.
