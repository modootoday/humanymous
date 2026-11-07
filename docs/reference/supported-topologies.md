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

See the [Rust Node adapter API](node-adapter.md) for current interfaces and
qualification limits. Its release-qualified platform list remains empty.

### Node-API options and explicit template ownership

See [options and template ownership](node-adapter.md#node-api-options-and-explicit-template-ownership).

### Node request ownership and outcomes

See [request ownership and outcomes](node-adapter.md#node-request-ownership-and-outcomes).

### Node package qualification boundary

See [package qualification](node-adapter.md#node-package-qualification-boundary).

## Experimental native nginx component

See the [native nginx adapter reference](nginx-adapter.md) for the experimental
Linux profile, historical component evidence and outstanding current-source gates.

### nginx provisioning and request projection

See [provisioning and request projection](nginx-adapter.md#nginx-provisioning-and-request-projection).

### nginx dispatch and resource limits

See [dispatch and resource limits](nginx-adapter.md#nginx-dispatch-and-resource-limits).
