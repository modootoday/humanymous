---
id: plan:root/20261107140000
title: Model training correction migration slices
status: draft
created: 2026-11-07T14:00:00Z
updated: 2026-11-07T14:00:00Z
rel:
  depends_on: [sot:root/model-training-correction]
---

# Model training correction migration slices

Root authorized only the source record slice: training/record.rs, training/mod.rs,
minimal lib.rs export and module tests. This source is prepared; all execution,
kernel and subsequent lifecycle slices remain unallocated. No manifest changes.

Subsequent root allocations qualified the corrected first slice at exact source
a0ebc9ae4da4c52020f6e39bb614d1e14d4e0b2e: fmt,13unit+1fixture, all-target check
and strict Clippy passed, including an actual owned-file parser test. First-source
68898522 fmt failure remains historical. This supersedes the execution-pending
statement only for that parser slice; kernel/lifecycle remain unallocated.

1. Accept contracts and independent critique: module placement, RNG parity strategy,
   bounds, required cohorts, temporal ties, weak-label quarantine and lab lifecycle.
2. Preserve immutable baseline and existing inference fixture provenance. Link each
   legacy row/test to existing Rust evidence or an explicit unimplemented gate.
3. Implement shared record/kernel/evaluation and bounded file reader; unit gates
   include warm start, training intermediates, splits, cohorts and absent support.
4. Implement opt-in bounded sink and correction sensors with no production mutation.
5. Implement exact-byte stage/shadow/lab lifecycle and canary/controller snapshots.
6. Implement thin bootstrap/correction binaries after root manifest changes; actual
   file/process pilots must prove both accepted and rejected candidates.
7. Root/runtime/product-lab owners coordinate Docker integration, source closure and
   final row dispositions. Preserve every legacy responsibility until reviewed.

Next lowest L1 sub-slice authorized by root: legacy Standardize only, source and
fmt/spec work. standardize.rs plus training/mod.rs export and4module regression
tests are prepared; all4tests are NOT_RUN. This does not implement Train or resolve
RNG/holdout/lifecycle choices. Next allocation needs scoped fmt first, model units,
check/strict Clippy on an immutable source; no slot has been retained or requested
by source authorship itself. The prior parser14-pass receipt remains exact-source.

Owner6 later qualified e7d8f9a3 with17library+1fixture passed, including4new
standardization tests; fmt/check/strict Clippy passed. No target remains reserved.
This supersedes NOT_RUN for Standardize only, not Train/RNG/evaluation or whole
legacy rows230/231. Next lowest kernel candidates remain evaluation primitives
with explicitly separate gate policy, or RNG after sequence compatibility decision.

Root next authorized source-only Evaluate reduction: evaluate.rs uses existing
Prediction values, exact class counts and float32 rates with absent denominators
unavailable. Four new tests are NOT_RUN; prior18-pass receipt is not inherited.
RNG targets exact Go single-stream compatibility; source/fixture sequence freeze
is required before RNG implementation. No new workload has been started.

Subsequent separate Windows allocation qualified immutable aeefd20c:4reducer tests
passed,17prior library tests filtered; fmt/all-target check/strict Clippy passed.
No own overlays, source changes or common target reuse. Fresh exclusive reducer
target was returned at terminal; remaining evaluation adapter/cohorts/time/RNG,
Train and lifecycle responsibilities remain open.

## RNG L0 proposal for acceptance: no execution or implementation yet

### Retained inputs and observability

Installed VERSION identifies go1.26.3 at2026-05-04T20:36:18Z. Exact rng.go,
rand.go, normal.go, regress_test.go, rand_test.go, VERSION and BSD LICENSE bytes
are retained privately; rng-upstream-provenance.json binds paths/hashes/sizes.
go.mod specifies minimum go1.25.3, not proof of any prior runtime version; root's
explicit compatibility target remains1.26.3. rng-normal-seed0-upstream.json retains
20authoritative NormFloat64 expectations from regress_test.go lines304-323 as
decimal strings. TestRegress reseeds0 before EACH method, takes20calls, and skips
methods with no return. Shuffle is NOT covered. No new execution is claimed.

| Unmodified surface | Observable | Missing |
|---|---|---|
| ml-train fixed data, epochs0 | all cold W1/W2/biases, mean/std, bundle | individual draws/state, pre-split shuffle order |
| ml-train epochs1/2 | final weights; actual training grant required | per-example gradients/swaps/intermediate state |
| TestWarmStart_UsesBaseAndKeepsShape | pass/fail and W2[0]/B2 copy assertions after30epoch base | full warm state, no-init draw count, mixed order |
| mlcorrect-train epochs0 with base | candidate only if gates pass; otherwise exit3/no file | failed-gate weights/permutation/state |
| upstream TestRegress | retained per-method goldens and aggregate test verdict | mixed normal+shuffle snapshots |

Train starts its OWN new seed stream:27W1 normals then1W2 normal PER hidden unit;
warm consumes no initialization draws. Epochs shuffle the already-permuted order
in place. Bootstrap CLI has a SEPARATE stream for generation/pre-split shuffle,
then Train resets its own stream. Fixed -data avoids synthetic draws but not the
distinct pre-split shuffle. Date-derived version metadata must be retained raw
and excluded only from a declared numeric comparison, never silently overwritten.

### Exact fixture envelope and matrix

Proposed schema `humanymous.go1263-training-stream/1` requires these fields:

- provenance: source commit,7upstream hashes/license, tool binary hash/version/host,
  command, input/base byte hashes, observation kind and actual execution status.
- config: seed signed-i64 DECIMAL STRING, feature_dim27, hidden, epochs,
  lr_f32_bits as8hex digits, warm_requested/effective, validated base shape/schema,
  rejection status and explicit draw/swap/output budgets.
- input: ordered stable sample IDs, labels,27f32 feature bitstrings per sample,
  mean/std bits, timestamp/cohort/split membership. No provenance upgrade.
- initialization: ordered normal16hex-f64 outputs and resulting8hex-f32 W1/W2,
  bias zeros/calA1/calB0 and primitive draw counter.
- snapshots: phase(seed/init/epoch-before-shuffle/after-swap/after-example/end),
  epoch/example/swap ordinal, tap/feed,607state u64 words as16hex strings,
  primitive draw count, complete sample-ID order and weights/gradients only when
  an actual later SGD allocation observes them. Missing values are null+reason.
- oracle_class PER observation: retained_upstream, unmodified_go_cli_output,
  unmodified_go_test_assertion, rust_derived_reference or NOT_OBSERVED. Source
  golden values cannot fill unobserved mixed-state fields.

Seeds:0,1,-1,2147483647,-2147483648,i64::MIN/i64::MAX as strings. Pure shuffle
sizes0/1/2/3/7/32/100000; mixed sizes3/7, hidden1/2/16, epochs0/1/3, cold and
shape-valid warm. Reject wrong dim/ragged rows/hidden mismatch/stale schema BEFORE
draws, with explicit rejection fixture. Proposed trace caps4096primitive draws,
128swaps,3epochs and1MiB output; retain production ceilings hidden256/samples100000/
epochs1000 without claiming those full sizes were traced. Cover signed seed modulo,
zero fallback, wrapping state, rejection draws and empty/singleton no-draw behavior.

### Bounded unchanged oracle invocation proposal, NOT RUN

After a distinct root allocation: retained unmodified Go1.26.3, GOTOOLCHAIN=local,
GOPROXY=off/GOSUMDB=off/GOWORK=off, owned temp cache/output, jobs1 and20s deadlines.
Example: `go test math/rand -run ^TestRegress$ -count=1 -timeout=20s`.
Do not pass printgolden; compare existing expectations rather than replacing them.
No Go instrumentation, new Go tests or Go generator is proposed.

A separately allocated zero-epoch initialization pilot may build unchanged
ml-train from the immutable baseline, then invoke its binary with
`-data fixture100.jsonl -hidden 1 -epochs 0 -seed 0 -out cold.json`.
Exactly100bounded aggregate records would be authored by an approved Rust fixture
tool, not generated here. Repeat only admitted seeds/shapes. This provides cold
weight observations, not order/state proof. Warm test and epochs1/2 involve actual
training and remain unallocated. No Go build/test/CLI or model initialization ran.

### Proposed Rust-only kernel and cross-check, pending acceptance

Exact proposed paths: training/rng/{mod.rs,source.rs,normal.rs,shuffle.rs,tables.rs,
tests.rs}; one owning API, existing dependencies, no Cargo or trainer integration.
Port rng607/tap273 seed recurrence/cooked607words with explicit wrapping u64 state;
preserve Uint32 extraction and Float64 retry when rounding produces1. Preserve
128entry ziggurat tables, integer/float widths and draw consumption. Shuffle uses
internal int31n Lemire reduction, NOT public Int31n/Intn. n<=100000 excludes the
huge-index Int63n branch; reject over-cap input instead of approximating it.
BSD notices and root-owned third-party attribution are required before shipping.

Rust-only differential proposal: a slow spec reference with explicit607word vector
rotation/logical indices/draw transcript, independently reviewed against frozen Go
statements, compared to circular-state production kernel AFTER EVERY draw/swap.
Cover warm no-init and successive in-place epochs; anchor primitive/normal output
against retained upstream goldens and later unchanged CLI cold weights. Label
all derived state fixtures rust_derived, never fabricated Go observations. Inject
bounded primitive tapes for normal-tail/rejection and shuffle-rejection branches;
these are algorithm cases, not seed-stream oracles. Shared table/reference origin
is a correlated-error risk, not independent cross-language proof. Integer/fast-path
bits, normal-tail log/exp supported-target differences and unobserved SGD state
must remain explicit. Full mixed-stream/SGD parity cannot be declared from these
vectors alone. No Rust RNG code or new test execution is authored by this proposal.

## Later user-authorized RNG source implementation checkpoint

Direct user authorization now permits the above pinned Rust kernel/differential
tests, with no execution until a separate allocation. Source paths are
training/rng/{mod,source,normal,shuffle,initialize,tables,tests}.rs, LICENSE and
INTEGRATION.md; training/mod.rs exports rng. Existing .txt upstream table data and
normal JSON fixtures are retained in model/tests/fixtures/go1263-* without executing
Go. No Cargo/dependency/feature/shared attribution file was changed.

Twelve authored tests include table literal equality, upstream seed0 Int63/normal
goldens, an independent modular-seed/rotation feedback reference across ring wraps,
per-primitive mixed stream checks, cold/warm/multi-epoch order comparisons, invalid
warm shape/schema/nonfinite states, seed modulo, no-draw empty/singleton shuffles,
atomic budget/size refusal, and injected uniform/normal/shuffle rejection/tail tapes.
All12are NOT_RUN; no formatter/compiler/kernel/Go execution has occurred. Source
inspection alone verified607+128+128+128table literals against pinned upstream
spellings and matching LICENSE hash, not runtime sequence equivalence.

Next allocation must run fmt FIRST, these12 tests, broader model regressions,
check/strict Clippy and selected closure with first failures preserved. Kernel API
draw ceiling1000000 is explicit; full sample/epoch training budgets remain future
integration. Root owns shared third-party attribution before packaging. The full
29row responsibility, Go mixed-state observability gap, architecture math tails,
SGD/cohort/time/lifecycle/CLI and production freeze remain unaltered.

## GPU ML research handoff, no implementation allocation

The owned SoT now contains exact-source candidate families, OS/architecture/vendor/
runtime matrix, arithmetic/parity and cpu/auto/require-gpu contract proposal, bounded
lifecycle/settlement and L0-through-product evidence ladder. Root's structural
canon retains shared policy authority; Performance owns measured crossover and
upper consumers do not infer model activation from GPU discovery.

Source-derived workload handoff:27f32 inputs, H16 default/H256 proposed bound;
H16 uses448products and465parameter floats,521floats/2084bytes including stats/
calibration (excluding metadata),108input bytes per row. Offline100000feature
vectors alone use10.8million bytes. These are formulas, not latency/VRAM readings.
Existing CPU proofs are correctness-only14/18/4 receipts; no matched GPU or CPU
prediction benchmark exists. No speedup/crossover ranking is justified yet.

Research priority is CPU tiny online reference, optional offline independent-row
batch pilots after exact parity/tuple/resource acceptance. Keep ordered stats,
Go RNG and sequential SGD on CPU. Evaluate Candle tagged0.9.1 baseline, Burn
v0.22.1 and independently reviewed wgpu v30.0.1; do not silently combine their
transitive versions or pin old Candle because latest-doc tag lookup was unresolved.
All actual backend/feature/device memory/SDK/driver and supported-target math
qualification remains missing. No Cargo, driver/SDK install, shader, unsafe code,
model download, workload or experiment change is authorized by this research.

Each slice needs Rust 1.99.1/edition2024, scoped tests/fmt/Clippy, allocated actual
pilot and source-bound receipts. No GPU, model download, training, inference,
Cargo or Docker execution is authorized by this plan. No docs/** changes.
