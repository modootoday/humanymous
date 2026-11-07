# Runtime integration request

## Current follow-through v0.3 (source/contract work, validation held)

Owner gate-edge/rust-runtime-followthrough normally claimed after authority
e346834/e8ddb4c and audit5528d74 released. Current normative contracts live in
runtime-application.sot.md and plan:root/20261107061820 (R01-R17). Historical native
slice results below remain historical; none qualifies this follow-through.

Current validation policy is the single program-roadmap/delivery canon: local
coding/fmt/bounded pure unit with Cargo concurrency1; actual process/port/IPC/
ACL/SCM/browser/Docker/CrossOS uses free standard hosted runners. No new local
runtime/browser/Docker/global-private scan is permitted. This chat has no running
owned process/container/session. Interactive human/AT/physical/independent
authority evidence cannot be inferred from a hosted label.
The earlier --locked baseline at1ee2c928 exited1 before tests because the shared
lock needed update; actual tests0, retained as failure. Do not bypass --locked.

Accepted shared browser API is integrity::request::browser::{Binding,canonical,
PROTOCOL,MAX_SAFE_INTEGER,acknowledgement,verify_acknowledgement}; root commits
53abb0b/fabed20 and client667e98f. Consumers import them, never copy bytes/crypto. Actual field,
error, lifetime, profile and ACK contracts are in owning SoT v0.3. Server profile
source is implemented; actual browser wire join is NOT_RUN. Initial profile rejects every Cookie,
preserves application Authorization and explicitly opts routes into browser RIT.
Required collector/TLS/H2 stay422 until actual evidence qualifies each plane.

Authority transport exact API: Listener::bind_ipc(endpoint,&Peer),
serve_handler_until(handler,Limits,shutdown_future,drain_timeout_ms). Handler
gets owned LocalRequest{method,target,headers,authorization,body} and actual
LocalContext{peer()/check()/remaining()}; returns bounded LocalResponse::json.
It owns no socket. Use context.check after durable work and before effects.
Explicit until/flush completion is required for graceful Stopped; drop/timeout
is not success. Runtime management credentials/allowlist/RBAC remain separate.

Audit supplies ControlAuthority/ActionPermit and CheckpointWitness/
WitnessServiceClient/Log::open_with_external_witness/flush. Use the real shared
issuer and transport, not synthetic booleans or a copied SealedAudit owner.
Shared audit-owned sealed Log+Custody lifetime owner is a later audit task.
rotate_approved currently binds the old ciphertext generation only, not a new
secret/destination; full destination-bound management rotation is unsupported.

Pending Cargo delta: add runtime dev-dependency
`humanymous-collector = { path = "../collector" }` to exercise the actual shared
client Session/ACK state machine against native TCP, not a copied algorithm.
This reuses an existing workspace package/version; root must refresh runtime's
Cargo.lock dependency list before --locked tests (root reports this is done).
No production dependency added.
New dependency/membership/feature requirements
will be recorded here before implementation; only root changes shared manifests
and lock. Collector/integrity remain root-owned. Native capture expansion is
narrowly authorized in future but not yet claimed/edited. All R01-R17 remaining
requirements and external ownership are tracked in the plan, including cookie
compatibility, typed adapter0.2 receipts, full H2, Pass, actual service credential
provider and whole-product human/AT/Docker gates. Product readiness remains false.

Source checkpoint entrypoints for hosted verification:

```
cargo test -p humanymous-runtime --locked --lib tests::browser
cargo test -p humanymous-runtime --locked
cargo clippy -p humanymous-runtime --all-targets --locked -- -D warnings
```

The browser module contains two authored fixture tests using the real
collector::session::Session against runtime native TCP/shared ACK and a separate
origin counter. Latest source execution is NOT_RUN. It checks bootstrap10fields,
no bootstrap ACK, CHALLENGE+ACK progress without origin, empty-sessionId report
projection, fixture ALLOW audit-before-origin, original application Authorization,
duplicate Set-Cookie, forged origin ACK stripping, replay and Cookie refusal.
Native helpers remain synthetic controls, not ordinary browser/human/AT success.
Root's actual BrowserSession API can connect to the new reserved bootstrap and
fetch collect/protected with matching exact origin/authority and caller scope.
required.collector remains422 pending fresh provenance, and there is no public
admin, servicehost, Pass or TLS/H2 support in this checkpoint. R15 and R05/R06
remaining work continue in later commits; this is not whole-plan completion.

Intermediate production-only cargo check passed before final report/test edits;
it is not validation of final bytes. Formatter and public-only spec check are
the only checks bound to this source checkpoint. No new local runtime/Docker/
browser execution was performed. Free hosted join/negative pilots belong to
CI/Compose/root; no paid resources are requested or created.

Owner: detection-core / rust-runtime-next. Status: source implemented and runtime
package integrated; full workspace/CI/Docker qualification belongs to root. No further source changes are
planned unless validation finds a defect.

Integrated lock delta: runtime now also directly depends on `socket2 = "0.6.5"`
(already in the current Tokio closure). Safe native MSG_PEEK is needed to detect
client FIN/RST after synchronous durable append; Tokio cached try_read missed
that abort in the regression test. The runtime dependency list is now reflected
in root's shared Cargo.lock. No new upstream version required.

Root added `crates/runtime` to workspace.members and updated Cargo.lock. No
shared manifest, lockfile, CI or lab file is edited by this lane. Package remains publish=false.
Dependencies reuse current workspace closure: authority, audit, integrity,
network, scoring, signals, serde, serde_json, sha2, hex, base64, getrandom,
zeroize and tokio 1.54.1 (net/io-util/time/rt-multi-thread/sync/macros).

The first native binary is explicitly loopback-only HTTP/1.1, complete bounded
body, exact route inventory and loopback owned origin. It owns volatile session
epochs, client-asserted report ingestion and RIT binding. No TLS/H2 capture,
Rust browser collector, browser recovery, admin or full product qualification.
Required absent planes reject. Docker product integration is requested only
after the native source/tests are ready; native fixture tests are separate.

## Interfaces and execution

- Binary `humanymous-runtime`: no args, bounded JSON through inherited stdin
  (close after writing), inherited piped stdout emits one readiness JSON line.
  Source of the exact JSON fixture is `tests/native_process.rs`. Audit paths
  must be canonical and owner-private; audit HMAC/signing keys are stable
  provisioned values, never automatically replaced on malformed recovery.
- POST `/_humanymous/session`: empty complete body, exact configured Host;
  returns fresh session/key/counter0/expiry. No browser cookie or admin authority.
- POST `/_humanymous/collect`: exact session-bound ClientReport JSON plus RIT.
  This is client-asserted ingestion. It does not qualify a browser collector.
- Configured protected exact method/path routes: RIT mandatory; query bytes,
  all ordered headers (except signature), body, tenant, installation, origin, revision,
  authority and deadline bind through `session::canonical`, with the shared
  integrity seed/token implementation. Consumers MUST call that API, not copy
  the binding algorithm. Counter increments on authenticated attempts even if
  later audit/collection/origin work fails. Seed counter is previous accepted n.
- Application Authorization is reserved for runtime capability and stripped
  before origin. A separate upstream Authorization header/stream/upgrade profile
  is unsupported; do not install this slice over an arbitrary existing app.
- All responses close; complete request body64KiB, origin response64KiB plus
  headers16KiB. No compressed body rewriting, TE/chunked/upgrade, duplicate
  request headers or encoded/dot-segment paths. Responses preserve Set-Cookie
  duplicates in their exact validated opaque origin frame.
- ALLOW is consumed directly by the original application call after durable
  Log.append and deadline/liveness checks. Errors/unsupported/CHALLENGE do not
  dispatch. Origin failures after request bytes may be ambiguous and never retry.
- TLS/H2 and browser collector required routes are422/unready. Pass, browser
  recovery, admin/dual-control actions, remote workload auth, origin bypass
  isolation, independent witness and full Core/Gate parity remain absent.

Commands with root's integrated runtime socket2 lock dependency:

```
cargo test -p humanymous-runtime --locked
cargo clippy -p humanymous-runtime --all-targets --locked -- -D warnings
rustfmt --edition 2024 --check crates/runtime/src/lib.rs crates/runtime/src/main.rs crates/runtime/tests/native_process.rs
```

## Measured and pending

Windows x64 Rust1.99.1 actual-source detached harness:12unit/native-TCP tests
plus1native child-process pilot,13passed/0failed/0ignored; Clippy-Dwarnings passed.
The native process pilot really runs session issue, collect, fixture ALLOW to
an independently counted origin, replay refusal, authenticated WAL restart,
credential invalidation and wrong-key startup refusal. The TCP abort test uses
an independent OS thread to deliver FIN during synchronous audit. Client report
fixtures with zero motor/typing richness do not claim a real human/browser ran.

Root had already added runtime membership and an earlier source snapshot passed
11unit+1process workspace tests. The final direct socket2 dependency subsequently
was integrated by root; runtime-only --locked workspace tests now pass13/13.
Raw logs/private harness metadata: evidence:root/20261107002939. Sanitized summary
and exact source commitments: evidence:root/20261107003506.

Docker product runner request: use actual binary and session::canonical with
complete literal request headers in final order; no handwritten JS or copied
token algorithm. This loopback-only listener needs client/runtime/owned-origin
in one isolated container/network namespace. Do not widen listen/origin binding
to public IPs to fit existing adapter lab topology. Docker executable is absent
from this chat's PowerShell PATH; no Docker success is claimed here. Root may use
its qualified Docker environment. Full browser/control outcomes remain not_run.

Native storage privacy follow-through: authority::transport::private_directory
now delegates Windows owner/DACL verification to audit::storage::permissions.
It queries the actual opened non-reparse handle and requires protected owner-only
DACLs, without PowerShell. Custody consumes the same native descriptor verifier
with its distinct inheritance requirement. No ACL repair occurs during startup.
Windows CLI still has separate inline script debt; this change does not claim
all authored product sources are Rust-only yet.

## Prepared control matrix

| Case | Expected observation | Execution |
| --- | --- | --- |
| Coherent ordinary Chrome + real collector/RIT | Actual human control, separate origin/audit counter | not_run; Rust collector absent |
| Keyboard-only with no mouse richness | Same policy path, no motor/speed condition | browser not_run; zero-motor fixture wiring passed |
| Real screen reader / AT | No required event speed/richness; record actual UX/recovery | not_run; browser recovery absent |
| Missing collector-required/TLS/H2 |422and origin0 | native unit passed; Docker pending |
| Body/target/method/tenant/revision/cookie/header-order mutation | Reject before origin | native binding tests passed |
| Replay/expired/aborted/audit unavailable | No protected origin dispatch | native/TCP tests passed; Docker pending |

Final-product readiness stays false. No scoring policy, threshold, band or rule
predicate was changed. No shared Cargo/CI/lab files were edited by this lane.

See sot:root/runtime-application and its implementation plan for acceptance.


## Root follow-through: sealed startup v0.2

The runtime now consumes audit.keys.directory and audit.keys.passphrase through
private inherited JSON. Inline hmac_key_hex/signing_seed_hex are rejected. Keys
must already exist in native sealed custody; startup never provisions or repairs
identity. SealedAudit owns both Log and lifetime Custody, so application code
cannot accidentally discard the key lock while retaining the log. Passphrase
buffers are consumed and zeroized before bind. CLI provisioning/rotation,
independent witness, service integration and full product qualification remain.
The exact updated fixture is tests/native_process.rs; that test preprovisions
synthetic sealed material, then validates real child-process restart and refusal.
Updated Windows native tests:13unit/TCP +1process,14passed. Root Docker process
pilot passes with sealed wrong-password/missing/tampered/concurrent-writer
refusals, actual origin2/audit4 and restart invalidation. Source/image commitments:
evidence:root/20261106235519, sealed-runtime-custody-results.json. Earlier source
snapshots above remain historical; new Cross-OS qualification is required.

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
- Explicit user goal set active for all R01-R17 on2026-11-07. Goal is incomplete;
  owned process/container count0. Next source tranches are lifecycle budgets and
  actual local management/dual-control joins, with external owners for neutral
  sealed audit startup, OS-protected unattended credentials, collector provenance
  and hosted product orchestration. No source-only checkpoint completes the goal.
- Lifecycle budget source and one pure unit fixture authored; compile/test NOT_RUN
  pending root's global Cargo slot. Added success fields drained/audit_flushed;
  unconsumed429/error writes now cancel on observed stop. Shared runtime APIs stay
  compatible. Current hosted42348518353/63f69ae cannot qualify these later edits.
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
R08 source adds per-session monotonic fixed windows with bounded validated config,
after next-token verification/consumption. Two pure window fixtures plus one
hosted-only native TCP/shared-client fixture now cover quota boundary/rollback,
invalid-proof non-debit, actual429 ACK progression, replay refusal and no origin
accept. This fixture is resource timing/protocol evidence, not human speed/motor
classification. Source compilation and all fixtures remain NOT_RUN; no local
native process or Cargo execution. Required missing collector/TLS/H2 still422.
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
R06 adds one pure in-memory AsyncRead/AsyncWrite flow-control fixture: stalled or
broken downstream after the head must stop further origin consumption within a
bounded read-ahead window rather than buffer the1MiB origin. The model uses no
sockets/OS processes and does not qualify actual network abort/drain behavior.
New fixture execution remains NOT_RUN. Actual deployed resource monitoring,
malformed socket, force-stop and independent custody/witness pilots remain open.
R15 run_report source now attempts final flush after returned drain failure and
returns typed bounded ExitFailure phase/facts; existing run keeps io::Result.
A pure phase/leak fixture covers all returned error combinations. Hosted Unix
controller-close fixture asserts actual drain failure plus successful final flush,
not Stopped. Both remain NOT_RUN. Blocking flush/unknown supervisor force-stop
cannot produce a receipt. No current-source Cargo/runtime/process execution.
R02/R05 origin-error ACK source now retains the signer in outer connection state
and signs pre-response origin failures with the actual local status. A hosted-only
shared-client/TCP fixture increments the independent origin-effect counter before
sending a malformed response, verifies503ACK/counter progress and replay refusal,
and asserts the effect count remains1 rather than claiming zero effects. Client
report/audit are explicit fixtures; not actual browser/durable product evidence.
New fixture execution and current compile/Clippy remain NOT_RUN. No local native
process/Cargo execution; no origin retry or partial-response second write.
R07 pure scoped OperationalSnapshot/PreparedSettings candidate source is authored.
Persisted payload decoding requires exact compact canonical bytes and16KiB budget;
CAS validates parent, exact scope/node, resource settings and safe revision bounds.
Two pure fixtures cover canonical bytes/independent SHA256 vector, frozen candidate,
stale parent, scope mutation, forbidden scoring fields, noncanonical input and
revision exhaustion. The initial commitment vector is
 a022446c62c485f1cc18cf7a51c7f32dc7a753c68a39a105fe3484c57b61e867
(system SHA256 calculation, not a passed Rust test). All new execution NOT_RUN.
Candidates grant no effects. Read-only settings endpoints remain unchanged;
actual snapshot store/audit commit/apply/session invalidation/restart recovery
must still be wired. Audit has proposed TrustedSnapshot native API; canonical
source checkpoint and qualification are awaited before adoption claims.
R07 source now uses actual audit TrustedSnapshot7dba59d:64KiB canonical semantic
envelope, explicit CreateNew/Existing, exact inspected native.pending recovery,
authenticated contiguous journal, candidate/WAL/finalize and opaque committed
receipt. Application consumes it to change resource fields only and clear live
sessions without reviving a poisoned clock/audit epoch. Shared global admission
rebuilds its resource limiter after a committed rate change. ServiceHost settings
bind restores before listener readiness and holds the native lock through drain/
flush. Operator HTTP/IPC mutation command path remains read-only until separately
wired; actual failure must poison the serving epoch. External-witness provenance
and current verified checkpoint coverage are mandatory; journal append flushes
before grant, and unwitnessed recovery-tail settings events refuse startup.

Two pure crash/journal fixtures and one hosted-only actual snapshot/WAL/custody/
same-process durable signer fixture are authored NOT_RUN. The latter stages and
commits, invalidates an old session, retains the signer's actual head through
writer restart, and restores revision1/resource values with no event replay. This
is not independent process/OS/organization qualification. A temporary shared
Witness key-holder derives only the synthetic writer public key; it never signs
or grants. The distinct durable signer performs actual signatures and fsync.

CLI d9ad5c7 observed runtime library dependency compile and Clippy PASS after4
runtime diagnostics were fixed. Its final9file manifest before/after is identical
SHA256d183d10df0f7b8297bb2a0251c87f95dc5b413c38cbf47ee18c58fafbd83642c.
No runtime unit/native test executed there. After the source freeze was released,
only hosted Unix fixture canonicalization changed among those9files; later root-
approved capture/challenge/rustls manifest edges are separate graph work. Do not
promote that dependency result to latest whole-runtime all-target qualification.

Root approved the manifest window: capture/challenge path edges and exact
rustls0.23.46 std/ring/tls12(defaultfalse) were written and frozen; root owns lock
resolution. No Cargo command, push or dispatch from this owner. Source8132117
actual run42355168729 is separate from these worktree changes. Its predecessor
42354873001/a26d startup_failure with0jobs/0tests remains a failed attempt.
Readiness source fix: service::standalone_profile(&Configuration) and ServiceHost
now share service/readiness.rs. Pure schema-default/managed-difference fixture and
hosted actual standalone-host parity assertion are authored NOT_RUN. Defender
owner consumes the full result and preserves exact identity/capability checks;
no subset fallback or duplicated expected JSON. Root graph6500ec23 resolves the
approved38member dependency window; this owner ran no Cargo. Exact813 run is live
and remains separate from this source fix. Operational Rust source will freeze
for root-assigned compile/pure validation after this checkpoint.
TLS/H1 consumer source WIP (NOT_RUN): `ServiceHost::bind_tls` accepts trusted
`Arc<rustls::ServerConfig>` plus existing optional custody policies, validates
native buffered/H1 scope before opening state, and joins its listener through
the same service shutdown path. `tls::CapturedRequest` owns PendingHandshake,
complete shared HTTP parser and exact Connection; worker cancellation revokes
and synchronously joins, including future drop. Original ClientHello enters the
shared network report/unchanged Engine, negotiated ALPN is actual metadata.
`check_h1_idle` before decision/post-audit/post-connect refuses observed closure
or trailing/pending input before origin bytes. This is point-in-time liveness,
not a guarantee against a later FIN. All collector/H2 prerequisites stay refused.
TLS serving is native buffered only; no browser/TLS, H2 or NativeStream claim.
Actual hosted handshake/body/origin, cancellation, post-audit abort and service
drain qualification remain required. No local actual OS/network test executed.

Historical frozen709b0e2 validation: Root-allocated A passed locked all-features
all-targets no-run, all-targets Clippy with warnings denied and27 scoped pure
units (0failed/0ignored). Recursive454-file before/after closure was identical,
SHA256fee08397a7379570b5f3efc08cab75cd4bb610c829d369e0831b58f75449765b.
This predates the TLS consumer WIP and is not its compile/test evidence.
`qualification-contract.json` owns required native component test inventories:
43 common library cases plus3 Unix cases, and native_process1. CI must verify
exact names/counts with exact-source --list and independently attempt both
harnesses; ignored/filtered/missing cases fail. This is source-authored inventory,
not executed coverage or full product/human/TLS qualification. CI consumes this
single data source so owner test additions cannot silently drift hardcoded totals.
Resource operator source WIP (NOT_RUN): `ServiceHost::bind_with_operator` requires
separate observer/operator Credentials and actual externally witnessed durable
settings. Same-credential or missing-premise profiles refuse before custody.
Native POST `/management/v0.1/settings/commit` accepts only exact scope, expected
revision and bounded OperationalSettings; existing header/CSRF/native-peer checks
apply. Defaults remain read-only. No scoring or destructive control route exists.

A queue of capacity1 carries the actual LocalContext into the serialized app
accept loop. Commands run between requests, recheck native deadline/cancellation
and the service stop watch throughout durable commit and before applying, then
invalidate sessions/update resources and current management profile. CAS failures
are non-mutating. Ambiguity poisons and terminates serving; no retry or successful
receipt. Receiver scope drops queued contexts before management drain joins;
Application holds store custody through final host audit flush. Both plain/TLS
loops use this owner. Operator role cannot use observer reads.

New pure role/scope/header/no-scoring negative and Unix actual IPC/witness/CAS/
capacity/invalidation/restart cases are authored only. The actual case verifies
capacity32 admits32 fresh sessions then rejects the next, not only JSON metadata.
The witness is a real same-process durable component fixture, not independently
deployed. Source inventory is now44 common plus4 Unix library cases (48Unix/
44Windows), plus native_process1. No execution occurred in benchmark quiet window.
Actual cancellation/crash/failure-matrix and Windows operator pilots remain open.
Historical exact813 hosted component evidence recovered: run42355168729 ended
FAILURE; its successful MSRV job128359709010 ran runtime library41 and native
process1, zero failed/ignored/filtered. Native TCP ACK/stream/quota and Unix
observer management drain/controller-close passed in that predecessor. Command
was workspace default-feature all-targets tests on Ubuntu24.04/Rust1.99.1, not an
independent current all-features runtime gate. Runtime tree at813 is
f7c85ec9f75c2b56601c15ca6667832be04b331b. It predates current witnessed settings,
readiness builder, TLS and resource operator. No browser/human/full-product or
current-source qualification is inferred. Sanitized exact names/counts attached
in evidence:root/20261107061821; raw CLI capture stays private in20261107061820.
R10 target data source (NOT_RUN): `control::KillTarget` derives immutable
installation/node scope and fixed compact JSON protocol/install/node/state:killed
from trusted configuration, then uses audit's shared target_commitment. It
creates no authorization or kill state. Policy revision changes keep the target;
new installation/node requires explicit migration. One independent106byte vector
and scope/revision fixture authored. No kill, ban or erasure effect is enabled.

Audit owner requires fresh exact externally retained controlledger proof before
kill admission/restart: covered kill restores disabled; consumed retained kill
without covered effect is unavailable/pending, not healthy or automatic replay.
Existing witness exact-head check rejects new/rolled-back local WAL after signer
advanced, but cached broker snapshots/joint rollback are not independence or a
lease. Actual permanent kill waits for the shared RetainedControlState guard.
Source library inventory now45common+4Unix=49Unix/45Windows; native_process1.
R10 assessment source WIP additionally consumes actual `RetainedControlState`,
pinned authority and `SealedAudit` to join retained intent with covered effect.
It rejects other consumed targets (future protocol changes cannot rearm), foreign
bindings, missing external coverage or effect-without-intent contradiction.
Private nonClone/nondeserializable KillStatus retains scope/target/pin/anchor/
audit-extent; data is not a grant or a lease. Actual ServiceHost control issuer,
startup/admission serialization and retained-permit-only effect remain unwired.
Pure classification matrix authored NOT_RUN; inventory50Unix/46Windows/native1.
Required typed control query/Scope API producer is audit9c47f8e4dfb8dcfe0dee4e3359bb3c5e573050d5.
Producer and this consumer remain source-only until post-quiet allocated validation.
