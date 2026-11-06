---
id: sot:root/adapter-service-contract
title: Experimental adapter decision service protocol
status: active
created: 2026-11-06T22:15:00Z
updated: 2026-11-06T22:15:00Z
verify: manual
rel:
  governs: [adapters/node/**, adapters/contracts/**]
  depends_on: [sot:root/adapter-ecosystem]
---

# Experimental adapter decision service protocol

## Rule

`humanymous.adapter/0.1` is a private experimental application protocol, not
Draft v0, an RFC, or a replacement for frozen RIT. Rust alone owns decisions,
capture provenance, replay state, credentials, policy, challenge and audit.
Node does not compute scores, verify RIT or mint security receipts. Version
0.1 authorizes bounded complete HTTP requests only. No stream authorization,
remote transport, capture upload or admin method exists in this version.

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
