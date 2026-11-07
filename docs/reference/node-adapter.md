---
layout: doc
title: Experimental Rust Node adapter API
description: Express and Hono interfaces, options, lifecycle and qualification limits.
permalink: /reference/node-adapter.html
reviewed: 2026-11-07
---
# Experimental Rust Node adapter API

**Diátaxis quadrant:** Reference. **Audience:** Node/Bun application integrators.

Review the [deployment boundaries](supported-topologies.md) before selecting
this experimental profile.

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

`optionsJson` requires the scope, peer, route and limit fields below and rejects
unknown or duplicate fields. Values
identify the independently provisioned authority scope; they are not discovered
from untrusted request headers.

| Fields | Source-level bound or responsibility |
| --- | --- |
| `installation`, `tenant`, `origin`, `revision` | Nonempty ASCII scope identities, up to 128 bytes each |
| `endpoint`, `server`, `credential` | Local Unix filesystem socket or Windows named pipe, expected authority peer and privately provisioned credential; credential is 32–256 ASCII letters/digits/underscore/hyphen |
| `routes` | Nonempty inventory of at most 4096 pathnames; excludes query/fragment/percent/backslash forms and the reserved `/_humanymous` subtree |
| `timeoutMs` | 1–30000 milliseconds for the bounded body/decision lifecycle; not a requirement for a person to finish an interaction within that interval |
| `maxBody` | 0–1048576 bytes of complete request body; zero permits an empty body only |

The current shared configuration also accepts optional
`syncInputDeadlineEnforced`, defaulting to `false`. This is a declaration for
synchronous host integrations; setting it does not install a server-side input
deadline or qualify Node input timing. The Node bridge now shares configuration,
envelope projection and process admission with the native framework adapters.
Earlier Node component results must be rerun against that refactor; sharing the
implementation does not qualify another framework or platform.

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

The pending request's deadline starts before body collection. Body completion
does not grant a new request deadline: the authority binding carries the original
deadline and dispatch compares the returned expiry with it. The configured IPC
timeout is not permission to invoke the application after that original budget
has expired. This remains source-level lifecycle behavior pending the actual
body/IPC/host timing pilots.

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

## Component container boundary

The current host-pilot contract requires five harness groups: three actual-host
groups and two framing groups, with no ignored or filtered groups. Its four
Node/Bun × Express/Hono tuples require 12 distinct result rows and 60 actual
cases: four production-authority non-ALLOW cases, 28 synthetic-evidence ALLOW
cases, 16 synthetic-authority negative cases and 12 unavailable-authority cases.
These are required denominators, not current passing results. Synthetic ALLOW
exercises the host path without establishing production browser provenance;
neither these cases nor five passing harness groups qualify an npm package or
the whole product. Preserve missing rows and unreached cases as incomplete.

The Node pilot image contains pinned Node/Bun hosts, Rust build inputs, generated
preload glue and compiled host tests plus the actual authority executable. Its
entrypoint runs the native host test driver; it is not an application runtime
image or a full browser deployment. The Dockerfile selects a non-root UID, but
network isolation, read-only filesystem and resource bounds must also be applied
and verified by the caller. Copying those settings into a test description does
not establish that they were effective in a particular run.
