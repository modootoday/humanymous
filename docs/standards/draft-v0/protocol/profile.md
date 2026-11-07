---
layout: doc
title: Proposed evidence and decision profile
description: Candidate interoperability requirements and unresolved wire-format questions.
permalink: /standards/draft-v0/protocol/profile/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---

# Proposed evidence and decision profile

**Diátaxis quadrant:** Reference. **Audience:** collector, gateway and adapter implementers.

**Status:** independent project draft. This is a proposed requirements model,
not a complete wire specification or adopted standard. No implementation can
claim conformance until the unresolved encoding and validation work is complete.

## Requirement language

The [candidate terminology](terminology.md) defines the actors, provenance and
binding meanings used below.

The key words MUST, MUST NOT, SHOULD, SHOULD NOT and MAY are interpreted as
described in [BCP 14](https://www.rfc-editor.org/rfc/rfc8174.html) only when
they appear in uppercase. They express candidate requirements of this project
profile, not a statement that an external standards body has approved them.

Requirement identifiers `P-01` through `P-10` label the candidate requirements
of this **profile**. They are stable project-draft references, not priority
levels or identifiers assigned by a standards organization. Each requirement
has a descriptive heading; the [validation matrix](../validation/conformance.md#required-corpus)
links to those headings.

## Actors and deployment profiles

A **collector** supplies observations with provenance. A **decision service**
evaluates those observations under a named policy. An **enforcing adapter**
binds the response to the pending request before invoking the origin. An
**origin** executes the application action. Administration is a separate local
trust domain and is not a participant in public request routing.

| Profile | Available evidence boundary |
| --- | --- |
| Direct edge | Original client connection when actually captured at that edge |
| Trusted terminator | Authenticated facts from a configured terminator, separate from downstream connection facts |
| Application | Framework observations after its server's processing and normalization |
| Managed runtime | Only the metadata and lifecycle hooks provided by that host |

### P-01: Observation provenance

A producer MUST identify whether a fact is directly server-observed,
authenticated-forwarded, application-asserted, browser-asserted, derived or
not observed. A consumer MUST NOT promote an unauthenticated assertion into a
server observation.

### P-02: Required capability availability

A producer MUST declare unavailable and partial capabilities. A consumer
MUST reject a protected route's readiness when a required capability cannot be
provided; it MUST NOT invent a fingerprint or silently substitute another peer.

## Evidence model

| Group | Required semantics before wire freeze |
| --- | --- |
| Identity | Schema/profile version, authenticated producer, installation, tenant and node |
| Causality | Internally assigned event, connection, request/stream, parent and decision references |
| Time | Server receipt time, expiry and monotonic duration; client clock remains an assertion |
| Request | Method, original target and authority, final route and origin identity |
| Headers | Duplicate/order/casing provenance and relevant component commitments |
| Body | Exact selected byte/encoding layer, length, digest, complete/partial status and capture limit |
| Credential | Verified type, principal reference and authorized scope; never the private key or bearer token |
| Quality | Dropped/truncated/overflow state and reason, separate from the security outcome |

### P-03: Request and decision binding

The receiver MUST bind evidence and decisions to producer, tenant,
connection and request/stream, method, target, relevant body/header commitments,
final route/origin, policy revision, freshness and a replay-resistant correlation
value. It MUST reject mismatched, expired, oversized or unsupported envelopes.

### P-04: Connection and stream isolation

A connection-level observation MUST NOT be reassigned to another
connection, tenant or unrelated multiplexed stream. A client-supplied request or
trace identifier MUST NOT itself establish authoritative identity.

### P-05: Idempotent delivery

Retransmission MUST NOT duplicate budget consumption, audit commitment
or an origin side effect. A cached decision MUST retain its original binding.

The following illustrates relationships only. It is not a normative encoding:

```json
{
  "schema": "humanymous.evidence.v0",
  "tenant": "example-tenant",
  "request": "example-request",
  "stream": "example-stream",
  "route": "example-upload-route",
  "source": "direct_server_observed",
  "body": { "state": "complete", "byte_layer": "received-content" },
  "quality": { "dropped": false }
}
```

This illustrative object omits required authentication and commitments and MUST
NOT be used as a production request or treated as an accepted complete envelope.

## Decision and effect boundary

### P-06: Decisions and errors

A decision MUST distinguish ALLOW, CHALLENGE and DENY from invalid input,
missing capture, dependency failure and timeout. An adapter MUST NOT translate
an unknown decision or required-version mismatch into ALLOW.

### P-07: Complete-body verification before effects

Where complete-body integrity is required, verification MUST finish
before forwarding body bytes or invoking an origin handler that can cause side
effects. Buffering or spooling MUST have explicit size/time/resource bounds.

### P-08: Streaming integrity lifecycle

Unbounded streaming, WebSocket and streaming remote procedure calls
MUST use a separately specified authorization/integrity lifecycle. A unary or
partial-body check MUST NOT be described as whole-stream integrity.

### P-09: Decision containment and revalidation

Decisions MUST carry only explicitly permitted mutations and challenge
references. They MUST NOT carry management credentials or operations. Final-route
mutation after authorization MUST trigger revalidation or be rejected.

### P-10: Audit independence and visible loss

Security audit and required admission controls MUST NOT depend on an
untrusted trace sampling flag. Observation/export loss MUST remain visible.

## Compatibility and unresolved wire rules

Existing humanymous request-integrity tokens retain their existing compatibility
contract. This proposal does not silently replace them with a new signature
profile or modify product detection scores.

Before wire freeze, specify field types and limits; absent/null/empty semantics;
Unicode and canonicalization; duplicate/unknown fields; digest/signature coverage;
authentication and key discovery; negotiation; error codes; replay storage;
stream handling; and extension/deprecation. These are open blockers, not defaults
that each adapter may choose independently.

## IANA considerations

This project draft requests no assignment. No HTTP field, media type, well-known
URI or other registry identifier is claimed allocated. Any future request needs
the relevant registry's policy and a reviewed specification.

## References

Candidate normative dependencies: [HTTP semantics](https://www.rfc-editor.org/rfc/rfc9110.html),
[BCP 14](https://www.rfc-editor.org/rfc/rfc8174.html),
[HTTP Message Signatures](https://www.rfc-editor.org/rfc/rfc9421.html),
[digest fields](https://www.rfc-editor.org/rfc/rfc9530.html).
The exact adopted transport profile remains open.

Informative context: [Trace Context](https://www.w3.org/TR/trace-context/),
[robots](https://www.rfc-editor.org/rfc/rfc9309.html),
[current Web Bot Auth Internet-Draft](https://datatracker.ietf.org/doc/draft-ietf-webbotauth-httpsig-protocol/).
