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

## RNG12 bounded validation profile ready for one root allocation

Frozen source is728888051b7dacfa3e3181cf71da4fff2f526bd4 with ZERO overlays.
Current13owned source/license/fixture file hashes and sizes match the original
checkpoint; no model code changed during this preparation. Exact twelve names,
upstream seven-file identity/license, commands, exclusions and deadlines are
registered in rng-validation-profile.json. No oracle/kernel/formatter/Cargo run
was performed and no compiler/GPU/process lease was retained.

Proposed fresh exclusive target: target/validation-model-rng-72888805 (absent at
preparation). Native Windows Rust1.99.1, jobs1/inc0, locked/offline and test-threads1.
After actual admission free>20GiB/commit<85%, execute fmt FIRST(30s), RNG-filtered
library tests(180s), library check(120s), strict Clippy lib/tests(120s), total450s.
The focused command is `cargo test --locked --offline -p humanymous-model --lib
training::rng::tests --jobs 1 -- --test-threads=1`; expected12executed and21prior
library tests filtered IF compilation succeeds. Existing inference integration
fixture is deliberately not rerun. Clippy compilation is not test execution.

Fmt was explicitly withheld previously and may fail first on the authored layout;
preserve this failure and immutable candidate, never preannounce12passes or silently
format the archive. Routine source correction requires its own new frozen bytes
and receipts. Do not carry72888805 results onto corrected source. Release target
at terminal before evidence-only work; no broad sysroot/cache census or repeat of
prior tests merely to refresh status. Source-table/output expectations and
Rust-derived mixed state/epoch comparisons retain their distinct evidence classes.

First allocated attempt did not reach fmt or a Cargo child: timezone-kind loss in
the persisted deadline guard caused a false expiration. Cleanup actually completed
in90.168993s, inside390/450s, with the original wrong calculation retained alongside
corrected DateTimeOffset analysis.12RNG tests remain NOT_RUN; selected closure is
incomplete. Target/marker settled and no retries/source edits occurred. Before a
renewed grant, correct orchestration to parse original UTC strings once as
DateTimeOffset and keep typed offset-bearing deadlines; do not convert materialized
DateTime objects through locale strings or hide preparation gaps. Profile0bd901e2
is historical, not an active lease or successful qualification.

## Repaired monotonic orchestration: literal review snippets, NOT EXECUTED

Root requires ONE in-process Stopwatch for any renewed attempt. The earlier
DateTimeOffset-based runtime deadline proposal is superseded: UTC values are audit
strings ONLY and must never drive admission, remaining budget or WaitForExit.
Do not renew the failed clock, restart the stopwatch or split execution into fresh
tool processes. Prepare the complete orchestration body BEFORE a new allocation;
only an explicitly granted run starts the following clock. This is review-only
operational orchestration, not a new product implementation or executed script.

### Clock initialization and first persistence

```powershell
$rngClock = [System.Diagnostics.Stopwatch]::StartNew()
$rngAuditStart = [DateTimeOffset]::UtcNow.ToString('o')
$rngWorkBudgetMs = [long]390000
$rngHardBudgetMs = [long]450000
$rngRun = 'C:/workspaces/humanymous/.spec.local/root/evidence/NEXT_GRANTED_RNG_RUN'
$rngUtf8 = [System.Text.UTF8Encoding]::new($false)
New-Item -ItemType Directory -Path $rngRun -ErrorAction Stop | Out-Null
$rngAudit = [ordered]@{
    started_utc = $rngAuditStart
    work_budget_ms = $rngWorkBudgetMs
    hard_budget_ms = $rngHardBudgetMs
    stopwatch_frequency = [System.Diagnostics.Stopwatch]::Frequency
    monotonic_start_elapsed_ticks = $rngClock.ElapsedTicks
    controller_pid = $PID
    source_commit = '728888051b7dacfa3e3181cf71da4fff2f526bd4'
    overlays = @()
}
[IO.File]::WriteAllText(
    (Join-Path $rngRun 'clock.json'),
    ($rngAudit | ConvertTo-Json -Depth 4), $rngUtf8
)
```

The clock precedes the first filesystem preparation operation; the first write
records fixed budgets, not wall-clock deadlines. The same live object survives all
preparation, native children, hashing and cleanup. Reading clock.json later is
for evidence only. A process restart cannot resume or reconstruct this budget.

### Remaining work and cleanup budgets

```powershell
function Get-RngRemainingMs([long]$budgetMs) {
    $left = $budgetMs - $rngClock.Elapsed.TotalMilliseconds
    return [int][Math]::Max(0, [Math]::Floor($left))
}

if ((Get-RngRemainingMs $rngWorkBudgetMs) -le 0) {
    throw 'Work budget exhausted: enter cleanup without another child'
}

$rngTick = [ordered]@{
    audit_utc = [DateTimeOffset]::UtcNow.ToString('o')
    elapsed_ticks = $rngClock.ElapsedTicks
    elapsed_ms = $rngClock.Elapsed.TotalMilliseconds
    work_remaining_ms = Get-RngRemainingMs $rngWorkBudgetMs
    cleanup_remaining_ms = Get-RngRemainingMs $rngHardBudgetMs
}
```

No DateTime.Parse, UTC comparison, JSON time roundtrip, locale conversion or
Elapsed reset is permitted in control flow. Fresh memory admission is checked
before each new command: free>20GiB/commit<85%; no new compiler at free<16GiB or
commit>=88%. WMI observations are bounded separately and do not restart the clock.
Check remaining work before/after archive IO and each selected input hash. Timed
out preparation or a vanished source is a first failure, not a hidden retry.

### Native child wait and bounded settlement

Every archive/metadata/formatter/compiler native command uses this pattern with
its actual owned process, command log and per-step cap. The entire command list
and code body exist before preparation starts; the model must not construct the
next command through another tool turn while a target lease is held.

```powershell
$rngWaitMs = [Math]::Min($rngStepCapMs, (Get-RngRemainingMs $rngWorkBudgetMs))
if ($rngWaitMs -le 0) { throw 'No work budget: do not start child' }
$rngStepEndElapsedMs = [Math]::Min($rngWorkBudgetMs,
    $rngClock.Elapsed.TotalMilliseconds + $rngWaitMs)
$rngChild = Start-Process -FilePath $rngExecutable -ArgumentList $rngArguments `
    -WorkingDirectory $rngSource -WindowStyle Hidden -PassThru `
    -RedirectStandardOutput $rngStdout -RedirectStandardError $rngStderr
$rngChildAudit = [ordered]@{
    pid = $rngChild.Id
    started_utc = [DateTimeOffset]::UtcNow.ToString('o')
    started_elapsed_ms = $rngClock.Elapsed.TotalMilliseconds
    wait_budget_ms = $rngWaitMs
}
[IO.File]::WriteAllText($rngChildReceipt,
    ($rngChildAudit | ConvertTo-Json), $rngUtf8)
$rngWaitMs = [int][Math]::Max(0, [Math]::Floor(
    $rngStepEndElapsedMs - $rngClock.Elapsed.TotalMilliseconds))
$rngFinished = $rngWaitMs -gt 0 -and $rngChild.WaitForExit([int]$rngWaitMs)
if (-not $rngFinished) {
    try {
        if (-not $rngChild.HasExited) { $rngChild.Kill($true) }
    } catch [InvalidOperationException] {
        if (-not $rngChild.HasExited) { throw }
    }
    $rngSettleMs = Get-RngRemainingMs $rngHardBudgetMs
    if (-not $rngChild.HasExited -and
        ($rngSettleMs -le 0 -or -not $rngChild.WaitForExit($rngSettleMs))) {
        throw 'OWNED_CHILD_UNSETTLED: keep marker/lease and report actual handle'
    }
    throw 'FIRST_FAILURE_TIMEOUT: settled child, no next workload or retry'
}
if ($rngChild.ExitCode -ne 0) {
    throw ('FIRST_FAILURE_EXIT_' + $rngChild.ExitCode)
}
```

The remaining work budget is rechecked after launch/receipt IO so those costs
cannot extend the390swork phase. Finally cleanup verifies exact marker token and
uses only450s minus Elapsed. Return a target ONLY after actual child settlement
and owned marker release; retain uncertain ownership if cleanup cannot prove it.
OS-level uninterruptible IO/termination latency remains a platform risk, not a
claim that Kill or a request deadline proves termination. No foreign processes,
shared cache deletion or GPU reset may be used to force deadline compliance.

### Positive local-input inclusion, no absolute-ancestor target exclusion

Derive required manifests/Rust inputs from EACH selected resolved local package
and the13frozen source/license/fixture paths. Hash every required path even if an
ancestor contains a directory named target. Never use absolute-path substring
filters such as contains('/target/') to select source closure.

```powershell
$rngSourceRoot = [IO.Path]::GetFullPath($rngSource).TrimEnd([char[]]'\/')
$rngSourcePrefix = $rngSourceRoot + [IO.Path]::DirectorySeparatorChar
$rngRequired = [Collections.Generic.HashSet[string]]::new(
    [StringComparer]::OrdinalIgnoreCase)
foreach ($rngPackage in $rngSelectedPackages) {
    if ($rngPackage.source) { continue }
    $rngManifest = [IO.Path]::GetFullPath($rngPackage.manifest_path)
    if (-not $rngManifest.StartsWith($rngSourcePrefix,
        [StringComparison]::OrdinalIgnoreCase)) {
        throw 'LOCAL_INPUT_OUTSIDE_FROZEN_ARCHIVE'
    }
    [void]$rngRequired.Add($rngManifest)
    Get-ChildItem -LiteralPath (Split-Path $rngManifest) -Recurse -File `
        -Filter '*.rs' -ErrorAction Stop | ForEach-Object {
            [void]$rngRequired.Add([IO.Path]::GetFullPath($_.FullName))
        }
}
foreach ($rngOwnedFile in $rngFrozenSourceFiles) {
    [void]$rngRequired.Add([IO.Path]::GetFullPath(
        (Join-Path $rngSourceRoot $rngOwnedFile.path)))
}
foreach ($rngPath in $rngRequired) {
    if (-not $rngBeforeByPath.ContainsKey($rngPath) -or
        -not $rngAfterByPath.ContainsKey($rngPath)) {
        throw ('MISSING_SELECTED_INPUT: ' + $rngPath)
    }
    if ($rngBeforeByPath[$rngPath] -cne $rngAfterByPath[$rngPath]) {
        throw ('CHANGED_SELECTED_INPUT: ' + $rngPath)
    }
    if ([string]::IsNullOrWhiteSpace($rngBeforeByPath[$rngPath]) -or
        [string]::IsNullOrWhiteSpace($rngAfterByPath[$rngPath])) {
        throw ('EMPTY_SELECTED_INPUT_HASH: ' + $rngPath)
    }
}
```

Before/after maps use normalized full paths, ordinal-ignore-case keys and nonempty
SHA256 values. Record required manifest count, Rust file count, frozen13-file count
and total compared files separately from difference-record count. Registry/config/
tool/env closure remains finite and selected; generated target outputs are outside
the selected source roots, not removed through an absolute ancestor filter. All
above snippets are NOT_RUN; no renewed lease, Stopwatch or compiler has started.

## Accepted core-family consolidation handoff, root integration only

The human accepted the eight-package baseline; root sequences its first seven
engine-context consolidation into crates/sdk/humanymous-core. model-core-handoff.json
pins all24current model files, hashes/bytes, proposed per-file destination, dirty
state, active handles/leases and migration hazards. This is a reviewable mapping,
not permission for this owner to move files, rename packages or edit common Cargo.
Current model source is clean and last changed at72888805; no pending model source
diff needs a separate checkpoint. No compiler handle, marker or target is retained.

Proposed atomic source deltas ONLY after root group freeze:

1. Preserve src/model/{inference,training} contexts and model/mod.rs exports;
   merge the old model manifest into root-owned core manifest, no helper crate.
2. Map humanymous_behavior::features and humanymous_signals::report to same-core
   behavior/signals paths preserving type identity; rewrite four crate::inference
   references to the model context and scoring's model imports in the same slice.
3. Keep inference implementation modules private, Runtime State and GoRng state
   private, DrawSource private and all rng pub(super) helpers/tables restricted to
   rng. No unsafe/FFI exists here; preserve core forbid(unsafe_code). The public
   crate must NOT expose every training/diagnostic/activation method via a facade.
4. Root decides deliberate visibility: inference consumer traits/types need a
   defined core model contract; training helpers/snapshots can stay crate-restricted
   or test/explicit offline APIs pending public API review. Narrow pub(crate) only
   with consumer compilation; do not broaden pub(super) to resolve relocation.
5. Move test data only in root's atomic mapping. Three RNG include_str paths gain
   one model nesting level; use CARGO_MANIFEST_DIR-based tests/fixtures/model
   references rather than fragile parent counts. The inference integration test
   also needs new crate path and fixture subdirectory. Keep golden JSON, unchanged
   Go .txt oracle DATA and full BSD LICENSE bytes identical. Root owns aggregate
   attribution updates before packaging; old source-proof JSON remains immutable.
6. Dev support currently depends only on serde/serde_json, but accepted platform
   integration needs its explicit testing capability; prevent platform process/
   native/GPU code leaking into core normal/WASM graph. Update the Windows evidence
   runner's literal crates/model/src input path with its owner or source closure
   will silently miss the new context.

Authored model functions total33unit(5inference+8record+4standardize+4evaluate+
12RNG), plus1fixture harness with340vectors. This is NOT33executed passes:
the12RNG remain NOT_RUN and the first run failed in orchestration before Cargo.
After consolidation test names acquire model:: prefixes and aggregate filtered
counts change; never reuse the former21filtered expectation as a core denominator.
Fresh root-scoped fmt/check/strict Clippy, model/scoring parity, focused RNG under
a new monotonic grant and owner-qualified collector/WASM/workspace/product gates
remain required. Historical a0eb/e7d/aeefd receipts do not qualify moved source.
Root must freeze exact paths/source before edits; no moves, branches, synchronization,
shims, Cargo writes or new workload were performed by this handoff.

Each slice needs Rust 1.99.1/edition2024, scoped tests/fmt/Clippy, allocated actual
pilot and source-bound receipts. No GPU, model download, training, inference,
Cargo or Docker execution is authorized by this plan. No docs/** changes.

## First actual RNG validation on Root's moved core tuple

Root allocated and executed the consolidated humanymous-core validation. Independent
read-only inspection of its retained raw output confirms all12model::training::rng
unit tests passed, within145unit plus17integration passes (162total), zero failed,
ignored or filtered. All33model unit cases and the model inference integration
harness passed. Clippy stderr ends with a successful dev-profile completion;
Root reports locked/offline tests and strict all-target/all-feature Clippy exit0.
Exact argv and terminal process handles remain Root-owned evidence, not recreated
by this owner. core-first-rng-validation.json pins raw log hashes and the moved
model source records from Root's143record pre-test manifest; a fresh read-only hash
comparison against that manifest found0differences. Root also reports0before/after
differences. This is the FIRSTactual RNG12 proof on the moved tuple, not retroactive
qualification of72888805 or the failed pre-compiler attempt.

Go upstream output expectations, Rust-derived state/mixed-stream reference checks
and injected tapes retain their separate provenance. No Go runtime was executed;
Go internal states remain unobserved. No product e2e, training/CLI pilot, activation,
GPU or additional-platform parity is established. Prior NOT_RUN statements describe
the historical checkpoints; the moved tuple now has this separately bound receipt.
This owner started no formatter/compiler/test. Root's implementation freeze remains.

## Proposed next allocation: real offline online-SGD kernel (review only)

Status: DESIGN_READY_FOR_ROOT_REVIEW; no implementation, Cargo change, compiler or
pilot allocation. This accepted-RNG successor implements legacy Train and connects
existing Standardize, Mlp inference and Evaluate into a real offline experiment.
It does not close all mltrain rows, correction, temporal/cohort gates or CLI work.
The legacy train.go comment calls this mini-batch SGD; lines121-154 actually update
immediately after EACHsample. Preserve online SGD, not an optimizer reinterpretation.

### Exact implementation boundary and private API

Root allocation would loan only core src/model/training/sgd.rs, sibling
sgd/tests.rs and sgd/pilot.rs, plus training/mod.rs for a pub(crate) mod sgd.
No manifest, public facade, inference math, RNG tables, scoring or runtime edits.
All new types/functions are pub(crate) at most, subordinate to Core model::training;
no compatibility crate, GPU, tensor library, new dependency, download or executable
foreign-language template. Existing serde/serde_json suffice for the pilot only.

Proposed API: train(config, reviewed_rows, statistics, expected_schema, base,
limits) -> Result<TrainingResult, TrainError>. TrainConfig carries signed hidden/
epoch fields, f32 learning_rate, i64 seed and bounded version. TrainingRow borrows
FeatureVector plus binary label; construction from Record/Sample remains an explicit
caller review decision, NOT an automatic promotion of Admission::ReviewRequired.
Quarantined rows must never enter the record-to-training pilot adapter. Pure numeric
rows are mathematically usable but do not claim provenance authentication.
Standardization is the existing typed statistics; base is Option<&MlpBundle>.
TrainingResult owns candidate MlpBundle and bounded summary (effective config,
updates, epochs, RNG draws and final row-ID order). No per-update production trace,
partial candidate, callback, filesystem writes, activation or evaluation threshold
change. Test-only intermediate snapshots stay cfg(test), bounded tiny fixtures.
The kernel borrows rows/base/stats immutably; it permutes owned row IDs instead of
mutating caller data. This preserves Go's arithmetic/sample sequence while making
its caller-visible in-place train-slice shuffle an explicit nonmutation delta.

### Source contract and numerical order

Source anchors: internal/mltrain/train.go lines76-160 Train,162 Sigmoid,165-181
Forward and185-219 Evaluate; train_test.go lines37-65 existing learnability/warm
tests. Bind exact file hashes in sgd-next-slice-proposal.json before implementation.

Normalize hidden<=0 to16 and finite LR<=0 to0.05 as Go; signed epochs<=0 performs
zero updates as Go. Reject NaN/infinite LR before defaulting. Hidden>256 or epochs
above1000 is a typed limit failure; no truncation. Version<=128UTF-8bytes. Validate
schema, binary labels, every finite feature/stat/base weight, 27-wide shapes and
STRICTLY positive std before allocating the working candidate. Zero/negative std
is rejected: Go Train divides directly whereas Forward substitutes1for zero; this
is an explicit fail-closed admitted-domain delta. Do not silently reuse Forward's
zero-std fallback in training. Finite negative features remain signed.

Create ONEGoRng(seed, explicit draw budget); initialize on that same stream using
the qualified cold/warm API. Requested malformed warm base stays an error; never
silent cold fallback. Initialize biases0 cold, copy every base weight/bias warm;
use caller's new mean/std and requested version/schema, reset CalA=1,CalB=0 even
when base calibration differs (Go behavior). No RNG draw for valid warm copy.
Initialize IDs0..n once; for each epoch shuffle the CURRENTID order on the existing
stream, without reseeding or restoring identity. Empty input and zero epochs may
return valid initialized candidates with zero updates; these are NOT learnability
or admission success. Avoid repeated empty-epoch loops while preserving zero draws.

For each sample: z[i]=(x[i]-mean[i])/std[i], ordered f32 subtract then divide;
hidden h ascending, acc=b1[h], i ascending acc=acc+w1[h][i]*z[i]; save hpre and
hact=(acc>0 ? acc : +0); logit starts b2 and accumulates w2[h]*hact[h] in h order.
p=float32(1/(1+exp(float64(-logit)))) with negation in f32, exp/division f64 then
cast. No clamp, FMA/mul_add, vector reduction, batching, momentum, regularization,
loss clipping, learning-rate schedule or parallelism. BCE derivative d=p-label.
For each h: gW2=d*hact[h]; dHact=d*OLDw2[h]; dHpre=dHact iff hpre[h]>0 else+0;
then ascending i w1[h][i]=w1[h][i]-((lr*dHpre)*z[i]); b1[h]-=lr*dHpre;
w2[h]-=lr*gW2. Only after all h update b2-=lr*d. Computing dHact from updated W2
is wrong and must have a discriminating fixture. ReLU derivative is zero at±0.
Every semantic intermediate/product/update must be finite or reject the entire
result; f64 exp overflow for large negative finite logit is allowed only as the
legacy sigmoid limit p=0, not exposed nonfinite weights. Keep exact signed-zero
bits when the source operations produce them. Final bundle validates shapes,
schema and all finite fields; existing Mlp loads it, no serving installation.

### Deterministic capacity and error behavior

Limits tighten hard ceilings:100000rows,256hidden,1000epochs,1000000updates,
100000000feature-hidden-update units (updates*hidden*27),1000000primitive RNGdraws.
Use checked integer products BEFORE work; no saturating successful counters or
partial extra epoch. The 800row*40epoch*8hidden legacy scenario fits at6912000units.
One RNG budget spans initialization plus every shuffle, including rejection draws;
exhaustion discards the entire result, not a reseed/new stream. Resource summary
reports actual draws/updates, not predicted counts. No time-dependent arithmetic
or wall-clock branch in the pure kernel; an allocated supervisor owns real deadlines.

Working space: owned ID vector O(n), weights O(27h), reusable z[27], hpre/hact O(h)
and bounded version/stat copies. No normalized n*27matrix, per-sample heap churn,
epoch history or full gradient tape. Use fallible reservation for owned dynamic
buffers; inherited RNG initialize currently uses infallible bounded allocation,
so do NOT promise recoverable whole-process OOM. Typed categories: Config/Limit/
Schema/Label/Statistics/WarmBase/Nonfinite/RngBudget/Allocation; diagnostics omit
features, raw records, provenance strings and filesystem paths. Any failure leaves
all caller inputs/base/statistics unchanged and yields no candidate for publishing.

### Meaningful tests and parity classifications

1. Hand-derived exact f32 one-step warm fixture:27zeros except z[0]=1,
   h1,w1=0,b1=1,w2=0.5,b2=-0.5,lr=0.25,label1. Initial logit0,p0.5;
   expected w1[0]=0.0625,b1=1.0625,w2=0.625,b2=-0.375; other w1bits unchanged.
   This catches updated-W2 backprop, sign, feature and bias ordering. Separate
   inactive fixture: z[0]=1, all other z=0, w1=0, b1=-1, w2=0.5, b2=0,
   lr=0.25, label1; hpre=-1 and p=0.5. Only b2 becomes0.125; all hidden
   weights/bias and W2 remain unchanged. It does NOT inherit b2=-0.5 above.
   hpre=+0/-0 cases exercise strict derivative; multi-hidden cancellation fixture
   detects reassociation/FMA. These are analytic/Rust-derived, NOT Go-run goldens.
2. Independent small scalar reference (test-only, separate update formulation)
   compares every weight/intermediate after2samples*3epochs and ID order/draw count;
   include cold/warm, negative/modulus/extreme seeds, changing statistics and
   nonidentity second-epoch order. Shared math/RNG dependencies are disclosed.
3. Zero/negative epochs/default config, all-weight warm-copy/Calreset and no alias;
   empty input; hidden/sample/update/work/draw limits exactedge+1, overflowed
   products, late invalid label, malformed base, std0/negative, nonfinite features,
   finite subtraction/product/update overflow, exhausted shuffle transaction.
   Every failure asserts bit-identical caller/base/stats and no returned candidate.
4. Reuse immutable existing inference fixtures and existing RNG tests; trained
   bundle Mlp predictions feed existing evaluate::evaluate, inclusive p>=0.5.
   Missing human/bot support remains None; abstention rejects evaluation. Do not
   duplicate serving Forward. Repeat identical numeric inputs for bitwise candidate,
   final IDs and counters equality; version-only change must not alter weights.
5. Port legacy separable800train/200holdout,H8,40epochs,LR0.1,seed2 using Rust
   deterministic synthetic dataset seed1 with EXACTGo test data draw order; report
   acc>=0.95,humanFPR<=0.05,botTPR>=0.95 as offline learnability regression only.
   This port has Rust-derived inputs until an actual Go transcript is allocated.
   No inference-only fixture or learnability score proves SGD cross-language parity.
6. Later SEPARATEGo1.26.3 oracle allocation: unchanged source plus reviewed bounded
   Rust harness invokes Go tooling, no generated foreign implementation. Capture
   exact config/input bits, initial weights, ID orders, selected intermediates and
   final weights/calibration/prediction bits on the accepted host; abs0/rel0. If
   unchanged Go exposes only finalbundle, label intermediates Rust-derived rather
   than imply visibility. No Go execution is authorized by this proposal.

### Actual offline pilot and allocation request

The first coding allocation prepares real kernel plus actual-file unit pilot,
all NOT_RUN until Root gives a source-bound execution slot. Pilot uses create_new
owned temp train/holdout JSONL files with disjoint explicit IDs/times/cohorts and
reviewed synthetic provenance. First bounded-parse and prevalidate ALL train and
holdout records, admission decisions, disjoint IDs, both holdout class supports,
schema and byte/row/work limits. Then compute statistics on TRAINONLY, train an
in-memory candidate, load it through existing Mlp and evaluate held-out counts.
All validation, numeric and evaluation failures happen BEFORE any candidate file
creation. Only a complete valid offline result may serialize via create_new;
close it, read EXACTcandidate bytes, reload existing Mlp and verify identical
predictions/counts, bundle identity/finite shape and reproducibility. Successful
offline evaluation and learnability thresholds confer NO promotion or activation.
Negative actual-file pilots: late invalid record, quarantine, byte/row cap, absent
holdout class and training overflow leave no published candidate. An I/O failure
after create_new may leave an owned partial file: classify it separately as failed
unpublished output, retain/report its owned handle/path for safe settlement, and
never treat it as a complete candidate or claim no bytes were created. A readback
or roundtrip failure likewise leaves only a failed unpublished owned artifact.
Never overwrite an existing sentinel artifact. Cleanup only verified owned files
after handles settle. This is an actual native filesystem+offline kernel pilot INSIDEtests,
not a bootstrap CLI/process, temporal/cohort promotion gate or full product e2e.
No automatic quality-based promotion, and no claiming authenticated sample source.

Proposed later slot: exclusive fresh source-keyed target, jobs1, test-threads1;
fmt check first, focused SGD tests+file pilot, all Core regression, locked/offline
all-target/all-feature check and strict Clippy. Root assigns exact durations,
source/manifest/registry hashes and process budget before any command starts.
Use one monotonic supervisor and settle only owned child handles/target lease;
retain first failure, changed-source invalidation and excluded tests. No new
compiler invocation, GPU or dependency resolution was performed for this proposal.

Acceptance for implementation: Root approves these exact paths/private API and
explicit deltas (nonmutation, finite/positive-std validation, bounded capacities,
malformed-warm refusal) before code. Time split/tie policy, per-cohort evaluation,
AUT, calibration, drift, sinks, correction lifecycle and command/process surfaces
remain individually tracked future slices; candidate must stay offline until those
separate admission gates exist. This is concrete Train progress, not whole ML closure.

### Accepted source grant and final numerical/allocation caveats

Root accepted the bounded offline SGD design including both X-review corrections
and allocated ONLY training/mod.rs, training/sgd.rs and training/sgd/** to this
owner. Actual detection-core and sot-plan claims were read back before source work.
No formatter/compiler/test execution is allocated. Root retains all other Core,
Cargo/dependency, inference/scoring and activation ownership.

Warm copying with NEWmean/std and CalA=1,CalB=0 may change predictions even with
zero epochs: weight continuity is not function/prediction continuity. Existing
RNG initialize AND shuffle's order.to_vec perform bounded infallible allocations;
fallible SGD-owned reservations do not promise whole-process OOM recovery.
Go1.26.3 permits FMA: Rust explicit non-FMA ordered arithmetic is this deterministic
implementation target, while abs0/rel0 cross-language parity remains UNQUALIFIED
until an exact toolchain/target/CPU/build Go oracle allocation. Do not introduce a
nonzero tolerance or patch upstream source to force a compatibility claim.

### Real SGD source checkpoint under accepted grant

The four allocated source paths now implement the real ordered offline online-SGD
kernel,10numeric regression tests and2actual-file pilot tests. All12are NOT_RUN.
sgd-source-checkpoint.json binds exact file hashes, test names and next command
matrix. No formatter, compiler, target allocation, pilot or Go execution occurred.
No manifest/dependency, inference, scoring, activation or GPU code changed.

The borrowed-row API validates full input and capacities before creating the RNG;
one stream initializes and shuffles successive existing ID orders. A private
candidate is discarded on arithmetic/limit/draw failure. Tests include analytic
old-W2 and inactive/signed-zero cases, an explicit non-FMA cancellation boundary,
per-sample all-weight bit comparisons against a separate Rust gradient formulation,
cold/warm seed/epoch stream comparison, zero-epoch function discontinuity, invalid
and overflowing input/configuration, and deterministic legacy separable learnability.
References share exp/RNG and remain Rust-derived, not a Go execution transcript.
The native-file test source fully prevalidates both datasets/support/admission,
trains/evaluates in memory, then create_new writes and reads back exact artifact
bytes. It checks reproducibility, sentinel preservation and no output for late
invalid/quarantined/absent-class/capacity/numerical failures. Post-create I/O failures
are explicitly owned unpublished outputs; no success is emitted from partial I/O.

Public API is unchanged: sgd and every callable/type are pub(crate) or narrower.
Until a separately allocated bootstrap caller exists, the registration carries a
non-test-only scoped expect(dead_code) with its exact reason; it does not suppress
Clippy or test warnings and must be reviewed/removed at caller integration. RNG's
bounded infallible allocation caveat remains. No prior162pass receipt qualifies
these new sources. Formatting/compilation have not been observed and may require
source fixes in the separately allocated slot. Root must bind package/target/
features/profile/toolchain/CPU and source hashes before the command matrix runs.
Next gate: fmt check FIRST, focused12SGD tests including actual-file pilot, full
Core regression, locked/offline all-target/all-feature check and strict Clippy,
jobs1/test-threads1 under a fresh exclusive target and monotonic budget. All
remain pending Performance lease return and Root's explicit execution allocation.

### First formatter-only admission: failure retained, no source mutation

Root allocated one60second formatter-only window for c9ba2d24. The actual cargo
fmt --package humanymous-core -- --check terminated nonzero and emitted retained
raw diffs. Before apply, the path guard failed to strip Windows extended-length
prefixes and falsely classified the owned paths as foreign; orchestration stopped
without mutation. Read-only normalized inspection identifies exactly the4allocated
SGD files. A fresh hash comparison against the full Core before manifest finds
zero differences. No apply/final check, compiler/test/Clippy or target lease exists.
The child's exact numeric exit was held only in memory and not persisted before
the guard exception; record nonzero, not an invented exact exit. No first-tuple
formatting pass is claimed. Raw stdout/stderr and before hashes are pinned by
sgd-first-format-admission.json. The formatter process was synchronously reaped
before the guard failed; no retained owned handle. Further formatting needs a
renewed Root window with prefix normalization and result persistence before guards.

### Renewed formatter-only admission: new source formatting pass

Root renewed a separate60second window after the retained first failure. Fresh
Core before/after snapshots each contain151files; only the4delegated SGD paths
changed, zero foreign differences. Only cargo fmt --package humanymous-core
modified Rust sources; apply exit0 and subsequent cargo fmt --package
humanymous-core -- --check exit0. Both handles are terminal/reaped; total monotonic
window738.4491ms. Raw step exits were persisted before subsequent guards; normalized
Windows extended-length prefixes correctly. sgd-format-second-admission.json
pins the new4source hashes, full raw snapshots/logs and exact format diff bytes.
This is a formatting pass for the NEWsource tuple only, never c9ba2d24. Earlier
failure/guard evidence stays immutable. No semantic/API/lint-attribute edits were
made; module-wide expect(dead_code) remains under X review. All12SGD tests, actual
file pilot, Core regression, compiler check and strict Clippy remain NOT_RUN.
No retained formatter/compiler handle or target lease. Next command matrix remains
focused12SGD -> fullCore -> all-target/all-feature check -> strictClippy, jobs1,
under a NEWsource-bound execution allocation; no such commands ran in this window.

### Accepted X P2 source corrections after formatter checkpoint

Root accepted the exact temporary non-test private-module expect(dead_code), with
removal when the actual offline composition caller exists; no fake caller/public
export or general lint policy is introduced. Test linting remains active. Root
then allocated concrete P2 fixes AFTER7406195f's separate formatter receipt.

OwnedDirectory now records only successful create_new input/output creations,
including outputs before fallible writes so partial-output cleanup obligations
are retained. Explicit finish returns Result and is asserted by each actual-file
pilot. Drop only attempts fallback and cannot certify cleanup. A collision fixture
creates an entry outside this ledger, checks AlreadyExists and empty ownership,
requires cleanup to report failure, verifies unchanged sentinel bytes, then has
that fixture creator remove its own entry before successful finish. No blind
four-name deletion or recursive cleanup remains. This simple pilot ownership
ledger makes no hostile-race/native file identity/custody claim.

All7negative actual-file cases now compare exact PilotError: Input for malformed/
record/byte caps, Admission for quarantine, Support for absent class, Training(Limit)
for work cap and Training(Nonfinite) for numeric overflow. Added finite-negative
logit exp-overflow/sigmoid0 regression and version128/129byte acceptance/refusal.
Authored denominator is now15tests (12numeric+3actual-file), ALL NOT_RUN on this
new tuple. Root's immutable fc9 Collector strictClippy failed101 at the SGD nested
warm-base condition; its raw failure hash is pinned by sgd-review-fixes-source.json.
Only the specifically authorized collapsible_if let-chain correction was applied,
with the same predicates, left-to-right order and WarmBase error. No lint allow,
cap-lints or unrelated fixes. Root reports the corresponding WASMcheck passed;
that does not qualify these new changes or establish remaining Clippy cleanliness.

No formatter/compiler/tests were started for these amendments. Previous formatter
pass applies ONLY7406195f; next needs fresh format/check plus15focused tests, full
Core and all-target/all-feature check/strictClippy under separate source-bound
Root allocation. No owned compiler handle or target lease is retained.

### First actual SGD qualification: finite Windows epoch, explicit audit limits

Root allocated390seconds work/450total including prep, max3fresh source attempts,
exclusive target/core-consolidation-v0, jobs1/incremental0/offline locked and serial
tests. Admission free37727059968bytes and commit61.37020777207274percent passed.
A single persistent Stopwatch included prep, model/tool gaps and all3attempts.
Actual terminal/lease return at231696.7279ms; all children reaped, marker absent.
Tool output identifies cargo/rustc1.99.1, x86_64-pc-windows-msvc, LLVM23.1.1.

Attempt1 actual fmt exit1 remains retained. Only delegated pilot/tests whitespace
changed, committed81f98d7d; attempt2 fmt/SGD15/fullCore177/check passed, strict
Clippy101 found2needless_range_loop warnings in test reference. Preserve that first
failure. Minimal iterator-only reference changes committed42ecee7d, no expected
values or numeric/admission predicates changed. Attempt3 immutable Git archive at
42ecee7d passed fmt, ALL15SGD tests including3actual-file pilots, fullCore160unit+
17integration=177unique tests, all-target/all-feature check and strictClippy.
Focused15had145filtered and are a SUBSETof177, not192unique passes. Zero failed,
ignored or fullCore filtered tests. Prior NOT_RUN statements remain historical;
this receipt qualifies only its immutable source tuple and actual Windows venue.

sgd-first-actual-validation.json pins all3archive joins/fullsource before/after
counts and hashes, exact argv/PIDs/terminal exits, monotonic dispatch/end anchors,
actual15case inventory, tool binary hashes, scoped config/env, raw logs and failures.
Full archived-source counts3149/3153/3153 remain0differences in attempts1/2/3.
Resolved registry
selection includes36packages/1445manifest/checksum/Rust records, post-execution
snapshot and immediate reread0differences. IMPORTANTaudit limitations: there was
NOpre-execution registry snapshot, so pre/post registry equality is NOTestablished.
Exact OS child StartTime was not persisted; observed monotonic dispatch remaining
values and terminal elapsed/PID/exit are retained, while redirected-file creation
UTC is an audit anchor only. No whole OS/linker/environment closure is claimed.
Do not silently upgrade these limits into a stronger reproducibility claim.

Actual file pilots passed complete read/admission/support/training/evaluation/
serialize/readback, deterministic artifact equality, sentinel preservation,
7exact negative categories, unowned collision preservation and explicit cleanup
failure/success. Rust-derived reference/analytic evidence remains separate from
unexecuted Go training transcripts; FMA/abs0rel0 cross-language parity remains
unqualified. No bootstrap CLI/process, GPU, activation, correction/cohort/time
admission or Docker product e2e success follows from this real kernel qualification.
Temporary private non-test dead_code expectation still requires removal when the
real offline composition caller is wired. Whole ML migration remains incomplete.

## Next verification design: unchanged Go1.26.3 SGD endpoint oracle

Status: REVIEW_ONLY_NO_EXECUTION. Core177/SGD15 is real Rust component proof,
not Go SGD bit parity. No compiler, Go command, data generator or source change
is authorized by this design. Root retains the current sole Lab execution lease.

### Unchanged surfaces and their observability ceiling

Use ORIGINAL cmd/ml-train and cmd/mlcorrect-train/main.go+gate.go, internal/mltrain,
behavior, mlserve and signals, with byte hashes and Git archive closure. No authored
Go, wrapper Go main, modified tests, source instrumentation, foreign template,
-gcflags arithmetic override or patched expectations. Existing Go tests may run
unchanged but their pass footer exposes only their assertions, not internal arrays.

- ml-train -data requires>=100rows; its seed creates a PRE-SPLIT GoRng, shuffles
  all rows once, val=firstfloor(n/5), train=remaining. Train then constructs a NEW
  GoRng with the SAMEseed; do NOTcontinue the split RNG into initialization. This
  separation is outside the kernel's single-stream init+epoch contract. The -gen
  path consumes different data-generation draws, so this oracle uses -data ONLY.
- mlcorrect-train reads anchor then optional oracle, stable TS-sort and cut
  n-floor(n*testFrac); Train uses a fresh seed stream and valid base. It writes a
  candidate only if ORIGINALcohort/TPR gates pass. Keep default gates unchanged.
- Both output final mean/std, all weights/biases, calibration, dimensions/schema
  and clock-generated version as JSON. These floats can be decoded to f32 bits.
  Cold epochs0 exposes initialization weights; warm epochs0 exposes copied weights,
  new train statistics and reset calibration. Those are actual final output
  observations, not snapshots of an in-progress training state.
- Neither CLI emits per-row extracted features, shuffled IDs, RNG draws/state,
  hidden pre/activation, logit, p, derivatives or per-update weights. All such
  proposed Rust trace fields MUSTstay Rust-derived/unobserved-in-Go. Epoch1/3 runs
  yield independent endpoint observations, not a replay transcript within one run.
- stderr metrics are rounded3decimal places (cohort FPR4); exact Go evaluation
  probability/rate bits remain unobserved. Warm stderr cohort human/FP integers,
  train/test counts and gate outcome AREobserved. Rust re-evaluating the emitted
  Go bundle does not magically observe Go Predict bits. No new telemetry implied.

### Exact synthetic numeric data and warm base (Rust generation only)

Propose320JSONL rows in original order i=0..319, no blanks, LF, explicit required
label/behavior and metadata. All behavior fields zero except mouse.meanCurvature:
label=1 iff i%2==0; j=(i/4)%4; curvature=(label==1 ? 0.75 : 0.25)+j/64.
All values are exact binary rationals. ts=10000+i; source=catalog for1, pass for0;
cohort=synthetic-a iff i%4==1, synthetic-b iff i%4==3, synthetic-bot otherwise.
Example row0 is {"label":1,"behavior":{"mouse":{"meanCurvature":0.75}},
"ts":10000,"cohort":"synthetic-bot","source":"catalog"}. Rust's actual generator
will serialize complete/default behavior consistently and pin the resulting bytes,
not manually concatenate floating-point executable foreign code. No real human/
accessibility evidence is implied by these intentionally synthetic cohort names.

Restricted fixture feature vector is analytically27zeros except index1=curvature:
all log1p inputs0 and presence flags false. Record exact f64 JSON values and
expected f32 bits plus shared feature order/schema in a separate Rust manifest.
Parse the SAMEdata bytes through each program. Rust must assert its extracted
vectors match that analytic manifest. Go per-row vectors remain source-inferred,
not directly observed; emitted27mean/std values are actual aggregate observations.
This fixture isolates SGD from nonzero log1p differences. Existing unchanged Go
behavior tests and retained Rust feature fixtures are supplementary, not proof of
new arbitrary Go feature inputs. A later nonzero-log fixture needs its own review
and cannot inherit this restricted feature-domain claim.

Warm base: hidden1, featureDim27, correct shared schema; w1[0][1]=8 and other w1=0,
b1=[8],w2=[1],b2=-8; mean27zeros,std27ones; calA=2,calB=-0.5;
version=synthetic-oracle-base. Root-reviewed Rust generator writes/pins complete
JSON and exact float bits. No Go tool needed to construct it. Correction split
at0.25 yields240train/80test,40test humans,20per synthetic human cohort. Increasing
TS avoids the legacy tie ambiguity. New train statistics and calA=1/calB=0 are
expected evenepochs0. This separable base is intended to satisfy DEFAULT0.005FPR,
0.80TPR,min20support gates without relaxation; if any gate fails, retain exit3/no
candidate and stop, do not change data, thresholds or assertions during the epoch.
Do not describe this as functional continuity of the original calibrated base.

### Bounded proposed run matrix and exact commands (NOT_RUN)

Root would first allocate a Rust-authored qualifier under existing private
training/sgd/oracle.rs, test-only and existing qualification feature, not a public
API forwarding facade. It genuinely generates numeric data and orchestrates the
unchanged binaries, with explicit allocated execution opt-in; no hidden downloads
or default Cargo test subprocess launch. A separately reviewed ignored qualification
test selector is acceptable only when run explicitly with --ignored; ignored is
NEVERcounted as a pass. No source for this qualifier exists yet.

The Rust controller supervises std::process::Command children, creates ONLYowned
fresh directories/files, caps JSONL<=1MiB, each artifact<=1MiB, each log<=4MiB,
<=7CLI invocations, one child at a time, jobs1, no network, one epoch Stopwatch.
It records actual StartTime/dispatch and terminal timestamps/elapsed/PID/exits and
settles children/drains before cleanup/lease return. Per-stage times and overall
390/450style limits are Root allocations, not automatically granted here.
Pre-snapshot source ANDselected resolved module/tool/config data BEFOREexecution,
then post-snapshot; do not repeat the prior post-only registry audit limitation.

Direct original commands below use a fresh immutable archive as cwd, an allocated
absolute Go1.26.3 executable and an OWNEDoutput directory (paths are review tokens):

    <go1263> version
    <go1263> env -json GOVERSION GOOS GOARCH GOAMD64 GOEXPERIMENT GOFLAGS GOTOOLCHAIN CGO_ENABLED GOROOT GOMOD GOMODCACHE GOCACHE
    <go1263> build -mod=readonly -trimpath -buildvcs=false -p=1 -o <owned>/ml-train.exe ./cmd/ml-train
    <go1263> build -mod=readonly -trimpath -buildvcs=false -p=1 -o <owned>/mlcorrect-train.exe ./cmd/mlcorrect-train
    <go1263> version -m <owned>/ml-train.exe
    <go1263> version -m <owned>/mlcorrect-train.exe
    <owned>/ml-train.exe -data <owned>/samples.jsonl -out <owned>/cold-h1-e0.json -hidden 1 -epochs 0 -lr 0.03125 -seed 2
    <owned>/ml-train.exe -data <owned>/samples.jsonl -out <owned>/cold-h1-e1.json -hidden 1 -epochs 1 -lr 0.03125 -seed 2
    <owned>/ml-train.exe -data <owned>/samples.jsonl -out <owned>/cold-h1-e3.json -hidden 1 -epochs 3 -lr 0.03125 -seed 2
    <owned>/ml-train.exe -data <owned>/samples.jsonl -out <owned>/cold-h3-e3.json -hidden 3 -epochs 3 -lr 0.03125 -seed -1
    <owned>/mlcorrect-train.exe -anchor <owned>/samples.jsonl -base <owned>/base.json -out <owned>/warm-e0.json -hidden 1 -epochs 0 -lr 0.03125 -seed 2
    <owned>/mlcorrect-train.exe -anchor <owned>/samples.jsonl -base <owned>/base.json -out <owned>/warm-e1.json -hidden 1 -epochs 1 -lr 0.03125 -seed 2
    <owned>/mlcorrect-train.exe -anchor <owned>/samples.jsonl -base <owned>/base.json -out <owned>/warm-e3.json -hidden 1 -epochs 3 -lr 0.03125 -seed 2
    <go1263> test -mod=readonly -p=1 -count=1 ./internal/mltrain -run '^(TestTrainAndEvaluate_Separable|TestWarmStart_UsesBaseAndKeepsShape)$'
    <go1263> test -mod=readonly -p=1 -count=1 ./internal/behavior -run '^Test(Extract_|SchemaHash_)'

Use child env GOTOOLCHAIN=local,GOPROXY=off,GOSUMDB=off,GOWORK=off,GOENV=off,
GOMAXPROCS=1,CGO_ENABLED=0,GOOS=windows,GOARCH=amd64,GOAMD64=v1;
GOFLAGS/GOEXPERIMENT explicitly empty for first tuple. Set exclusive GOCACHE and
preexisting pinned offline GOMODCACHE; cache absence or wrong Go version refuses
before tests. No go mod tidy/get, toolchain upgrade or dependency/source edits.
Module graph/checksums/source files actually resolved by these unchanged packages
must be captured with their commands when allocated; include Go stdlib+tool/compiler
binaries and go env, go.mod/go.sum, base/input/output bytes and binary build info.
No production path, existing output overwrite, real gate signing/staging or cleanup
of shared caches. Original Go uses os.WriteFile, so supervisor precreates OWNEDfresh
output DIRECTORY, checks each output absent and never points Go at a sentinel.

### Comparison, FMA and identity decisions

Cold comparator reproduces pre-split Shuffle(seed), val/train selection/order,
train-only standardize, then fresh Train(seed). Warm comparator reproduces source
stable-sort/cut on unique TS with train-only standardize and supplied base. Record
all derived ID orders/draw counters as RUST_DERIVED; Go prints only coarse counts.
No standardization leakage from holdout. Input float64LR0.03125 casts exactlyf32.

For each successful final Go JSON: preserve raw bytes/SHA, reject malformed or
nonfinite/shape/schema mismatches, decode every mean/std/W1/B1/W2/B2/Cal field to
f32.to_bits; compare every bit to Rust, abs0/rel0, no negative-zero canonicalization.
Compare dimension/schema exactly. CLI version is clock-generated, not flag-settable:
validate its original prefix/shape against captured run time and pass the OBSERVED
version into the Rust config for fieldwise artifact comparison. Record that version
was observed input to the comparison, not a predicted deterministic value. Do not
mask numerical fields, compare pretty JSON bytes across different serializers, or
claim deterministic raw Go bytes across changing version timestamps. Warm stdout
SHA digest must independently match the exact emitted candidate bytes.

Stop on the FIRSTnumeric difference and report case, field/index, Go/Rust bits,
source/tool/binary/input/base hashes, exact build flags, target/CPU capabilities
and decoded values. No tolerance waiver, golden rewrite, source patch, -N/-l or
FMA-disabling flags to manufacture equality. Go1.26.3 may fuse operations; Rust
explicit non-FMA source order is not proof the Go target chooses it. Record actual
CPU model/ISA/FMA capability, OS/arch, GOAMD64, compiler and Rust target-cpu/features/
profile/RUSTFLAGS/config; a matching tuple qualifies only those7endpoint cases.
Other CPUs/targets/optimizations remain unqualified. If desired, preserve Go
objdump/compiler assembly as diagnostics without claiming it observes runtime data.
No new Go intermediate observation is inferred from matching final bits.

## Proposed deliberate offline Core boundary for a real Rust CLI caller

This is an API REVIEWproposal ONLY; no exports, source/Cargo changes or fake
calls. The Go oracle can use the genuinely exercised private kernel in a Core
qualification test; it does not justify exposing SGD internals to consumers.

The future real CLI should call ONEreviewed composition operation under Core
model::training::offline, tentatively fit_offline(train_rows, holdout_rows,
config,limits,base)->Result<OfflineTrainingReport,OfflineTrainingError>. Its public
numeric row DTO borrows FeatureVector+binary label and is explicitly NOTproof of
source authentication; caller owns reading/admission/cohort/time/provenance review.
The function validates both datasets/support/limits, computes TRAINONLYstatistics,
uses the private SGD kernel, loads existing Mlp and reduces existing evaluation,
then returns an owned OFFLINEcandidate MlpBundle, exact Counts and bounded training
summary (effective shape/epochs/LR/update/draw counts). It does REALcomposition,
not a forwarding-only re-export. Keep raw GoRng/Snapshot/InitialWeights/update and
final permutation diagnostics private/test-only. Expose only deliberate DTOs and
stable error categories; no broad pub use sgd::* or facade-wide training export.
Candidate return is never activation authority or a required-cohort promotion pass.

Core remains pure: no filesystem/process/Go/time/network/backend/GPU code in that
public operation. CLI owns bounded real files, exact-byte output/create_new/nooverwrite,
cleanup reporting, user-facing config and exit codes. Immutable borrowed base and
new statistics/calibration caveat remain explicit. Require actual CLI command wiring
and its real accepted/rejected process pilots IN THE SAMEintegration slice before
removing temporary non-test expect(dead_code); never remove via a fake reachability
call or by widening all private types. Root approves public naming/error/versioning
and source ownership before any implementation. Existing public Record/statistics/
evaluation types may be reused deliberately, but are not an excuse to freeze an
accidental full training API. A minimal accepted numeric CLI bridge does not yet
supply temporal split/tie semantics, per-cohort support/FPR/AUT, correction/drift/
calibration, authenticated labels, gold-anchor lifecycle, dual-control, production
activation or Docker product validation. Track each separately; whole ML remains
incomplete even if every oracle endpoint and bootstrap caller later passes.

## Concrete offline Core/CLI candidate source (evidence only, not integrated)

Status: PREPARED_RUST_CANDIDATE_NOT_COMPILED_OR_REVIEW_ACCEPTED. Per Root grant,
ALLnew Rust source is stored ONLYunder this owner's evidence directory. No live
Core/CLI source, public visibility, Cargo, workspace/Root snapshot, activation,
NativeReady, GPU/network or scoring paths changed. Core177/SGD15 remain historical
42ecee7d results and do not qualify any of these candidate files.

### Exact source/destination/API map for Root review

- offline-core.candidate.rs -> crates/sdk/humanymous-core/src/model/training/offline.rs;
  real public domain operation model::training::offline::fit_offline, with explicit
  OfflineInput/Config/Limits/Report/Metrics/CohortMetrics/Summary/Error. It composes
  revalidation/admission/support/time, train-only statistics, existing privateSGD,
  existing Mlp and existing evaluator. It is not an SDK forwarding/helper crate.
- offline-core-tests.candidate.rs -> same Core training/offline/tests.rs.
- offline-cli.candidate.rs -> proposed final CLI src/offline_train.rs;
  offline-cli-tests.candidate.rs -> final CLI src/offline_train/tests.rs.
- offline-cli-pilot.candidate.rs -> final CLI tests/offline_training.rs; real
  CARGO_BIN_EXE_humanymous child, finite30seconds/log<=1MiB, explicit cleanup.
  Existing live composition root is crates/cli/src/main.rs; final intended family
  root is crates/apps/humanymous-cli. That directory currently has qualification
  data but no live Cargo/src. Root's accepted21member candidate/final8 movement
  owns the exact destination/binary-root reconciliation, not this evidence slice.

The exact proposed router delta is a PRIVATE mod offline_train in the genuine
humanymous main composition root and, BEFORENativeReady/runtime-service/provisioning
branches: if command=="model-train-offline", call offline_train::run(args.collect())
and propagate its real result through the existing main error/exit boundary. No
fake reachability call. No NativeReady flag, runtime host, signing/staging actor
or device bootstrap dependency is needed for this pure offline operation.
Root must deliberately declare CLI->Core normal dependency using an existing
workspace package, not a new helper/re-export package; live old CLI has no such
dependency yet. serde/serde_json/sha2/hex are already in CLI/Core dependency tuples;
no new third-party dependency is requested. All Cargo/router changes remain review
proposals, not performed changes. For test-module path attributes, preserve the
specified final sibling directory layout rather than compiling evidence filenames.

Only at accepted REALcaller integration, add pub mod offline to training and REMOVE
its temporary non-test expect(dead_code) on sgd; keep sgd pub(crate), its helpers
private and no broad public export. Neither registration nor exception is edited
here. Root reviews final API/error/versioning/source ownership before integration.
Candidate source contains5Core unit tests,2private CLI unit tests and2real process
pilot tests (the negative pilot has6cases); ALL9tests/actual pilots NOT_RUN.
No formatter invocation is authorized for the candidate by its preparation alone.

### Deliberate contract and unresolved source decisions

Public input uses existing Sample to reuse canonical aggregate extraction. Because
Sample has public fields, candidate REVALIDATESraw Record and recomputes admission
and feature bits; forged cached features cannot bypass shared extraction. Consistent
catalog/pass strings and --reviewed-input are caller review acknowledgements, NOT
proof of oracle/human authentication. Quarantine refuses. Required labels are0/1;
raw/derived nonfinite values, malformed data, mismatched schema and bounded metadata
refuse before result. Fixed FeatureVector encodes27dimensions; existing privateSGD
validates every base tensor/schema/finite field and requested hidden shape.

Combined train+holdout<=100000, cohorts<=64, metadata<=128UTF-8bytes, mandatory
explicit TS/cohort, strict max(train.TS)<min(holdout.TS), no tied cross-boundary
window. Empty cohort normalizes default; missing cohort refuses. Require both
labels in EACHset and min holdout-human support for EVERYhuman cohort present in
train ORholdout. Default20, explicit bounded CLI --min-cohort-humans selects offline
support. A cohort cannot disappear into an aggregate success. These are DELIBERATE
stronger operation preconditions, not a claim the complete temporal/cohort/AUT/
promotion policy is implemented. No public human-FPR/TPR promotion threshold is
added or changed: all counts/rates are DATA. Remaining sample authenticity, class/
cohort policy acceptance and default20require Root source-contract review before
shipping. Current strict-time operation does not replicate bootstrap Go's random
split; do not present them as the same split policy.

Training preserves privateSGD default hidden/LR, zero/negative epoch semantics,
finite capacity/draw budgets and rejected malformed warm request, ordered f32
updates and one init+epoch stream. Warm weights/biases are copied, but newly
computed train statistics and CalA=1/CalB=0 replace base statistics/calibration
evenepoch0. Report/model mean/std expose these TRAINONLYstatistics, never holdout.
No preserved function/accuracy claim. Existing bounded infallible RNG allocation
and new bounded collections do NOTguarantee whole-process OOM recovery.

Core returns model+counts/per-human-cohort rates+effective bounded summary with
activation=false,signed=false. The flags express DATAstatus and are not security
tokens or a new way to activate. Raw RNG snapshots/ID order/optimizer internals
stay private. Core has no file/process/time/network/NativeReady side effects.

CLI requires explicit train/holdout/output/version/hidden/epochs/LR/seed/support
and --reviewed-input, rejects duplicate/unknown flags, caps argument lengths,
uses existing bounded JSONL reader and <=1MiB base/model/receipt. It validates,
trains and evaluates fully BEFOREcreating output. Artifact is PLAINoriginal
MlpBundle JSON (Go-interoperable unsigned model); separate JSONstdout receipt has
model digest, statistics-through-model identity, metrics/cohorts/summary and
activation=false/signed=false/go_bit_parity=unqualified. No new serialization
shape is forced onto the Go bundle loader. Read/write create_new handle, sync,
seek/readback same handle, exact bytes; existing files refuse unchanged. Post-create
I/O or stdout failure retains explicitly named unsigned/unpublished output as a
cleanup obligation. No blind rollback/delete or signing/promotion/network action.
Same-handle readback is a simple I/O check, NOT hostile-race/native path custody.

### Meaningful candidate unit/process verification matrix

Core tests cover train-only mean despite changed holdout distribution, unsigned/
noactivation data, strict TS tie/missing TS, missing cohort, quarantine/source
mismatch, forged/NaN cached features, per-cohort missing support, combined cap,
hidden257, NaNLR, version129, wrong schema, malformed warm shape, raw nonfinite,
and full warm-weight preservation/new-stats/Calreset with zero updates/draws.
Existing privateSGD tests remain reused, not duplicated into another trainer.

CLI units cover exact numeric flag parsing, absent review, duplicate/incomplete/
unknown flags and activation option refusal. Actual-process source uses OWNcreated
input/log files and controlled child's fresh output obligations, finite timeout,
kill+wait only owned child and explicit finishResult (Drop only unwind fallback).
Accepted process checks Go MlpBundle decode, train-only mean, Calreset, unsigned/
activationfalse receipt/digest, deterministic second output and unchanged existing
sentinel. Six negative processes: cross-boundary time tie, quarantine, disappearing
human cohort, hidden257, NaNLR and malformed warm base; each must exitnonzero and
leave no candidate file. The ownership helper never claims native hostile-race
custody. A future allocated native I/O fault pilot must separately qualify partial
write/stdout-failure reporting; that is not implied by source review or mock tests.

Next gates require Root API/contract review, actual accepted8family snapshot/router/
manifest integration and a NEWsource-bound execution allocation: fmt first, Core
focused offline tests, existingSGD/Core regression, private CLI units, actual CLI
accepted/refused process pilots, all-target/all-feature check/strictClippy. Target/
source/registry BEFORE+AFTER/tool/config/environment and true child start/exit
records must be captured; do not inherit prior epoch audit gaps as full closure.
No tests were run here and no compiler/target lease is held.

### Go1.26.3 relationship remains explicit

872dec70's unchanged originalGo CLI7endpoint oracle remains the proposed direct
Go authority: no Go/harness/source patch or foreign template. Matching final
Go float bits is UNQUALIFIED until allocated exact tuple execution; no abs/rel
waiver or FMA expectation patch. Go can FMA; record toolchain/CPU/target/buildflags.
Core's strict-time public operation cannot consume a shuffled bootstrap holdout
as if it satisfied time separation. The originalGo bootstrap oracle should keep
using the privateSGD qualifier; correction CLI stable unique-time split can map
explicit train/holdout inputs into this new operation for numeric comparison.
Go CLI final bundle/stats are observable; IDs/gradients/per-update states and exact
prediction-rate bits remain unobserved, as previously documented. No candidate
here supplies that missing telemetry. Future full temporal split/tie/AUT policy,
cohort promotion gates, correction/drift/calibration lifecycle, signed admission,
gold-anchor management, dual-control/activation and Docker product validation
remain separate unfinished responsibilities. This concrete caller/API candidate
advances offline training independently of NativeReady, not whole ML completion.
