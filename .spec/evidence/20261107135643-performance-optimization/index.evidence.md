---
id: evidence:root/20261107135643
title: Performance readiness and four source-bound library builds
status: active
created: 2026-11-07T13:56:43Z
updated: 2026-11-07T14:32:38Z
rel:
  depends_on: [sot:root/performance-optimization, plan:root/20261107135643]
files:
  - {path: baseline.json, sha256: 1675ecf2977e228bc9db346c28f0ef97e60d566f05783740e548866b0fc85b72, bytes: 7777, store: repo}
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
unknown. Initial canonical/token fixture inventory counts came from source;
subsequent actual native execution is recorded separately below and in
r1_verification. The prior public comparison's four files match their index hashes;
this integrity check does not repeat or broaden that historical execution.

The performance owner acquired the existing project-coordination lane through
the unchanged compatibility protocol and verified actual provider/session.
Root subsequently narrowed public evidence authority to this exact index and
baseline.json; the owner released/reclaimed its own lane and reverified identity.
No new lane mutator, global writer-mode change or product-code grant resulted.
Root later separately delegated exactly canonical.rs, canonical/tests.rs and
examples/canonical_performance.rs under Integrity. Streaming normalization,
regression tests and a fixed Rust-only benchmark were prepared and formatted
without compiling during the Pages window. The later B allocation executed
Integrity all-target tests, check and strict Clippy on immutable c4786b67.

## R1 native functional verification

All commands actually terminated0. Tests passed24 library cases, one request
parity harness containing81 vectors, one resource parity harness, and one verifier
parity harness containing27 vectors:27 tests total, not108. The example's test
harness ran0 tests; its benchmark main was not executed. Failed/ignored/measured/
filtered counts were all0. Cargo rounded footers were6.06seconds for tests,
3.12seconds for check and1.56seconds for strict Clippy. Scoped fmt also passed.

Source closure compared2563 files before/after with0 difference records.
Cargo artifact identities selected30 external packages plus3 workspace packages;
the selected registry package source/archive/index/config closure compared1041
files before/after with0 differences. The initial broader55842file registry
census is retained, but no unchanged-whole-registry claim follows. Eighteen Cargo
config candidates, three inherited environment keys and three observed tool hashes
were equal. Clippy plugin/driver hashes were captured after-only, without claiming
before-byte equality or complete artifact reuse authority.

The private R1 verification manifest lists18 files with SHA256
2cc4339684a26a5a49c87474e4b5abe179dc80a9fac2c969f4cf57374c61c8c1.
Independent read-only source review found no narrow semantic blocker. Its
measurement caveats remain: hot in-process pairs are not independent process
repetitions, and the internal120second threshold is cooperative after preparation.
Release compilation, actual function A/B execution and product-runtime validation
remain not run. No optimization speedup or allocation count is claimed.
Root later accepted ordinary tool-session supervision and observed terminal
status for the fixed trusted function example, without child spawning, network
or external input. This does not establish a hard OS wall-clock or product
cleanup guarantee. Exact release and direct-binary execution allocations remain
required. Fixture inventory fields now explicitly distinguish initial inspection
from the subsequent native parity execution receipt.

The controlling SoT retains build and runtime scope, exact-byte regression
requirements, security invariants, resource admission and Docker product gates.
The plan lists the first bounded measurements and the remaining full objective.
