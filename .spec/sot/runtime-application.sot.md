---
id: sot:root/runtime-application
title: Runtime application and session owner
status: active
created: 2026-11-07T00:20:25Z
updated: 2026-11-07T06:18:20Z
verify: [cargo test -p humanymous-runtime --locked, cargo clippy -p humanymous-runtime --all-targets --locked -- -D warnings]
rel:
  governs: [crates/runtime/**]
  depends_on: [sot:root/rust-migration, sot:root/adapter-authority, sot:root/server-adapter-expansion]
---

# Runtime application and session owner

## Rule

### Operational follow-through v0.3: accepted contracts, not qualification

This section supersedes v0.1 only for explicitly selected new profiles. Existing
native whole-header RIT remains byte-identical. Accepted means implementation is
authorized; implemented, source-validated, actual browser and whole-product gates
remain separate. Follow-through is tracked in plan:root/20261107061820.

Runtime owns application/session, ingress-to-origin enforcement, recovery and
management handlers. Root owns shared Cargo/lock/toolchain, collector and the
shared integrity browser codec. Authority owns authenticated native IPC and
adapter wire contracts; audit owns cryptography, durable control permits,
custody, witness and storage. Reuse these owners, never copy their algorithms.

#### Browser binding and capability boundaries

Browser Fetch cannot predict forbidden Host/Content-Length/header order. Add an
explicit `humanymous.browser/0.1` profile rather than weakening native0.1. Its
route configuration must opt in; existing routes default to native. Store the
profile in the issued session and reject unknown/mixed/profile-mismatched input.
Signature validity does not qualify original TLS/H2, code execution, collector
coverage or a human. Required absent planes remain422, including collector until
the separate fresh-provenance contract and actual integration qualify it.

Configuration adds optional `browser:{origin:string}` and each route adds
`rit_profile` with values `native_full` (default) or `browser_auth_only`.
The initial plaintext listener accepts only origin=`http://` plus exact configured
authority, without trailing slash/path/query. Browser routes require the explicit
browser configuration; this does not expand loopback bind scope.

Use `humanymous_integrity::request::browser::{Binding, canonical, PROTOCOL,
MAX_SAFE_INTEGER}` from root commit53abb0b and ACK API from fabed20. Binding has
exact fields installation,tenant,origin_identity,revision,authority,session,
method,target,counter,bucket,deadline,content_type,application_authorization,
body:&[u8]. Canonical bytes start `humanymous.browser/0.1\n`; fourteen fields
follow as decimal UTF-8 byte length, colon, value, newline. The last field is
lowercase SHA256 of the exact buffered body, not a reserialized object. The final
newline is mandatory; no additional field or empty line exists. The shared codec
owns validation, formatting and hashes; consumers call it directly.

Counter/bucket/deadline are canonical decimals within JavaScript's exact integer
domain0..9007199254740991; request counter begins1. Scope IDs are bounded128,
authority1024, canonical32-byte base64url session43, target8192. Content-Type
and explicit application Authorization are final Request values with only outer
ASCII space/tab trimmed; internal spaces are preserved and CTL/non-ASCII values
reject. Limits are1024/8192 respectively. The shared codec's body ceiling1MiB
does not raise this runtime profile's64KiB ingress ceiling. GET/HEAD have no body.
The runtime may accept missing Content-Length only for explicit browser GET/HEAD
with zero body; TE, duplicate framing and ambiguity still reject.

POST `/_humanymous/browser/session` accepts an empty JSON object, exact configured
Host/Origin and same-origin Fetch Metadata. It returns exactly ten JSON fields:
protocol,session,key,counter(0),expires,installation,tenant,origin,revision,
authority. Scope is server provisioned, expiry bounded and keys fresh; no arbitrary
client identity or public administration authority is minted. Browser control
and protected requests carry X-Humanymous-Protocol, X-Humanymous-Session,
X-Humanymous-Counter, X-Humanymous-Bucket, X-Humanymous-Deadline,
X-Humanymous-Token. Application Authorization is separate and preserved to origin.
The unique bootstrap route selects the protocol before credentials exist: its
protocol marker may be absent, but if present must equal the browser protocol.
Bootstrap rejects Authorization and X-Humanymous-Session fields. This matches
root's actual BrowserSession::connect content-type/empty-object request.

Initial browser credential support is explicitly cookie-free / explicit
Authorization-only (including unauthenticated application routes where allowed).
Reject every observed Cookie field. Clients use credentials=omit. A bootstrap at
the reserved path cannot observe cookies scoped to a protected application path;
it must not pretend to bind them. Per-target prepare or trusted application
identity binding for HttpOnly/path-scoped cookies is a mandatory later contract,
not removed from the overall objective. No global-cookie-digest shortcut exists.

Bootstrap/collect/mutating requests require exact configured non-null Origin.
Same-origin GET/HEAD may omit Origin only with an authenticated browser-profile
session and same-origin Fetch Metadata. Whenever supplied, null/mismatching
Origin rejects. Metadata requires site=same-origin, mode=same-origin,dest=empty.
Clients construct the final Request with mode=same-origin,credentials=omit,
redirect=error, explicit buffered bytes and no automatic retry. Unknown
normalization/encoded-path/stream profiles remain unsupported, never downgraded.

Use shared seed(session_key,last_counter) and compute_token/verify over the
browser canonical bytes. Runtime checks exact next counter, current ten-second
bucket plus/minus1, deadline>now and <=session expiry and <=now+configured timeout
(1..30000ms). Reserve counter before downstream work, never resurrect after
error. Clock rollback poisons the epoch; restart invalidates every session.
Client owns at most one in-flight request; uncertain outcome invalidates keys.

#### Counter acknowledgement

Only after consuming RIT may the gateway add X-Humanymous-Protocol=the browser
protocol, X-Humanymous-Counter=n and X-Humanymous-Ack. Use shared
`browser::acknowledgement(key,request_token,n,bucket,status)`; verification uses
`verify_acknowledgement` with the same fields and presented MAC. The shared
implementation signs `humanymous.browser/0.1/ack\n` plus length-prefixed request
MAC/newline and decimal actual HTTP status/newline, using seed(key,n) then the
unchanged compute_token with n and bucket. Do not duplicate the implementation.

Strip all origin-supplied x-humanymous-* response fields and emit gateway-owned
values only. ACK binds consumption and actual status, not ALLOW, response body,
collector, capture or human identity. Post-consumption4xx/5xx may carry ACK;
pre-consumption rejection never does. An audit-poisoned epoch stays fail-closed
even if an ACK proves its counter was spent. Client advances only after complete
bounded body reception and a valid request-token/n/bucket/status-bound ACK.
Missing/invalid ACK, timeout, abort, drop, redirect or body/Fetch failure invalidates
the client epoch without guessing counter state or retrying. Initial browser
responses are buffered; streaming browser ACK is a separate unsupported profile.

#### Collection and adapter join

For the new browser collect route only, an omitted/empty ClientReport.sessionId
is assigned from the already authenticated session capability. A nonempty
mismatching ID rejects403 after consuming the signed attempt. The request MAC
still covers the original received bytes, not the server projection. Native0.1
retains strict exact report/session matching. This lets root's BrowserSession
keep capabilities private while sending actual snapshot_json output. It does
not authenticate producer execution or qualify required.collector.

A fresh collection receipt must distinguish server-issued epoch binding and
server-observed receive time/consumed counter from client-asserted version,
collection epoch and measured coverage. Root owns its collector envelope and
actual Rust/WASM lifecycle. Default partial guard/advanced observations do not
become complete merely because they were signed. Until that accepted contract is
implemented and exercised, required.collector stays422. Native/report assertions
remain distinct from actual transport observations and VerifiedRitProfile.

Adapter0.1 strips reserved integrity headers and has null-only capture/stream
fields. Therefore its required.rit remains422. A future adapter0.2 or trusted
in-process join must bind a one-use runtime-owned receipt to authenticated adapter
credential epoch/peer, installation/tenant/origin/revision, request nonce,
connection/optional H2 stream, exact method/target/authority, ordered sanitized
headers and complete-body digest. IPC booleans never create evidence authority.
Runtime is the sole session/replay owner; authority keeps its own envelope nonce.

#### Proxy and recovery

One response-head parser must serve buffered and explicitly bounded streaming
profiles. Preserve duplicate Set-Cookie, application Authorization, entity bytes,
status, headers and HEAD/204/304 semantics. Reject ambiguous CL/TE, duplicate
framing, malformed chunks/CRLF/terminal framing, unsupported coding/trailers,
upgrades and over-budget frames. Strip hop-by-hop and nominated fields and rebuild
only required framing. Track response_started before any client write: after a
partial response, close sockets on error; never append a new JSON503 response.
Complete request integrity and durable audit precede the first origin byte.
Response streaming does not authorize request streaming, retries or failover.
Explicit byte/lifetime budgets and backpressure are required; SSE must flush
before origin close. Failure after dispatch can have an origin effect and is not
reported as a zero-effect rejected precondition.

Pass recovery owns scene/instance/session/nonce/expiry/replay state and uses the
shared challenge proof-of-work/attestation primitives and frozen Engine. Porting
the legacy realEventOK motor-count/path-length/timing/coalescing gates is forbidden
by current accessibility constraints. Do not add its soft timing signals under
scoring freeze, train on scene success as confirmed human, or claim a puzzle is
human attestation. Only the server may mint l7.pass.solved after exact scoped
instance, crypto, required attestation and durable audit; consume atomically.
The shared Engine may upgrade only the existing score-based CHALLENGE path,
never DENY or hard-rule CHALLENGE. Missing stable observed identity is not
replaced with a client fingerprint/header for an identity-budget claim.

#### Management, settings, service and witness

Management consumes authority e346834/e8ddb4c shared transport. Use
Listener::bind_ipc(endpoint,&Peer), LocalHandler::handle(LocalRequest,LocalContext),
LocalResponse::json and explicit serve_handler_until(handler,Limits,shutdown,
drain_timeout_ms). Request includes method,target,ordered headers,authorization,
body; context.peer is actual OS identity and context.check returns trusted Unix
millis only while cancellation/monotonic/rollback conditions pass. Recheck after
durable work and immediately before effect. Separate listener/credentials,
route/RBAC/Origin/CSRF/scope policies are mandatory; session/adapter credentials
never administer. Bounded JSON65KiB only; no handler-owned socket or forwarding.

Explicit until stops accepts, globally cancels, aborts/joins tracked connections
and waits for all admission-owning contexts/workers. TimedOut is incomplete
shutdown, not graceful success; drop alone gives no drain acknowledgement.
Runtime creates no detached worker escape, retains custody through all work,
and reports graceful Stopped only after drain and durable flush.

Audit d001ee6/5528d74 supplies ControlAuthority and non-Clone/non-deserializable
ActionPermit, scoped Proposal/Approval/Administrators, durable one-use ledger and
subject-bound erasure permits. Two distinct pinned signatures must bind exact
scope/action/target/expiry; permit.execute consumes ownership and rechecks before
effect. Never trust approver names from a body. Permanent bans/kill/erasure remain
dual-control. rotate_approved currently approves the current ciphertext generation,
not a particular new secret/prepared ciphertext: it is not full destination/body-
bound rotation. Require a future prepared-candidate flow or reject that profile.

Operational settings must be typed/bounded, parent-revision CAS, audit-before-
apply, persisted/restored and invalidate old sessions. Frozen scoring/rule/weight/
threshold/band changes are not exposed by this write API. Without durable state,
support only read-only settings and validated restart configuration. Prefer
authenticated audit events as authoritative operational state over pretending
two separate stores are atomic. Partial persistence or ambiguous effects poison
the epoch and need explicit recovery. Stable-identity abuse/ban claims require
an actual producer; global/session admission429 is not fingerprint flood defense.

ServiceHost is an OS-independent lifecycle owner. CLI owns SCM/system-manager
signals, signed descriptor/artifact/generation verification and credential broker;
runtime owns bounded configuration decoding, native private-state checks, sealed
startup, listeners, typed readiness and stop/drain/flush outcome. Accept an owned
private reader/validated opened handle, never secret argv/env or raw numeric
handle reconstruction. Preserve existing argv0/private stdin/stdout supervision.
Readiness separates listener-bound, route capability, audit/witness state,
non-secret policy commitment and product_qualified=false. Stop accepts, prohibit
new origin dispatch after observed stop, drain, flush and report actual outcome.
Synchronous Read/fsync/witness I/O cannot be made hard-bounded by a watch channel;
trusted broker owns actual I/O deadlines and child reaping. Uninterruptible I/O
or drain errors never become successful Stopped; already sent effects are
irreversible. Unattended restart remains unqualified without a boot credential
provider. Do not reuse installer privilege admission as service execution auth.

Inject trusted CheckpointWitness and required WitnessDurability into the library
and call audit Log::open_with_external_witness. Reuse audit WitnessServiceClient
codec; broker owns peer authentication, actual deadlines and process lifetime.
Missing configured transport refuses required-witness startup, never writer-only
fallback. Same-user/co-located service is not independently qualified authority.
Shared audit-owned Log+Custody lifetime owner and streaming-memory-bounded restart
remain audit follow-ups; do not copy runtime SealedAudit into another crate.

Future native capture ownership is narrowly authorized after claim and binding
agreement: network/src/capture/**, one lib export and producer tests only. Leave
existing tolerant parser/scoring/report heuristics unchanged. Qualify actual TLS
only after bounded complete ClientHello observation plus successful handshake on
the same connection epoch. H2 requires original SETTINGS/WU/PRIORITY and complete
HEADERS/CONTINUATION/HPACK ordering with per-stream association before framework
normalization. Partial/error/absent capture stays absent; H1 success is not fullH2.
Use owned synthetic lab credentials, no personal keys or third-party probes.

Browser data errors:400malformed,401session,403Origin/binding/token,
409profile/revision/replay,422required absent plane,429admission,
503clock/audit/state. Management authentication has its own non-discoverable
allowlist/status policy; it does not inherit data-plane authority.

Review reflected: red P0 browser forbidden headers, scoped-cookie gap, missing
Origin/CL GET, mixed profiles and ACK ambiguity; blue one owner per primitive;
SRE partial writes/drain/custody and synchronous I/O; DPO no raw audit or secret
canonical diagnostics; evaluator no component/fixture/CHALLENGE-to-human success.
Verdict: accepted for implementation; full product qualification still blocked.

Version 0.1 is an experimental native loopback HTTP/1.1 application owner, not
full Core/Gate parity. Configuration arrives over private inherited stdin;
readiness goes to private piped stdout. Bind only explicit loopback IP addresses
for listener and owned origin. There is no public administration endpoint.
Do not widen to public plaintext listeners before a separate TLS/capture contract.

One process owns installation/tenant/authority/revision, exact method/path route
inventory, bounded volatile sessions, shared frozen Engine and durable audit Log.
Restart invalidates every session. Issue random 256-bit session capability and
independent random integrity key at POST /_humanymous/session (empty body).
Do not log either. Admission is bounded before issue; sessions have a fixed TTL,
capacity and no live eviction. Clock rollback poisons the epoch.

POST /_humanymous/collect ingests a bounded shared ClientReport using the session
capability and a rotating token for its exact body. It is client-asserted evidence,
not an authenticated browser collector. Only its client fields are merged; the
client cannot supply server/network/scoring fields. Session ID must match the
capability. Raw report/body/headers are volatile and never included in audit.
Reject unknown/duplicate/non-L1-L4 signal IDs, server collection claims, invalid
verdict/confidence; rebuild layer/weight/score/expected metadata using the shared
registry constructor, never client-supplied numeric scoring fields.
There is no browser collector or Pass recovery shipped by this slice.

Every collect/protected request requires Authorization: Bearer <session>,
X-Humanymous-Counter, X-Humanymous-Bucket, X-Humanymous-Deadline and
X-Humanymous-Token. Counter starts at 1. Use existing integrity::request seed,
canonical, compute_token and verify without changing their frozen byte protocol.
The application-specific signed text is domain `humanymous.runtime/0.1\n`,
then length-prefixed UTF-8 installation, tenant, origin identity, revision, authority, method,
exact origin-form target, decimal deadline (each `byte_length:value\n`), then
every observed header name and value in wire order/casing using the same
length-prefixing, excluding only X-Humanymous-Token, then one empty line and
the existing canonical headers/session/body. Application cookies and all other
forwarded fields are therefore bound too. This is a new application binding,
not a relabelled Node adapter protocol. Key provisioning returns key and initial
counter0 only to the initiating loopback client. No bootstrap token grace.
Counter advances atomically after valid authentication, before collection,
scoring/audit/origin. Valid authenticated attempts remain consumed on later error.
No automatic retry, cache or failover after ambiguous origin effects.

HTTP framing is strict bounded HTTP/1.1 with one request/connection and close:
16KiB headers, 100 unique case-insensitive headers, 64KiB body, mandatory canonical
Content-Length (including zero), exact Host. Reject TE/upgrade/Expect/trailers,
obs-fold, control/non-ASCII header values, duplicate headers, absolute targets,
percent/backslash/dot-segment paths and reserved unknown routes. Query remains
part of the integrity binding. Only fixed GET/POST/PUT/PATCH/DELETE/HEAD methods.
No stream, multipart reconstruction, decompression or response transformation.

Observe native HTTP version/header order/casing from the actual socket. Header
values never manufacture client evidence. TLS/H2 are absent on this listener;
required raw TLS, original H2 or browser collector returns422. RIT-required routes
are supported by the session owner, without claiming resistance to a coherent
T4 browser/client. All protected routes require RIT in this first profile.
Route requirements and active policy revision are server configuration, never
client assertions. Use shared network::report and scoring::Engine exactly once.
Only ALLOW can connect to the configured owned loopback origin, after successful
durable audit and a repeated deadline check. CHALLENGE is a blocked decision,
not human success. DENY/errors/unsupported/expired/truncated requests have zero
origin calls. Audit failure poisons the epoch. Audit stores minimized scope,
opaque request ID, revision, verdict and commitment; never raw report or tokens.

Forward complete observed body and safe end-to-end headers, stripping runtime
credentials and hop-by-hop framing, rebuilding Host/Content-Length/Connection.
Origin response is a bounded opaque HTTP/1.1 frame returned once, preserving
duplicate Set-Cookie. Reject ambiguous response framing and unsupported upgrades
or transfer encoding; an origin error can already have an effect and is never
retried. Socket deadlines bound reads/writes; synchronous disk append may exceed
the budget, but deadline is checked before origin. Cancellation after origin
dispatch cannot undo an origin effect. Independent origin bypass denial and
TLS, process isolation, real-browser/keyboard/AT controls and Docker product
e2e are later mandatory qualification gates. No speed/motor requirement exists.

### Review reflected

Blue: reuse one engine/integrity/network/audit implementation. Red P0: method,
target, tenant, body, policy and deadline binding; no first-request grace; strict
framing and replay reservation. SRE P1: loopback-only plaintext scope, bounded
admission/storage/deadlines, audit poison and no retry. DPO P1: no raw audit;
session TTL and volatile storage. Evaluator P0: client assertion is not browser
collector; fixture ALLOW is not browser/human success. Residual T4 and independent
witness/antirollback custody are explicit. Review verdict: ready for bounded
implementation; blocked for full product qualification.

## Why

The existing local adapter authority has no application session owner. This
slice adds a real listener/session/integrity/collection/enforcement composition
without pretending to implement absent raw planes or introducing a detector.

## Verification

### Sealed audit startup integration, configuration v0.2

Runtime startup accepts only an explicitly preprovisioned sealed custody source;
inline HMAC/seed fields are rejected. A private inherited passphrase is consumed
into a zeroizing buffer and cleared from application configuration before bind.
Acquire and retain exclusive custody for the entire application lifetime, reuse
the authenticated writer identity for the existing audit Log, and never mint
replacement keys when storage is absent, pending, tampered or wrongly unlocked.
Provisioning and rotation remain supervisor/CLI operations, not HTTP endpoints.
Both custody and audit directories must pass native owner checks. No implicit
recovery, independent-witness claim or erasure permit is added. Existing tests
using a mock audit do not qualify this startup; the real process Docker pilot
must preprovision a synthetic sealed store, prove restart identity and test
wrong-password, missing-store and competing-writer startup refusal.

Unit and native TCP tests must prove scope/method/target/body/replay/deadline,
collection session isolation, absent-plane refusal, capacity/clock/audit poison,
audit-before-origin and truncated request no-origin using real origin counters.
Use explicit fixture control reports only for bounded ALLOW wiring. Prepare
ordinary-browser/keyboard/AT cases as not_run until Rust browser collection exists.
Docker product e2e is the completion authority; native tests are not that gate.

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
R15 failure accounting: attempt the final audit flush after all owned listener
work has ended even if drain returned an error. Never convert either failure to
Stopped. service::ExitFailure reports bounded Drain/AuditFlush/DrainAndAuditFlush
stage and independently observed drained/audit_flushed facts, without raw paths
or secret-bearing source chains. ServiceHost::run_report returns that typed
outcome; existing run preserves io::Result compatibility by wrapping ExitFailure.
Synchronous flush remains supervisor-bounded, not watch-preemptible. An unreturned
flush/force-stop is unknown/incomplete, not an inferred successful flush.
R02/R05 post-consumption origin errors: keep the consumed browser ACK signer in
the per-connection response state until dispatch/write ends, not only inside the
successful decision branch. If origin connect/read/framing fails before any client
response byte, the local error response carries the consumed counter ACK bound
to its actual status. Pre-consumption errors/admission remain unsigned. After any
response byte has started, close without a second response. Deadline/abort/drop
still invalidates the client epoch if no complete ACK response can be delivered.
A503 ACK proves counter consumption only; an origin already dispatched may have
acted. Never infer zero effects or automatically retry from that ACK.
R07 typed CAS candidate model: OperationalSnapshot serializes fixed struct fields
in order protocol,scope,node,revision,settings with protocol
humanymous.operational/0.1. Scope reuses the exact five-field ManagementScope;
node is the configured audit node. Revision is an integer0..MAX_SAFE_INTEGER.
Snapshot decoding is bounded16KiB, strict unknown-field rejection, exact current
scope/node/policy revision and the shared resource-settings validator. A candidate
requires exact parent revision and checked revision+1; no scope mutation can be
smuggled through desired settings. Commitment is lowercase SHA256 of UTF8 domain
humanymous/runtime/operational-state/0.1 followed by one NUL byte then exact compact
serde struct JSON bytes. PreparedSettings is not Deserialize/Clone and freezes
those bytes plus parent/new commitments; it is a candidate, never an action permit
or durable commit receipt. Actual private store, authenticated journal commit,
apply/session invalidation and crash recovery remain required before exposing
writes. No scoring fields, secret credentials or implicit policy migration exist.
R07 durable pipeline: SettingsStore owns audit TrustedSnapshot(namespace
runtime-settings,max64KiB) and stores a canonical semantic envelope with committed
and candidate OperationalSnapshots. Native .pending is a separate interrupted
atomic replacement. A dedicated runtime.settings.commit/v1 event uses request_id
settings:<canonical revision>, correlation_id exact candidate commitment,
tenant_id and config_version exact provisioned scope; audit stamps node/sequence.
The authenticated journal must start0 and advance contiguously, with canonical
32-byte hex commitments, exact tenant/node/policy binding. No values are hidden
in reason/risk fields. CreateNew is explicit and only when file+journal absent.
Pipeline: stage candidate+parent durably, audit commit, finalize private envelope,
then issue nonserializable CommittedSettings. Errors after staging poison the
owner; no retry or successful receipt. The actual caller must poison the serving
epoch on an ambiguous failure. Startup selects only the WAL-committed snapshot:
finalize audited candidate or discard only an uncommitted semantic candidate
whose parent still matches WAL. Missing/mismatched/rolled-back state refuses.
Explicit native pending recovery validates both typed envelopes/current-or-parent
journal binding, promotes exact inspected bytes under lock, then runs the same
semantic recovery. Default requires native pending absent. Policy requests native
durability explicitly; file-sync never implies directory durability. This source
uses authenticated SealedAudit records and a shared native store, not arbitrary
HTTP evidence. Operator route/apply/session invalidation/hosted failure pilots
remain separate required joins, with all writes refused until wired.
R15 shared readiness identity: service::standalone_profile(&Configuration) is a
pure expected full host-profile builder for the standalone executable (no witness,
management, recovery or persisted settings). ServiceHost uses the same builder
with actual owned startup facts. Defender/lab must compare the complete result,
not drop strict identity/scope checks or duplicate optional-field defaults.
Configuration::readiness remains a capability/configuration view, not the full
host manifest. Expected profile construction is not listener/producer attestation.
Source813 Docker receipt26=0evaluation/19errors remains unchanged; readiness-field
mismatch is a source-proven incompatibility, not a recovered raw failure cause.
R13 joined ingress ownership: runtime::tls::CapturedRequest consumes a shared
PendingHandshake, receives exactly one bounded complete HTTP/1.1 request using
the existing http::read parser, and retains that same Connection with the request.
Only absent or negotiated http/1.1 ALPN is accepted; H2 is unsupported here.
All blocking TLS reads/writes execute on an owned worker. Future cancellation
revokes its same socket and synchronously joins before releasing the owner;
normal completion joins before returning observations or reporting a write.
The absolute capture lifetime includes queue time and is never reset. A dropped
consumer or failed response has no retry path. This component does not enable
required.raw_tls or establish remote liveness after synchronous audit. Actual
same-owner idle checks, request evidence/engine join, ServiceHost TLS profile,
origin dispatch and hosted cancellation pilots remain mandatory before serving
claims. External process watchdog remains responsible for ultimate shutdown.
R13 TLS/H1 consumer extension: ServiceHost::bind_tls explicitly receives the
trusted ServerConfig and optional existing management/recovery/settings policies.
It rejects browser or NativeStream routes and non-H1 ALPN before opening custody.
The same serialized Application consumes the exact CapturedRequest, obtains
ClientHello only from its retained Connection, and passes it to the shared
network report/unchanged Engine. Actual negotiated ALPN replaces the report's
offered-default metadata. check_h1_idle is required before decision, after durable
audit and after origin connect immediately before sending application bytes.
Expiry/controller checks remain independent; upstream read uses the minimum
RIT/connection deadline. The native TLS profile advertises raw-TLS capability
only for this injected transport; product_qualified remains false and required
collector/H2 remain fail-closed. Plaintext profile still refuses raw-TLS routes.
This source contract needs hosted TLS/body/abort/cancel/drain/origin evidence;
configuration readiness must not be reported as actual runtime qualification.
R17 native component inventory: crates/runtime/qualification-contract.json is the
single owner-maintained data contract for required library/integration targets,
features, exact common/Unix-only case names and per-OS counts. CI must compare
exact-source --list inventory against it, attempt both targets even if one fails,
and retain zero ignored/filtered requirements. Source-authored inventory is not
execution evidence and mismatches fail rather than adapting expected coverage.
This inventory does not grant full product/browser/human/TLS qualification.
R07 native resource operator profile: ServiceHost::bind_with_operator requires
existing ManagementPolicy, a distinct OperatorPolicy credential, durable
SettingsPolicy and pinned external witness before opening custody. Both roles
use the same configured actual native peer, but independent constant-time checked
credentials; neither session nor observer capability can write. Identical role
credentials refuse before state opens. POST /management/v0.1/settings/commit
accepts only {scope,expected_revision,settings}, exact five-field scope and the
existing bounded native-only header envelope. Unknown fields/scoring values,
foreign scope, unsafe revision and wrong method/target refuse.

A bounded one-command queue carries the actual LocalContext to the serialized
Application, between complete application requests. Recheck context/epoch before
preparing CAS, throughout stage/audit/witness/finalize and immediately before
apply. Store commit receipt alone permits resource application; invalidate old
sessions, apply fields and update the management profile before returning the
nonsecret snapshot. CAS/precondition rejection does not stage a mutation.
Ambiguous post-preparation persistence/application failure poisons the serving
epoch and terminates its listener, with final audit flush attempted. No automatic
retry or successful receipt after expired/cancelled context. A dropped response
can leave a durably applied change; the parent revision remains consumed.

The accept-loop scope owns the command receiver, so shutdown drops queued native
contexts before management drain joins. Application retains SettingsStore custody
through the final host flush. Observer views use the shared current profile;
operator credential cannot read observer routes. Ordinary host constructors stay
read-only. This profile never authorizes scoring, kill, erasure, rotation or bans;
those destructive effects still require retained dual-control permits. Actual
IPC/witness/CAS/restart/cancellation/crash qualification remains required.
The command rechecks the service stop watch as well as LocalContext at every
persistence/application boundary and enforces nondecreasing trusted wall time
within the command. Session epoch observation is repeated before/after apply;
resource mutation cannot revive a poisoned/rolled-back epoch. Observer settings
responses remain writable=false and separately indicate operator_available;
role availability never changes the observer's permission.
R10 permanent node kill target: fixed UTF-8 compact serde field order
{protocol,installation,node,state}, protocol=humanymous.runtime-kill/0.1 and
state=killed, no trailing newline. Installation/node derive from trusted startup
configuration; shared audit::control::target_commitment owns hashing/framing.
Policy revision/tenant/origin metadata do not change this node-level target;
installation/node migration is an explicit separately authorized operation.
Target data is not authorization, state, a retained grant or effect completion.
The fixture install/runtime-test payload is106bytes and shared commitment is
b5e9469977ea5654078b437e756878919f4ca58e5b127e450179b226b8cf2cef.

Permanent kill admission/restart must consume fresh authenticated controlledger
state bound to exact independent latest retained anchor, issuer public identity,
control scope/action/target. A covered authenticated kill restores killed;
retained consumed kill without covered effect is unavailable/pending, never
healthy and never generic permit replay. Missing/mismatched evidence refuses.
Expired grants or policy revision changes cannot rearm. Established state uses
existing identity/allow_new=false and fresh brokered witness handshake/current
pin: cached witness Ready/Signed snapshots are not a distributed lease and cannot
resist joint rollback/reprovision/compromised broker. Hold actual kill mutation
until the audit-owned non-deserializable RetainedControlState guard is wired.
R10 assessment source seam: KillStatus::assess consumes the audit-owned
RetainedControlState, trusted pinned authority public identity and actual
SealedAudit. It rejects foreign scope/action/target/public identity, absent
external audit coverage, or any other consumed target for KillSwitch in that
scope; a future target protocol cannot silently forget earlier intent. Reuse
Scope::validate for control identities; do not duplicate its grammar.

runtime.kill.engaged/v1 effect records bind stamped node, request_id
kill:<canonical nonzero32byte operation hex>, correlation_id stable target digest,
verdict KILLED and enforcement_action/mode/fail_mode kill_switch/enforce/fail_closed.
Only independently covered records establish the effect. No intent/no effect
classifies Clear, retained intent/covered effect Killed, retained intent with no
or ambiguous effect Pending. Effect without retained intent or malformed binding
refuses. KillStatus has private fields/noClone/Deserialize and retains exact
scope/target/public/control-anchor/audit-extent bindings; it is not a grant or a
freshness lease. Actual ServiceHost admission/issuer serialization and effect
consumer are still required; source assessment alone enables no kill endpoint.
The runtime.kill.* event namespace is reserved for permanent effect history; an
unknown event/version in this namespace refuses rather than becoming absence.

### Observer/broker-loss v0.1 — semantics accepted, typed API pending

Root accepted R2 architecture and R3 loss semantics, not full channel/native
backend/typed API acceptance, implementation or qualification. This delta
answers plan:root/20261107102000 lifecycle step8 under topology471058e9: transient
provisioner P, persistent observer O=bootstrap broker/receiver, independent native
service S. Authority owns shared exchange/lifetime types; native-security owns
actual principal/process-instance proof; CLI owns native manager integration and
observer supervision. Runtime owns readiness, admission, effects and work drain.
The initial Windows launcher is two independently SCM-owned own-process roles,
S and O, under distinct service principals. P is transient; W is O's owned
subordinate witness child. No local SCM/account mutation follows from this design.
Required missing observer/witness is operational fail-closed, not a new detection
signal or scoring verdict. No automatic permanent kill, erasure, ban, custody
reset or human-presence inference occurs. Existing standalone fixtures do not
qualify this required-observer profile. R1 protected publisher/discovery/process
pins/cross-SID access and R3 typed activation/revocation still need shared-owner
review before code; the broader R01-R17 goal remains unchanged.

**Identity and activation.** A service epoch begins Starting with readiness and
application-effect admission false. Only Authority/native-security-produced
actual observer instance/purpose/scope/epoch evidence may install its lifetime
revocation source; PID text, paths, environment, a JSON boolean or token
possession cannot. Broker=observer must be verified equality. One-use bootstrap
ACK proves exact credential acceptance by O before public ServiceReady; it is
not a continuing lifetime lease, witness availability or service readiness.
Management may be bound passively before ACK if needed for its exact offer, but
no management mutation/application effect is enabled before authenticated ACK,
required witness and final epoch/liveness checks all succeed. Authority must
freeze one shared phase order; Runtime must not invent a parallel ACK protocol.

A one-shot configuration/receipt channel's expected closure is not itself
observer death. A separately typed persistent lifetime channel or actual bound
process-instance handle must define ongoing loss. Lost authenticated lifetime
channel, signaled actual instance death, invalid epoch or expiry of an explicitly
authenticated machine lease revokes the epoch. Human inactivity, motor richness,
request pace or keyboard speed never does. If leases are used, their initial TTL
and expiry/restart policy must be accepted by shared owners and Root. Ordinary
traffic/ACK receipt does not implicitly renew. Authority proposes finite TTL/no
renewal (cap24hours proposed), not a Root-accepted universal default. Expiry has
its own epoch_expired reason, distinct from observed process death. No implicit
new epoch, rebind or stop-deadline extension occurs.

**Atomic revocation and effects.** The service owns one epoch admission gate.
Its initial Ready transition and first Revoked transition are serialized with
application-effect commit, independent of async task polling. Revocation records
first-loss monotonic time/cause, disables future live readiness and new ingress/
management authority, and notifies all owned work exactly once. Starting,
Revoked, Quiescing and Draining cannot issue application-effect permits. No
reconnect, duplicate message or late ACK can restore Ready in that epoch.
Subsequent status reports use this state; already delivered Ready receipts remain
historical observations and cannot be recalled or treated as lifetime leases.

First origin connection initiation and first application-byte acceptance must
serialize against the same gate. A check followed by an async connect/write
outside the gate is insufficient. The proposed implementation performs the
bounded nonblocking initiation/try_write under the gate and records the actual
result there; never hold the gate over await, fsync, witness I/O or an unbounded
syscall. WouldBlock commits no first application byte: await readiness outside,
then reacquire and recheck. If revocation linearizes first, do not initiate the
connection or accept a first origin byte. If the kernel accepted bytes first,
classify OriginStarted-before-loss even if delivery/response occurs later. A
transport attempt already initiated may complete after loss; cancel it before
application bytes. No claim of instant remote-death detection, zero later packet
delivery or rollback of a prior effect is made. Shared future H2/TLS origin
writers must enforce an equivalent commit boundary, not reuse a cached permit.

**Accepted-work rules.** Logical admission closes atomically; accept loops then
close listeners. Kernel-queued or already accepted connections still recheck the
gate. Revoke the observer's volatile management credential/contexts and session
epoch; retain production custody and audit history. Independent authenticated
native-manager stop remains possible. observer-status grants no management
mutation, permanent kill or erasure. ObserverClientLease stop/close can close its
own exact-epoch resources but cannot authorize graceful S stop: normal S stop
needs separately verified native-manager/service-lifecycle authority or a
separately accepted typed stop grant. O retains W/lease through actual S flush/
stop proof. Closing O/liveness alone follows failed/unknown loss handling.
Do not transfer observer authority to an
installer, replacement process or new connection. No hot rebind is in this slice.

| Work at revocation | Required disposition |
|---|---|
| Bootstrap/ACK incomplete | Never publish Ready; revoke fresh credentials; cancel/join startup I/O; fail or report unknown |
| Parsed or RIT-consumed, no effect committed | Cancel before origin; do not restore counters or retry; best-effort bounded actual-status consumption ACK does not imply ALLOW or zero effects |
| Audited ALLOW, no first origin byte | No origin dispatch; retain the actual decision and record cancellation separately rather than rewrite scoring |
| Origin partially submitted | Cancel further upstream writes and join ownership; effects may already exist; never classify zero-effect or retry |
| Complete request submitted before loss | Bounded response/read/cleanup may finish; no new request/dispatch or upstream retry; actual response status is not service health |
| Response already started / native stream | Complete only within existing/drain byte and time bounds or close; never append a second503 after partial output |
| Resource/admin command before durable staging | Discard/reject under revoked context; never apply later from the queue |
| Resource staging or retained grant consumed | Preserve ambiguity and poison/fail closed; restore only authenticated committed resource state; destructive intent without covered effect stays Pending and needs explicit fresh approved recovery, never generic replay |

**Deadlines and finalization.** Let t0 be the first revocation time. Work's
absolute deadline is at most min(original request/connection deadline,
t0+configured loss-drain budget). Service drain uses min(existing stop deadline,
t0+loss-drain budget); repeated stop/loss/renewal cannot reset either. Preserve
StopBudget's strict drain+final-audit reserve<actual native-manager deadline.
Normal authorized stop keeps O/witness usable through returned final flush. O
must not terminate S as an owned child: S remains independently manager-owned.

On loss, cancel and join every accepted request/TLS/IPC/control worker; dropping
a future or forgetting a blocking reader is not drain. Keep available witness
handles until actual final audit attempt. A direct independent witness may still
work after O exits; if O's loss breaks required witness I/O, report that fact and
never fall back to writer-only success or stale Ready/Signed caches. Final flush
must return and satisfy required durable/external coverage to be completed.
Synchronous unreturned I/O still needs CLI/native manager's independent watchdog;
forced exit74 is unproven drain/flush, not success. O may reap only its own
children after the service result, or after explicitly declaring deadline failure.
No persistent production keys/state are erased as observer cleanup.

Observer loss is a failed service epoch even if some drain/flush facts are
completed. Outcome must retain cause=observer_lost and separate completed/failed/
unknown facts for accepted-work drain and final required audit/witness flush;
never turn this path into graceful Stopped solely from listener closure, exit0,
ACK text or a successful local fsync. If loss happens after an already completed
normal service stop, preserve that historical service result and separately fail
observer/combined lifecycle evidence. Operational lifecycle audit uses a distinct
runtime.lifecycle.observer_lost/v1 event with nonsecret epoch/reason metadata;
never place transient loss in the permanent runtime.kill.* namespace. Audit
failure cannot reopen admission or hide unknown effects. No secrets/raw native
identifiers go into public receipts. The exact additive outcome/schema/API is a
shared-owner review item, not permission to silently break current consumers.

**Typed wiring and verification.** Authority must provide an unforgeable
observer-instance/epoch acceptance plus continuing revocation source with owned
cancel/join and bounded I/O; Runtime consumes it rather than duplicating OS/ACL
checks. All management contexts/effect gates share that epoch. Native-security
must independently validate noncircular protected trust, process replacement
and same-PID reuse defenses. Exact constructors, bootstrap order and finite
lease/restart policy need final review. The two-SCM-role architecture and atomic
fail-closed loss semantics themselves are already Root-accepted.

Pure tests cover irreversible phases, first-loss deadline minima, no implicit
renewal, separate causes/facts and no destructive action on loss. Actual ordinary
owned-process fixtures must kill/close the real verified O at bootstrap, before
ACK/Ready, during audit, between gate checks/first write, during partial body,
response/stream and admin staging; observe actual independent origin byte/effect
counters, durable audit/retained state and worker/child termination. A scheduling
barrier is not effect or native identity evidence. Distinguish no-effect,
possible-effect, committed and unknown. Run independent genuine hosted SCM/
protected actor cases and each OS profile; ordinary WSL/Windows success is not
hosted attestation. Full Core/Gate/observer/witness product remains Docker-only
with canonical topology/source/actor/denominator evidence. Existing47/51 native
case gates do not cover this new profile.

**Critique reflected; verdict needs-work.** Single-author persona checklist,
not independent panel approval: Blue requires shared typed boundary and one
atomic effect gate; Red forbids stale ACK/PID reuse/check-then-await races; SRE
requires nonextending bounds, independent manager ownership and honest unknown
flush; privacy forbids secret/identifier leakage or loss-triggered crypto-shred;
evaluation separates native, hosted and product scopes. P0 blockers are the
shared authenticated lifetime/expiry provider, atomic origin commit gate and
bounded cancel/join witness/startup exchanges. Current Application's pre-write
watch check followed by async write_all is not this stronger atomic contract.
P1 blockers are accepted additive outcome fields, final phase order and actual
race/SCM/OS qualification. Root accepted R2/R3 semantics; R1 native backend and
typed activation/API review remain pending. None is accepted as a production
residual or implementation waiver.
The independent startup watchdog begins before any protected reader, unseal,
witness or ACK operation and spans public readiness publication. Its absolute
deadline is no later than the minimum shared handshake and actual native startup
manager bound; retries cannot restart it. An expected one-shot EOF is not a
substitute for that watchdog or for a continuing authenticated lifetime source.

**R1 native backend acceptance review (selected architecture accepted; backend pending).**
Root accepted two decisions after reviewing the exact native-security draft
plan:root/20261107103828 at SHA256
6b97bda59859527a7e9735f93bca34b68ab215514afba5ddc64faf99bf2b8778.
First, signed PUBLIC authorization targeted to the actual S/O instances replaces
the draft bootstrap shared-secret Hello. Authority must assign an explicit new
protocol version/domain; there is no fallback to the old secret Hello. Signature
possession alone proves neither role nor process identity. Both retained native
instance proofs remain mandatory. The service-generated observer credential
remains secret and may cross only the subsequently verified channel.

Second, TransportPrepared is a distinct native lifecycle phase. SCM_RUNNING
means native process/discovery readiness only. Application readiness, ingress,
unseal and writer activation remain false. The independent absolute startup
watchdog and native Stop handler must already be armed. Existing Running/public
Ready publication cannot be reused as application readiness in this phase;
doctor, status and installation consumers must preserve the distinction.

These decisions authorize specification and native-primitive probe preparation,
not full R1/channel implementation or qualification. Remaining P0 requirements
include actual cross-SID primary TokenUser/retained-handle query rights without
SeDebug/admin fallback; create-new SCM/root ownership and existing publisher-key
no-reset binding; typed P acceptance acknowledgements without management power;
process-wide one-claim consumption; clock rollback and conserved deadlines;
actual loaded-artifact binding; atomic secret release/epoch revocation; and an
explicit unattended restart/republication policy for transient P. P's private
WinCred availability after logout/restart must not be assumed. No 24-hour default
lease or implicit renewal is accepted. Linux/macOS remain required open backends.
Native-security owns the proposed safe query/discovery/cross-role ACL probe API
and must report feature/unsafe/API gaps before implementation. This decision
authorizes no local SCM/account mutation or new Runtime source/dependency edge.

Runtime requires the native-security-owned publisher to create protected role
trust and fresh pair-bound grant material for the actual retained S/O instances.
A read-only keyring lookup, caller-supplied Read/Write or a callback returning a
nonce is not publication/provenance. Publication must specify issuer authority,
exact protected storage/atomic consumption, discovery, expiry, both process pins,
minimal cross-role process/token/pipe rights and cancel/join ownership. Consumers
receive private-constructor verified grant/instance types, never PID/path/handle
integers reconstructed from frames. P stays transient and never becomes the
management peer. No missing-store reset, privilege escalation or ACL repair is
an authentication fallback.

Primary-source critique adds the following requirements before accepting R1:

- SCM's QueryServiceStatusEx PID can be invalid in START_PENDING/STOP_PENDING and
  is invalid when stopped. Define usable pre-secret instance discovery without
  waiting for RuntimeReady that itself depends on the authenticated pair. Any
  accepted TransportPrepared distinction above must not be inferred as
  application readiness; actual native discovery still needs qualification.
  [Microsoft QueryServiceStatusEx](https://learn.microsoft.com/en-us/windows/win32/api/winsvc/nf-winsvc-queryservicestatusex).
- Cross-principal retained process handles need an explicit supported rights
  profile for query/token/liveness. Do not assume another service's default
  process DACL permits access, request PROCESS_ALL_ACCESS or silently enable debug
  privilege. Missing rights remain Unsupported/refused.
  [Microsoft process security](https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights).
- CRYPTPROTECT_LOCAL_MACHINE is machine-bound, not recipient-role encryption:
  any local user obtaining the blob can decrypt it. A proposed use must prove
  exact protected storage/recipient ACL/publisher integrity and its limits; the
  flag alone cannot construct a verified role grant.
  [Microsoft CryptProtectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata).
- Named-pipe default access and broad generic-write grants are insufficient.
  FILE_GENERIC_WRITE includes create-instance permission; define exact owner and
  individual allowed client rights for both O-owned bootstrap and S-owned
  management endpoints. Neither Everyone/anonymous defaults nor same-SID-only
  transport qualifies the new distinct-principal profile.
  [Microsoft named-pipe rights](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights).

| Profile | Current R1 disposition | Required producer work |
|---|---|---|
| Windows distinct-principal S/O SCM roles | Signed-public-grant and TransportPrepared architecture accepted; concrete backend not implemented or qualified | Actual issuer/publication and unattended restart, retained handles/TokenUser, explicit cross-SID endpoint/process rights and atomic release/revocation |
| Linux/WSL production roles | Existing ordinary native component results do not qualify R1 | Native-security must define actual protected unattended publisher, service discovery/process pins and cross-principal transport; no UID-only substitute |
| macOS production roles | No R1 acceptance or qualification inferred | Native-security-owned protected producer and actual domain/instance/transport profile; unavailable capabilities remain Unsupported |

This matrix deliberately grants no production support from the current ordinary
Windows47/WSL51 fixtures. Native-security must supply the exact backend/API and
supported OS matrix, independently critiqued before channel code. The remaining
P0 is a concrete producer gap, not permission to write a synthetic grant factory.

### Private observer gate S1 — source model, not production authority

Root authorized a private native-independent gate slice while typed producer
acceptance remains pending. service::epoch is compiled ONLY under cfg(test).
Its initially closed Gate binds an epoch and one shared Arc identity; readiness
and admission derive from one mutex-protected phase. Test-private activation is
not exported or compiled into the production service. Starting, manager-stopping,
revoked and poisoned states refuse new work. A first-loss latch records one
monotonic instant/cause; subsequent loss and manager stop cannot overwrite it or
extend a derived min(original, first-loss+drain) deadline. Overflow refuses.

Private Work retains its exact gate identity, transport-attempt fact, accepted
byte count and unknown adapter-result fact. Connect initiation and each bounded
try-write model run synchronously under the same gate as revocation. WouldBlock
accepts no bytes and a later attempt must reacquire/recheck. No future/await,
audit, fsync, native query or join belongs in this critical section. Previously
accepted partial bytes remain recorded after loss; an invalid adapter count is
unknown and cannot be reused or classified as zero effects. This test model's
closures are not public authority callbacks or proof of a real socket boundary.

Seven authored native-independent tests cover closed/foreign/irreversible epochs,
first-loss/deadline conservation, pre-connect/pre-write refusal, WouldBlock,
partial/unknown submission and manager-stop distinction, concurrent first-write
linearization, and poisoned mutex fail-closed behavior. On immutable snapshot
30d8585544e021d91a281754717d82fd752ae297 (Runtime source9ad15bce), allocated
Windows all-target check and strict Clippy passed; these exact seven model tests
passed with0failed/0ignored/52filtered. Recursive508-file content/path inventory
was unchanged. This does not qualify actual observer/socket boundaries.
No ServiceHost, credential writer, native loss monitor
or real origin path is wired, and existing ExitReport schemas are unchanged.

Proposed integration ownership follows Authority's reviewed design direction:
Authority owns the shared neutral epoch core because Runtime already depends on
Authority. Native-security supplies a private-constructor actual bootstrap epoch;
Authority's private composition factory also requires independent verified W and
conserved budgets. One unique owner retains native pins/monitor cancel+join and
restricted gate clones go to both credential enqueue and Runtime origin adapters.
Owner Drop revokes synchronously but never certifies monitor join or audit flush.
Restricted clones cannot activate/rebind, renew or authorize manager stop.

Before moving this model into that shared core, jointly accept exact types and
private constructor/phase ownership; concrete nonblocking connect/write return
types; same-gate ready evidence without Authority-to-Runtime dependency; native
loss-source cancellation/join; and separate failed/unknown drain/flush facts.
The sealed Runtime readiness seam remains P0: no bool, generic caller callback,
JSON assertion or test activation may promote a production epoch. S1 is not an
alternative observer implementation or a completion claim for R09/R15/R16.

**Independent Authority review reflected; shared API still pending.** The S1
Gate clone currently exposes model activation/revoke/manager-stop methods within
the private test module. It MUST NOT become the shared production API unchanged.
The proposed shared core has a unique noncloneable ObserverEpochOwner retaining
native/W pins and monitor cancellation/join, plus cloneable ObserverEffectGate
handles restricted to observations and concrete effect admission. Only the owner
may publish verified phase transitions or first loss. Restricted clones cannot
revoke as an authority, activate, renew, replace an instance or authorize stop.
Owner Drop must synchronously latch revocation before releasing pins, with a
distinct owner-abandoned fact if actual observer death was not observed. Drop
does not establish monitor join, request drain, audit flush or normal service stop;
explicit bounded close returns those completed/failed/unknown facts separately.

The model's connect io::Result<()> is deliberately not a production socket-result
contract. A reviewed concrete adapter must distinguish SubmittedPending (kernel
accepted initiation, including nonblocking connect-in-progress), Connected,
WouldBlockNoAcceptance (no initiation accepted), InitiationFailed, and Unknown.
EINPROGRESS must not be mislabeled as WouldBlockNoAcceptance or completed origin
work. These are transport facts, never evidence of an application byte, request
completion or business effect. Work already submitted before loss is cancelled
without claiming the kernel or remote endpoint rolled it back. No retry may
reclassify an uncertain prior initiation as absent.

The concrete write adapter must similarly distinguish accepted byte count,
WouldBlockNoAcceptance, a documented zero-new-byte failure/closure, and Unknown.
Previously accepted bytes remain accounted separately. An error must not be
assumed to prove zero possible bytes without the actual kernel API contract;
unknown or invalid adapter returns cannot grant a fresh retry/effect permit.
Exact platform wrappers, finite nonblocking syscall behavior and actual socket
race/cancellation tests must be reviewed before integration. No future, readiness
boolean or untrusted generic closure can stand in for those concrete operations.

Authority owns this shared core; the Runtime S1 model must move/be replaced when
accepted, not remain a second independently revocable production gate. Exact
sealed Runtime readiness evidence, native/W constructors and activation phase
order remain P0. This review changes the API proposal only: current seven model
tests have only the scoped source30d85855 model qualification above; there is no
new production wiring or dependency edge.

### Shared browser HTTP contract controls — authored, not executed

Root accepted two narrow real-TCP Runtime control fixtures reusing the existing
collector/pilot/fixture.json scope/routes verbatim. Runtime-owned
browser-http-contract.json publishes their case IDs, exact expected status/ACK/
consumption/epoch behavior, request bounds and original fixture SHA256. Lab may
consume this structural contract; the current sole actual BrowserSession pilot
and its selected/filtered denominator are unchanged. No collector/Lab/shared
graph source is edited by this Runtime slice.

The first case checks exact ten-field bootstrap with no ACK, before-report403
CHALLENGE with valid consumed ACK, report200 with valid ACK, and required fresh
collector422 without ACK/counter and with local shared Session invalidation.
The second sends an exact65536-byte signed request through frozen policy and
then a signed65537-byte declaration, expecting parser413 before body/counter/
decision audit with no ACK and client epoch closed. That oversized native control
sends HEADERS ONLY; it does not claim actual Fetch can receive413 while a full
body is being transmitted. Actual browser reset/network failure must not be
relabeled as observed HTTP413: Lab must independently inspect gateway response,
audit/effects and client invalidation. Both cases independently require no origin
connection and join the owned Runtime server after a bounded stop.

The client codec's1MiB complete-response bound is an upper client limit, not a
negotiated Runtime capacity. Runtime's64KiB request and existing147456-byte
buffered-origin frame bounds remain unchanged. Bootstrap carries no hidden
budget negotiation. Missing/invalid ACK, partial response, cancellation, timeout
or closed epoch never authorizes automatic retry or counter restoration. Native
controls use the real shared Rust Session and HTTP but synthetic report/audit;
they do not execute BrowserSession/Fetch or prove browser/human/AT or independent
witness isolation. Actual ServiceHost response stripping and the existing genuine
Lab flow remain separate gates. On immutable snapshot
f8e04e332dde57676a1e6e1095ea8078af83c1b7, allocated Windows all-target check,
strict Clippy and fmt passed; both exact controls passed with0failed/0ignored/
59filtered. All2462 snapshot files,125 registry packages/6576 registry files and
14 ancestor/Cargo-home configuration candidates remained unchanged. This proof
uses collector fixture SHA256
87a45f212f2974078fb618c1a55bc92b4e0bafddbc1035907f0162ba84be76e9;
later Root collector/fixture changes are outside this qualified snapshot.
TLS ALLOW-to-origin and native service readiness remain open.

### Observed Ready composition proposal — architecture direction only

Root accepted Runtime-owned final Ready composition, not production types or
qualification. Authority must not consume a RuntimeReadyReceipt or depend on
Runtime. Native-security/Authority first produce VerifiedObserverPreparation:
actual native publication/pins, verified W, the same irreversible gate and
restricted bounded transport. It has NO credential receipt yet. Runtime's
proposed private prepare_observed consumes this preparation BEFORE any existing
bind/unseal/writer operation. Requiring an observer credential ACK at this point
would cycle because the service has not provisioned that credential yet.

One private ObservedStartupOwner retains the preparation, native/W/monitor
ownership, partial host resources and conserved deadlines. Later Offer/actual
ACK creates the receipt-bound PreparedObserverSession inside that SAME owner.
Final activate_observed consumes this owner after actual listener/capture/TLS/
audit readiness checks, privately constructs RuntimeReadyReceipt, and performs
complete ServiceReady enqueue plus Active transition under the same gate.
Neither a public boolean nor an independently supplied Runtime proof is accepted.
Existing ServiceHost::bind is not a verified TransportPrepared constructor and
must not be called first to bypass the pre-unseal requirement.

ActivationFailure must retain a noncloneable cleanup owner, failure stage and
ReadyDelivery fact (NotSubmitted/Partial/Submitted/Unknown), rather than dropping
or extracting the observer secret, transport, pins, W, monitor or host resources.
Failure synchronously revokes; explicit bounded close attempts cancellation,
monitor/worker join and required flush with separate completed/failed/unknown
facts. Drop grants no async completion. Loss before complete Ready enqueue keeps
origin admission closed; partial Ready grants no origin/retry/deadline reset.
Loss after a completed enqueue makes that receipt historical, never a reusable
live lease. Exact consuming types, native producer and concrete transport
contracts remain jointly reviewed P0s; this section implements no channel/API.

**First sealed source slice.** Runtime now imports Authority checkpointfecdb4d1's
actual startup::observer opaque phase guards. Authority's production preparation,
session and failure each contain a private empty NativeProof; even tests cannot
construct one. native_preparation returns Unsupported. Runtime's PRIVATE
ServiceHost::prepare_observed consumes only VerifiedObserverPreparation, boxes
the exact Configuration owner before validation, and returns a private
ObservedStartupOwner. It does not call existing bind, unseal, writer opening,
native verification, Offer or Ready. No public constructor, trait, parent flag
or DTO can substitute for the missing proof.

One private Retained<R, AuthorityGuard> holds guard before boxed resources in
declaration order. Cancellation consumes the actual Authority phase into its
retained ObserverStartupFailure without extracting/replacing Runtime resources.
The same RetainedFailure<R, AuthorityGuard> remains the cleanup owner. Guard Drop
synchronously cancels/revokes before resource field destruction; this is not
monitor join, accepted-work drain or required audit flush. The later bounded
cleanup/transport/receipt/Ready APIs remain unavailable and no successful cleanup
result is invented. Receipt-bound guard storage is authored but no promoter exists.

Three native-independent tests exercise the actual generic boxed retention/fail
path with private test guards: allocation identity through failure, retained
resources through repeated failure, and guard-drop-before-resource-release order.
They never mint Authority's uninhabited production proof. Allocated immutable
6c02bd222e2982aaeeeffa47eec6ec00797bd8a9 passed all-target check, strict Clippy,
fmt and these exact three tests (0failed/0ignored/61filtered). All2506 snapshot
files,6576 registry files,14 ancestor config candidates,66 compiler/target-lib
files and14 selected build-environment hashes stayed unchanged. Native verified
activation, actual resource I/O and production cleanup/JOIN are not qualified.

Exactly scoped dead-code expectations cover the unreachable private preparation
entry, receipt-bound variant and production failure-inspection method while
native/receipt/cleanup producers remain Unsupported. Each item names its removal
condition in source; there is no blanket module allow or public export for lint.
Remove these expectations when the corresponding verified caller/promotion/
bounded cleanup consumer is wired. They waive no logic, test or interface failure.

### L3 temporal observation consumer contract — preparation only

Structural-development's exact-role registry assigns Runtime the single
application-scoped temporal owner; Network owns the kernels and capture/provider
boundaries. Provider checkpoint6c72a47f exposes history::{anomaly,rate},
correlation::{Registry,Stats,Outcome,Scope,FleetInput} and traffic::{Log,check,
Association,Coverage}. That CURRENT source was handed off UNTESTED. Earlier MAD
or socket/capture passes do not qualify these new kernels or Runtime serving.
This section prepares a consumer contract, not a signal/readiness activation.

One future Application-owned context retains each configured finite kernel and
an ordered Duration clock anchored to the actual application lifetime. It must
not create a second global registry, fork a temporal predicate, infer configuration
defaults, or use wall-clock buckets as interchangeable monotonic ticks. Runtime
owns exclusive mutable access and observation ordering; Network owns arithmetic,
window/capacity/GC semantics and the existing signal definitions. Root owns all
scoring policy and shared graph. A title/layer label grants no provider-path edits.

Consumer provenance remains separate from statistical association. Session keys
come from authenticated Runtime state, not a client session claim; TLS/H2 fields
come from the exact retained capture Connection when that plane is required.
Do not derive trusted JA4/subnet/fleet identity from normalized headers, loopback
peer text, client reports or traffic address backfill. traffic::Association::
AddressOnly never authenticates a connection epoch; Coverage::UnqualifiedHistory
is not provider health. Fresh context/coverage and any required missing-plane
enforcement still need actual producer qualification.

Missing and incomplete outcomes are preserved explicitly:

- history::Unavailable, including time regression/budget/nonfinite refusal,
  cannot become zero/normal; latch lost admission coverage. Later success does
  not reconstruct skipped observations or clear that gap automatically.
- anomaly warmup/evicted_key describes limited history, not established normal
  behavior or continuity. Rate IgnoredEmptyKey is not a measured Normal0.
- rate::Coverage::WindowExpanded applies while now<complete_after. At equality
  the kernel returns Retained for that expansion gap only; unrelated latched
  admission gaps remain. Retained never means healthy complete capture.
- correlation::Stats::Absent and Scope::MissingKey are missing, not zero counts.
  LocalOnly/LocalDegraded/CoordinatorReportedUnqualified retain their exact origin;
  none qualifies required fleet coverage. A trait callback/Reported vector is not
  an actual atomic coordinator. Reported fleet counts only feed the already
  weight-zero velocity path and cannot silently replace load-bearing local data.
- traffic insufficient/unqualified history cannot fill required TLS/H2 coverage,
  turn an absent signal into OK or reopen a previously incomplete epoch.

Observation admission points must be mapped to the actual legacy serving calls
before activation, preserving frozen observation semantics. Raw TLS acceptance
and authenticated application requests are distinct events. Request RIT replay,
response retry, status polling or audit failure must not accidentally append the
same application observation twice. No automatic replay/reset of temporal state
can make a failed request look unobserved or healthy.

The kernels commit individually, not as a cross-module transaction. Runtime must
track operation AND outcome category, not assume Ok means an append. Rate
IgnoredEmptyKey and correlation Scope::MissingKey do not commit clock/history;
traffic::check and rate::level are pure inspections. Correlation Registry already
feeds one internal generic MAD population: Runtime must not feed the same
correlation sample into an additional MAD detector. For calls that actually
committed, if a later module refuses, preserve
prior facts and a composite coverage gap; neither roll back earlier histories nor
retry them as a fresh observation. A private progress record is not a public
healthy capability. Session/observer epoch changes, configuration expansion,
GC/eviction and explicit resets must retain their distinct coverage effects;
session invalidation cannot erase shared quotas or manufacture fleet history.

Before code adoption, jointly freeze explicit limits, key/provenance producers,
legacy observation points, failure/partial-progress projection and reset/GC rules.
Then qualify pure consumer negatives and actual capture/session/time/audit
composition within this cluster; required admission remains closed until the
corresponding provider gates pass. No new required-route flag, enabled default,
Engine predicate/weight, capability claim or Runtime implementation is added by
this contract preparation. Full product remains Docker-only and source-bound.
