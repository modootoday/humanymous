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

Document version 0.1.1. Root accepted existing-package placement, lab-only lifecycle,
weak-label quarantine, single-buffer loading and fail-closed cohort support.
Only the first source slice (training/record.rs, training/mod.rs and minimal lib.rs
export with module tests) is authorized. Compilation and pilots remain unallocated.
New crates, manifests, binaries and subsequent kernel/lifecycle implementation
remain unapproved. Full legacy responsibility
remains in scope. No retirement, training, inference or runtime parity is claimed.

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

Self-critique verdict: needs-work before implementation. Root must accept module
placement, offline bounds, RNG compatibility strategy, required-cohort/time policy,
weak-label quarantine and lab-only lifecycle API. Independent adversarial review
is still pending. No scoring delta is proposed. Final baseline mapping and Go
retirement belong exclusively to root after source-bound qualification.
