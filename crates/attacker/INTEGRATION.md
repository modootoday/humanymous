# Wargame integration contract v0.2

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
and Windows unit checks exist; Docker context integration is pending.

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

## Open requirements and next owner

Compose implements OwnedLab using shared custody/config/origin context and runs
the contextual Docker image. No network success is claimed before this step.
After Compose releases red-catalog, wargame corrects measured defects and records
source-bound private raw and sanitized public evidence. All7unsupported rows
remain open: real human/browser, AT, accessible Pass, remote signed identity,
T4 real browser, full Gate original TLS/H2, safe live audit-outage injection.
Preserve existing HR-12 CHALLENGE. Independent durable witness is a separate
qualification even if its component binary becomes available.
