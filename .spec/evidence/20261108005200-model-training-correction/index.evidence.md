---
id: evidence:root/20261108005200
title: Model training correction read-only source inventory
status: active
created: 2026-11-07T14:00:00Z
updated: 2026-11-08T02:40:47Z
rel:
  depends_on: [sot:root/model-training-correction, plan:root/20261107140000]
files:
  - {path: record-source-slice.json, sha256: 93798404b482f7f79002f41742455237fde0d468c6e2bf4ded2fead72b7cf20b, bytes: 606, store: repo}
  - {path: record-validation.json, sha256: 1c4817263edad015048eb6aebf98baba803f8b7ebb720508ffb75a78cd2629d3, bytes: 1076, store: repo}
  - {path: source-inventory.json, sha256: f2680379e9ecede8a520bfc1d58052623f21cd3795634c302e091c0aa90c0d33, bytes: 16297, store: repo}
  - {path: standardize-validation.json, sha256: 244809da8e82543a06c16a80a952bd06610c23e1a3540acfcfaff7357e795b53, bytes: 1089, store: repo}
  - {path: evaluate-validation.json, sha256: 4032655a3c889eab255ead8193ae715448d88f7d679bad78efe0fe2a705901bc, bytes: 1002, store: repo}
  - {path: rng-upstream-provenance.json, sha256: d9f8ad32d68017a6613a652f4c776f325c2e2048a1aa6de16e607d2296d60b22, bytes: 1899, store: repo}
  - {path: rng-normal-seed0-upstream.json, sha256: 1dc34670f9779270d57307a4e71701ead87402b02ac35318b8d5fc9c28976989, bytes: 1246, store: repo}
  - {path: rng-source-checkpoint.json, sha256: 96387eb06ecb372439d9370bec517fe981b0576df624f5107defc5f4eac95e41, bytes: 3079, store: repo}
  - {path: gpu-research.json, sha256: 9b04d66b27bced22d9c32f7b218cca431aae9f82971070735323956ff7442af1, bytes: 5186, store: repo}
  - {path: rng-validation-profile.json, sha256: 9ef43265eb2dda0d957c52ba5a0d8abbc4fbe78fdc402d395ee9e4fd82686ec9, bytes: 8074, store: repo}
  - {path: rng-first-attempt.json, sha256: 13a5649a6e29cca6c13e122e206bfb39ea0f3b6ddc3f6eb57177cbfe55de3e67, bytes: 1473, store: repo}
  - {path: model-core-handoff.json, sha256: 643d8461ebd89fc49565a4d71c6d2f887c735cc64335b562610a6bfdb6a0ced9, bytes: 11818, store: repo}
  - {path: core-first-rng-validation.json, sha256: 3ae57b917a9c7885fd115d62b12121a06bdfffc5ca24d4aa9fae3d0deecba2a4, bytes: 7781, store: repo}
  - {path: sgd-next-slice-proposal.json, sha256: 341f2ff53fc312ddebecc9cdedd87cb5273dcf194f0af6596263b2511ccb8ee5, bytes: 2372, store: repo}
  - {path: sgd-source-checkpoint.json, sha256: a0333038d91c2bc9dd061db82af7acbe5b3c8100e90336d93a1d2256f82e0ee4, bytes: 3471, store: repo}
  - {path: sgd-first-format-admission.json, sha256: 23ab0a79de28c8898a1ffce7f7b2206e29b39346ecd1b123de9c5bdc100a82b0, bytes: 1676, store: repo}
  - {path: sgd-format-second-admission.json, sha256: 4ce2ec7bdc009be3cd93225f10350958c7600adb9afe9e9a0f14f0d0cf7c8fa5, bytes: 4008, store: repo}
  - {path: sgd-review-fixes-source.json, sha256: 1c997880092da87d88c1389bd9f85608ca45371ed35e85886a1e286a36e7ab70, bytes: 2537, store: repo}
  - {path: sgd-first-actual-validation.json, sha256: be033c879f1298a32f0c5b975e24e4099dec5049a6f737cdca8280e825dec059, bytes: 19785, store: repo}
  - {path: go-sgd-oracle-caller-design.json, sha256: ff4426deff4b5aaae09882f1266fc5cb9b2567d91465dd84fff8aa6f80b119cc, bytes: 3317, store: repo}
  - {path: offline-core.candidate.rs, sha256: b9680e3d553a0046855e43e267f065d91d49e1eda05fa5ee3bb1fe3d1f83d9d2, bytes: 10065, store: repo}
  - {path: offline-core-tests.candidate.rs, sha256: 1748b033e485c0ab28c9466bd4be1bf7d06fca30b5c200b449a4219e83a438e0, bytes: 5982, store: repo}
  - {path: offline-cli-tests.candidate.rs, sha256: 8b7e3042b645a2e236bb04d61fa7c879d03b9e34d94657ed0c58a4174879027e, bytes: 1526, store: repo}
  - {path: offline-cli.candidate.rs, sha256: 4be87a3f4e6b72960acde370850c1248f5c89d3cee313232651ad6b99730c300, bytes: 6342, store: repo}
  - {path: offline-cli-pilot.candidate.rs, sha256: ed3439c67e770306ef9141cd64fd4a78288b2d16439490dc1a11f69e35d3f3e2, bytes: 12161, store: repo}
  - {path: offline-api-cli-candidate.json, sha256: 1e2a99f867d864b32e407aee6898d00439d7d40eb96ca09c6a8508acb1c230c9, bytes: 2545, store: repo}
  - {path: offline-cli-pilot-lifecycle-review.json, sha256: 21395f49f36849e13653fdfd90e814ca8df6343f6e26c26e5dcf96560c3a70e3, bytes: 2168, store: repo}
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

## Next source-only evaluation checkpoint and metadata repair

Root authorized training/evaluate.rs with training/mod.rs export. Original
/files/231 Evaluate maps to bounded pure inference::Prediction reduction; no
inference call or forward pass is duplicated. Four authored tests cover confusion
counts/inclusive0.5/float32 rates, absent classes, unusable predictions/late invalid
rows, and budgets. All4new tests remain NOT_RUN. Prior17library+1fixture applies
only to e7d8f9a3. Full evaluation adapter, cohorts/AUT/time gates, SGD/RNG and
baseline rows230/231 remain open. RNG source/fixture freeze is a dependency,
not a new execution grant.

Delivery's4unlisted metadata findings did not reproduce: scoped P2 including private
documents returned0errors. Full scoped check instead exposed one I1 duplicate ID
shared with CLI's private document. This evidence now uses evidence:root/20261108005200;
its own SoT reference is updated. Spec derives IDs from bundle paths, so the public
bundle directory was also moved from20261107140000-model-training-correction to
20261108005200-model-training-correction. Four adjacent JSON receipt/inventory files
were checked byte-identical across relocation; raw run paths, hashes and creation
timestamps remain intact. No CLI document or global private tree was edited.
The intermediate ID-only commit aeefd20c failed scoped spec (I1/S2/L1) and is
preserved in history; this relocation corrects that failure without rewriting it.

### Global P2 attachment inventory correction

Independent review subsequently reproduced the exact four P2 errors only with
public GLOBAL `spec check --no-local --json --limit 20`: the four adjacent JSON
files were not listed in frontmatter files. Earlier scoped zero results did not
cover that attachment inventory gate. `spec attach` now registers exact path,
SHA256 and byte length for each file. All four JSON files were verified unchanged
before/after registration; only this index was modified. The subsequent global
public check exited0 over248documents with0errors/0warnings, and the directory
scoped check also exited0. This is not a claim about the full private global tree,
model execution or new evaluator test results. Historical failures remain intact.

## Actual separate Windows reducer qualification

Root allocated a separate lightweight Windows slot with a fresh exclusive target
target/validation-model-reducer-aeefd20c, distinct from Delivery and prior model
targets. Current model source matched frozen aeefd20cd66c454edcd0e3901f08627b80ad76be;
no own overlays were applied. Initial observation was35.3904GiB free and63.2629%
commit; each actual step has a fresh retained admission. Native Rust1.99.1,
x86_64-pc-windows-msvc, jobs1/inc0, locked/offline.

Fmt FIRST, reducer-filtered library tests, all-target check and strict Clippy all
exited0. Actual footer:4passed,0failed,0ignored,17filtered. This supersedes the
new4tests NOT_RUN status only for this exact source; the previous17 tests and
inference fixture were not rerun or counted as new passes. No first failure
occurred in this allocation. All processes are terminal and the target was returned
before closure/evidence work. evaluate-validation.json retains scope/count/closure
facts; no training, model activation, scoring, dependencies, GPU or WSL changed.

Raw directory: `C:/workspaces/humanymous/.spec.local/root/evidence/20261108020500-model-training-correction-reducer/`.
`results.json` records all4terminal commands; `reducer-tests.stdout.txt` has exact
names/footer, stderr identifies the native executable. Closure-before/after,
selected-packages, config-paths, per-step admission and environment JSON files
retain the bounded inputs; `summary.json` records the comparison. System SDK/DLL
closure is not measured. The full29-row migration and future gate/SGD/RNG/lifecycle
remain open, with no execution reservation retained.

## RNG L0 retained-source preparation

No workload was run. Local installed Go VERSION is1.26.3. Seven unchanged upstream
source/version/license files were copied byte-for-byte to
`C:/workspaces/humanymous/.spec.local/root/evidence/20261108021500-model-training-correction-rng-l0/upstream/`.
rng-upstream-provenance.json records exact hashes/sizes; rng-normal-seed0-upstream.json
retains20source-golden decimal expectations and the exact upstream source identity.
This is retained authoritative source data, not an oracle execution or mixed-stream
fixture. The plan now specifies exact seed/shape/epoch/input/midstate fields,
unmodified bounded oracle invocations and observability gaps, plus proposed Rust-only
kernel/reference scope. Root acceptance is needed before RNG implementation or
fixture/oracle workloads; existing full model scope/freeze remain unchanged.

## User-authorized Rust RNG source and differential tests

The user subsequently authorized Rust RNG implementation with no execution before
separate allocation. Seven Rust files in training/rng plus module export implement
single seeded stream, bounded draw state, normal, shuffle and cold/warm helpers.
All12new tests are NOT_RUN. No Rustfmt, Cargo, Go, model initialization, training,
inference, download, GPU or process pilot was executed for this candidate.

Static source inspection compared all607cooked and3x128normal table spellings
against pinned Go1.26.3 (only typed float1 versus1.0 differs). Full BSD LICENSE
SHA256 remains911f8f5782931320f5b8d1160a76365b83aea6447ee6c04fa6d5591467db9dad.
Unchanged source fixtures are table DATA, never executable Go. Upstream output
goldens versus Rust-derived reference/state/tape evidence are explicitly separated
in tests and INTEGRATION.md; no Go-observed midstate values are fabricated.
The later allocated tests must establish actual results, source closure and math
platform limits. Root-owned aggregate attribution and full-training budget remain
integration gates; existing14/18/4pass histories do not qualify this new source.

Owned scoped spec check passed with0errors/0warnings. The public global snapshot
instead had one P2 error in the browser owner's viewport-source-candidate.json
attachment inventory; no foreign repair was attempted. This checkpoint claims no
global green and no formatter/test execution. All kernel/source/fixture hashes
are registered in rng-source-checkpoint.json; future execution must bind to the
actual committed candidate and preserve any first failures.

## ML GPU eligibility research, 2026-11-08 KST

gpu-research.json binds read-only Win32 inventory, model source hashes, source-derived
shape/byte counts, exact reviewed upstream source URLs/versions and evidence gaps.
No GPU/runtime/driver probe or load, shader, dependency change, model download,
training/inference/benchmark, compiler or actual device pilot was run. Existing CPU
experiments and the RNG72888805 NOT_RUN checkpoint were left unchanged.

WMI reports RTX4070Ti/driver32.0.16.1088 and i9-14900K on Windows11x86_64. This is
hardware inventory, not proof that any CUDA/Vulkan/DX12 context or physical compute
is usable. VRAM, GPU memory pressure and runtime feature support are NOT_MEASURED.
Machine-specific PNP ID is omitted. The owned SoT separates real/software adapter
qualification, tiny-online CPU priority and offline batch/training eligibility.
No candidate currently meets frozen exact output/order/RNG equivalence. Nonzero
diagnostic tolerances are explicitly non-accepting; activation stays prohibited.

Primary source review includes Candle0.9.1 tagged manifests (newer0.11.1 source-tag
reconciliation unresolved), Burn v0.22.1 manifest and0.22.0 API docs, wgpu v30.0.1
tagged platform/MSRV source and versioned device/queue/feature docs, NVIDIA official
hardware/Windows CUDA/floating-point references. Source/version, compatibility
eligibility, source proposal, actual pilot and product proof remain separate.

Owned model scoped spec passed0errors/0warnings. A concurrent public-global snapshot
reported one foreign E1 in the browser owner's gpu-boundary-research.json attachment
hash, not this model bundle. No foreign metadata was repaired and no global-green
claim is made. This research checkpoint does not reserve a compiler/GPU target.

## Frozen RNG12 validation handoff

rng-validation-profile.json pins728888051b7dacfa3e3181cf71da4fff2f526bd4,
no overlays, all13owned source/license/fixture hashes and12exact RNG test names.
Static reinspection found current source identical to the frozen checkpoint.
Go1.26.3 seven-file source identity and full BSD license remain preserved; Go
output expectations do not become observed mixed internal state. The proposed
fresh Windows target is absent/unreserved. Commands are fmt FIRST, focused12RNG
tests (21prior tests expected filtered), scoped check and strict Clippy. Deadlines,
admission, exclusions, selected closure and first-failure behavior are explicit.
This is READY_FOR_ALLOCATION, not an execution receipt:0tests/formatter/compiler/
oracle runs,0live handles and no target lease. Prior correctness proofs stay bound
to their original sources; full training/calibration/lifecycle remains incomplete.

## First granted RNG12 attempt: harness failure before any Cargo child

Root granted profile0bd901e2 with immutable72888805/no overlays,390swork+60scleanup
from the persisted preparation clock. A fresh target was acquired with actual
CreateNew/readback marker. Archive and all13owned file hashes verified. Before
metadata/closure/fmt/test launch, the guard falsely reported expired work time:
ConvertFrom-Json materialized a UTC timestamp, then DateTime.Parse of its implicit
string lost UTC kind and applied the local+09offset again. This was an orchestration
failure, not a compiler/RNG compatibility result. No hidden retry occurred.

Clock literal19:15:24.5965004Z to cleanup observation19:16:54.7654934Z is actually
90.168993seconds, within BOTH deadlines. The original erroneous32490.7654934s and
hard-deadline-exceeded flag are preserved in raw deadline-failure.json; separate
corrected-time-analysis.json records the32400sconversion error. Do not rewrite
the first failure or infer a real deadline overrun from the erroneous calculation.

Target contained only its matching owner marker, copied/read back before release;
marker and empty directory were removed, no cache cleanup or foreign termination.
Target is returned and no child handles exist. RNG tests executed0; metadata,
fmt, check and Clippy NOT_RUN. Only archive/13file proof exists; selected registry,
config, tool and environment closure was NOT_CAPTURED. Public rng-first-attempt.json
registers the failure, corrected clock arithmetic and exclusions. Raw directory:
`C:/workspaces/humanymous/.spec.local/root/evidence/20261108044500-model-training-correction-rng12/`.

Any renewed attempt requires an explicit allocation and reviewed corrected
orchestration: parse original ISO8601 strings once as DateTimeOffset, retain the
offset-bearing type across persistence, and use prebuilt bounded supervision for
the whole preparation/work interval. No RNG source was changed to mask this failure.

### Reviewed monotonic repair proposal, no renewal or execution

Root superseded the earlier DateTimeOffset deadline suggestion: ONE live Stopwatch
must begin before preparation IO;390swork and450shard remaining derive solely from
Elapsed. UTC DateTimeOffset.ToString('o') is audit-only and never parsed for control.
The owning plan contains literal initialization/persistence, remaining/WaitForExit,
bounded child settlement and required selected-local-manifest/Rust inclusion
snippets. It prohibits absolute-ancestor /target/ filtering and requires each
selected local manifest/Rust input plus13frozen files in BOTH before/after maps.
No code source changed;72888805 remains frozen,12tests remain NOT_RUN, and the
failed clock is not resumed. This is a concrete review proposal awaiting a fresh
grant, not an execution receipt, active target lease or new product script.

## Exact model-to-core path/hash handoff

model-core-handoff.json records24current files under crates/model, all hashes/bytes
and proposal-only destinations in the accepted humanymous-core model context.
Last model source commit72888805; model source tree clean,0live compiler/process
handles and0target leases; failed RNG target remains absent. Only owning plan/
evidence are edited here. Root exclusively freezes/sequences path, manifest/import,
module/fixture and branch integration. No compatibility helper package is created.

The plan audits existing private/pub(super)/future pub(crate) boundaries, four
crate::inference references, behavior/signals cross-context types, scoring consumers,
manifest-relative fixture hazards, BSD data preservation and the Windows closure
consumer. Required new tests are separate from historical14/18/4passes and RNG12
NOT_RUN. Public family acceptance does not establish all APIs public, native/WASM
compatibility, RNG qualification or completed29row training/correction migration.

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
