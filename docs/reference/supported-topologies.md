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

### Same-transport capture candidate

A separate Rust capture context now contains a bounded first-ClientHello
assembler and a `tls::Connection` owner that consumes the actual TCP socket and
completes a rustls handshake. Its negotiated ALPN comes from that connection,
not the first protocol offered by the client. HTTP/2 observations are fed from
the same owner's decrypted bytes and borrowed by stream ID; the passive capture
parser alone cannot establish that provenance. Complete header sections do not
establish complete request bodies or application authorization.

The initial owner has an absolute lifetime of at most 30 seconds, disables early
data and refuses HelloRetryRequest. Observed terminal I/O failure, cancellation
or expiry revokes access to its handshake/request evidence. A supervisor's
revocation handle does not cancel merely by being dropped; dropping the
connection owner shuts down the socket. Supervised worker joining and request,
tenant and stream binding still belong to the consuming runtime.

This candidate is not yet a qualified runtime ingress or framework profile.
Pure parser fixtures and compilation do not prove actual TLS/HTTP2 serving,
cancellation, origin isolation or product behavior. Original-hop observations
describe the client only when this deployment terminates that client's original
connection; behind retermination they describe the proxy hop. Existing adapter
and loopback profiles must keep refusing required unavailable TLS/H2 planes until
their actual integration is qualified. HTTP/3 and human identity are not implied.

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

## Other native host candidates

These developing source boundaries do not add release-qualified host tuples.

| Candidate | Current boundary and remaining qualification |
| --- | --- |
| Go HTTP origin | Uses the existing Rust Gate; there is no new Go SDK or provisioned Go adapter producer. An untouched external `net/http` fixture and bounded HTTP cases are selected but unexecuted. chi, Gin, Echo, Fiber and fasthttp each require separate host evidence; proxy compatibility does not establish in-process middleware or pooled-buffer safety |
| Python WSGI/ASGI | Rust-native `humanymous_python.Wsgi(app, options_json)` and `Asgi(app, options_json)` are candidate interfaces. No wheel or runtime tuple is qualified. WSGI needs the actual server's raw target and input deadline; ASGI needs its declared HTTP/1/raw-path boundary. Candidate package Python-version metadata does not establish CPython ABI, free-threaded, subinterpreter or framework support |
| ASP.NET Core | Safe Rust request-lifecycle and managed shim source now exist with a normalized producer profile. Actual middleware/PInvoke execution, NativeAOT, GC/abort ownership and NuGet installation remain unqualified; the generic native ABI is not itself that managed binding |
| Apache | Safe lifecycle/replay and opt-in native module source exist. The exact httpd/APR/MPM/compiler/header tuple, pool cleanup, after-fork initialization, reload, cancellation and origin isolation still need native qualification. Source archives and selected header hashes do not prove the installed ABI |
| Common C/C++ boundary | Source exports the pointer-free by-value `humanymous_native_abi_v1` buffer/lifecycle command boundary over one safe owner. This is not an installed application security SDK. Header generation configuration and Rust units do not prove actual C/C++ layout, dynamic loading, unload, exception or allocator compatibility |

The Go candidate's synthetic trusted-session cases cannot attest browser or
human activity. Origin contact counts and observed application mutations are
different denominators: a successful transport contact alone does not prove a
business effect. A narrow HEAD/duplicate-cookie probe does not qualify 204/304,
streaming or post-origin rollback. Preserve failed and unexecuted cases when
evaluating each selected fixture and compiler input.

The Python candidate host contract has three groups and seven framework-object
cases for dependency-unavailable behavior and trusted-server lifespan forwarding.
Those cases are not an actual network server, production authority, successful
authenticated ALLOW, installed wheel, response-streaming or cancellation-race
qualification. Empty qualified tuples remain the current support declaration.

### Managed source follow-through

Current ASP.NET source declares `HumanymousHost(supervisorConfiguration)` and
`HumanymousMiddleware.InvokeAsync(HttpContext)`. The private configuration must
come from the supervisor, never a request or public log. The shim's
`LibraryImport` targets the separate `humanymous_dotnet_v1` entrypoint; a passing
generic `humanymous_native_abi_v1` unit test does not qualify that binding.
Its source guards a 64-bit little-endian layout and copies data through bounded
by-value commands. SafeHandle leases retain native ownership across the awaited
decision; source declarations alone do not prove GC, disposal or loader safety.

Install this candidate before all body readers, response writers and protected
effects. Source collects input to EOF, forwards cancellation to the native
request, copies approved bytes for replay and performs a final native handoff
check before calling the application. It restores the original request stream
afterward. A seekable stream already advanced from its beginning rejects; this
does not detect or qualify every custom earlier consumer of nonseekable input.

The current shim maps DENY to 403 and other non-ALLOW verdicts, including
CHALLENGE, to 503. It does not inject a challenge recovery page. These are
candidate implementation mappings, not generic Draft v0 status assignments.

The middleware also catches selected exceptions from the downstream application.
A later 503 or connection abort therefore does not prove that the application
was never called or that its effects rolled back. The candidate package requires
a supplied native artifact and runtime identifier; that check alone does not
verify its hash or package lineage. .NET 10 target metadata,
AOT-compatible settings and a pinned SDK do not qualify an installed package,
JIT/NativeAOT tuple, managed cancellation race or complete product.
