# Shared framework consumer contract

Document version:0.3. Status:provider-reviewed draft with CA-HANDOFF-01 open. This is the L0 contract
for the existing L4 Node/Bun, Python ASGI/WSGI and Ruby Rack consumers. It does
not introduce a new crate, runtime API, privilege, supported tuple or readiness
flag. Existing source/path ownership and the original complete adapter/package
goal remain unchanged.

## Problem and boundaries

Each language must project an actual host request through one shared request,
deadline, admission and Authority contract. Language bindings own ABI, body and
host-liveness adaptation; they do not implement scoring, native peer inspection,
secret providers, IPC framing/verification or durable audit. Production and
pilot qualification consume genuine lower-provider evidence. Pure models and
caller-supplied booleans cannot substitute for those providers.

The implementation language remains Rust1.99.1/edition2024. Existing explicitly
accepted glue/unsafe exceptions do not expand. Detection, capacity64, verdict
vocabulary, accessibility and the T4 ceiling remain unchanged. No publication,
hosted dispatch or privileged allocation is authorized by this document.

## One shared request lifecycle

1. Parse the original private configuration through `Configuration::parse`.
   Unknown/duplicate members, unsupported peer/endpoint/route or missing bounds
   reject. Configuration and credentials are not Debug/public receipt material.
2. Acquire `Admission` before adapter-owned complete-body allocation and start
   one `Pending` before adapter input consumption. Only successful acquire constructs the RAII guard.
   Dropping it releases its one slot; saturation never evicts another request.
3. Read the complete transfer-decoded, still content-encoded body within the
   configured byte and original time budget. Partial/disconnected/oversized
   input cannot be declared complete. Sync hosts need actual server input
   deadline enforcement; a configuration boolean is not evidence it exists.
4. Project `Observation` from the actual host API. Strip internal/proxy authority
   headers, retain the supported target/body/header semantics, and leave every
   unavailable capability absent/false. Call shared `encode` and the
   Integrity-owned selected-profile parser. No alternate envelope/nonce/crypto
   implementation is introduced in a binding.
5. Call `Pending::decide` with that same configuration, observation and body.
   Authority's existing client owns actual peer authentication and response
   decoding/binding. The expected nonce, tenant, origin, revision and deadline
   come from the outstanding request/configuration, not response claims.
   For one incoming protected request, the L4 lifecycle owner permits at most
   one decision attempt and one origin handoff. It must not recreate `Pending`,
   re-encode a fresh nonce, retry after failure or mint a replacement ticket to
   obtain another handoff. The current `Pending::decide(&self)` API does not
   enforce this type-level rule; server replay state owns nonce reuse rejection.
6. Keep the actual bound result single-owner across every await, event-loop
   queue and VM/GIL/GVL entry. Inspect `verdict()` only to select the branch.
   For ALLOW, finish replay construction, argument preparation and trusted
   liveness callbacks first. Check the same original `Pending` and consume the
   actual bound result immediately before the trusted application invocation.
   No further await, queue or host callback may intervene before that invocation.
   A non-ALLOW branch may consume and reject without invoking the origin.
   Only ALLOW can invoke the protected origin, with the same captured body and
   sanitized request. DENY/CHALLENGE/error/expiry/unknown never invoke it.
   `BoundResponse::consume(self)` consumes that result object, not the server's
   nonce state or the host's request lifecycle. `verdict()` is inspection only.
7. Propagate cancellation and release each owned body, permit, callback/task and
   reader through its owning runtime. A known-origin effect requires the real
   request's bound ALLOW, not a second decision or recreated response object.

This is required behavior, not a statement that every current host path has
passed actual tests. Transport status details remain host-specific: for example,
an ASGI exception can produce server500 while a synchronous WSGI refusal returns
503. A non-2xx response alone does not prove correct binding or cleanup.

`check_handoff` is a local clock/budget check. A later await, event-loop queue or
GVL entry can intervene before the actual origin call; it is not an atomic native
process-epoch admission gate. Actual host qualification must cover delayed
invocation, expiry and cancellation at the claimed handoff boundary. It cannot
infer an epoch/lease proof from this synchronous check or `verdict()` inspection.

### Current implementation deviations and CA-HANDOFF-01

The lifecycle above is a requirement, not an assertion that all hosts already
reuse `Pending`. Node currently has a private wall-clock deadline state and
passes the full configured timeout to its Authority client. Its queued callback
also converts the bound result to a copied status early. It must adopt the
original shared monotonic budget and retain the actual result until final VM
admission. A timer is not proof that a delayed callback ran before expiry, and
wall-clock rollback must not extend the original request budget.

ASGI currently consumes before its tail poll, replay construction and queued
application helper. Its final admission belongs inside the owning-loop callback
that actually invokes the application, after queue delay and cancellation
inspection. Moving consumption immediately before scheduling that callback
would leave the same gap. Neither ASGI nor Node is closed by the synchronous
WSGI/Rack source correction.

The WSGI/Rack source correction retains the result through replay preparation
and the last trusted disconnect callback, then checks the original budget and
consumes immediately before the application call. Python prepares the tuple and
Ruby interns the method name before that gate. This is source-only until the
affected SDK/ABI checks and actual regression run are bound to the new bytes.

Required regressions must obtain a genuine bound ALLOW through the accepted
component Authority fixture, then delay the final trusted connected callback
past the remaining original budget while it returns connected. Observe zero
origin calls/effects, and pair it with a timely connected control that reaches
the origin exactly once. Observe that ALLOW was actually obtained before the
delay; an earlier timeout or always-disconnected fixture cannot establish this
regression. ASGI/Node additionally need delayed owning-loop admission, expiry
and cancellation controls, including original monotonic expiry under wall-clock
rollback for Node. Do not mint a replacement result or use a copied verdict as
the grant. Exact SDK tuple, source, case counts and no-origin observations remain
required; none has been executed by this draft.

## Provider and consumer ownership

| Provider seam | Owning contract | L4 consumer obligation |
| --- | --- | --- |
| Schemas, producer identities and frozen decisions | Root-owned Integrity/signals/scoring | Use selected `AdapterProfile`/expected identity and existing verdict; never mint raw capability or change weights/rules. |
| Admission/config/envelope/deadline | `framework-adapter` | Reuse the same primitives; preserve byte identity, exact routes, one original budget and acquire-only guard. |
| Authenticated decision IPC and bound response | Authority owner | Consume opaque verified results; preserve nonce/scope/expiry and audit-before-success semantics; do not copy IPC, peer or secret code. |
| Native actor/pipe/reader/process ownership | Native Security and Process Ownership owner | Use only the published capability class actually qualified for the target. Direct-child capture, symbolic lifecycle, inventory, systemd design and full-tree cleanup are different scopes. |
| Native ABI handles/buffers where a host uses that ABI | Native ABI owner | Require the exact supported handle/buffer/export/compiler/allocator contract; no expanded unsafe or independently invented allocator owner. |
| Durable audit/recovery | Audit owner, consumed through Authority | Do not construct a second log for enforcement or reset missing history. Test-only evidence/log fixtures remain labelled test-only. |
| Package and distribution qualification | L4 binding owner plus L5 CI/Distribution owner | L4 proves installed ABI/API and production feature surface; L5 owns shared archive/content/trust infrastructure. Neither substitutes metadata for actual package bytes. |

Rust configuration-owned credential memory is distinct from host-owned immutable
option strings and encoded IPC buffers. The native/Authority owners must provide
the applicable ingress and encoded-secret lifetime contract. Using a zeroizing
Rust field does not prove all host/serialization copies were erased. Private
transport input and raw diagnostics stay out of public logs/receipts.

The stable existing service seam is the protocol0.1
`client::{decide, ExpectedBinding, BoundResponse}` with the configured producer
profile. Expected identities remain `humanymous.node`, `humanymous.python` and
`humanymous.ruby`, version0.1.0-experimental.1. Profiles select Node raw/Fetch,
Python ASGI/WSGI and Ruby Rack normalization through Integrity, not caller claims.
Current Unix peer verification is UID peer credentials; current Windows scope is
SID/DACL plus server PID. Neither states a full process-instance/cross-principal
R1 attestation. Existing component IPC receipts do not establish current production
Ready, actual language ABI or unattended producer/secret-store admission.

Each pilot selects only the native capability it needs. A separately accepted
trusted finite direct-child component profile need not wait for every unrelated
native facility, but requires its own genuine failed-start/IO/stop/original-
deadline/join/cleanup receipt on the frozen API. If full-descendant containment
is required, direct-child success cannot provide it. The new common capture/
direct-pipe source at7a1e8dee is source-ready, not qualified L2 evidence. Its
`parent_context` codec is dev-support syntax, not a mandatory framework-runtime
edge or production identity proof. No alternative profile becomes admitted by
this draft; Python's current explicit refusal remains until its profile and
controller decision and applicable actual provider receipt are accepted.

## Language-specific boundary obligations

| Consumer | Required actual boundary proof |
| --- | --- |
| Node HTTP1 / Express | Genuine socket/header/body events, abort-before-handoff, callback/reference lifetime and selected N-API tuple; rawHeaders proves only that host header view, never raw TLS/original HTTP2. |
| Node/Bun Fetch / Hono | Actual owning runtime's Request/Response, bounded body cloning/replay, binary response and repeated Set-Cookie behavior, abort and native import; no Node raw evidence inferred from Fetch. |
| Python ASGI | Callbacks/futures execute on their owning event loop/context; body completion, disconnect and delayed-handoff expiry races are observed; cancellation retains/settles the pending receive; lifespan forwarding is tested separately. |
| Python WSGI | Actual server supplies bounded input and a trusted disconnect callback; partial input/idle closure and total upload deadline are distinct. CGI normalization does not reconstruct original duplicates/order. |
| Ruby Rack | Ruby API access and callbacks stay under the owning VM/GVL; application/callback roots survive GC/compaction; decision worker cancellation/join, delayed GVL handoff and trusted server liveness are real, not truthy values or request headers. |

Every host needs actual binary-body, repeated-cookie, non-ALLOW/no-effect,
unavailable/timeout/invalid-route, incomplete-body and disconnect-with-connected-
control evidence for its claimed surface. Required groups/counts remain in the
existing owning pilot contracts; this common contract cannot waive or shrink
them. Server/framework mount scope is explicit: a FastAPI mounted ASGI pilot does
not prove parameter validation or dependency injection. Django HTTP-only ASGI
does not become lifespan-capable because another framework passes lifespan.

## Promotion gates and receipt requirements

| Layer gate | Evidence required | Insufficient evidence |
| --- | --- | --- |
| L0 contract ready | Provider/consumer vocabulary, finite limits, missing states and owning API agreement | A renamed chat, diagram or method stub. |
| L1 shared behavior | Selected units/negatives, external API controls, fmt/check/strict Clippy and source/target/features | Pure success promoted to OS/FFI/secret/process proof. |
| L2 lower provider ready | Actual supported OS/provider identity, handle/reader/process/secret ownership and failure/cleanup facts for the capability consumed | Caller assertions, pure lifecycle/inventory, GroupOnly used as full tree, another OS or an unqualified allocation. |
| L3 service ready | Real authenticated IPC and qualified audit/native composition with replay/expiry/failure evidence | A synthetic ALLOW fixture promoted to production browser or service readiness. |
| L4 host ready | Actual claimed language ABI/server/GC/GVL/loop/cancellation/package-import tuple with the required provider and service receipts | A typecheck, old binary, object smoke, different runtime/version or missing/ignored required case. |
| L5 qualified delivery/product | Genuine required package/cross-OS/Docker product/accessibility/distribution gates owned by their respective layers | A local host-loopback test called product e2e, or successful archive metadata called installed native behavior. |

Each promoted receipt binds consumer/provider source and compatible artifact
identities, OS/architecture/toolchain/target/features, actual venue, exact selectors
and discovered/passed/failed/ignored/filtered counts, first failures, input closure,
capability limits, origin/audit observations where required, and actual cleanup.
Source/binary changes need an explicit affected-input assessment; unrelated
layers do not rerun automatically. Reuse is limited to a declared compatible
unchanged provider/consumer seam, not a guessed latest-graph equivalence.

Finite deadlines include a declared cleanup policy, not unbounded reader joins.
Unknown/Unsupported/cleanup-incomplete never become successful qualification.
No `qualified: true` flag, raw process ID, marker or path supplied by a consumer
can manufacture native ownership. Required actual tests remain pending until
the relevant provider and execution decision exist. Synthetic fixtures may run
only in explicitly accepted component scope and cannot cross this promotion gate.

Package receipts additionally bind the exact npm/wheel/native-gem bytes and
installed native module path/ABI, source/generation lineage, clean import, public
API resolution and production feature bits. Fixture/package-tool code must be
absent from production output. `releaseQualifiedTuples` stays empty until the
owning actual gates qualify that exact tuple; no publication follows from a pass.
Paths, peers and raw details needed for private verification do not automatically
become public receipt fields.

## Existing bounded evidence and remaining work

- `summary-014` in evidence:root/20261107075243 binds shared six units, positive
  external/no-run and negative direct-constructor controls, check/Clippy and
  independent CA-API-01 closure to sourceac7dc4a9. Evidence checkpointb0785cf7
  preserves that exact historical source and its source/registry/config ledgers.
- `summary-013` binds Python host-pilot typecheck/Clippy and two receipt-parser
  pure passes to d68682e4. It is not post-ac7 SDK/host or lifecycle qualification.
- `summary-010` binds actual local WSGI28 to7842b6a8. Later factory/shared-case
  extraction and ASGI42 are different bytes; their actual runtime remains pending.
- Node's older Windows52/Linux36 receipts do not qualify its later60-case/native
  import/package surface. Ruby default pure worker/driver results do not compile
  or exercise Magnus, the Ruby ABI, Rack/server callbacks or native gem install.
- The existing Python controller/ASGI `NOT_ACCEPTED` refusal remains. Lower
  capture/direct-pipe proofs, pure owned-tree models and systemd design must not
  be conflated. Production bootstrap/secret-provider and full lifecycle gaps
  remain with their lower owners; no foreign source edits or fake capability
  promotion fills them here.

## Critique reflected and next handoff

CA-API-01 forbids forged admission construction. The rejected process-group
check-then-signal approach forbids marker/PID authorization. The pure lifecycle
review separates owner identity, terminal observations and actual native proof.
Secret lifetime, unavailable raw evidence, accessibility, source/venue binding
and test-only ALLOW limits remain explicit requirements, not accepted omissions.

Provider critique of v0.1 is reflected here: consuming a result is distinct from
nonce replay and one host invocation; synchronous expiry checking is distinct
from atomic native epoch admission; current UID/SID/PID peer scope is not R1
attestation; dev-support codecs and source-ready process APIs are not qualified
providers. Native review accepted the v0.1 separation of secret copies and
capability classes without approving a backend. No implementation acceptance is
inferred from either review.

Independent critique identified CA-HANDOFF-01: WSGI/Rack checked before a
potentially slow trusted connected callback, and all four host paths consumed
too early. Authority confirmed the existing opaque result API suffices for the
correction; no new proof API is required. The narrow synchronous source fix
implements that accepted ordering, with actual regression qualification pending.
The next source slice follows the accepted seam and its lower-provider availability;
no new process/controller or heavy/privileged/hosted execution is granted here.
