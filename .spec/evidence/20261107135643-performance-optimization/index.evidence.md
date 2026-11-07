---
id: evidence:root/20261107135643
title: Performance readiness and four source-bound library builds
status: active
created: 2026-11-07T13:56:43Z
updated: 2026-11-07T15:01:03Z
rel:
  depends_on: [sot:root/performance-optimization, plan:root/20261107135643]
files:
  - {path: baseline.json, sha256: 7786428c2bd847a59d7175623521b911c441bce23dbf72c20e206c88f7d37cea, bytes: 13105, store: repo}
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
At this verification stage, release compilation and function execution were not
run. Their subsequent actual results are recorded below. Product-runtime
validation and an observed allocation count remain outside this receipt.
Root later accepted ordinary tool-session supervision and observed terminal
status for the fixed trusted function example, without child spawning, network
or external input. This does not establish a hard OS wall-clock or product
cleanup guarantee. Root separately allocated release build and one direct-binary
execution. Fixture inventory fields explicitly distinguish initial inspection
from the subsequent native parity execution receipt.
Premeasurement corpus inspection found15174 UTF-8 bytes across all81 oracle
canonical outputs, while one synthetic case has49152 header payload bytes before
framing. This byte-volume imbalance is not a timing result or representative
product traffic. The fixed corpus remains unchanged and was not reweighted after
timing. Case-sensitive JSON parsing preserved the deliberate header-key collisions;
the initial rejected parse's zero-count exploratory output was discarded.

## R1 release function comparison

The existing authority-followup-release target avoided another owner's reserved
target. One native locked/offline jobs1/incremental0 release example build on
immutable c4786b67 terminated0 in Cargo's rounded16.97seconds. Its emitted
623104byte executable SHA256 was
cce5b7da9dbbebe5d85a97184036688955e86ae0b68dc8bb46f6ff112a795365.
Cargo reported opt-level3/debug0 and existing release defaults, without a profile
change. Source2563 and selected registry1041 files compared unchanged; config18,
environment3 and observed tool3 records matched. Full SDK/reuse authority is not
claimed.

Exactly one direct process then terminated naturally with exit0. Before and after
binary hashes matched, as did the2563file source closure. Standard error was empty.
Strict validation accepted exactly13 top-level fields and10 timing rows:81oracle
plus6synthetic cases,16 warmup corpus iterations,5 alternating AB/BA pairs and256
corpus iterations per variant. Every row contains22272operations and16757760output
bytes. Fixture hash and canonical output digest are retained in baseline.json.

| Pair | Baseline ms | Candidate ms | Paired time reduction |
|---|---:|---:|---:|
| 0 | 20.3217 | 17.1348 | 15.68% |
| 1 | 19.3021 | 16.6484 | 13.75% |
| 2 | 19.5028 | 15.9280 | 18.33% |
| 3 | 18.9100 | 15.9785 | 15.50% |
| 4 | 19.0357 | 15.4773 | 18.69% |

The paired reduction median is15.68%, range13.75-18.69%, for this fixed corpus in
one hot process. This is an observed function comparison, not representative HTTP
throughput, per-request tail latency, measured allocation counts or independent
process reproducibility. The predeclared large-input byte imbalance is material.
There was no rerun or reweighting after seeing data. The120second threshold is
cooperative timed work after preparation, without a hard OS deadline claim.

Admission saw82.9513% host commit,29.7420GiB free physical memory and0 Windows
compiler processes. A later snapshot observed3 compilers; sparse snapshots do not
establish actual timing overlap or continuous quiet. CPU/RSS/full background load
remain unknown. The result does not establish machine-idle or production behavior.
After-only host context identifies Windows11Pro build26200 and an i9-14900K,
24physical/32logical cores with63.3379GiB visible memory. This experiment did not
set a CPU quota or memory limit; inherited job limits were not observed. The
after-only inventory does not prove matched before/after hardware controls.

Exact private rows are r1-benchmark.stdout.json, SHA256
b1eb3af9bd5f9a5a361ac420c3ba3545e56c4c036e0ba6d62a53f424c4376110.
The release/benchmark/corpus manifest contains19files with SHA256
0fbb24d63d113b46b684ba1bbde7f3feef2b45cd0c7f7894734d547409968129.
Actual PID, tool terminal receipt, binary/profile data and detailed input/closure
manifests remain in the owned private bundle. The B compiler/runtime reservation
was returned. The complete performance goal still requires the remaining build
cache/profile research and actual product runtime/security qualification.

The controlling SoT retains build and runtime scope, exact-byte regression
requirements, security invariants, resource admission and Docker product gates.
The plan lists the first bounded measurements and the remaining full objective.
