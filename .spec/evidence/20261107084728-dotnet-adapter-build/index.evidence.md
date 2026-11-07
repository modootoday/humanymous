---
id: evidence:root/20261107084728
title: Source-bound Rust adapter and export validation
status: active
created: 2026-11-07T08:47:28Z
updated: 2026-11-07T08:49:00Z
rel:
  depends_on: [sot:root/dotnet-adapter, plan:root/20261107073000]
files:
  - {path: summary.json, sha256: 240397f606283184870a661cb773f10c4be657f314681ab0676ba3731b0261c0, bytes: 4833, store: repo}
---

# Source-bound Rust component validation

Root-allocated local A executed core12 and export1 pure tests with
zero failed/ignored/measured/filtered. Both packages' all-target check and
all-target Clippy with denied warnings passed. Compiler identity is
rustc1.99.1 (58cf8f9c3 2026-10-13). Build jobs1/incremental0, shared default target.

The252selected internal source/configuration/managed inputs and shared
Cargo/lock/toolchain hashes were captured before and after: no drift. summary.json
includes all18owned input hashes and the common graph hashes; raw complete
manifests and logs are private in evidence:root/20261107084546. The measured source
was shared worktree source after checkpoint55e137f plus approved graph/corrective
work; no immutable whole-workspace commit is invented for that instant.

The exact three-source C# scope test and ignored host pilot compiled as Rust test
targets but did not execute. No C# SDK build, actual dynamic PInvoke, Kestrel,
managed GC, NativeAOT, clean NuGet install or Docker product result follows from
these13Rust tests. Qualified host tuples remain empty. Runtime library unload
is unsupported; zero request diagnostics do not establish worker termination.
The older8132117hosted run does not contain this new source and cannot qualify it.
