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
