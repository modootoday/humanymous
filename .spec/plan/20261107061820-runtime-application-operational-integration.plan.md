---
id: plan:root/20261107061820
title: Runtime application operational integration
status: active
created: 2026-11-07T06:18:20Z
updated: 2026-11-07T06:18:20Z
rel:
  governs: [crates/runtime/**]
  depends_on: [sot:root/runtime-application]
---

# Runtime application operational integration

## Goal

Complete the assigned runtime follow-through while tracking every missing
responsibility. A native/session or proxy slice does not complete this plan.
Rust1.99.1/edition2024 only; scoring freeze, accessibility, defense-only lab,
dual-control and missing-required fail-closed remain invariant.

## Steps

| ID | Requirement / accepted contract | Owner / prerequisite | Current status and acceptance |
| --- | --- | --- | --- |
| R01 | Shared14-field browser codec + counter ACK | root integrity;53abb0b/fabed20 | Root source-validated; runtime reuse pending; fixed byte/HMAC vectors, no copy |
| R02 | Explicit cookie-free browser session/collect/protected profile | runtime;R01 and normal lane claim | Source implemented, latest source tests NOT_RUN; actual shared-client nativeTCP fixtures authored, hosted execution pending |
| R03 | Fresh collection provenance/coverage | root collector + runtime join | Contract pending; required.collector422; actual epoch/version/coverage vs server time/counter separation |
| R04 | Actual browser Fetch/one-flight/ACK/body/abort lifecycle | root collector + runtime server | Pure client validated by root; actual join pending; actual browser and human/keyboard/AT separate |
| R05 | Shared response-head/framing and fidelity | runtime | Partial source for ACK strip/reframe, HEAD/204/304 and partial-write close; latest validation NOT_RUN; full framing/stream work remains |
| R06 | Explicit bounded response streaming | runtime;R05 | Not implemented; incremental SSE, backpressure, malformed chunks/abort/no retry; browser ACK stream separate |
| R07 | Operational settings/state CAS/restore | runtime; audit lifecycle | Not implemented; scoring writes refused; read-only until durable apply/replay exists |
| R08 | Bounded session/global abuse admission | runtime | Not implemented;429/capacity/concurrency/clock tests; no client-fingerprint identity authority |
| R09 | Actual separate local management handler | authority transport + runtime; e346834/e8ddb4c | Shared native transport qualified in its scope; runtime observer handler source implemented NOT_RUN; native IPC/peer/credential/RBAC/CSRF/scope negatives pending |
| R10 | Signed destructive controls and state effects | audit issuer + runtime | Audit primitives implemented; runtime join pending; exact two-party/one-use/target/expiry/restart/failure tests |
| R11 | Accessible Pass recovery + shared Engine fusion | runtime; shared challenge/root browser | Not implemented; no motor/speed gates or confirmed-human training; real input/AT separate |
| R12 | External witness library injection + deployment transport | audit/runtime/Compose | Audit client/service verified in component scope; runtime typed injection source implemented21b6c4b NOT_RUN; independent deployment remains unqualified |
| R13 | Actual TLS/H1 original capture | runtime + narrowly approved network paths | Not implemented or claimed yet; full observation+successful handshake+connection binding; private synthetic lab |
| R14 | Complete H2 frame/HPACK/per-stream capture and server | runtime/network;R13+binding | Required follow-up; no H1-to-H2/fullM2 success inference |
| R15 | ServiceHost private input/readiness/stop/drain/flush | runtime; CLI/shared until/audit flush | Source implemented, NOT_RUN; real lifecycle/failure/lock-release tests pending, synchronous I/O limits explicit |
| R16 | Actual CLI/system-service connection | CLI/runtime;R15+boot credential provider | Pending; signed descriptor/artifact/nonce, actual OS lifecycle, no unattended claim before credentials |
| R17 | Full source-bound product qualification | root/Compose/wargame/delivery/runtime | Pending; Docker full topology + actual effect/audit + ordinary human ALLOW/keyboard/AT; component results separate |

Mandatory remaining subrequirements: HttpOnly/path-scoped-cookie per-target
prepare or trusted application identity; adapter0.2 one-use request/session
receipt; shared audit-owned sealed Log+Custody owner; genuinely bounded-memory
WAL/Merkle recovery; prepared-candidate destination-bound rotation; complete
authenticated machine API and origin bypass semantics without fake browser
telemetry. Existing native0.1/adapter0.1 wire compatibility is retained.

Execution order agreed across chats: audit checkpoint→authority common client/
generic transport→audit two stdout-accounting test fixes→runtime. Root owns
shared Cargo/lock/toolchain and collector/integrity. This lane never edits those
files. Network expansion needs the separately approved narrow claim after
binding agreement; no existing parser/report/scoring edits.

Before each implementation tranche, update owning SoT/API status and integration
request. Run only authorized scoped checks, actual boundary pilot, then Docker
product gates in their qualified environment. Preserve raw failed/partial runs
privately and public sanitized source commitments through Specfold attach.
Compare LF working bytes against staged Git blobs; distinguish dirty source from
canonical commit. Commit only owned paths with git mutex and attribution.

## Record

- 2026-11-07: reread program-roadmap1.2, actual runtime sealed startup, collector,
  authority, audit and Go read-only oracles; applied coordinate-sessions,
  implement-sot-slice, design-sot/adversarial-critique. Three read-only reviewers
  returned browser/proxy-Pass/service-admin findings; no subagent wrote files.
- Shared native whole-header signer cannot be used by Fetch. Root accepted and
  implemented distinct browser codec53abb0b and ACK/client state fabed20. Cookie-
  free initial profile is explicit; path-scoped-cookie compatibility remains
  required. Exact agreements are promoted into runtime-application v0.3 above.
- Authority e346834/e8ddb4c provides real shared client/LocalHandler/explicit
  drain; Windows25+release3/Linux22+UID1 are owner-reported component results,
  not runtime validation or whole product. Audit d001ee6 provides witness/control
  operations,5528d74 only fixes two helper stdout-accounting tests.
- Actual read-only baseline attempt at HEAD1ee2c928247aacc748682a3a71bca9ad519941ff:
  cargo test -p humanymous-runtime --locked exited1 because shared Cargo.lock
  required update. Actual tests0. No lock bypass/update by this lane. Root later
  refreshed shared resolution; that does not turn the failed attempt green.
- User requested memory-surge triage before implementation. New cargo/rustc/
  build/browser/Docker/global-spec/subagent execution is held until root resource
  reassignment. No owned running process/container/session exists; all three
  review agents completed. No Desktop/other-chat/whole-WSL kill was attempted.
- Following audit normal release, gate-edge/rust-runtime-followthrough was
  claimed with runtime and own spec paths only. Current work is contract/source
  only; heavy validation remains held. No new implementation is validated yet.
- Root preserved waiting history and R01-R17 under private evidence20261106194410
  runtime-waiting-contracts.json. This plan is the owning follow-through canon,
  not an assertion the waiting requirements were implemented.
- Source-only R02 checkpoint adds explicit browser route/session profiles,
  Origin/metadata/cookie guards, exact shared browser canonical verification,
  consumed-counter ACKs, preserved application Authorization, origin reserved-
  header stripping and server projection of empty browser report sessionId.
  Native0.1 strict report matching and whole-header signer remain unchanged.
  Required collector/TLS/H2 still422. Two native shared-client fixtures are
  authored but NOT_RUN; they do not constitute actual browser/human validation.
- An intermediate production-only cargo check -p humanymous-runtime --locked
  -j1 with CARGO_BUILD_JOBS=1 passed before the report-binding/test additions.
  No exact intermediate manifest was captured; this result is not attributed
  to the final checkpoint bytes. No latest-source tests/Clippy/runtime/Docker/
  browser were executed locally. Scoped formatter passed; public-only Specfold
  check has0errors/1existing unrelated binary warning. Private global scan held.
- Validation policy now references the single program-roadmap/delivery canon:
  local coding/fmt/bounded pure unit, Cargo concurrency1; actual process/port/
  IPC/ACL/SCM/browser/Docker/CrossOS on free standard hosted runners. Fixture
  human/AT/interactive/device/independent-authority claims remain separate.
  Do not duplicate policy orchestration or remove required gates by path before
  the shared impact classifier exists. R15 source follows this checkpoint.

R15 source checkpoint adds service::{read_configuration,ShutdownRequest,Readiness,
ExitReport,ServiceHost}. ServiceHost::bind authenticates existing sealed custody
and binds ingress before readiness; run takes watch::Receiver<Option<ShutdownRequest>>.
The serialized application stops accepting on observed shutdown, checks the watch
before origin connect/write, bounds active asynchronous drain, and flushes the
retained audit before reporting stopped. Closed controllers and incomplete drains
return errors. No detached workers are introduced. Synchronous private input,
custody recovery and WAL flush require supervisor deadlines; this source cannot
promise their preemption. Main reuses this lifecycle with legacy inherited pipes;
actual SCM/SIGTERM controller, management listener and unattended credentials are
still R09/R16 work. This implementation and all lifecycle tests are NOT_RUN.
External witness injection source contract (R12/R15): private configuration may
set audit.require_external_witness=true (default false preserves legacy startup).
Without injection, startup refuses before opening custody/WAL or consuming the
unlock credential. startup::ExternalWitness::new consumes Box<dyn
CheckpointWitness>, a broker-pinned expected public key and WitnessDurability;
ServiceHost::bind_with_witness consumes that value and calls the shared
Log::open_with_external_witness. There is no failure fallback to writer-only.
The client is Send + owned/'static, not required Sync; all audit/pipe exchanges
remain serialized. Reuse audit WitnessServiceClient::connect/from_ready and its
private frame codec. Broker owns authenticated handles/peer, actual I/O deadlines,
child lifetime and reaping. FileAndDirectory is required for the Linux product
isolation profile; FileSynced remains explicit namespace-unconfirmed component
policy. Readiness records injection and required durability, while independence
qualification remains false pending deployment evidence. Injection itself is not
independence evidence. New missing-injection refusal test is authored, NOT_RUN;
all new source compilation/runtime/witness integration tests are NOT_RUN.
R15 lifecycle budget follow-through: service::StopBudget validates nonzero drain,
nonzero supervisor reserve and nonzero manager deadline, checked addition and
strict drain+reserve < manager deadline; drain retains existing1..30000ms bound.
It returns the existing ShutdownRequest for wire/API compatibility. This is a
configuration relationship, not a timer able to interrupt synchronous fsync or
witness I/O. CLI/broker must enforce the manager deadline and supervise actual
blocking work/process reaping. ExitReport success explicitly records drained and
audit_flushed only after those events complete. Closed controller, drain deadline,
audit flush failure and force-kill remain failures, never synthesized Stopped.
Admission429 and pre-response error writes must observe shutdown and drop pending
I/O immediately; they cannot extend a short drain to the ordinary connection
budget. Critique: equality/overflow budgets are rejected; positive typed fields
must not imply physical I/O preemption; partial response cancellation closes the
connection and never retries origin. Existing ShutdownRequest::new remains a
bounded component API without an OS supervisor-budget qualification claim.
- Explicit user goal set active for all R01-R17 on2026-11-07. Goal is incomplete;
  owned process/container count0. Next source tranches are lifecycle budgets and
  actual local management/dual-control joins, with external owners for neutral
  sealed audit startup, OS-protected unattended credentials, collector provenance
  and hosted product orchestration. No source-only checkpoint completes the goal.
- Lifecycle budget source and one pure unit fixture authored; compile/test NOT_RUN
  pending root's global Cargo slot. Added success fields drained/audit_flushed;
  unconsumed429/error writes now cancel on observed stop. Shared runtime APIs stay
  compatible. Current hosted42348518353/63f69ae cannot qualify these later edits.
R09 first real management slice: a separate filesystem UDS/local named-pipe
listener uses authority LocalHandler/LocalContext and serve_handler_until; no TCP
management listener or application-route forwarding. An independently generated
management credential uses authority Credential's exact constant-time Bearer
verifier and is provisioned only through an explicit trusted broker accessor,
never readiness JSON. Expected native UID/PID is rechecked in handler. Read-only
Observer role can POST only /management/v0.1/status with exact JSON five-field
installation/tenant/origin/revision/authority scope and management Host
humanymous.management.local. Reject duplicate headers, body >8192, non JSON,
Cookie/Origin/Referer/FetchMetadata/proxy headers and session/RIT credentials.
The bounded response contains scope and sanitized runtime capability snapshot,
not secrets/client data. No settings mutation, session reset, kill, erasure,
ban or key rotation exists in this slice; those remain refused pending exact
real signed permits and durable state effects. Management admission uses fixed
capacity8/rate20/timeout1000ms; stop uses validated service drain budget. Locality
is not admin authority. Runtime ingress continues to refuse reserved admin paths.
Critique reflected: transport peer filtering is not assumed; handler checks the
actual context peer. A credential is not imported by copying comparisons; shared
credentials owner supplies zeroized private provisioning next. Concurrent native
transport handlers read immutable snapshots and perform no detached work. Both
listener lifetimes must end before final audit flush; failure triggers sibling
shutdown and cannot release custody early or yield a successful ExitReport.
R09 source now implements the separate observer management listener and the
ServiceHost dual-listener join. ManagementPolicy consumes a separately provisioned
shared authority::Credential (generate or from_private_provisioning); scope,
role and native peer remain runtime policy, not token grammar claims. The runtime
never duplicates constant-time comparison or imports arbitrary bearer strings.
Both listener tasks are joined without detached runtime workers. This source and
two pure policy fixtures are NOT_RUN pending the allocated Cargo/hosted gates.
R07/R10 writes, durable state replay, independent credentials producer, actual
IPC/ACL/cancellation/lock-release pilots remain incomplete. Actual hosted run
42348518353 at63f69ae does not include these later changes.