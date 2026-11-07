---
id: evidence:root/20261107135643
title: Performance readiness and four source-bound library builds
status: active
created: 2026-11-07T13:56:43Z
updated: 2026-11-07T14:16:44Z
rel:
  depends_on: [sot:root/performance-optimization, plan:root/20261107135643]
files:
  - {path: baseline.json, sha256: 050222e45bf8fad49391e0afe26280d694247e0b74d8602572dfc3167bb81e2f, bytes: 5512, store: repo}
---

# Performance readiness and four source-bound library builds

Four root-allocated native Windows signals library builds actually terminated
with exit0 and Cargo build-finished success on immutable source
9b064dbd8942d4fad7f1623da97b6b64c0177a7f. Jobs1/incremental0, locked/offline,
dev profile and the exclusively assigned existing target were identical.

| Run | Cargo rounded footer, seconds | Artifact events | Fresh | Rebuilt | Unknown |
|---|---:|---:|---:|---:|---:|
| Initial existing target | 5.55 | 23 | 15 | 8 | 0 |
| Repeat1 | 0.18 | 23 | 23 | 0 | 0 |
| Repeat2 | 0.16 | 23 | 23 | 0 | 0 |
| Repeat3 | 0.18 | 23 | 23 | 0 | 0 |

Freshness describes Cargo artifact events, not compiler-cache or archive hits.
Initial source relocation/profile/feature compatibility was not certified. This
is not a controlled cold/warm or A/B optimization experiment. No tests, runtime
pilot, allocation count or speedup are reported. CPU/RSS remain unknown. Tool
wall times include shell/admission overhead and are separate from rounded Cargo
footer times. No tail-latency estimate is supported.

The profile is project-quiet/external-load-observed. Root identified unrelated
project compiler activity before release; our Windows compiler snapshots before
each run and after showed0 matches. WSL and full host CPU background load were
not observed. No machine-idle or noise-free claim. Every admission was below85%
commit and above20GiB free physical memory.

Extracted source closure contained2539 files before and after, with0 difference
records and identical manifest hash. Eighteen configuration candidates,18 hashed
inherited environment dimensions and two tool hashes were unchanged; children
explicitly set jobs1/incremental0. Complete artifact reuse authority is unproven.
All commands ended before the after-process observation, giving a69.4907721second
wall upper bound including gaps, below the300second grant. No further builds ran.

Raw logs, four copied Cargo HTML reports, closure manifests and corrections remain
private. The raw-files manifest contains24files and has SHA256
f4455b2e2d11430ce5b53fa9d1a480049c207d36c81dc75fca9fe80d506458f1.
An initial archive invocation failed before builds and was corrected. Exploratory
timezone-mixed duration arithmetic was discarded; explicit DateTimeOffset
arithmetic establishes the bound. These corrections do not alter Cargo results.

Native Cargo/rustc version commands both exited0. The record identifies the
Windows MSVC toolchain and LLVM23.1.1; external linker/native SDK identity remains
unknown. Existing canonical/token fixture counts come from test source, not a
new test run. The prior public comparison's four files match their index hashes;
this integrity check does not repeat or broaden that historical execution.

The performance owner acquired the existing project-coordination lane through
the unchanged compatibility protocol and verified actual provider/session.
Root subsequently narrowed public evidence authority to this exact index and
baseline.json; the owner released/reclaimed its own lane and reverified identity.
No new lane mutator, global writer-mode change or product-code grant resulted.
Root later separately delegated exactly canonical.rs, canonical/tests.rs and
examples/canonical_performance.rs under Integrity. Streaming normalization,
regression tests and a fixed Rust-only benchmark are now prepared and formatted.
No compiler/test/benchmark execution for R1 has occurred during the Pages window.
This source checkpoint does not change the four B1 execution observations above.

The controlling SoT retains build and runtime scope, exact-byte regression
requirements, security invariants, resource admission and Docker product gates.
The plan lists the first bounded measurements and the remaining full objective.
