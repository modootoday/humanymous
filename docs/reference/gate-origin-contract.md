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

The `native_full` profile reserves and binds `Authorization` for its own
runtime capability, then strips it before forwarding. Within that profile,
separate upstream application `Authorization` is unsupported. Do not place this slice
over an arbitrary existing application and assume its authentication survives.

Working source adds a separately selected `browser_auth_only` profile that
rejects ambient cookies and preserves application Authorization in the origin
request builder. Its response path inserts a consumed-counter ACK bound to the
actual local or origin response status. At runtime source `adb8e3ed`, native
component fixtures exercised the shared client/server model and these branches.
Those cases do not execute browser Fetch or establish a qualified browser
credential-preservation guarantee. The native-profile exception above remains
applicable when that profile is selected.

The default buffered profile accepts complete HTTP/1.1 requests, with 64 KiB
request and origin response body limits and a 16 KiB header bound.
Transfer-encoded/chunked requests,
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
The frozen native component runs exercised these decoder cases. Ordinary repeated
fields are not a blanket HTTP compatibility or credential-preservation guarantee.

### Explicit native response streaming

A route may select `response_profile: native_stream` instead of the default
`buffered`. It still requires a complete request body before dispatch; this is
not streaming upload integrity. Only the `native_full` integrity profile accepts
this option. Browser integrity and the optional TLS/H1 listener require buffered
responses and reject this combination.

The native response path forwards a bounded entity incrementally, with a total
1 MiB body limit and the connection/request deadline. It accepts validated length,
simple chunked or connection-close framing, strips forged gateway fields and
retains downstream backpressure. A known oversized body rejects before the
downstream response head. A later malformed or truncated stream closes without a
successful chunk terminator or a replacement success/error response. Bytes and
origin effects already delivered cannot be retracted.

An actual loopback fixture observed the first event before allowing the origin
to finish. It used a synthetic report and an in-memory audit fixture; this does
not establish independent durable audit, all socket cancellation races, graceful
service shutdown or general streaming proxy compatibility. No generic Draft v0
streaming wire or browser response-streaming support follows from it.

### Selected TLS observations

At later runtime source `1646c482`, six selected ordinary-user Windows TLS cases
passed, with 46 library cases filtered out. They cover same-owner handshake/ALPN
observation, malformed request refusal, handshake cancellation and policy/origin
agreement. Both synthetic client identities received `CHALLENGE` with HTTP 403
and no origin effect. This is actual TLS component evidence, not an observed
ALLOW-to-origin path, the complete library, real browser/human/assistive use or
independent witness deployment. Full TLS ingress qualification remains open.

### Local decision authority boundary

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
for separate observer status/settings reads and an optional independently
credentialed resource operator. The operator requires witnessed durable settings;
it does not authorize scoring changes or destructive controls. Its exact contract
and unexecuted native lifecycle gates are recorded in the
[operations reference](on-call-quick-reference.md#experimental-native-status-service).

`serve_handler_until` explicitly stops acceptance, cancels tracked connections
and waits for admission-owning workers. Its successful return acknowledges that
drain; a timeout is incomplete shutdown. Dropping the serving future requests
cancellation without acknowledging drain. Detached work must retain its context
and admission ownership until it ends. The consuming runtime must keep custody
and state alive while such workers remain and finish its required audit flush
before reporting graceful shutdown. These transport components do not establish
complete product lifecycle qualification.

The generic local-handler fixture uses a synthetic `GET /management/status`
consumer, distinct from the runtime's actual POST management routes. It checks
an accepted same-process peer, a different credential, a wrong route and Origin
refusal. That fixture does not itself establish cross-account peer rejection or
the runtime operator's scope, CAS and witness policy. Separate blocking-worker
cases check that cancellation preserves admission until the worker ends, and
that explicit stop waits before acknowledging drain. Their effect counter is
synthetic; it is not a durable application commit witness. Attribute execution
results to the exact source and operating system rather than treating generic
transport coverage as management-product qualification.

Keep method, target/query semantics, duplicate header meaning and exact body
representation intact. Complete-body integrity finishes before any origin
effect. Bound buffering and timeouts; do not retry non-idempotent requests after
partial forwarding. Streaming needs its own tested contract.

Management is never an origin-facing public route. The Rust forwarding and
adapter implementations require conformance and actual deployment pilots before
these properties can be claimed for a release. See [compatibility](../explanation/will-this-break-my-app.md).
