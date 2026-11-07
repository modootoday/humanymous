---
id: sot:root/model-training-correction
title: Model training and correction migration
status: draft
created: 2026-11-07T14:00:00Z
updated: 2026-11-07T14:00:00Z
verify: manual
rel:
  depends_on: [sot:root/rust-migration]
---

# Model training and correction migration

Document version 0.1.2. Root accepted existing-package placement, lab-only lifecycle,
weak-label quarantine, single-buffer loading and fail-closed cohort support.
Only the first source slice (training/record.rs, training/mod.rs and minimal lib.rs
export with module tests) is implemented and qualified at immutable source
a0ebc9ae4da4c52020f6e39bb614d1e14d4e0b2e: fmt,13unit+1fixture, all-target check
and strict Clippy passed, including the actual owned-file parser pilot. Evidence:
[[evidence:root/20261108005200]]. This supersedes the initial source-only status
for this bounded native component only; it does not qualify the full migration.
Standardization was later qualified at e7d8f9a3 with17library+1fixture passed and
fmt/check/strict Clippy. The next authorized source-only slice is evaluation metric
reduction was subsequently qualified at aeefd20c:4reducer tests passed,17prior
library tests filtered, fmt/check/strict Clippy passed in a separate native target.
[[evidence:root/20261108005200]] binds exact source/closure and exclusions.
The user subsequently authorized the pinned Rust RNG kernel and differential
tests as SOURCE ONLY. training/rng now owns explicit seeded state, bounded normal/
shuffle, cold/warm initialization helpers and12authored tests, all NOT_RUN.
New crates, manifests, binaries, SGD and lifecycle remain unapproved. Full legacy responsibility
remains in scope. No retirement, training-loop, production inference or full runtime
parity is claimed. Existing pure inference fixture comparison was executed only
under the explicitly allocated component unit-test grant.

## Scope and existing boundary

Preserve `internal/mlserve`, `internal/mltrain`, `internal/mlcorrect`,
`cmd/ml-train` and `cmd/mlcorrect-train` without duplicating existing Rust inference.
`crates/model/src/inference` already owns MlpBundle, float32 MLP, schema guard,
active/shadow runtime, threshold-provider and observer seams. Its existing fixture
test describes five bundles with 68 vectors each and exact float32 comparison.
This is source inspection, not a new execution receipt or full runtime proof.

L4 model observations remain annotation-only. Production weights, thresholds,
predicates and verdict bands are frozen. Training output never activates a model.
Calibration proposals MUST NOT be installed as a production ThresholdProvider.
Preserve legacy promotion/rollback and calibration functionality in an isolated
lab control plane; production activation requires a separately accepted design
and explicit authorization. Missing model evidence means abstention, never a
successful strict-route enforcement decision; required-signal enforcement remains
owned by runtime/scoring. T4 remains an intentional ceiling.

## Responsibility contracts

| Legacy responsibility | Existing Rust | Remaining contract |
|---|---|---|
| mlserve bundle/math/runtime | inference modules and inference_parity.rs | Bind every Go row/test and fixture hash to actual executed evidence; do not rewrite predictor |
| mltrain record | behavior feature extraction exists | JSONL label 0/1, raw BehaviorSummary, optional ts/cohort/source; one shared extraction path |
| mltrain sink | no implementation found in model | Explicit opt-in aggregate-only append writer, reopen preservation, count/error/close semantics and restricted access |
| mltrain training | no implementation found in model | Standardize, float32 SGD/BCE/ReLU, He initialization, seed, shape-safe warm start, evaluation, cohort FPR, stable time split, AUT |
| mlcorrect calibration | observer/provider interfaces only | Human-only clamped ACI state, EMA/counts; bots and ambiguous outcomes do not move calibration; isolated proposals |
| mlcorrect drift/features | FeatureObserver seam only | Page-Hinkley, PSI, 2-of-3 fused proposal, frozen reference and rolling windows |
| mlcorrect oracle | no implementation found in model | Independent provenance, ambiguous queue, Pass-fraction cap and solve-anomaly quarantine |
| mlcorrect shadow | ShadowObserver seam only | Comparable-pair counts, abstention directions, mean absolute delta, firing agreement and direction counts |
| mlcorrect bundle | Mlp loader/runtime setters only | Exact-byte SHA256, optional configured Ed25519 verification, schema pin, stage/active/previous identity, explicit promote/rollback transitions in lab |
| mlcorrect canary/controller | interfaces only | Arm/trip/graduate, one-shot callback outside locks, integrated snapshots and observer wiring; lab rollback only |
| both training commands | no equivalents found in model | Thin Rust process shells sharing one training kernel; JSONL, synthetic bootstrap, anchor/oracle/base inputs, metrics and candidate digest |

Proposed modules under existing `crates/model`: `training/{record,sink,kernel,
evaluate}`, `correction/{calibration,drift,features,oracle,shadow,bundle,canary,
controller}`. Two thin Rust binaries are proposed within this package, subject
to root manifest ownership approval. No new crate or duplicate feature extractor
is proposed. Rust 1.99.1 / edition 2024 applies to implementation and pilots.

## Compatibility and failure rules

### Next authorized source slice: population standardization

Root authorized the lowest deterministic L1 kernel without RNG/policy changes.
`training/standardize.rs` and its module export implement only Standardize from
original baseline row/files/231. Contract: preserve input order; float32 sums and
mean division; float32 squared-delta population variance divided by count before
float64 sqrt, cast to float32, then add1e-6. Empty input is mean0/std1; nonempty
constant input deviation is1e-6. Do not substitute sample variance, sorted or
float64 accumulation, a different epsilon placement, or blanket negative clamping.
Reject schema mismatch, zero/>100000 sample budget, over-budget input, nonfinite
features and nonfinite intermediate sums/variance with typed errors. Overflow
rejection is an explicit fail-closed hardening delta; no invalid bundle is emitted.
The kernel neither selects samples nor admits provenance nor alters their bytes.
Fixed-size shared FeatureVector arrays avoid a second schema/extractor definition.
This pure slice was qualified at e7d8f9a3 with4new tests passed; no
training-loop/RNG, temporal gate, lifecycle or activation authority is inferred.

### Next source-only slice: offline metric reduction and RNG L0 decision

Original row/files/231 Evaluate classifies p>=0.5 as bot and computes float32
correct/total, humanFP/humans and botTP/bots. training/evaluate.rs reduces existing
inference::Prediction values, without running or duplicating forward math,
Runtime configuration or model loading. Valid labels0/1 derive human/bot membership
from one field, avoiding inconsistent legacy Y/Human pairs. Return exact numerators
and denominators. Defined-denominator arithmetic is unchanged; absent support
returns None instead of legacy zero, an explicit reporting delta preventing missing
classes appearing measured. This reducer does not implement promotion, cohort
requirements, time split or label admission. Reject invalid labels, abstention,
nonfinite/out-of-range probabilities and zero/>100000/over-budget input counts.
Late errors reject the whole result. The later bounded native allocation executed
all4reducer tests at aeefd20c; no broader training or inference execution follows.

RNG L0 target: retain Go1.26.3 math/rand NewSource(seed) sequence compatibility,
not a generic seeded Rust generator. One stream feeds ordered NormFloat64 calls
for W1 then W2 per hidden unit and subsequent epoch shuffles; compatible warm start
consumes no initialization draws. Negative/zero seeds and shape rejection need
fixtures. Independent init/shuffle streams, alternate PRNG/normal algorithms and
seed-only reproducibility claims are excluded. Before RNG implementation, freeze
exact rng/rand/normal algorithm/table source hashes plus cold/warm/epoch-order and
intermediate state fixtures, or explicitly accept a non-byte-compatible versioned
format. No Go execution, fixture generation, initialization or RNG implementation
is granted here. This sequence-proof dependency does not block pure metric counts.

### User-authorized pinned RNG source implementation

The later direct user instruction accepts the L0 contract for Rust implementation
and differential tests, while explicitly withholding execution until allocation.
training/rng/{mod,source,normal,shuffle,initialize,tables,tests}.rs plus its LICENSE
and INTEGRATION record implement only the offline kernel. No formatter/compiler,
Go oracle, training/inference process or new runtime is executed by this grant.

Preserve Go1.26.3 signed seed modulo/fallback,607/tap273 wrapping state, Int63/
Uint32/Float64 retry,128entry ziggurat tables with original float widths, internal
int31n Lemire shuffle and descending in-place epoch permutations. Cold initialization
consumes27W1+1W2 NormFloat64 calls per hidden row; warm validates/copies without
draws and leaves the next shuffle on the freshly seeded stream. Shape/schema/
nonfinite validation precedes draws; requested malformed warm state fails closed
instead of legacy cold fallback. No public API installs a model or changes scoring.

The finite API supports hidden1..256, samples0..100000 and an explicit primitive
draw budget1..1000000. This is a bounded kernel profile, not qualification of the
entire future training sample/epoch maximum. Budget exhaustion is explicit and
compound methods roll back partial state/output; never silently reseed or fork a
new stream to continue. Full-training workload budgets remain an integration gate.

License text and original Go copyright notices are preserved; aggregate shared
attribution updates remain root-owned before packaging. Retained seed0 output
goldens are upstream expectations. State/order/init transcripts and the slow
rotation-reference comparisons are Rust-derived, never Go-observed snapshots.
Shared tables/sampling logic limit reference independence; tail ln/exp/sqrt rounding
needs actual target evidence. All12new tests, fmt/check/Clippy and oracle comparisons
remain NOT_RUN. Prior parser/standardization/evaluation receipts remain exact-source.

The accepted first slice uses explicit configurable ceilings of1MiB per physical
line including its newline,128MiB total bytes including blank lines,100000 parsed
records including quarantine,64 normalized cohorts and128 UTF-8 bytes per metadata
string. Configurations may tighten but never exceed these ceilings. Legacy rows
lack schema identity, so the caller supplies dataset schema and it must match the
shared extractor before reading. This checks compatibility, not dataset authenticity.
Required label and aggregate object must decode; only labels0/1 and finite raw and
extracted floating-point values are accepted. Unknown top-level fields are rejected.
Zero/equal timestamps remain preserved metadata for subsequent temporal policy.
Consistent catalog-bot and Pass-human source claims require external review;
weak/unknown/missing/inconsistent provenance is quarantined, never auto-admitted.
No training consumer is connected by this slice. All retained records carry their
original aggregate/provenance, including quarantine. Input errors reject the whole
result without returning a partial batch. Shared inference behavior is unchanged.
Optional metadata uses Option fields: omitted/null remains absent, distinct from
explicit timestamp0 and empty cohort/source; serialization omits absent fields.
Cohort counting normalizes absent/empty to default without overwriting the record.
Finite negative aggregates preserve the existing extractor's field-specific behavior
(log inputs clamp at zero, direct features remain signed); no blanket negative
rejection or duplicate clamping is introduced. Typed RecordError variants exclude
input bodies, paths and underlying JSON/I/O diagnostic strings.

Independent source-only critique: parser slice is ready subject to these contracts;
kernel/gate/lifecycle remains needs-work. A single pinned Go1.26.3 RNG stream drives
normal initialization then per-epoch shuffle; warm-start skips initialization draws.
Cold/warm/order/intermediate fixtures must cover multiple epochs as one sequence,
or root must explicitly accept a non-byte-compatible delta. Independently configured
required cohorts and positive human/bot support, timestamp ties/zero and anchor
membership/leakage policy remain prerequisites to later gate implementation.

Preserve JSON field names and float32 operation order, sigmoid's float64 exp then
float32 conversion, zero-standard-deviation fallback, schema identity and explicit
abstention. Training reproducibility needs an explicit RNG decision: Go seeded
normal/shuffle sequences are not implied by a Rust seed. Require retained golden
initialization/order/intermediate fixtures or a reviewed compatibility delta;
do not silently substitute a Rust RNG and claim byte-identical training.

Legacy CLI defaults: hidden16, epochs60, lr0.05, seed1. Bootstrap supports
gen/data/out; correction requires anchor, optionally oracle/base, test-frac0.25,
max-human-fpr0.005, min-bot-tpr0.80, min-cohort-humans20. These are offline gate
parameters, not authorization to alter production cut-points. Preserve correction
exit3 on gate rejection and no output writes; diagnostics stay on stderr and a
successful digest alone goes to stdout. Test an existing output file remains
unchanged on rejection. Base bytes must be read once and validated from that same
buffer; legacy double-read TOCTOU must not be reproduced.

Legacy temporal split is stable for equal/zero timestamps; it does NOT guarantee
strictly separated times when ties cross the cut. Report ties and missing times;
require a documented evaluation policy before claiming future-only holdout.
Per-cohort validation must enumerate required cohorts, fail closed on missing or
undersampled cohorts, and require bot support. Legacy iteration only over observed
human cohorts can vacuously pass an all-bot holdout. Retain this as a regression
case, not a behavior to ship silently. Frozen anchor leakage/deduplication and
source quotas need explicit reports. Failed/abandoned CHALLENGE is legacy weak
bot evidence, not ground truth; preserve the mapping in compatibility fixtures,
quarantine unreviewed weak labels from production-derived training admission.

## Security and bounded resource proposal

Before allocation, validate finite values, feature shapes, labels, timestamp/cohort
lengths and positive bounded configuration. Proposed initial offline limits,
pending root acceptance: 1MiB per JSONL line (legacy scanner limit), 100,000 records,
128MiB total input, hidden<=256, epochs<=1000, monitor window<=4096, <=64 cohorts
and <=128 UTF-8 bytes per cohort/source. Estimate dataset/weight/gradient memory
and operation count before training; root assigns the actual execution budget.
These are proposed safety deltas, not measured capacity guarantees.

Sinks are off by default, accept aggregates only, preserve provenance and avoid
identifiers/raw streams. Aggregates are potentially personal data. Require a
purpose/retention policy, native access-control pilot on each supported OS, bounded
queue and file quota, observable drops/errors, flush/close receipts and explicit
overflow policy. A mutex+flush is not proof of nonblocking request handling; never
perform synchronous disk I/O on the serving path. Durable flush is not fsync.
Erasure and kill-switch remain dual-controlled in their owning context.

Stage hashes/verifies/parses a single bounded immutable byte buffer; signed and
unsigned lab policy must be explicit. Configured key requires valid signature.
Do not call optional digest/signature configuration an authenticated supply chain.
Replay/stale identity, wrong key/schema, concurrent stage/promote/rollback and
callback reentrancy require tests. Snapshot APIs must not mutate drift event counts.
No unattended production model activation, training process launch or resource use.

## Verification and acceptance

Unit gates preserve each legacy test responsibility, malformed inputs, missing
cohorts/bots, tied time splits, warm-start shape errors, float32 intermediate math,
Pass poisoning, invalid signatures and one-shot canary transitions. Observer
callbacks must execute outside configuration locks; concurrent lifecycle and
overflow/drop behavior need meaningful tests. Preserve existing inference fixtures.

After root acceptance and allocated execution only: scoped cargo test --locked,
fmt and Clippy; actual Rust binary pilots with owned temporary files must exercise
successful bootstrap/correction, rejected poisoned cohort, missing cohort/bot,
invalid JSON/oversize input, existing-output preservation, sink reopen/flush,
single-buffer signed stage/shadow/manual promotion/rollback and source closure.
Compare generated artifact inference using the existing Mlp, not a forked forward
pass. Record exact binary/source/fixture hashes, commands, limits, exit status,
executed/ignored cases and first failures. Windows access control and Linux process
pilots remain separate. Product integration later requires the Docker-only owned
topology and runtime-owner agreement, with frozen scores/verdicts compared before
and after. Unit/source existence is never full product parity.

## Critique reflected and unresolved acceptance

Blue: reuse the inference seam and keep L4/Gate authority separate. Red: weak labels,
vacuous cohort gates and poisoned reference windows require admission checks. SRE:
bounded asynchronous collection, exact-byte staging and callback isolation are
required. DPO: aggregate traces require retention/access controls. Evaluator:
source inspection and historical fixtures are not new runtime proof.

Current verdict: the bounded parser slice was accepted by root, independently
reviewed without a new source blocker, implemented and qualified as described
above. Module placement, parser bounds, provenance quarantine and lab-only lifecycle
direction are accepted; this is not blanket acceptance of later lifecycle APIs.
Independent critique found kernel/gate/lifecycle needs-work: RNG sequence parity,
independently required cohorts and bot support, time/anchor leakage policy and
exact lifecycle approval/identity contracts remain decisions before those slices
can be implemented. No independent review is pending for the already qualified
parser source; subsequent-slice design acceptance and implementation review remain
open. No scoring delta is proposed. Final baseline mapping and Go
retirement belong exclusively to root after source-bound qualification.

## GPU eligibility research, 2026-11-08 KST: proposal only

The user's project-wide GPU request authorizes research and owned spec/proposals,
not backend activation, GPU work, driver/SDK installation, dependency/lock changes,
authored shaders/foreign code or unsafe exceptions. This section owns product ML
contracts only; root's structural canon owns shared scheduling/selection policy.
Selection vocabulary maps to cpu/auto/require-gpu in
[[sot:root/structural-development]]; no separate shared compute API is claimed.
Existing CPU experiments and RNG72888805 source-only status remain unchanged.

### Workloads and current CPU evidence

Input width27; legacy default hidden16, proposed cap256. MLP inference performs
27H hidden products plus H output products per row:448 multiply/add pairs atH16,
not a measured throughput figure. Parameters29H+1=465floats atH16; adding54
standardization and2calibration floats gives521floats/2084bytes, excluding JSON
metadata. Input is108bytes/row; return includes f32 probability and abstention.
Inference accumulates ordered f32 and evaluates sigmoid via f64 exp then f32 cast.
Shapes/identity/dtypes remain frozen. Source: crates/model/src/inference/mlp.rs.

CPU receipts are correctness proofs: a0ebc9ae13unit+1fixture, e7d8f9a317library+
1fixture, aeefd20c4reducer tests with17filtered. The inference fixture has5models
x68vectors and exact existing output assertions. No per-request CPU latency,
CPU/GPU throughput, energy or transfer crossover was measured. Test-suite elapsed
times are NOT prediction benchmarks. RNG72888805 has12new tests NOT_RUN.

| ML responsibility | GPU eligibility under frozen contract | Priority |
|---|---|---|
| Single-request tiny MLP | No qualified GPU equivalent; launch/transfer/queue overhead may dominate (inference, not measurement) | CPU reference first |
| Offline independent-row inference | Candidate for parallel rows, but each row's reduction/exp parity still required | Compare after an accepted isolated pilot |
| Sequential per-example SGD | Parallel minibatch/autodiff changes update order/algorithm; not a compatible replacement | Keep CPU; no accelerated training acceptance |
| Standardize | Parallel sum changes ordered-f32 means/variance | Keep existing CPU kernel |
| RNG/normal/epoch shuffle | Sequence/state/libm and warm draw-skipping must match pinned Go contract | CPU-only compatibility source |
| Reducer/cohort counts | Tiny bounded integer/branch work; transfers add a boundary | CPU; future GPU-resident metrics not assumed equivalent |
| Calibration/canary/lifecycle | Policy/state authority, not a throughput matrix kernel | CPU; activation remains separately prohibited |

### Exact reviewed sources, not installed dependency pins

- [Candle0.9.1 workspace](https://raw.githubusercontent.com/huggingface/candle/0.9.1/Cargo.toml)
  and [feature manifest](https://raw.githubusercontent.com/huggingface/candle/0.9.1/candle-core/Cargo.toml)
  expose CPU, optional CUDA/cuDNN and Metal. Rust API plus upstream kernels is not
  an all-Rust SDK supply chain. This is a resolvable tagged source baseline, not a
  recommendation to pin an older version. Latest docs identify0.11.1, but attempted
  source URLs0.11.1/v0.11.1 did not resolve; reconcile exact release source before
  any adoption.0.9.1 manifest does not establish Rust1.99.1 transitive build success.
- [Burn v0.22.1 manifest](https://raw.githubusercontent.com/tracel-ai/burn/v0.22.1/crates/burn/Cargo.toml)
  declares MSRV1.95 and features CUDA/ROCm/wgpu/Metal/Vulkan/WebGPU/CPU/Flex.
  [Burn0.22 Device docs](https://burn.dev/docs/burn/tensor/struct.Device.html)
  have explicit device factories and no implicit CPU fallback. Use an explicit
  backend, not feature-unification-dependent default ordering; docs0.22.0 and
  tag0.22.1 are distinct research inputs. [Fusion docs](https://burn.dev/docs/burn/backend/fusion/index.html)
  describe default CubeCL fusion, a numerical/order review concern.
- [wgpu v30.0.1 platform/MSRV source](https://raw.githubusercontent.com/gfx-rs/wgpu/v30.0.1/README.md)
  lists native Vulkan/Metal/DX12 and browser WebGPU; library MSRV1.87. It is a
  device/queue abstraction, not an MLP/trainer. Handwritten WGSL/GLSL/HLSL or
  passthrough unsafe code is NOT authorized. Burn/wgpu transitive versions must be
  reconciled rather than assuming this independently reviewed wgpu version fits.
- [CUDA Windows13.5 guide](https://docs.nvidia.com/cuda/cuda-installation-guide-microsoft-windows/index.html)
  documents OS/compiler/toolkit prerequisites; it is not permission to install or
  a claim that this host has that toolkit. [NVIDIA floating-point guide](https://docs.nvidia.com/cuda/cuda-programming-guide/05-appendices/mathematical-functions.html)
  explains non-associative sums and differing FMA rounding.

Declared MSRV below1.99.1 is only a syntax eligibility signal. Exact transitive
versions, upstream foreign kernels/build tools, SDK/license artifacts and actual
Rust1.99.1 platform builds remain unverified; no dependency was added.

### OS/architecture/vendor/runtime candidate matrix

These are eligibility hypotheses, not product support or measured capabilities.

| Target tuple | Candidate runtime/vendor | Status and requirement |
|---|---|---|
| Windows x86_64, NVIDIA | Burn/wgpu DX12 or Vulkan; Candle/Burn CUDA | Current inventory candidate; exact driver/runtime, parity and real pilot absent |
| Windows x86_64, AMD/Intel GPU | Burn/wgpu DX12 or Vulkan | No local adapter proof; enumerate features/limits before qualification |
| Linux x86_64, NVIDIA | CUDA or Burn/wgpu Vulkan | Separate driver/container/host tuple and artifact tests needed |
| Linux x86_64, AMD | Burn ROCm or Vulkan | Feature exists; exact vendor SDK/GPU compatibility not researched to a supported tuple |
| Linux x86_64, Intel | Burn/wgpu Vulkan | Driver/features and real allocation absent; no CUDA implication |
| Linux aarch64, supported GPU | Conditional vendor Vulkan/CUDA stack | No ARM GPU/SDK/packaging proof; do not inherit x86 support |
| macOS aarch64, Apple GPU | Candle Metal or Burn Metal/wgpu | Native Apple tuple needed; no local Metal proof |
| macOS x86_64, Metal-capable GPU | Conditional backend/hardware support | Intel/AMD Mac tuple unverified; M1 claims do not qualify it |
| Browser wasm32 | Burn/wgpu WebGPU where implemented | Browser/secure-context/permission/features differ; no native CUDA/Metal claim |
| Windows aarch64 or other targets | CPU reference until separately qualified | No GPU backend tuple accepted from package marketing |
| CPU/software or unknown/virtual adapter | CPU/fallback only unless physical hardware mapping proven | Never count stub/software throughput as hardware GPU proof |

Read-only Win32 inventory reports Windows11Pro10.0.26200 x86_64, i9-14900K
(24cores/32logical) and NVIDIA GeForce RTX4070Ti, driver32.0.16.1088, StatusOK.
[NVIDIA's hardware list](https://developer.nvidia.com/cuda/gpus) associates this
model with compute capability8.9. This is a model-name lookup, not a runtime probe.
No CUDA/Vulkan/DX12 device, usable VRAM, feature support or real compute dispatch
was tested. No nvidia-smi/toolkit/load command was run. Public evidence omits the
machine-specific PNP identifier. Do not derive GPU eligibility from CPU vendor.

### Numerical and decision acceptance

Compatibility mode requires absolute tolerance0, relative tolerance0, same finite/
abstain result and exact p_bot bits; require identical annotation/fire comparisons
at every existing cut-point, schema/version/artifact identity and final score/
verdict. No threshold, RNG, weight or promotion change is authorized. Zero-error
output alone does not excuse an unreviewed operation-order change.

Generic matmul/reductions, fusion/FMA, tensor-core reduced precision/TF32, mixed
precision and autodiff optimizer rewrites are not known to preserve the ordered
f32 reference. CPU's f64 exp then f32 cast is not interchangeable with device f32
exp. [wgpu30 SHADER_F64](https://docs.rs/wgpu/30.0.1/wgpu/struct.Features.html#associatedconstant.SHADER_F64)
is an optional native Vulkan feature, not a portable DX12/Metal/WebGPU guarantee.
Consequently NONE of the investigated GPU candidates currently satisfies exact
model/standardization/RNG/training equivalence. No backend may replace the CPU
reference based only on similar aggregate accuracy or a device's advertised FLOPS.

For separately approved exploratory offline diagnostics only, proposed error
reporting uses abs<=1e-6+1e-5*abs(cpu) and ULP histograms, with exact abstention
and threshold decisions separately counted. This nonzero diagnostic band is NOT
compatibility acceptance or permission to change outputs. Any boundary disagreement
refuses equivalent status. GPU-produced offline training artifacts would need a
distinct reviewed algorithm/artifact identity, all cohort/time gates and manual
promotion; that scope is not currently accepted. RNG streams stay CPU and unchanged.

### Proposed interface and selection priority, no source implementation

Separate pure BackendPolicy/Workload/ParityRequirement/Budget/QualifiedTuple from
an owned compute session. Workload includes online-single, offline-batch and offline
training; policy is CpuOnly, PreferGpu or RequireGpu. Descriptors bind API/vendor/
device class/driver/runtime/compiler/backend version/features to a model digest,
batch limits, parity receipt and execution venue. They do not grant activation.

1. Preserve the existing CPU reference and finish its current experiments unchanged.
2. Check policy, accepted dependency tuple, resource allocation and exact parity
   qualification BEFORE device initialization or submission.
3. CpuOnly always uses CPU. Online tiny MLP stays CPU; RequireGpu on an unqualified
   workload returns a typed refusal, never silently falls back or enables a model.
4. PreferGpu can use only an explicitly qualified hardware tuple AND measured
   end-to-end crossover for that batch/model; otherwise CPU with a finite reason.
   No global size threshold or automatic backend switch is introduced by this study.
5. Candidate research order: portable Burn/wgpu for vendor reach, then CUDA on
   qualified NVIDIA hosts and Metal on qualified Apple hosts. Compare Candle and
   Burn at the same workload; choose no speed winner without data. Direct wgpu
   custom kernels remain deferred behind shader/unsafe/implementation review.

wgpu [DeviceType](https://docs.rs/wgpu/30.0.1/wgpu/enum.DeviceType.html) separates
CPU/software, integrated, discrete and virtual devices; [fallback adapter options](https://docs.rs/wgpu/30.0.1/wgpu/type.RequestAdapterOptions.html)
can request software. A future RequireGpu check must verify physical/API features
and completed real compute, not accept a noop/software device or a marketing name.
Virtual adapters require actual physical-provider mapping before hardware claims.
CPU fallback is explicit and must preserve the frozen result; no silent half-GPU
half-CPU batch or resuming SGD from a partial device update. If the CPU itself
cannot complete, propagate typed failure/abstention under existing signal policy,
not fabricated success or relaxed strict-route enforcement.

### Crossover and bounded lifecycle proposal

Measure T_gpu=end-to-end startup/amortization + host packing + host-to-device +
queue wait + compute + device-to-host + synchronization/unpacking; compare to
identical CPU semantics. Report warm/cold p50/p95/p99 latency, throughput, CPU
utilization and allocation/VRAM context separately. No crossover exists in current
evidence. Rows27x4=108bytes and tiny weights suggest (inference) batching/residency
may be necessary; high kernel FLOPS is not proof of lower per-request latency.
[wgpu queue docs](https://docs.rs/wgpu/30.0.1/wgpu/struct.Queue.html) describe staged
copies and asynchronous submission/completion, so enqueue time alone is insufficient.

Proposed first isolated research budget, still requiring allocation: one GPU job,
batch1/16/64/256/1024/4096, H16 initially, logical tensors<=64MiB, staging<=16MiB,
one in-flight submission and queue<=4jobs. Driver/context overhead is separately
measured and is NOT promised to fit the tensor budget. Unified-memory capacity
must not double-count the same physical pool; driver, shape, residency, precision
or concurrency changes invalidate crossover receipts. Deadline/cancellation and
device-loss probes precede any training; exact startup/dispatch deadlines belong
to the reviewed provider pilot, not invented watchdog guarantees.

Session state: admitted->opening->ready->submitted->completed->closing, with
lost/timeout/cancelled terminal RESULTS but resource lease held until actual
completion/reap. Tag submissions/completions by session generation; reject stale
completion after device loss. Keep callbacks off ownership locks and short.
[wgpu Device](https://docs.rs/wgpu/30.0.1/wgpu/struct.Device.html) exposes loss callback,
destroy and polling, but cancellation after submission does not prove GPU work
stopped. Bounded chunks/owned process supervision need real pilot evidence;
never reset a shared GPU, kill another session or install a driver to force cleanup.
Reject OOM/unsupported features/device lost with typed finite reasons; PreferGpu
may restart a complete independent batch on CPU only after settling its old lease,
while RequireGpu refuses. Training restart/checkpoint semantics remain unimplemented.

### Required evidence ladder and current gaps

L0: review exact dependency/license/SDK tuple, Rust1.99.1 build and policy/parity
contract; currently NOT_DONE. L1: pure selector, bounds, refusal/fallback and stale
completion tests; NO_GPU_IMPL and NOT_RUN. L2: allocated real physical-adapter
enumeration plus actual dispatch/readback, timeout/cancel/loss/cleanup and memory
negative cases on each supported tuple; NOT_RUN. Software/noop/mock cases remain
separate. Then matched CPU/GPU numeric and crossover runs with exact inputs, binary/
driver/compiler/artifact hashes and excluded cases; NOT_RUN. L3/L4 provider/CLI
integration requires owner agreements, no serving hot-swap. L5 product authority
still requires owned Docker topology and actual runner; no GPU component result
becomes full product success. No future GPU pilot can retroactively qualify RNG
source-only or missing Go state/SGD/cohort/time/lifecycle responsibilities.
