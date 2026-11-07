---
id: evidence:root/20261107120400
title: Frozen capture stream metadata and record owner component qualification
status: active
created: 2026-11-07T12:04:00Z
updated: 2026-11-07T12:04:00Z
files:
  - {path: receipt.json, sha256: 19160ed5683d2e3cb97a9f840b9577ff0a0a616d529abb030db1d266fa7dc1e0, bytes: 14198, store: repo}
rel:
  depends_on: [sot:root/network-capture, plan:root/20261107072531]
---

# Frozen selected capture qualification

Root explicitly released B for immutable edf700bc selected validation. Git archive
extraction isolates source from concurrent worktree writers. All 2350 archived
source files, including root manifests, lock and toolchain plus the recursive
internal dependency closure, were hashed before and after: zero differences,
identical snapshot SHA256. The attachment also lists the 61 capture/network/
signals and root-context entries. Registry source trees were not independently
snapshotted; their versions remain bound by the unchanged locked graph.

Fresh admission: 32.43GiB physical free, 80% committed. Exclusive target
target/validation-capture-b-edf700bc, jobs1, incremental0. Rust1.99.1 on Windows
x86_64-pc-windows-msvc. No source, feature or dependency manifest was modified in
the frozen tree. B was returned as soon as the terminal gates and snapshot ended.

| Gate | Actual result |
|---|---|
| Locked capture lib tests | 27 passed, 0 failed/ignored/measured/filtered; 24.93s build, 0.06s tests |
| All-targets Clippy with hosted-pilot and warnings denied | Exit0; 9.36s |
| hosted_transport feature build with --no-run | Exit0; 2.60s; executable never run |

These include new H2 metadata, record-boundary accounting and compile-time
ownership bounds. They do not execute socket probe behavior. Five hosted cases
remain 0 executed, and runtime/TLS serving and Docker product qualification are
separate outstanding gates. The prior failed Docker813 receipt remains unchanged.
No GitHub environment was spoofed and no Docker or runtime pilot was launched.
Source, executable, raw-log hashes, compiler identity, exact command arguments and
stage timestamps are in the receipt; raw logs remain private.
