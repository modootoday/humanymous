---
layout: doc
title: Edge-to-origin contract
permalink: /reference/gate-origin-contract.html
reviewed: 2026-11-07
---
# Edge-to-origin contract

**Diátaxis quadrant:** Reference. **Audience:** application and proxy integrators.

The origin must reject direct bypass of the enforcement path. Strip or reject
externally supplied internal decision headers; only authenticated configured
peers may assert bound observations. A generally qualified application adapter
must preserve legitimate application credentials according to its declared profile.

## Experimental loopback runtime exception

The current bounded runtime reserves and binds `Authorization` for its own
runtime capability, then strips it before forwarding. A separate upstream
application `Authorization` profile is unsupported. Do not place this slice
over an arbitrary existing application and assume its authentication survives.

It accepts bounded complete HTTP/1.1 requests, with 64 KiB request and origin
response body limits and a 16 KiB header bound. Transfer-encoded/chunked requests,
upgrades, duplicate request headers and encoded/dot-segment paths are unsupported.
Validated duplicate `Set-Cookie` response headers are preserved as opaque origin
frames. Origin effects follow durable audit append and deadline/liveness checks;
an ambiguous origin failure is not retried.

The separate local IPC authority has a different capability boundary: its
currently reviewed route path rejects required browser collection, request-
integrity, original TLS and HTTP/2 planes. The loopback runtime's session/integrity
implementation must not be attributed to that IPC profile. See
[configuration](cli-config-policy.md) and [implementation mapping](../standards/draft-v0/validation/implementation-map.md).

Keep method, target/query semantics, duplicate header meaning and exact body
representation intact. Complete-body integrity finishes before any origin
effect. Bound buffering and timeouts; do not retry non-idempotent requests after
partial forwarding. Streaming needs its own tested contract.

Management is never an origin-facing public route. The Rust forwarding and
adapter implementations require conformance and actual deployment pilots before
these properties can be claimed for a release. See [compatibility](../explanation/will-this-break-my-app.md).
