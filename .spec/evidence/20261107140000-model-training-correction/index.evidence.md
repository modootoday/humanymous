---
id: evidence:root/20261107140000
title: Model training correction read-only source inventory
status: active
created: 2026-11-07T14:00:00Z
updated: 2026-11-07T14:00:00Z
rel:
  depends_on: [sot:root/model-training-correction, plan:root/20261107140000]
---

# Read-only source inventory

Source inspection only. No model/Cargo/Docker/GPU execution. The adjacent
source-inventory.json retains current and baseline Go hashes, row pointers,
legacy test names and current Rust inference/fixture hashes. It is a review
proposal, not a modification of the root-owned retirement registry.

## INTEGRATION proposal for root

Existing Rust inference should be reused and its actual historical receipts
located before claiming mlserve closure. Training, correction implementations,
sink and command process responsibilities are absent from the inspected model
package. Observer traits and setters are integration seams, not implementations
of calibration, drift, shadow statistics or authenticated lifecycle.

Root owns any accepted manifest changes, shared baseline registry disposition and
retirement. Runtime owns event ingress and serving integration; authority/audit
owners own approval and audit adapters. Keep training output and calibration
proposals isolated until explicit activation authorization. Do not edit another
owner's INTEGRATION.md; this section is the concrete reviewable proposal.

Notable gaps: legacy comments overstate dual-control (BundleManager::Promote has
no approval parameter); Stage and warm-base loading read bytes twice; cohort gate
can have no human cohorts; challenge abandonment is weak evidence; sink flush under
mutex is synchronous; timestamp ties defeat strict future-only claims; DriftMonitor
Gate increments events even when called through Snapshot. These require explicit
preservation-versus-hardening decisions and negative pilots.

No unit, process, inference parity or product e2e pass is claimed by this inventory.

## Verification and handover

Inventory contains29 baseline Go rows, all29 current hashes equal baseline;
the existing Rust package and fixture hashes are retained separately.
The initial scoped check exited0 before the handover paragraph was appended.
That paragraph introduced document-path citation errors; the follow-up review
reproduced two L2 errors and replaces them here with typed references:
[[sot:root/model-training-correction]] and [[plan:root/20261107140000]].
The preceding full `spec check` exited1 with72369errors and
13warnings across the shared tree, including unrelated private layout/attachment
findings. No shared-tree repairs were attempted. No commit was created.

Next owner reads AGENTS.md, the model SoT/plan, this inventory and
.agents/lessons/HARD-WON.md. Root reviews the pending design decisions and assigns
implementation/execution; until then model/Cargo/lock/docs/registry edits and all
training/inference/Cargo/Docker/GPU runs remain outside this session's authority.
Codex retains the source-review chat context; the persistent artifacts are the
handover authority. Run scoped spec check again after any design revision.

## Original 29-row coverage mapping

Pointers below address the original baseline inventory, not a new retirement
decision. Each row's full baseline/current SHA256 and original test names remain
in source-inventory.json. E means existing Rust source with execution linkage
pending; M means missing implementation plus qualification. Neither means ported.
Proposed modules below are relative to crates/model/src; no files are created.
Every test-file row retains every named test in the adjacent JSON as obligations.

| Original pointer | Original path | State | Existing or proposed Rust responsibility | Remaining proof |
|---|---|---|---|---|
| /files/7 | cmd/ml-train/main.go | M | proposed bootstrap binary; training kernel | real JSONL/synthetic process, flags, diagnostics, output |
| /files/8 | cmd/mlcorrect-train/gate_test.go | M | proposed training/evaluate tests | clean candidate, poisoned cohort, insufficient sample tests and process negatives |
| /files/9 | cmd/mlcorrect-train/gate.go | M | proposed training/evaluate | time split, per-cohort support, TPR, AUT, fail-closed emission |
| /files/10 | cmd/mlcorrect-train/main.go | M | proposed correction binary | anchor/oracle/base process, exit3, no write on rejection, digest stdout |
| /files/207 | internal/mlcorrect/bundle_test.go | M | proposed correction/bundle tests | signature/digest/schema/stage/promote/rollback negative and actual file pilot |
| /files/208 | internal/mlcorrect/bundle.go | M | proposed correction/bundle using inference/Mlp | bounded single-read bytes, verification, staged/active/previous transitions in lab |
| /files/209 | internal/mlcorrect/calibrate_test.go | M | proposed correction/calibration tests | budget, floor, human-only changes; proposal-only integration |
| /files/210 | internal/mlcorrect/calibrate.go | M | proposed correction/calibration | exact clamped update order, counts/EMA, finite input policy |
| /files/211 | internal/mlcorrect/canary_test.go | M | proposed correction/canary tests | breach, graduation, drift/poisoning and one-shot callback |
| /files/212 | internal/mlcorrect/canary.go | M | proposed correction/canary | arm-relative counts, underflow safety, callback outside lock; lab rollback pilot |
| /files/213 | internal/mlcorrect/controller_test.go | M | proposed correction/controller tests | initial threshold, ambiguous outcomes, calibration/drift/Pass anomaly |
| /files/214 | internal/mlcorrect/controller.go | M | proposed correction/controller implementing existing observers | event order, sensor fusion, pure snapshots, no production threshold install |
| /files/215 | internal/mlcorrect/drift_test.go | M | proposed correction/drift tests | Page-Hinkley rise, stable/shifted PSI, 2-of-3 behavior |
| /files/216 | internal/mlcorrect/drift.go | M | proposed correction/drift | reset/latching, PSI epsilon math, non-mutating gate snapshot |
| /files/217 | internal/mlcorrect/features_monitor_test.go | M | proposed correction/features tests | freeze reference, stable/shifted windows, observer-to-drift wiring |
| /files/218 | internal/mlcorrect/features_monitor.go | M | proposed correction/features | eight buckets, bounded reference/window memory, degenerate edges, malformed vectors |
| /files/219 | internal/mlcorrect/oracle_test.go | M | proposed correction/oracle tests | legacy mapping fixtures, Pass cap/anomaly, weak-label quarantine negatives |
| /files/220 | internal/mlcorrect/oracle.go | M | proposed correction/oracle | provenance, ambiguous queue, cap; reviewed admission distinct from compatibility mapping |
| /files/221 | internal/mlcorrect/shadow_test.go | M | proposed correction/shadow tests | divergence, agreement/abstention, controller snapshot |
| /files/222 | internal/mlcorrect/shadow.go | M | proposed correction/shadow | comparable denominator, direction counts, mean delta, no candidate serving |
| /files/223 | internal/mlserve/mlp_test.go | E | inference/mlp.rs tests; tests/inference_parity.rs | original test-to-receipt linkage and actual load/score pilot |
| /files/224 | internal/mlserve/mlp.go | E | inference/bundle.rs and inference/mlp.rs | fixture provenance, exact-source float32 executed receipt, malformed/load pilot |
| /files/225 | internal/mlserve/predictor_test.go | E | inference/runtime.rs tests | map all original tests to exact executed cases; observer integration pilot |
| /files/226 | internal/mlserve/predictor.go | E | inference/runtime.rs and inference/mod.rs | abstention/schema/active-shadow and threshold reset receipts; frozen verdict proof |
| /files/227 | internal/mltrain/record.go | M | proposed training/record using behavior extraction | JSONL field parity, raw-summary round trip, invalid label/provenance rejection |
| /files/228 | internal/mltrain/sink_test.go | M | proposed training/sink tests | append, disabled no-op, reopen preservation; OS process/access pilot |
| /files/229 | internal/mltrain/sink.go | M | proposed training/sink | bounded opt-in writer, flush/close errors, quota/drop telemetry, no serving disk I/O |
| /files/230 | internal/mltrain/train_test.go | M | proposed training/kernel and evaluate tests | separable data, zero-epoch warm copy, temporal/cohort/AUT legacy cases |
| /files/231 | internal/mltrain/train.go | M | proposed training/kernel and training/evaluate | RNG/intermediate fixtures, standardization/SGD metrics and actual offline artifact pilot |

Coverage totals:29 original rows,4 E and25 M;0 retirement decisions and0 new
runtime proofs. Existing inference fixture closure is a separate evidence-linking
task. Root acceptance decisions remain enumerated in the SoT; no source-only row
is marked complete. This matrix preserves the entire originally assigned scope.

Follow-up checkpoint: scoped spec check over this evidence directory and the
owned SoT/plan with `--no-local` exited0 with0errors/0warnings. All29 Go hashes
were rechecked against baseline with0mismatches; retained Rust snapshot hashes
also had0changes. The reported three-error checkpoint was not reproducible as
three errors: the current pre-fix run showed exactly two L2 findings, both fixed.
No code, graph, manifest, model execution or GPU/download work occurred.

## First source slice authorization and status

Root subsequently authorized only record.rs, training/mod.rs and minimal lib.rs
export with module tests. Source now implements the shared extractor conversion,
required record fields, labels/schema/finite checks, explicit tighten-only input
ceilings, streamed bounded JSONL and provenance quarantine. Four module tests are
authored for round-trip/extractor reuse/quarantine, invalid input, byte boundaries
and small buffers, record/cohort/configuration limits. They have NOT_RUN status.
No training, kernel, inference, lifecycle or scoring implementation was changed.
The original29-row baseline remains the review set: row227 now has partial Rust
record source, but its executed tests and actual file/process pilot remain open.
The historical4E/25M tally above describes the pre-implementation checkpoint.

Pending allocation: scoped locked model tests, scoped fmt/Clippy and actual owned
file-reader pilot with source hashes and exact receipts. No compilation was run
during the Pages heavy window. No product parity or completed migration is claimed.
Kernel RNG/intermediates/time holdout/lifecycle require independent critique before
subsequent implementation. The root-owned shared graph/manifests remain untouched.

Independent critique follow-up: optional metadata now distinguishes omitted/null
from explicit0/empty through Option fields; typed RecordError excludes source
body/path/parser diagnostics. Negative finite values intentionally preserve shared
extractor clamping/direct-feature semantics. Two additional module tests cover
presence/serialization, negative transform parity and diagnostic redaction; total6
authored tests, all NOT_RUN. Root and independent reviewer require subsequent
kernel RNG sequence, required-cohort/support, temporal/anchor and lifecycle critique
conditions; none of those slices is implemented or authorized for execution.

## Source checkpoint and next bounded allocation proposal

Seventh authored test explicitly separates behavior-present/label-missing,
label-present/behavior-missing, null label and null behavior from a valid explicit
label0 with an empty aggregate object. All invalid cases must return InvalidJson;
explicit label0 is preserved but missing provenance remains quarantined.
Seven tests are SOURCE_ONLY_NOT_RUN. Exact hashes are in record-source-slice.json.

Module manifest: existing crates/model/src/lib.rs gains only `pub mod training;`;
training/mod.rs exports record; training/record.rs owns record validation, bounded
reader and these tests. No Cargo.toml/Cargo.lock/dependency/new-crate/binary changes
are requested for this slice. Later kernel/lifecycle modules are not part of this
manifest proposal. Existing behavior and signals dependencies suffice.

Proposed commands AFTER root allocates a distinct target and execution slot, from
the repository root, using Rust1.99.1 and jobs1/incremental0 environment:

```text
cargo test --locked -p humanymous-model --lib training::record::tests -- --nocapture
cargo fmt --package humanymous-model -- --check
cargo clippy --locked -p humanymous-model --lib --tests -- -D warnings
```

Root supplies CARGO_TARGET_DIR and CARGO_BUILD_JOBS=1/CARGO_INCREMENTAL=0;
no target path or local execution slot is self-allocated here. The test receipt
must show seven executed parser tests with0failed/0ignored and preserve first
failures. Fmt may report formatting work because no formatter has been executed.
The existing inference suite is outside this narrow parser execution denominator.
An actual bounded file-reader pilot is still required after unit qualification;
its source/venue allocation remains pending, with no product e2e claim from these
commands. Capture source hashes before/after each authorized run. No global spec
or private-tree cleanup is proposed; scoped spec is the document gate.

## Allocated C validation of frozen 68898522

Root authorized an immutable archive of commit
688985225621cb91089514c77437e6e91e49cbd7 in the exclusive
target/validation-authority slot. Native Windows x86_64-pc-windows-msvc,
Rust1.99.1, jobs1/incremental0, locked/offline. The initial overbroad whole-sysroot
census was owner-cancelled before compilation after root clarified scope;
partial inventory is retained privately and does not prove any closure.

The corrected baseline includes4023 files: complete archived source, resolved
selected registry source packages, configured-file candidates, six selected native
tool executables, bin/lib compiler DLLs and selected native target library files.
Selected package graph contains33 packages; system SDK and system DLLs remain
unobserved. Before/after4023 hashes had0difference records;18configuration candidate
presence checks had0differences. Archive SHA256:
5f6c9bf3eec3d5888d8fe7c287877505eed31f201337632ea0ea2bc5164f00e6.
Fresh test admission measured82.8807% commit and29.2948GiB free physical memory;
each subsequent check had its own retained admission record.

| Actual command | Terminal result |
|---|---|
| cargo test --locked --offline -p humanymous-model --all-targets --jobs 1 | exit0;12unit passed (7parser +5existing inference),1fixture harness passed;0failed/ignored/filtered |
| cargo check --locked --offline -p humanymous-model --all-targets --jobs 1 | exit0 |
| cargo clippy --locked --offline -p humanymous-model --all-targets --jobs 1 -- -D warnings | exit0 |
| cargo fmt --package humanymous-model -- --check | exit1;training/record.rs layout differences |

The first unexpected failure was preserved and work stopped. Frozen candidate
bytes were not modified, and no formatter write or subsequent Cargo run occurred.
All compiler/test processes are terminal and C target was returned before the
read-only closure comparison. These actual parser results supersede NOT_RUN only
for the exact frozen source. No training loop, production inference, GPU, model
download, binary/process pilot or Docker product e2e was executed. Actual
file-reader pilot and formatting correction/requalification remain open.
Raw commands, admission, first failure, terminal footers and closure manifests
remain in the owned private model-training-correction-cwarm evidence directory.

## Formatting correction and actual file pilot candidate

Root authorized a formatting correction to record.rs and one actual file-reader
test within that same module. Pre-pilot rustfmt delta was verified as whitespace
and line breaks plus one optional trailing array comma; no behavior tokens changed.
The first fmt failure above remains historical and is not retroactively green.
The new owned-tempfile test uses create_new, explicit write/flush/close and actual
File/BufReader input. It covers accepted input, byte/record/cohort rejection and
late invalid input leaving a caller's previously accepted batch unchanged.
This measures local file handling, not OS ACLs, power-loss durability, a training
process or product e2e. New candidate has8parser tests, pending actual execution;
expected total is13unit plus1existing fixture harness, subject to actual footer.

## Corrected candidate qualification

Immutable source a0ebc9ae4da4c52020f6e39bb614d1e14d4e0b2e was tested in the
root-renewed C target/validation-authority allocation on native Windows
x86_64-pc-windows-msvc, Rust1.99.1, jobs1/incremental0, locked/offline.
Fmt FIRST exited0, all-target tests exited0 with13unit passed (8parser including
the actual file-reader pilot,5existing inference) plus1inference fixture harness;
0failed/ignored/filtered. All-target check and strict Clippy both exited0.
All execution handles are terminal. C was returned before read-only closure work.

Minimal selected closure contains1493 files spanning model/transitive path packages,
resolved selected registry sources, root Cargo/lock/toolchain, six selected native
executables, compiler DLLs and declared native target libraries.33selected packages;
18configuration candidates. Before/after1493 hashes had0difference records and
configuration presence had0differences. Source archive SHA256 is
a7b332ec8d72b95ffe5bc0acde92b26227ab840d5b6427100fe3c0ab71ac7ebd.
Raw commands, fresh admissions, exact footers and manifests remain in the owned
private model-training-correction-c2 evidence directory; sanitized terminal facts
are in record-validation.json. The first68898522 fmt failure remains retained.

The actual pilot used owned create_new temporary files and File/BufReader, verified
acceptance, total/line byte limits, record/cohort limits and late invalid row
rejection without replacing an existing caller batch. It claims neither OS ACLs
nor crash/power-loss durability. No training process, GPU/download, production
inference or Docker product e2e was executed. First parser slice is qualified for
this exact native component boundary only; the full29-row migration, RNG/kernel,
gate/time policy and lab lifecycle remain open with independent critique conditions.

## Independent verification locator

Exact raw run directory, retained without rewriting chronological receipts:
`C:/workspaces/humanymous/.spec.local/root/evidence/20261107150000-model-training-correction-c2/`.
Within that directory, `results.json` records all four command exit codes and UTC
start/end timestamps; `test.stdout.txt` contains the13unit and1fixture terminal
footers and the named actual_file_reader_accepts_rejects_and_preserves_caller_batch_on_error
pass. `test.stderr.txt` identifies the executed native test binaries. `fmt.stdout.txt`,
`fmt.stderr.txt`, `check.stderr.txt` and `clippy.stderr.txt` retain their outputs.
`closure-before.json`, `closure-after.json` and `closure-paths.txt` identify the1493
hashed files; `config-paths.json`, `selected-packages.json`, `metadata.json` and
`environment.json` define selected configuration/dependency/source scope.
`fmt-admission.json`, `test-admission.json`, `check-admission.json` and
`clippy-admission.json` bind fresh admission observations to each step.
`source.tar` is the immutable candidate archive; `summary.json` holds the completed
closure comparison and actual counts. These are verification pointers, not a rerun.
The original68898522 run remains separately retained in sibling directory
`20261107144500-model-training-correction-cwarm`, including the first fmt failure
and owner-cancelled overbroad preflight census. No immutable receipts were edited
to reconcile current SoT status.

## Next L1 source checkpoint: standardization only

Original baseline /files/231 (internal/mltrain/train.go) Standardize is now mapped
to crates/model/src/training/standardize.rs, exported by training/mod.rs. Original
/files/230 training tests remain broader unfulfilled obligations; no full row
closure is claimed. This kernel is independent of the unresolved RNG/time/cohort
policy choices and unlocks later training without changing any such policy.

Four authored tests cover empty versus constant defaults; population (not sample)
variance, negative features/feature order and unchanged inputs; float32 ordered
rounding versus float64/reordered sums; schema/budget/nonfinite input and finite
input sum/variance overflow rejection. All4 are NOT_RUN. Hand-computed expected
values are regression contracts, not newly captured Go-execution fixtures.
Only rustfmt and scoped spec checks were run. No Cargo, model initialization,
training loop, dependency change, production activation or target reservation.
Existing parser13unit+1fixture qualification at a0ebc9ae remains separate and is
not transferred to this new source. The broader29-row migration remains open.

### Actual owner6 standardization qualification

Root admitted immutable e7d8f9a3b640d6af88558dce24945654a0e8c502 in existing
exclusive target/validation-authority, native Windows Rust1.99.1 jobs1/inc0,
locked/offline. Fmt FIRST, model --lib tests, inference_parity fixture, all-target
check and strict Clippy all exited0. Actual terminal counts:17library passed
(4standardization,8parser including the actual file pilot,5existing inference),
plus1existing inference fixture;0failed/ignored/filtered. These actual4kernel
results supersede NOT_RUN only for this frozen candidate. No Go execution or
training-loop equivalence is inferred from analytic regression cases.

Raw run: `C:/workspaces/humanymous/.spec.local/root/evidence/20261107154500-model-training-correction-standardize/`.
`results.json` has all five terminal command receipts; `lib-tests.stdout.txt` and
`fixture.stdout.txt` contain exact footers, stderr files retain actual executables.
`environment.json`, `selected-packages.json`, `config-paths.json`, closure manifests
and per-step admission files bind the selected scope. `summary.json` and public
standardize-validation.json retain comparison/count facts. All processes became
terminal and owner6 target was returned before read-only evidence work.
No new target lease, GPU/heavy weights, process training, production activation,
threshold change or product e2e occurred. Full baseline rows230/231 remain open:
Standardize is only one function of the training/evaluation responsibility.
