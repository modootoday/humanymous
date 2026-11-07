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