---
id: evidence:root/20261107114349
title: Apache adapter MPM capability pure verification
status: active
created: 2026-11-07T11:43:49Z
updated: 2026-11-07T11:44:54Z
rel:
  governs: [crates/apache-adapter]
files:
  - {path: result.json, sha256: 0100af90b48ffa133ff3ed30c99de60e6e37ae3568fa0e977611a8bc1506ba23, bytes: 946, store: repo}
---

# Apache adapter MPM capability pure verification

## What was measured

Root allocated bounded pure verification of immutable source
2d66c0df7f5ea4e925a30d2f0b39c05bebd696ad in actual REA-Ubuntu Linux with
Rust1.99.1, jobs1/incremental0 and the isolated Apache target. Offline locked
safe library tests passed12, safe all-target check and warnings-denied Clippy
passed, and native pilot-bin diagnostic tests passed11. Both test groups had
zero failures, ignored or filtered tests. Full extracted source inventories
before and after matched. Admission80.72percent/32.77GiBfree; after81.53percent/
32.43GiBfree. The allocation was returned with no live validation process.

## How to read it

The twelve safe units include exact MPM capability identity and rejection
boundaries. The eleven coordinator units include static IO-kind privacy and
failed-ID projection. No selected-httpd/bindgen compilation, Apache process,
IPC/network or Docker pilot occurred during this allocation. This does not
qualify the changed native FFI query/probe path; it still requires the actual
root-reviewed hosted SDK and45-case retry. Earlier23/22/0outcomes remain failed.
