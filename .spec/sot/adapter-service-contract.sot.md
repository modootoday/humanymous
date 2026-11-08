---
id: sot:root/adapter-service-contract
title: Experimental adapter decision service protocol
status: active
created: 2026-11-06T22:15:00Z
updated: 2026-11-08T15:57:00Z
verify: manual
rel:
  governs: [adapters/node/**, adapters/contracts/**, crates/node-adapter/**]
  depends_on: [sot:root/adapter-ecosystem, sot:root/adapter-authority]
---

# Experimental adapter decision service protocol

## Live authority client owner (measured 2026-11-08)

Shared adapter decision clients consume `humanymous_runtime::authority::client`
from package `humanymous-runtime` (feature `authority`). Do not invent a
separate `humanymous-authority` library dependency for host adapters. Node
package path governance above is unchanged. Detail:
[[sot:root/adapter-authority]] and evidence
`.spec.local/root/evidence/20261108155700-cursor-spec-gap-audit/`.

## Rule

`humanymous.adapter/0.1` is a private experimental application protocol, not
Draft v0, an RFC, or a replacement for frozen RIT. Rust alone owns decisions,
capture provenance, replay state, credentials, policy, challenge and audit.
Node does not compute scores, verify RIT or mint security receipts. Version
0.1 authorizes bounded complete HTTP requests only. No stream authorization,
remote transport, capture upload or admin method exists in this version.

### Rust-only implementation continuation

New authored implementation and tests use Rust1.99.1/edition2024. Existing
adapters/node/src and test JavaScript/TypeScript are historical oracle/legacy
artifacts, not permission to add or alter foreign source. Native implementation
is humanymous-node-adapter using pinned napi3.16.1/napi-derive3.7.0/napi-build2.6.0.
Upstream macro-generated FFI is separate from safe authored Rust. No eval,
embedded script string, generated application wrapper or JSON subprocess bridge
substitutes for real framework callbacks. Node/Bun load the `.node` artifact;
Rust-created callbacks integrate with actual upstream Express and Hono objects.

The authority-owned shared client verifies actual server UID or Windows pipe
SID/ACL plus authority PID from supervisor provisioning, scoped bearer, strict
bounded framing and exact response binding. Dropping the async decision future
on host cancellation closes the IPC exchange; no retries. The host owns one
total body/decision deadline. Missing authority client or untested ABI tuple is
an explicit incomplete gate, never a successful fallback.

### Provisioned producer profiles

Version0.1 retains Node wire compatibility. A trusted authority configuration
chooses AdapterProfile::Node or AdapterProfile::Nginx before parsing; an envelope
cannot select its own producer policy. Both use version0.1.0-experimental.1.
Node accepts id humanymous.node and node-raw/fetch-normalized only. nginx accepts
id humanymous.nginx and nginx-raw/nginx-normalized only. nginx-raw means the real
HTTP1 ingress parser preserves ordered casing/duplicate pairs; nginx-normalized
means it does not. Neither implies original client TLS or H2. nginx modes have
peer=null, connectionId=null, rawHeaderOrder=(mode==nginx-raw); Node Fetch retains
the same null conditions. capture/stream=null, complete bounded body, byte
validation, tenant/origin/revision/nonce/auth/audit and seven response fields stay
unchanged. These modes describe a contract, not measured nginx/native support.

### Accepted native host API and CI boundary

The native package exports express(optionsJson), hono(optionsJson,app,streamModule),
htmlHook(nonce,scriptPath), and capabilities(). Options JSON is a fixed serde
schema with installation/tenant/origin/revision, endpoint, credential, server,
routes, timeoutMs (1..30000), maxBody (0..1048576). server is the shared authority
Peer schema: Unix kind/uid or Windows kind/sid/pid; pid is the authority process,
not the Node child. Every field is required; duplicate/unknown members reject.
Routes are an explicit complete pathname inventory, not an authorization policy
replacement. Hono app.fetch is invoked only after complete byte validation and
one consumed bound result. Node root middleware precedes every application
parser/effect. Native unsupported modes cannot be upgraded by application headers.

Accepted shared API (authority owner, source implementation in progress):
ClientOptions(endpoint,credential,server,timeout_ms,AdapterProfile::Node),
ExpectedBinding(id,tenant,origin,revision,deadline), async client::decide returning
non-Clone BoundResponse with expires() and consuming consume()->Verdict. Native
callbacks retain min(response.expires,original deadline), consume once and check
that boundary again immediately before origin. Monotonic/wall-clock/peer checks
remain authority-client responsibilities; host cancellation drops the future.

Host object references are leases owned by the originating Node-API environment,
released on terminal paths. Body ownership is process-bounded to64 outstanding
body/decision requests; exhaustion is a dependency503, never a score/verdict.
Bounded streaming input is paused until cancellation/end/error/deadline listeners
are registered. Response SSE/download streams remain owned by the application.
Forwarding trust headers are stripped while preserving framework-normalized
Authorization/Cookie and ordered sanitized header evidence. Async iteration,
ordinary event routing, upgrades/CONNECT and abort/backpressure require real host
tests; source compilation alone does not qualify those lifecycles.

Measured provenance delta: Bun1.3.14 Node HTTP compatibility rawHeaders preserves
duplicate pairs in the tested corpus but lowercases field names. Its Express
projection is therefore fetch-normalized with rawHeaderOrder=false and null
peer/connectionId, so the shared engine cannot treat normalized casing as a raw
tell. Node24 HTTP1 retains node-raw after the same socket corpus. Bun Hono uses
the actual Bun.serve Fetch topology; Node Hono uses upstream getRequestListener.
These measured tuples are separate from a universal runtime support promise.

The native createExpressServer(optionsJson,app) owns request dispatch before app
parsers, and rejects upgrade/CONNECT events before any application callback.
Operators must not add competing request/upgrade/connect listeners or mutate
the authorized routing after installation. The advanced express() middleware
alone cannot secure independent server upgrade listeners; those modes remain
unqualified without the owned server or a qualified Rust edge. Public management
is never mounted. Replay delegates only data/readable/end events, preserves other
request events and chaining identity, and supplies bounded Symbol.asyncIterator.
Fetch reconstructs from the original Request plus sanitized headers/verified body
to preserve request metadata; later HEAD/304/metadata edits await validation.

Accepted delivery receipt proposal: target/node-native/qualification.json,
schema humanymous.node-native-qualification/1. Exact fields source_sha,
package_manifest_sha256,package_version,provenance_sha256,full_native_qualified,
host_runs,authority_docker,advertised_tuples. Host runs retain runtime/version/
os/arch/status/test_count/failed/ignored,authority_artifact_sha256,evidence_profile,
verdict_counts with separate ALLOW/CHALLENGE/DENY/error and origin_effect_count.
Only production-application-http runs qualify that limited profile; synthetic
trusted evidence for real-engine/audit ALLOW compatibility is separate and never
qualifies browser/capture. Every advertised tuple is qualified and exactly
matches capabilities.releaseQualifiedTuples; empty/unavailable receipts block
publication. The CI owner consumes same-run immutable archive/receipt/provenance
hashes rather than rebuilding during publish. No qualified receipt exists yet.

Pinned upstream NAPI CLI3.10.12 generates ESM/CJS loaders and declarations from
Rust exports/metadata; these are explicitly upstream generated glue, not authored
application scripts. Host-pilot artifacts, legacy JS/tests, private evidence and
node_modules are excluded from the production archive. Native loader overrides
and installed host environment remain supervisor-controlled. No npm publication
or complete native/Docker product qualification follows from generated glue.

### Transport and authentication

POST `/adapter/v0.1/decision` over HTTP/1.1 on a configured absolute filesystem
Unix socket or local Windows named pipe. No TCP fallback, URL, DNS, redirect,
abstract socket or remote UNC pipe is allowed. Content-Type is application/json;
Content-Length is exact UTF-8 bytes. Bearer credentials contain 32..256 ASCII
base64url characters and are provisioned independently of admin credentials.
The service checks credential scope (installation, adapter, tenant, origin,
revision), actual IPC peer identity and per-peer bounded concurrency/rate.
The endpoint is separate from administration; a decision token never authorizes
admin operations. Unix socket parent and mode/owner, Windows restrictive DACL
and PIPE_REJECT_REMOTE_CLIENTS are service/install responsibilities. The Node
client cannot attest those ACLs: missing verified peer/ACL provisioning blocks
production qualification. Loopback-looking headers are never peer evidence.

### Framing and schema

UTF-8 JSON with no BOM, invalid UTF-8, duplicate object keys, non-finite numbers,
unknown fields or unsupported version. Objects are unordered; arrays preserve
order and duplicates. JSON escaping has no signature semantics; no bespoke
cryptography is introduced. Rust must use a duplicate-key-rejecting parser,
validate the schema, and never deserialize through a permissive map first.
Request envelope maximum 1,600,000 bytes; response maximum 65,536 bytes.
Integer values are JSON safe integers; timestamps are UTC Unix milliseconds.
All fields listed below are mandatory; omitted is invalid. Only peer, connectionId, capture and stream
permit null. IDs/identity strings are bounded; no arbitrary debug payload.

Request exact fields:

| Field | Type and meaning |
| --- | --- |
| protocol | literal humanymous.adapter/0.1 |
| adapter | exact object id=humanymous.node, version=0.1.0-experimental.1, installation=1..128 ASCII identity characters; service authenticates declared identity/version against provisioned credential |
| connectionId | UUID v4 for actual Node application socket (WeakMap lifetime), null in Fetch; scoped to installation/credential epoch, never evidence of original-client connection behind a proxy |
| capture | null; authenticated raw-client capture absent in this version |
| id | lowercase canonical UUID v4, server-generated adapter request nonce |
| tenant, origin, revision | each 1..128 ASCII `[A-Za-z0-9._:-]`, configured identity, never client header |
| deadline | safe integer, future, at most 30 seconds after receipt; bounded clock skew installation check |
| method | HTTP token, uppercase method received, at most 32 bytes |
| target | origin-form path plus untouched query, 1..8192 UTF-8 bytes, no fragment, whitespace, NUL, backslash, encoded slash/backslash/NUL or dot segments |
| authority | request authority, 1..1024 ASCII bytes, no userinfo/whitespace; Rust validates configured origin authority and Host consistency |
| peer | Node socket.remoteAddress string or null for Fetch; application-connection peer only, never original-client claim |
| headers | ordered arrays `[name,value]`, each ASCII token name and string value without CR/LF/NUL; at most 100 entries and 32768 combined UTF-8 bytes |
| headerMode | node-raw or fetch-normalized; Fetch coalescing/casing loss explicitly represented |
| body | exact object layer, complete, length, sha256, base64 |
| capabilities | exact boolean object rawTls=false, originalH2=false, rawHeaderOrder=(headerMode==node-raw), completeBody=true |
| stream | null, no upgraded/streaming request authorization in 0.1 |

Body layer is literal `transfer-decoded-content-encoded`: chunk framing removed
by host HTTP parser; compressed entity bytes remain compressed, no JSON parsing
or multipart reconstruction. complete=true, length=0..1048576, sha256=64 lower
hex chars (including empty body), base64=canonical padded RFC4648 standard
base64 of exact bytes. Rust decodes canonically and recomputes length/digest,
then runs existing RIT as applicable. Transfer/framing trailers are unsupported.
Reject conflicting/duplicate Content-Length, TE+CL, invalid authority/Host,
invalid framing/obs-fold and unsupported connection fields at ingress; runtime
HTTP parser rejection does not qualify normalized Fetch as raw header capture.
Request Authorization/Cookie values are necessary evidence only inside local
authenticated IPC; do not log/persist envelope bodies or credentials.

Internal `x-humanymous-*`, forwarded/x-forwarded-*, cf-connecting-ip,
true-client-ip and x-real-ip are removed before decision and application. They
cannot grant locality or capture capability. Original trusted capture, when
later specified, requires Rust-owned authenticated connection/stream binding;
this version cannot elevate it from headers.

### Binding, replay and errors

The service binds each id to authenticated installation/adapter peer plus
tenant/origin/revision/method/target/authority/ordered sanitized headers/body
digest and complete body state. It validates current configuration and required
capabilities before scoring. Missing required signals are an input error, never
fabricated ALLOW. Store nonce atomically until at least deadline plus configured
clock skew; reject any duplicate (including same payload) with 409. No retries
or decision caching. Restart/reload requires durable replay state or fresh scoped
credentials/configuration epoch; unavailable store fails closed. Cross-tenant,
route, body, session or connection reuse cannot create another origin effect.
The adapter retains result only within the outstanding call and invokes origin
once after matching the response; its own id is never adopted from trace headers.

Successful HTTP 200 exact response fields: protocol (same literal), id, tenant,
origin, revision (all equal request), expires (safe integer > now and <= deadline),
verdict (ALLOW, DENY or CHALLENGE). No response mutation, arbitrary Location,
script, admin data or application header injection. ALLOW permits one invocation
of the already-bound handler and body. DENY produces 403; CHALLENGE produces
409 JSON and no origin effect. Browser challenge recovery is unqualified until
the Rust browser data-plane endpoints and session contract land.

Transport/input errors are non-200: 400 schema/framing, 401 credentials, 403 scope,
409 replay/config binding, 413 size, 422 required capability/body verification,
429 capacity, 503 audit/state/service dependency. Error body is bounded and not
interpreted by the adapter. Any non-200, malformed/mismatched/expired response,
disconnect, cancellation or timeout becomes 503 no-store, no origin invocation.
Cancellation closes IPC request; service discards uncommitted computation but
never rolls back a committed nonce/audit as though another request could reuse it.

### Lifecycle and capability boundary

Adapters are installed before body parsers and every protected origin effect.
Explicit complete route inventory is checked against reserved namespace
`/_humanymous` (configurable single path segment). New/unknown routes fail closed.
The reserved subtree is intercepted and unavailable in 0.1, never forwarded to
an application or IPC admin method. Collector/challenge route serving requires
a future explicit versioned data-plane contract. Express root mounting only;
mount prefixes/rewrites after authorization are prohibited. Fetch target is
URL-normalized and cannot claim original-target fidelity; signed/raw-target
routes require Rust edge, not Fetch-only middleware.

Bounded input consumption awaits decision before origin effects. No streaming
upload, CONNECT, WebSocket or gRPC qualification. SSE/download response streams
pass through without interception/buffering; application owns cancellation and
backpressure after authorization. Request trailers reject. No retry after an
origin call. Hono uses an outer Fetch wrapper, then app.fetch on the authorized
request. Node stream adapter preserves same IncomingMessage identity for Express
compatibility; replay behavior needs tested Node/Bun version qualification.

HTML insertion is an explicit template helper only: caller supplies CSP nonce
and serves reviewed script under its reserved data-plane integration. It returns
escaped-by-validation markup and CSP nonce source, does not rewrite CSP/HTML,
ETag/Content-Length/compression/cache/HEAD/204/304. Helper availability does not
mean collector script is served by this experimental version. No raw TLS/H2/H3,
frame inspection, audit durability or admin capability is invented by middleware.

## Why

The shared authority needs an implementable exact wire boundary before Node and
nginx can connect. Limiting v0.1 avoids new token crypto and false claims of
whole-stream or browser recovery support. Public proposed-model standards remain
separate from this internal experimental contract.

## Verification

Node/Bun real Express/Hono fixtures use an explicitly mock decision authority.
They are conformance/compatibility pilots, not full Rust or Docker e2e authority.
Actual Rust peer/ACL/auth/replay/capability/audit integration, native OS tuples,
real browser recovery and isolated Docker acceptance remain release blockers.
Official lifecycle references: https://nodejs.org/api/http.html,
https://expressjs.com/en/5x/api/, https://hono.dev/docs/api/hono,
https://bun.sh/docs/runtime/http/server (consulted 2026-11-07).
