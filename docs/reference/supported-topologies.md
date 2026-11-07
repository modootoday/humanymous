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
