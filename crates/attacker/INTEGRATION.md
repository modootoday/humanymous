# Wargame integration contract v0.4

2026-11-07: attacker/defender are actual root workspace members. This supersedes
the old membership request. Packages remain publish=false, excluded from normal
installations, public runtime images, npm and published attacker images.
Rust1.99.1/edition2024 only. Use explicit --bin now that a Rust verifier exists.

## Accepted ownership and dependency direction

Compose and runtime owners accepted supervisor-owned child execution, not remote
signed installation binding. Wargame owns attacker/defender. Compose owns
test/lab/runtime, deployments/lab, image/runner integration. Root owns shared
Cargo/lock/toolchain. Direction: test/lab/runtime -> attacker -> defender ->
existing runtime/integrity/audit/scoring. No reverse testlab dependency or forked
verdict/RIT implementation exists. Agreement is accepted; library implementation
and Windows unit checks exist. Source1ee2c92's Docker context integration passed
19evaluated/7unsupported with no errors/mismatches. The current source-only v0.3
follow-up was not compiled at47499a5; v0.4's later pure validation is below and
hosted runtime requalification remains pending.

Defender added runtime/integrity/audit path dependencies and serde_json, base64,
zeroize, sha2 from the existing closure. Root refreshed the lock. No additional
manifest/dependency changes are currently requested.

## Exact exported Rust API

- `humanymous_defender::lab::{LabError, OwnedRuntime, VerifiedSnapshot}`.
- `OwnedRuntime::spawn(&serde_json::Value) -> Result<Self, LabError>` validates
  the existing complete Configuration, starts only fixed sibling
  humanymous-runtime via private stdin/stdout, and checks Linux loopback-only
  namespace, actual child image and expected readiness. No caller URL, PID,
  port, executable or socket attachment API exists. Install both binaries beside
  one another in the contextual image. No shell or environment inheritance.
- `OwnedRuntime::stop(&mut self)` kills/reaps, including error-path Drop cleanup.
  Stop before opening the actual WAL/custody owner for snapshot recovery.
- `VerifiedSnapshot::read(&humanymous_audit::log::Log, origin_effects: u64)`
  projects actual authenticated full history. Caller owns the real origin counter
  and opens Log with its provisioned identity/trusted head. `since(&before)`
  rejects changed chain prefixes, sequence and origin counter rollback.
- `humanymous_attacker::network::OwnedLab` has exactly
  `spawn(&mut self) -> Result<OwnedRuntime, LabError>` and
  `snapshot(&mut self) -> Result<VerifiedSnapshot, LabError>`.
- `network::run(&mut impl OwnedLab, seed: u64) -> NetworkReport` retains every
  scheduled error/unavailable row. `NetworkReport::validate(&self)` recomputes
  exact catalog, accounting, expectations and binary continuity. Inspect
  `scoped_passed`; `full_product_qualified` is always false here.
- `LabError(&'static str)` contains safe stage codes, never credentials/bodies.

Supervisor initializes empty authenticated history before first snapshot, keeps
the same sealed identity/config and origin counter across cases, and returns
complete audit history each time. Each case gets a fresh runtime session epoch.
Required route inventory: POST /effect with RIT/raw_header_order; POST /tls with
raw_tls; POST /h2 with original_h2; POST /collector with collector required.
Origin consumes literal body effect, counts actual calls and returns duplicate
Set-Cookie headers. All share the same network-none namespace. Outer Docker
runner binds actual hardening, source/image/topology/namespace inspection.

## JSON and error contract

`schema=humanymous.wargame-network.v2`,
`evidence_type=supervisor-owned-runtime-child-http1`, seed:u64 orders cases only.
Rows: id/tier/class/status, steps[{id,role,http_status?,response_verdict?,error?}],
processes[{runtime_sha256,supervisor_sha256,installation,revision,binding}],
effects?{origin_effects,audit[{seq,event_type,verdict,risk_score,rules,record_hash}]},
expectation_matches?, reason?. Status: evaluated/error/skipped/unsupported.
Only evaluated has expectation_matches; unsupported has no actual effects.
Receipts must retain the same source/scope; audit sequences cannot overlap.

26scheduled =19implemented network scenarios+7unsupported producer/control cases.
Totals and by_class preserve all denominators. audited_policy_verdicts comes from
actual runtime.decision audit; HTTP403 is not necessarily DENY and COLLECTED is
not a policy verdict. human_executed, assistive_technology_executed and
full_product_qualified remain false. No synthetic control is a real participant.

```
cargo test --locked -p humanymous-attacker -p humanymous-defender
cargo clippy --locked -p humanymous-attacker -p humanymous-defender --all-targets -- -D warnings
cargo run --locked -p humanymous-attacker --bin humanymous-attacker -- --seed 42
cargo run --locked -p humanymous-attacker --bin humanymous-wargame-verify -- NETWORK-REPORT.json
cargo run --locked -p humanymous-attacker --bin humanymous-wargame-verify -- --require-full NETWORK-REPORT.json
```

Verifier exit0: internally consistent scoped pass. Exit2: consistent failed
acceptance or --require-full while unqualified. Exit1: inconsistent/malformed
report or I/O. Pure simulation remains v1 JSON and is never network evidence.

## V0.3 boundary/verifier delta (implemented, validation pending)

`humanymous_attacker::verification::verify_bytes(&[u8])` returns Verification,
schema humanymous.wargame-verification.v1, with internally_consistent, mode,
scoped_passed, full_product_qualified and totals. The CLI handles both simulation
v1 and network v2. V1 re-executes the existing pure Engine/catalog and compares
every report field, accepting equivalent integer/float JSON numbers with exact
integer comparison where available. This is current-source replay, not automatic
historical-source equivalence. V2 validates actual measured records without
manufacturing network execution. Mode distinguishes both paths. Unknown schemas,
malformed/oversize JSON and inconsistent contents return safe static LabError.
No dependency, RIT protocol or network v2 schema changes. The existing Python
verifier is an unused prototype pending owner-led retirement; this Rust path
does not execute it.

Final source rejects absent/malformed IPv4 route headers, returns typed startup
reader resource errors rather than panicking, and retains simultaneous execution
and audit-snapshot errors. Replay/tampering and missing-route tests are authored,
were unrun at47499a5. Local formatting completed after the accepted policy change;
later v0.4 pure tests/Clippy passed, while hosted Docker/collector/audit and final
Git byte closure remain pending. Only source and
canon writes occurred after reclaiming red-catalog; peer paths are untouched.

## Accepted measurement follow-through

Compose's54017 immutable success remains intact:33orchestration assertions,
runtime origin2/audit4, network26scheduled/19evaluated/7unsupported,
errors0/mismatches0. Rust verifier independently accepted that raw network JSON;
--require-full returned2. Audit observed ALLOW2/CHALLENGE3/DENY1, including
legitimate controls; these are not an attack bypass rate.

Compose implemented origin-contact counting before validation and actual setup
failure reporting as19errors+7unsupported.60627's fresh source passed71outer
assertions plus the same complete network matrix, with verified cleanup/strict2
and separate UID/volume witness component. Runtime-binary sidecar fusion and
human/AT remain absent. Subsequent host parser/report edits and current library
source are not qualified by60627.55200's first failed witness attempt lost its
original dirty source graph during retry-path reuse; other failure artifacts are
archived in55537. The intact54017 success is unaffected.60036 preserves a later
witness timeout failure.60627 does not retrospectively qualify prior failures.
Compose's INTEGRATION records the exact paths and component limits.

## Open requirements and next owner

Compose implemented OwnedLab using shared custody/config/origin context. After
source closure, wargame must run the current final source on standard hosted
execution through the same Rust Docker runner, collector and fresh audits, then
attach private raw
and sanitized public evidence. All7unsupported rows
remain open: real human/browser, AT, accessible Pass, remote signed identity,
T4 real browser, full Gate original TLS/H2, safe live audit-outage injection.
Preserve existing HR-12 CHALLENGE. Independent durable witness is a separate
qualification even if its component binary becomes available.

Execution policy is authoritative only in `.spec/sot/program-roadmap.sot.md` and
`.spec/sot/delivery.sot.md`. Local work is coding/fmt and resource-bounded pure
fastchecks; actual child/port/IPC/permissions/browser/Docker/Cross-OS gates are
hosted. This owner does not dispatch duplicate jobs, waive missing private raw
recovery or infer real human/AT from a runner label. V0.3 was an uncompiled
checkpoint; v0.4 is pure validated and still lacks fresh hosted runtime proof.

Candidate boundary: root excludes this v0.3 draft from its first hosted candidate
containing runtime67c1818. This source will be a later owned not_run checkpoint
after that first Git transaction. First-run results do not qualify this draft;
root collects runtime/CI outcomes before a next-batch push. No worker dispatch,
local runtime or Docker execution is authorized by the checkpoint.
The first hosted source is bfc9f62335ae41906996c6b01b0ae9e7993feeed,
run42347732666/attempt1 (root-reported in progress). The later v0.3 source-only
checkpoint remains not_run and must be qualified under its own subsequent SHA.

## V0.5 step attribution (validation pending)

The existing network v2 fieldset remains unchanged. Verification now checks each
step id/role in catalog order: evaluated rows require all steps; failed rows may
retain an empty or partial prefix. Replay's successful protected request remains
synthetic_control and its subsequent replay remains attack. Relabelled controls,
reordered steps and appended observations are rejected even when HTTP/audit totals
are unchanged. The fixture is a failed unit ledger only. V0.4's25pure results and
earlier hosted sources do not qualify this follow-up. No dependencies changed.

Lab integration must check the actual serialized bytes with
humanymous_attacker::verification::verify_bytes and preserve this outcome beside
the actual source/image/report evidence; a Rust unit fixture is not a hosted round.
The lab owner has been asked to wire this seam in its owned paths.

## V0.4 strict report acceptance (pure validated; hosted runtime not_run)

Review found that47499a5's initial v1 Value decoding discarded duplicate keys and
v2 typed decoding accepted unknown fields. Both are intentionally tightened:
the report-specific Serde visitor first rejects duplicate decoded object keys
at every depth, including escaped spellings and dynamic-map entries. No last-key
selection precedes schema/replay verification. NetworkReport, NetworkRow, Step,
NetworkTotals, ProcessReceipt, SnapshotDelta and AuditEvent reject unknown fields.
V1 compares the entire current-replay fieldset and thus rejects unknown/missing
fields without modifying the production signal/report DTOs. New schema extensions
require explicit version evolution; silent extension acceptance is not promised.

This is strict structure and semantic replay, NOT canonical JSON bytes/JCS:
whitespace, key order, equivalent string escapes and mathematical JSON number
spellings remain compatible. Seed still requires an unsigned integer. Exact
integer comparison is retained where possible. Finite float equality is used
only for the intentionally compatible numeric representation comparison.

V1 proves agreement with the current pure engine/catalog replay, not with a
historical engine. V2 proves internal catalog/accounting/observed-field
consistency. It does not independently authenticate a claimed binary/image hash,
verify the full WAL cryptographic chain from this minimized projection, or prove
that network/participant execution occurred. Those claims require the separately
retained source/image and authenticated-generation evidence. internally_consistent
must not be rewritten as authenticity or product qualification.

Four added pure tests cover duplicate root/nested/escaped/dynamic keys, unknown
fields at every owned DTO level, and valid failure-ledger decoding. The latter is
unit-only, with scoped_passed=false, never a mock successful runtime. No scoring,
runtime wire, Cargo dependency or local runtime/Docker execution changes. This
source follows47499a5 and the reported actual63f69 source; no old run qualifies it.

Root-allocated local Cargo1slot measured25pure tests passed/0failed/0ignored on
Windows x86_64 Rust1.99.1, with --locked -j1/test-threads1/CARGO_INCREMENTAL0.
Scoped Clippy --all-targets/-Dwarnings passed. The fixture constructs only failed
observation ledgers, never runtime success or participants. Exact base
6733128bebf393a8873db7298681381fbb7a4c14 plus working patch/new parser snapshot are
retained privately. This does not qualify the whole dirty dependency graph or
hosted runtime. Raw evidence07070124/public summary07070333 retain separation
from earlier source results. No dependency or runtime execution was added.
