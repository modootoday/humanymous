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

Working source adds a separately selected `browser_auth_only` profile that
rejects ambient cookies and preserves application Authorization in the origin
request builder. Its response path inserts a consumed-counter ACK bound to the
actual local or origin response status. These branches have not been built or
executed in the current review, so they are not a qualified credential-preservation
or browser integration guarantee. The existing native-profile exception above
remains applicable to the verified slice.

It accepts bounded complete HTTP/1.1 requests, with 64 KiB request and origin
response body limits and a 16 KiB header bound. Transfer-encoded/chunked requests,
upgrades, duplicate request headers and encoded/dot-segment paths are unsupported.
Validated duplicate `Set-Cookie` response fields retain their separate values.
Origin effects follow durable audit append and deadline/liveness checks;
an ambiguous origin failure is not retried.

Current response-decoder source accepts bounded complete HTTP/1.1 responses with
an exact Content-Length, a simple chunked body or a body delimited by connection
close. It decodes chunked bodies and reframes chunked/close-delimited entities
with Content-Length and `Connection: close`; this is buffering, not response
streaming. Decoded entities remain bounded to 64 KiB. Chunk extensions, trailers,
ambiguous lengths, interim responses and protocol upgrades reject. HEAD, 204 and
304 responses cannot carry entity bytes. Gateway-owned `X-Humanymous-` fields and
selected hop-by-hop fields are removed before response metadata is added.
These changed decoder paths and their new fixtures have not yet been executed
in the current qualification run; ordinary repeated fields are not a blanket
HTTP compatibility or credential-preservation guarantee.

The separate local IPC authority has a different capability boundary: its
currently reviewed route path rejects required browser collection, request-
integrity, original TLS and HTTP/2 planes. The loopback runtime's session/integrity
implementation must not be attributed to that IPC profile. See
[configuration](cli-config-policy.md) and [implementation mapping](../standards/draft-v0/validation/implementation-map.md).

## Native local decision client

The shared Rust `humanymous_authority::client::decide` API uses an explicitly
provisioned adapter parser profile and independently supplied outstanding-request binding.
It checks the actual local server UID or Windows SID/process identity before
sending the scoped bearer. An envelope's declared peer is an application
observation, not proof of that IPC server identity. The client has no TCP fallback,
redirect or automatic retry.

The returned `BoundResponse` is not Clone. `consume(self)` rechecks wall-clock
rollback, authority expiry and the monotonic budget. Inspecting `verdict()` alone
does not perform that consumption check or invoke the application. The host must
consume the response, recheck cancellation/deadline at its dispatch boundary and
invoke only the originally bound handler at most once. A Rust move-only response
does not itself prove that an arbitrary application performs one business effect.
Non-200 service responses and malformed, mismatched or expired replies become
client errors rather than application ALLOW.

## Local handlers and shutdown

The Unix candidate requires an absolute filesystem socket in a private canonical
directory, checks owner/mode and kernel peer UID, and compares the socket's
device/inode before and after connection. Startup never unlinks an existing
socket path. Listener cleanup only attempts removal of its original socket
identity; neither startup failure nor a stale filename authorizes deleting a
live endpoint. Trusted ancestor control and same-owner/privileged compromise
remain deployment assumptions.

The Windows candidate accepts only a local pipe name, applies the configured
protected descriptor, rejects remote admission and checks the server PID plus
pipe privacy before sending the credential. Its asynchronous write wrapper
avoids blocking flush/deferred-close behavior; successful enqueue or flush is
not proof that the peer consumed the message. The owner observes closure within
its deadline, and cancellation can discard remaining bytes. Actual DACL,
cross-principal, endpoint-replacement and shutdown cases need source-bound
native execution; these checks do not establish application effect completion.

The shared local transport also provides `LocalHandler`, `LocalContext` and
`LocalResponse` for separately authorized consumers. Actual OS peer context,
bounded framing and cancellation do not grant administration authority. A
management consumer still needs its own listener, credential, exact routes,
Origin/CSRF/scope checks and audit/control issuers. The decision binary continues
to refuse administration routes; generic handler availability is not completed
runtime management integration. Current runtime source now consumes this seam
for a separate observer-only status listener; its exact contract and unexecuted
native lifecycle gates are recorded in the
[operations reference](on-call-quick-reference.md#experimental-native-status-service).

`serve_handler_until` explicitly stops acceptance, cancels tracked connections
and waits for admission-owning workers. Its successful return acknowledges that
drain; a timeout is incomplete shutdown. Dropping the serving future requests
cancellation without acknowledging drain. Detached work must retain its context
and admission ownership until it ends. The consuming runtime must keep custody
and state alive while such workers remain and finish its required audit flush
before reporting graceful shutdown. These transport components do not establish
complete product lifecycle qualification.

Keep method, target/query semantics, duplicate header meaning and exact body
representation intact. Complete-body integrity finishes before any origin
effect. Bound buffering and timeouts; do not retry non-idempotent requests after
partial forwarding. Streaming needs its own tested contract.

Management is never an origin-facing public route. The Rust forwarding and
adapter implementations require conformance and actual deployment pilots before
these properties can be claimed for a release. See [compatibility](../explanation/will-this-break-my-app.md).
