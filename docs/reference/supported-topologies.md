---
layout: doc
title: Deployment capability boundaries
permalink: /reference/supported-topologies.html
reviewed: 2026-11-07
---
# Deployment capability boundaries

**Diátaxis quadrant:** Reference. **Audience:** infrastructure integrators.

These are qualification profiles, not a list of released Rust deployments.

| Profile | Boundary |
| --- | --- |
| Direct TLS edge | Original handshake and protocol observations require capture before normalization |
| CDN or load-balancer origin | Upstream TLS is not the client's TLS; original facts are unavailable without an authenticated supported channel |
| Framework middleware | Request lifecycle hooks do not imply raw network or response-transformation capability |
| Managed edge hook | Provider observations, body access and execution limits require explicit qualification |
| Native server module | Worker, reload, application binary interface (ABI) and ownership behavior need independent pilots |

Required unavailable capabilities prevent route readiness. A deliberately limited
profile must declare its gaps. HTTP/3 capture is not qualified by an HTTP/2
implementation. Keep management local and authenticated in every profile.
See the [integrator guide](../start-here/integrator.md).

## Rust network components versus a live capture path

The Rust network library transforms supplied observations. Its presence does
not mean the loopback runtime or an application adapter captures original TLS,
HTTP/2 or TCP data.

| Component | Implemented boundary |
| --- | --- |
| `tls::parse_client_hello`, `tls::parse_record` | Observe supplied handshake/record prefixes. Legacy tolerance can accept a declared length beyond captured bytes or incomplete trailing extensions; success is not strict handshake validity or complete capture |
| `http2::Fingerprint` | Retains supplied SETTINGS/header/pseudo-header ordering; constructing a value does not prove a complete connection preface or header block was captured |
| `http2::FrameMonitor` | Bounded passive frame accounting over supplied bytes, not an HTTP/2 server or complete stream validator |
| `http::HeaderInfo` | Carries explicit order/casing reliability flags from the capture owner; a normalized header list must not be relabeled reliable raw evidence |
| `report::Observation` and `report::build` | Assemble supplied observations and operator classifications into the compatibility report; absent TLS emits an explicit unobserved-plane marker |

Record reassembly, strict completeness validation, trusted capture provenance and
connection/request binding remain responsibilities of the actual ingress owner.
A structure, parser success or server-source label cannot authenticate its own
input. Forwarded client IP and operator-supplied proxy/datacenter classifications
are not automatically independently observed original-peer facts.

The compatibility report field `negotiated_alpn` is currently populated from the
first offered ALPN entry in the supplied ClientHello. That field name must not be
read as proof of the protocol actually negotiated with the server. Preserve the
captured byte layer and source when evaluating it; do not silently reinterpret
this compatibility behavior as a newly qualified observation.

The current runtime's missing original TLS/HTTP/2 capability remains a readiness
limit. See [implementation status](production-vs-reference.md) and the
[origin contract](gate-origin-contract.md) before selecting a deployment profile.

## Experimental Rust Node-API framework bridge

Current source exposes the following interfaces. Release qualification is still
in progress: `capabilities()` reports an empty `releaseQualifiedTuples` list.
Earlier component runs do not qualify later source changes or every Node/Bun,
OS, architecture and framework combination.

| Interface | Integration boundary |
| --- | --- |
| `createExpressServer(optionsJson, app)` | Owns the HTTP server around an existing Express app; enforcement precedes app parsers/effects and unsupported upgrade/CONNECT events reject |
| `express(optionsJson)` | Advanced root middleware, installed before every parser/effect; independently installed upgrade listeners are outside its coverage |
| `hono(optionsJson, app, streamModule)` | Outer Fetch handler; requires the real upstream `node:stream` module and validates the bounded complete request before calling the app |
| `htmlHook(nonce, scriptPath)` | Explicit template markup and CSP source only; serves no collector script and performs no automatic response or CSP rewriting |
| `capabilities()` | JSON description of available observations and lifecycle limits; not a deployment qualification receipt |

Do not add competing server listeners or mutate protected routing after binding.
Options select a complete route inventory and an independently provisioned local
IPC authority peer and credential. There is no TCP or administrative fallback.
The bridge delegates decisions to the shared authority; it does not add another
scoring engine or inherit the loopback runtime's browser request-integrity API.

Node HTTP/1 raw observations retain declared order/casing. The Bun HTTP shim uses
normalized evidence, and Hono Fetch observations are normalized; neither can be
promoted into strict raw-header or original TLS/HTTP2 evidence. Missing required
planes prevent readiness. Complete request bodies are bounded to 1 MiB, with
64 process-wide body/decision owners; these are resource limits, not interaction
speed or motor-richness requirements.

Application-owned response streams are distinct from authenticated upload
streaming. WebSocket, CONNECT, gRPC and request trailers remain unqualified.
Application credentials/cookies and response cookie meaning belong to this
adapter's preservation contract; do not confuse it with the separately selected
cookie-free runtime browser profile. Actual source-bound authority/origin,
packaging and platform pilots remain required before rollout.

### Node-API options and explicit template ownership

`optionsJson` requires every field below and rejects unknown fields. Values
identify the independently provisioned authority scope; they are not discovered
from untrusted request headers.

| Fields | Source-level bound or responsibility |
| --- | --- |
| `installation`, `tenant`, `origin`, `revision` | Nonempty ASCII scope identities, up to 128 bytes each |
| `endpoint`, `server`, `credential` | Local Unix filesystem socket or Windows named pipe, expected authority peer and privately provisioned credential; credential is 32–256 ASCII letters/digits/underscore/hyphen |
| `routes` | Nonempty inventory of at most 4096 pathnames; excludes query/fragment/percent/backslash forms and the reserved `/_humanymous` subtree |
| `timeoutMs` | 1–30000 milliseconds per decision operation; not a requirement for a person to finish an interaction within that interval |
| `maxBody` | 0–1048576 bytes of complete request body; zero permits an empty body only |

A configured route alone does not authorize a request: the shared parser and
actual authority still validate the envelope, required observations and scope.
Request bodies are committed as transfer-decoded/content-encoded bytes using
exact length, SHA-256 and standard padded Base64. Internal and forwarding trust
assertions are removed before the shared parser consumes the envelope.

`htmlHook` accepts a 16–128-byte nonce from its restricted ASCII alphabet and a
bounded same-origin absolute `.js` path, then returns `markup` and `cspSource`.
It does not generate randomness, provision a script or install the CSP policy.
The application must provide an appropriate nonce, serve the intended script
and insert both outputs into the same eligible application-owned response.
A generated tag for a reserved path does not make that path available through
the adapter. Template insertion is not evidence of authenticated collection.

### Node request ownership and outcomes

The Express guard refuses an already-ended request, an existing `body` property,
text-decoded input or a non-HTTP/1 request. Install it before every parser and
effect. After complete-body verification and a bound, unexpired ALLOW, it replays
the buffered bytes through request body methods/events while retaining the
original request for other events. Compatibility with custom request mutation
still requires an actual application pilot.

The Hono wrapper refuses an already-used body or aborted signal and constructs
an authorized Request from the original with sanitized headers, verified body
bytes and the original signal. That source path is separate from proof that
every host preserves metadata and cancellation correctly. Neither path streams
an unverified upload to the application.

| Outcome before the application call | Adapter response |
| --- | --- |
| Bound, unexpired ALLOW | Invoke the application; its own status and response stream remain application-owned |
| DENY | 403 JSON verdict |
| CHALLENGE | 409 JSON verdict; no injected challenge HTML or implemented recovery service |
| Input, body, capacity, dependency or decision failure | 503 JSON error |

Adapter-generated JSON responses request `Cache-Control: no-store`. The owned
Express server separately rejects upgrade/CONNECT with an empty no-store 503
response. These mappings are implementation-specific, not generic Draft v0
status assignments. A 503 is not a DENY verdict.

Source checks cancellation and decision expiry before entering the application
and consumes the bound decision once. It does not implement business-operation
idempotency or retract an application effect after dispatch. Response streaming
remains handler-owned; permit release is not proof that a download completed,
a client received it or a side effect was rolled back. Actual abort/race/timeout
and accessible recovery outcomes remain execution gates.

### Node package qualification boundary

The package manifest points CommonJS and ESM imports at upstream-generated
native loaders and separate type declarations. Its explicit archive list excludes
legacy handwritten source/tests and host-pilot programs. Missing generated output
or a missing native binary blocks the package inspection path; compilation alone
does not establish that an installable archive is correct.

The Rust package driver owns generation/inspection exclusively, invokes the
pinned upstream generator with locked Cargo resolution, checks generated LF text
and provenance markers, typechecks declarations and loads both entrypoints in
Node and Bun. Inspection writes a `component-only` provenance record with file
hashes. It is not a read-only command or a complete framework/authority pilot.
Do not use it as a supposedly lightweight local check under a runtime hold.

`engines.node >=22` is an installation floor. Configured native build targets,
loader import success and provenance hashes do not establish release-qualified
OS/architecture/runtime tuples. A source-bound qualification receipt and actual
authority/origin, archive and platform gates remain required; no npm publication
or generally supported installation is implied here.

## Experimental native nginx component

A Rust nginx module now exists for an initial Linux HTTP/1.1 profile. It uses
an upstream nginx C engine through the pinned ngx-rust SDK; it is not a Rust
replacement for the engine. Default library unit tests do not qualify the native
ABI, nginx binary/build flags or a deployed configuration.

The `humanymous_guard` directive consumes supervisor-provisioned private
configuration. The authority must independently select the nginx producer
profile. The ingress projection binds an exact method/path inventory, Host,
complete body and scope, then reuses the shared authority client. A generic
parser accepting nginx envelopes is not proof of native module execution.

The declared profile bounds complete uploads to 1 MiB. Non-HTTP/1.1 requests,
file-backed/incomplete bodies, transfer-coded uploads, trailers, upgrades and
gRPC are outside its support. Internal/forwarding assertions are sanitized;
original TLS/H2 capture, HTML/CSP insertion and response-body filtering are not
provided. Operator configuration remains trusted: prevent post-authorization
rewrite and origin bypass, disable proxy retry/cache, and apply the declared
buffering and upstream-control-header restrictions before claiming deployment
isolation or unchanged handler-owned streaming.

DENY maps to 403. Working source returns a no-store 409 JSON
`challenge_unavailable` response for CHALLENGE; that is not an accessible
recovery service. The JSON/timer follow-through has not been qualified by the
historical 35-group Compose result. Historical positive ALLOW pilots used
explicit test-only synthetic evidence; the actual production application-HTTP
profile's 409/no-origin observation does not prove a successful human flow.
Independent witness, cross-user isolation, sustained backpressure and full
browser/human/assistive-technology qualification remain separate outstanding gates.
