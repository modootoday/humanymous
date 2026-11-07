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
| R06 | Explicit bounded response streaming | runtime;R05 | Source native bounded stream wired; pure/hosted incremental fixtures NOT_RUN, actual backpressure/abort/drain negatives pending; browser ACK stream separate |
| R07 | Operational settings/state CAS/restore | runtime; audit lifecycle | Not implemented; scoring writes refused; read-only until durable apply/replay exists |
| R08 | Bounded session/global abuse admission | runtime | Source global/session/capacity/serialized concurrency implemented; pure and hosted429fixtures NOT_RUN; no client-fingerprint identity authority |
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
R05 bounded complete-response framing extension: isolate origin response parsing
from request framing. Decode complete chunked entities with checked hex lengths,
checked decoded64KiB budget and explicit128KiB wire budget; reject TE+CL,
non-chunked/duplicate TE, malformed/truncated chunks and trailing data. Chunk
extensions and nonempty trailers remain explicitly unsupported and fail closed,
not silently dropped. Preserve duplicate ordinary headers/Set-Cookie and raw OWS;
strip hop/reserved gateway fields and reframe decoded entities with exact CL.
Bounded close-delimited complete bodies may be reframed after EOF; keep-alive
origin headers are allowed only with valid complete framing, never forward
Connection nominations. HEAD/204/304 remain bodyless;204 forbids CL/TE.
This complete-buffer implementation does not claim R06 incremental streaming,
full trailer support, original TLS/H2 provenance or browser body authentication.
Native/browser request codecs and frozen engine are unchanged. Pure malformed
framing and boundary fixtures precede hosted origin/abort/effect validation.
R05 source complete-buffer framing now decodes bounded simple chunked bodies and
reframes close-delimited EOF bodies, retains duplicate cookies, strips origin ACK
forgeries and rebuilds decoded CL. Chunk extensions/nonempty trailers/upgrade
remain explicit unsupported refusals; streaming and informational responses are
not qualified. Three new pure malformed/boundary fixtures authored NOT_RUN.
Hosted-only Unix service fixtures2 now exercise real shared IPC status with
credential/scope negatives, custody exclusion until stop, both listener cleanup,
explicit drained/flushed outcome and controller-close failure. They use actual
sealed/WAL implementations with synthetic fixture keys, no external witness;
this is not independent witness/product/human evidence. Never execute these via
a local broad cargo-test command. No current-source tests have executed.
Public spec check briefly reports duplicate external plan id20261107072500 in
native-abi/network-capture; root notified, owner files left untouched.
R08 authenticated-session admission: bounded session/global admission is resource
control, never a fingerprint/identity/scoring tell. Add validated private setting
session_rate_per_second (default100, range1..10000). Each volatile session owns one
fixed monotonic one-second window; no global per-client-key map or live eviction.
A valid next RIT is verified and consumed before session admission, preserving
browser ACK progression on429. Invalid signatures/replays never debit a victim's
session quota. Session-bound429 occurs before report mutation/scoring/audit/origin;
consumed browser responses retain actual429 ACK. No verdict/rule/weight changes.
Window reset uses elapsed monotonic duration, rejects rollback at the pure seam,
never extends a request deadline and cannot revive expired sessions. Global
pre-auth admission remains shared Admission; capacity rejects new sessions while
live epochs remain intact and expired entries are pruned. Serialized ingress
bounds concurrency1; no new throughput or identity flood-resistance claim.
Pure quota/boundary/rollback tests and shared-client429counter tests are required;
actual global/concurrent/socket origin-effect tests remain hosted-only.
R08 source adds per-session monotonic fixed windows with bounded validated config,
after next-token verification/consumption. Two pure window fixtures plus one
hosted-only native TCP/shared-client fixture now cover quota boundary/rollback,
invalid-proof non-debit, actual429 ACK progression, replay refusal and no origin
accept. This fixture is resource timing/protocol evidence, not human speed/motor
classification. Source compilation and all fixtures remain NOT_RUN; no local
native process or Cargo execution. Required missing collector/TLS/H2 still422.
R06 explicit native bounded-stream profile: route.response_profile defaults to
buffered; native_stream opts in to incremental entity forwarding with1MiB total
entity cap,8KiB copy buffer and existing exact signed request/connection deadline.
BrowserAuthOnly+native_stream configuration is refused: browser full-body/ACK
contract is unchanged. Shared response-head validation/header stripping is reused.
Content-length streams preserve exact length; chunked/EOF origins are decoded and
re-encoded as downstream chunked, emitting final zero chunk only after successful
completion, so premature close is detectable. Malformed size/CRLF/truncation,
extensions/nonempty trailers, excessive bytes and unsupported upgrades fail closed;
after the response starts, close without appending503 or retrying origin. Known
length over cap is refused before downstream head. HEAD/204/304 stay bodyless.
Backpressure is awaited, no unbounded queue/detached producer. Stop/drain cancels
or bounds active streaming with the same custody lifetime and no new origin after
observed stop. This is request integrity, not response-body authentication or
infinite SSE availability. Required collector/TLS/H2 remain independent422.
R06 source now wires route.response_profile=native_stream through the actual
connection origin branch after audited ALLOW, client liveness and exact deadline
checks. Browser routes cannot select it. The stream reuses response-head and
chunk-size validators,8KiB buffers and1MiB entity cap; no detached producers or
retry. Three in-memory async decoder fixtures plus one pure config-profile
fixture are authored. A hosted-only TCP fixture gates origin completion on proof
that the client already received its first SSE event; synthetic client report
and mock audit ordering are explicitly fixture controls, not durable/product
qualification. All new source compile/Clippy/tests remain NOT_RUN. Root's earlier
shared Clippy observed9deadcode errors while this wiring was WIP; no suppression
was added. Current wiring references every stream helper. Source requires a new
exact hosted/pure validation result, not attribution of the earlier failure to
success. Actual abort/backpressure/deadline/drain/malformed socket pilots remain
required. Global Cargo slots A/B stay root-assigned; no local Cargo started.
R05 connection nominations: parse bounded comma-separated Connection tokens with
existing HTTP token grammar, reject empty/invalid nominations and nominations of
framing/Host/Trailer authority, then strip nominated response fields case-
insensitively in the shared encoder for buffered and native stream profiles.
Static hop fields include Connection/Keep-Alive/TE/Proxy-Connection and proxy
credentials; origin reserved gateway metadata is always stripped. Ordinary
nonnominated duplicate Set-Cookie and application fields retain original bytes.
Normative references: RFC9112 message framing and RFC9110 section7.6.1 Connection.
https://www.rfc-editor.org/rfc/rfc9112.html
https://www.rfc-editor.org/rfc/rfc9110.html#section-7.6.1
R07 read-only operational view: expose a typed bounded OperationalSettings snapshot
(session capacity/TTL, connection deadline, global and session resource rates),
separate from scoring policy/scope/secrets. Configuration validation reuses this
one validator; status readiness includes this nonsecret snapshot. Native Observer
may POST exact /management/v0.1/settings with the same exact scope envelope and
credential/native-only rules as status. Response is explicitly readonly; no CAS
write or durability claim until actual private snapshot/audit replay service is
available. All mutation methods/paths and scoring weight/threshold/rule/band fields
remain refused. Audit PrivateState is currently crate-private; request its owning
API instead of copying native permissions, locks, atomic storage or cryptography.
R05 response nominations source adds case-insensitive Connection-field stripping
in the shared encoder (buffered+native stream); protected framing/authority
nominations fail closed. One pure fixture authored NOT_RUN. R07 actual native
Observer settings route now returns a nonsecret OperationalSettings snapshot,
with one shared resource validator used at configuration startup. Two pure
settings-schema/route fixtures and an extension of the hosted Unix IPC fixture
are authored NOT_RUN. Frozen scoring and secret fields are absent/refused;
settings_writable remains false. Full CAS apply/durable replay/recovery is still
required and depends on an audit-owned private snapshot service, not copied OS
storage. No current-source Cargo/native process/IPC/browser execution occurred.
R12/R15 shared sealed owner migration: audit29ee901 owns SealedLog custody+WAL+
witness lifetime and all key loading/recovery logic. Runtime startup retains only
its compatible public SealedAudit adapter and Configuration/Peer mapping, calls
SealedLog::open with UnlockCredential::Utf8 and PinnedWitness, and never extracts
mutable Log/Custody/keys. Existing authority private-directory checks still bind
the explicit configured Peer UID/SID before secret consumption; shared audit
checks its own actual custody owner. Required missing witness refuses first.
ExternalWitness delegates key pinning to PinnedWitness; no copied pin validator.
Public readiness and witness durability/independence distinctions are unchanged.
Explicit recovered startup requires a supplied witness plus independently pinned
head and TailRecovery choice; no implicit history/key provisioning, effect retry
or tail erasure. Actual recovery coverage/ambiguous tail is reported separately
and does not qualify full witnessed product state. Audit shared pure6/Clippy
results are producer scope only; runtime adoption compile/hosted tests are pending.
Shared startup migration now consumes audit29ee901 SealedLog/PinnedWitness and
removes runtime-owned key loading, HMAC/signing seed composition, WAL creation
and custody ownership logic. Public compatibility wrappers retain configured
native Peer binding and missing-witness-first refusal. ServiceHost::bind_recovered
requires an explicit witness/TailRecovery and independently retained trusted head;
startup rejects missing head or history-provisioning mode before reading state
or consuming unlock. Readiness reports bounded startup recovery coverage facts,
not arbitrary pin/key data or completed effects. One pure missing-pin refusal
fixture uses a rejection-only witness whose I/O methods panic if invoked; no
synthetic successful witness is claimed. Runtime adoption compile/hosted tests
remain NOT_RUN; shared audit pure6/Clippy results remain producer scope.

Git coordination incident record: a failed claim raced audit29ee901 and the old
PowerShell batch continued to owned runtime1a9c48a because it lacked an immediate
native exit-code guard. Verified history audit29ee901→runtime1a9c48a→root82df720;
shared index empty, no index.lock, git-ops free. Audit owner independently confirmed
its source commit/files preserved and no observed loss. No reset/restore/unstage/
amend/force action was taken. All subsequent writes must check successful claim
exit AND actual git-ops session before add/commit, and release only that session.
Historical hosted evidence recovered2026-11-07: run42348518353 at exact source
63f69ae9de703939b7b9091e2f215b12bea265de completed FAILURE,16jobs:3success/13failure.
MSRV job128338631933 explicitly executed runtime15tests with15passed/0failed/
0ignored/0filtered, including the native shared-client ACK fixture. That source
predates later management/stream/admission/shared-owner work. These are native
fixture/component results with client-asserted report and mock audit, not actual
browser/human or full witnessed product evidence. Latest source remains NOT_RUN.
Private captured CLI stdout (not original HTTP response bytes) is attached under
evidence:root/20261107061820; sanitized exact-source summary is attached under
public evidence:root/20261107061821. Other failed native probe details remain
unrecovered; no success is inferred. No rerun/dispatch occurred.