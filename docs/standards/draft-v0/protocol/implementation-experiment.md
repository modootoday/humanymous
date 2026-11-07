---
layout: doc
title: Experimental adapter wire and implementation boundary
description: Concrete adapter protocol behavior, separate from the developing Draft v0 wire profile.
permalink: /standards/draft-v0/protocol/implementation-experiment/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---
# Experimental adapter wire and implementation boundary

**Diátaxis quadrant:** Reference. **Audience:** adapter implementers and protocol reviewers.

**Status:** informative implementation experiment. The literal protocol
`humanymous.adapter/0.1` is a bounded application protocol, not the generic
Draft v0 encoding, a registered identifier or an adopted standard. Parser
acceptance, actual authenticated service execution and origin effects are
different evidence. No Draft v0 conformance follows from this page.

## Transport and framing

The local service uses `POST /adapter/v0.1/decision HTTP/1.1` over a configured
filesystem Unix socket or local Windows named pipe, with one closed connection
per request. It has no TCP fallback or administration method. Authentication
checks the real operating-system peer and a separately provisioned scoped
bearer credential; forwarding headers do not establish locality.

The current authority creates a fresh process-epoch credential from 32 OS-random
bytes, encoded as 64 lowercase hexadecimal characters. Its presentation is the
exact case-sensitive `Bearer ` prefix followed by those bytes. The private
provisioning constructor checks that representation; it does not establish a
caller's entropy, authorization or supervisor identity. This local implementation
grammar is not a generic Draft v0 authentication or discovery scheme.

The service header bound is 16384 bytes including the terminating delimiter,
with at most 100 service headers. The JSON envelope bound is 1600000 bytes and
response bound 65536 bytes. Host, Authorization, `Content-Type: application/json`
and exact Content-Length are required. Repeated Authorization, Host,
Content-Length, Content-Type and Connection headers reject; other service-header
duplicates remain ordered input for the handler's policy. Transfer-Encoding, TE,
Trailer, Expect and Upgrade reject. Content-Length uses unsigned decimal without
leading zeroes, except the single digit zero. Service header names are ASCII
tokens; values are ASCII with permitted horizontal tabs trimmed at their edges.
These are service framing rules, not a claim to preserve arbitrary origin bytes.

The decoder rejects truncated input and bytes already buffered beyond the declared
entity. It reads the exact entity without waiting for EOF; it does not prove that
no additional bytes could arrive later. The caller closes the connection and
does not interpret a second response. Request Connection values of close or
keep-alive are syntactically accepted, but do not enable persistent multiplexed
service requests; responses require Connection: close. Callers supply the
whole-connection deadline; framing alone does not
install a timer.

## Local service lifetime and admission

The shared listener validates capacity of 1–128, rate of 1–10000 admissions per
monotonic one-second fixed window and a connection timeout of 1–30000 ms.
This is one provisioned peer per service instance, not an unbounded per-client
rate map or a sliding-window guarantee. Overload rejection has a separate 10-ms
drain/write budget, so an overloaded connection may close without a complete 429.

The service applies the connection deadline to read, handler and write work.
While the handler runs, the connection also watches for extra input or closure
and marks its context cancelled. Blocking work can outlive the connection future;
its context retains the admission permit until that work really ends. Handlers
must recheck liveness before effects and after durable work. Timeout is not proof
that a worker stopped or an already committed effect rolled back.

Controlled shutdown stops acceptance, cancels tracked connections and waits for
their tasks and admission-owning workers within an explicit drain budget. A drain
timeout is an error, not a graceful-stop acknowledgement. Reusing this transport
for another local handler does not grant access to administration through the
decision endpoint; each handler owns its separate credential, scope and route
authorization. These are source contracts awaiting deployment-specific evidence.

## JSON model

The parser accepts fixed-schema UTF-8 JSON. It rejects a byte-order mark,
invalid UTF-8, duplicate keys including escaped duplicates, unknown fields and
missing fields. Nullable is not optional. Objects are not a canonical signature
encoding; no new signing algorithm or JSON canonicalization is defined here.

| Group | Implemented fields and bounds |
| --- | --- |
| Version/producer | Exact `protocol`; `adapter` with provisioned id/version and installation identity |
| Identity/scope | `id` is a canonical lowercase UUID version 4; `tenant`, `origin`, `revision` and installation use bounded ASCII identities, up to 128 characters |
| Deadline | `deadline` is an unsigned safe JSON integer in UTC Unix milliseconds, strictly after the supplied clock and at most 30000 milliseconds ahead |
| Request | `method` is an uppercase token up to 32 bytes; `target` is origin-form up to 8192 UTF-8 bytes; `authority` is bounded ASCII; ambiguous escapes/dot segments reject |
| Claimed connection | Required `peer` and `connectionId` are string-or-null; neither establishes original-client or kernel-authenticated provenance |
| Unsupported planes | `capture` and `stream` are required and explicitly null |
| Headers | `headers` is an ordered array of name/value pairs, at most 100 entries and 32768 combined UTF-8 bytes; CR/LF/NUL reject |
| Observation mode | `headerMode` and `capabilities` must agree with the trusted parser profile; original TLS/HTTP2 claims are unavailable |
| Body | Exact `layer`, `complete`, `length`, `sha256`, `base64`; complete transfer-decoded/content-encoded bytes, up to 1048576 bytes |

The body uses canonical padded standard Base64. The parser decodes it and
recomputes exact length and lowercase SHA-256; it does not parse JSON or
reconstruct multipart content. Inner JSON header values may contain non-ASCII
text, whereas service HTTP framing is constrained separately. This distinction
does not establish lossless arbitrary origin-header byte fidelity.

## Producer profiles and service readiness

The current authority configuration selects the producer profile independently
from request contents; omission defaults to Node. It passes that provisioned
selection to the shared parser. Every identity below requires exact version
`0.1.0-experimental.1`.

| Provisioned `adapter` | Required producer identity | Accepted observation modes |
| --- | --- | --- |
| `node` | `humanymous.node` | `node-raw`, `fetch-normalized` |
| `nginx` | `humanymous.nginx` | `nginx-raw`, `nginx-normalized` |
| `python-asgi` | `humanymous.python` | `asgi-normalized` |
| `python-wsgi` | `humanymous.python` | `wsgi-normalized` |
| `ruby-rack` | `humanymous.ruby` | `rack-normalized` |
| `apache` | `humanymous.apache` | `apache-normalized` |
| `aspnetcore` | `humanymous.dotnet` | `aspnetcore-normalized` |

These are parser and provisioning contracts, not installed framework middleware,
package availability or release-qualified host tuples. In particular, a native
Rust request core does not establish Apache module or ASP.NET Core integration.
The mode names do not authenticate original network capture. Actual native IPC,
server identity, body/cancellation ownership, origin isolation and negative-case
pilots still need source-bound results for each claimed host.
A request must never select its own trusted profile from an unauthenticated field.

The authority's configured route inventory is narrower than the envelope parser:
1–4096 distinct method/path pairs, ASCII paths without percent escapes, query,
fragment, backslash or dot segments, with CONNECT and the reserved namespace
refused. Lookup compares the method and the target portion before `?` exactly;
it does not decode or normalize a path to find another route. Queries remain in
the validated request. A parser-accepted non-ASCII or percent-encoded path cannot
therefore be treated as a currently configurable authority route. Exact configured
Host/authority comparison also preserves case and port rather than supplying a
generic host-normalization rule.

## Private startup and audit composition

The current executable rejects command-line arguments and interactive stdin or
stdout. It reads an EOF-terminated configuration of at most 65536 bytes and opens
audit state before binding the listener. Those checks do not authenticate who
created the redirected handles, prove that they are private pipes or impose a
startup-read deadline. The supervisor owns provenance, secrecy, timeout and
cleanup. Readiness includes a private credential and explicitly reports no
external-parent attestation; it must not be copied into public logs.

Sealed startup uses existing custody through the shared `SealedLog` owner;
inline audit keys are a separately labeled nonproduction component mode.
The library accepts a broker-pinned witness, but the current executable passes
no witness to that seam. A configuration requiring an external witness therefore
refuses startup in this executable; a JSON field cannot instantiate a trusted
witness or prove independent custody. Library support and an actually integrated
supervisor/service deployment remain different qualification boundaries.

The IPC authority rejects routes requiring original TLS, HTTP/2, authenticated
browser collection or request-integrity planes not provided by that service.
The separate loopback runtime has session/request-integrity behavior, but that
does not extend this protocol's capability. See the
[implementation map](../validation/implementation-map.md).

## Replay, effects and errors

The authority checks scope, revision, exact route and capabilities before its
serialized nonce reservation. A duplicate nonce rejects, including an identical
payload. A nonce remains consumed after later failure. Bounded capacity,
retention, clock rollback and credential epoch/restart handling need their own
executed evidence. This is not general exactly-once processing across crashes.
The current nonce map is process-local and retains each reservation through its
deadline plus configured clock skew; expired entries are removed during later
eligible decisions. It is not a durable logical-operation ledger. Clock rollback
poisons the authority, and restart requires a fresh credential epoch rather than
reusing old credentials with an empty replay map.

An accepted service response follows durable audit append. Append failure
poisons the authority. A response contains only protocol/id/tenant/origin/
revision, expiry and verdict, without arbitrary mutations or management data.
Whether an application invokes the bound handler exactly once requires actual
adapter/origin observation. Do not retry an ambiguous origin effect.
A failure after nonce reservation can consume that nonce without producing a
decision audit record. Conversely, a record can persist before the final deadline
check refuses the response. Decision records therefore do not supply a complete
attempt denominator, and a missing successful response does not prove no audit
state changed.

The shared client independently validates the outgoing envelope against its
provisioned profile and pending id/tenant/origin/revision/deadline. It authenticates
the native server and accepts only HTTP 200 with the exact response binding,
known verdict and an expiry after receipt but no later than the request deadline.
The client accepts an earlier expiry; the current authority emits the request
deadline. Unknown, missing or duplicate reply fields
reject rather than creating a usable decision.

The returned `BoundResponse` has single-owner consumption and checks wall-clock
rollback, authority expiry and a monotonic deadline. Inspection of its verdict
is not consumption or a final host-liveness check. Dropping the client future
closes its IPC without automatic retry; this does not retract a previously
performed origin effect. Native peer checks and private credentials are transport
assumptions, not a standalone signed portable decision token.

| Failure family | Current service status |
| --- | --- |
| Malformed envelope/framing | 400; body/capability parser diagnostics may map to 422 |
| Bearer authentication | 401 |
| Actual peer, credential scope or reserved route | 403 |
| Revision, route inventory or replay | 409 |
| Required unavailable capability | 422 |
| Replay capacity and rate/concurrency limits | 429 where applicable |
| Scoring, audit, clock or dependency failure | 503 |

Errors are not ALLOW/CHALLENGE/DENY verdicts. Exact implementation status/error
behavior is version-bound and is not automatically a Draft v0 rule. The
[synthetic parser vectors](../validation/vectors.md) test a narrower boundary.
The table describes service responses. The shared client converts a non-200
service response into its own 503 `authority_status` failure; individual host
adapters then apply their documented response mapping. Do not assume that a
service's 401, 409 or 429 is forwarded unchanged to an application client.
