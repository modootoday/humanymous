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
| Go HTTP origin | Rust Gate integration for Go HTTP origins uses the existing Rust Gate, without a new Go SDK or provisioned Go adapter producer. One source-bound eight-case hosted component run passed as described below. chi, Gin, Echo, Fiber and fasthttp each require separate host evidence; proxy compatibility does not establish in-process middleware or pooled-buffer safety |
| Python WSGI/ASGI | Rust-native `humanymous_python.Wsgi(app, options_json)` and `Asgi(app, options_json)` are candidate interfaces. No wheel or runtime tuple is qualified. WSGI needs the actual server's raw target and input deadline; ASGI needs its declared HTTP/1/raw-path boundary. Candidate package Python-version metadata does not establish CPython ABI, free-threaded, subinterpreter or framework support |
| Ruby Rack | Rust extension source exposes `Humanymous.wrap(app, options_json)`. Registration at the outer application boundary needs actual raw target, content length, authority and a server-enforced input deadline. The intended precompiled gem targets one exact Ruby minor ABI; platform tags do not establish installed gem, Rack/Sinatra/Rails, GC/GVL or ABI compatibility. No runtime tuple is qualified |
| ASP.NET Core | Safe Rust request-lifecycle and managed shim source now exist with a normalized producer profile. Actual middleware/PInvoke execution, NativeAOT, GC/abort ownership and NuGet installation remain unqualified; the generic native ABI is not itself that managed binding |
| Apache | Safe lifecycle/replay and opt-in native module source exist. One exact Linux tuple passed the 45-case native pilot described below. Other versions, platforms and module graphs remain unqualified. Source archives and selected header hashes alone do not prove the installed ABI |
| Common C/C++ boundary | Source exports the pointer-free by-value `humanymous_native_abi_v1` buffer/lifecycle command boundary over one safe owner. This is not an installed application security SDK. Header generation configuration and Rust units do not prove actual C/C++ layout, dynamic loading, unload, exception or allocator compatibility |

The ASP.NET Core packed-candidate restore configuration maps its experimental
package identity to an owned feed, with selected SDK dependency prefixes mapped
to the upstream feed. This is declared restore routing, not a verified restore
or dependency closure. Its notice preserves the project's and compatibility
code's license boundaries while explicitly leaving final native dependency
notices and vulnerability review as release gates.

The Apache candidate separates a safe lifecycle crate from its native module.
The default native feature set does not build the httpd hooks.
Default-feature all-target compilation and lint have passed separately from
the nine safe lifecycle/replay tests; those checks do not build the server hooks.
The opt-in
generator accepts only `x86_64-unknown-linux-gnu`, requires explicit absolute
include roots and the exact libclang library, checks selected header hashes,
and records the observed include closure and generated bindings digest.
That build receipt is evidence about ABI inputs, not successful module loading
or request enforcement. The separate `pilot` executable and `pilot-probes`
hooks must not be confused with the production module's behavior.

The safe Apache request owner requires the current child PID and generation,
an initial request, bounded admission and an original budget of at most 30
seconds. It copies at most 1 MiB and exposes the body only after exact-length
completion. Cleanup cancellation and child shutdown invalidate even completed
input. Replay retains admission, checks liveness on each read and supports
bounded byte, line and speculative reads; replay itself grants no authorization.
These source-level properties do not prove that httpd invokes cleanup, enforces
socket deadlines or consumes the bound authority response before origin effects.

The authored native bridge is narrower still: a dedicated plain HTTP/1 listener,
without a virtual-host chain, `mod_ssl` or `mod_cache`. It requires a single
Content-Length and refuses transfer coding, Upgrade, trailers and Expect.
Configuration must be child-owned private input and declare enforced synchronous
input deadlines; the source also sets the socket timeout from the remaining
original budget. Normalized observations provide no peer/connection capture.
The source maps both DENY and CHALLENGE to HTTP 403, and boundary failures to
503; this does not implement accessible challenge recovery or select Draft v0
status codes. Final checks compare method, target, authority, headers and URI
before consuming the bound ALLOW once. Callback ordering, host-pointer validity, filter behavior and origin isolation
need evidence from the exact native host tuple; source review alone is insufficient.
Its current pilot contract requires 45 named scenario/MPM pairs across event,
worker and prefork, with separate failed and not-run totals and observed cleanup.
The contract contains no executed results or qualified artifact identities.
The coordinator and receipt validator have source-bound execution evidence.
At source `b4000944`, [hosted run 42374566498](https://github.com/modootoday/humanymous/actions/runs/42374566498)
passed all 45 distinct scenario/MPM pairs: event, worker and prefork each passed
15, with zero failed or not run. Pilot and container/image cleanup were observed;
both attach and container processes exited successfully. The pinned Apache
2.4.70/APR 1.7.6/APR-util 1.6.6 Linux tuple built both production and opt-in probe
modules using generated bindings. This qualifies that component tuple's plain
HTTP/1 explicit-Content-Length boundary, shared authority/WAL and owned origin
witness. It does not qualify later source changes, arbitrary module graphs,
SSL/HTTP/2, Windows Desktop or the complete product.

The earlier source `b5c15f0d`, [run 42373232097](https://github.com/modootoday/humanymous/actions/runs/42373232097),
remains failed historical evidence: 35 passed and ten failed, all in prefork;
pilot cleanup was not established despite separate container/image cleanup.
The later successful run does not rewrite those earlier observations.

The coordinator and validator require all 45 distinct case
IDs, recompute outcome totals and compare the expected source, image and artifact
digests. Successful parsing is not a passing pilot: process exit, all required
cases and cleanup must also satisfy the contract. Unverified report counts remain
unknown rather than becoming zero failures. The public projection permits bounded
compiler codes and source locations; raw diagnostics and artifacts stay private.
Its cleanup checks concern the owned container and image tag, not all shared
image contents, build caches, volumes or external effects. The current host
source now separates hosted Linux from Windows Desktop engine profiles. The
Windows path selects the explicit Desktop client and `desktop-linux` context,
checks its named-pipe endpoint plus the provisioned server ID/version and Linux
OS type, and records engine identity. Hosted Linux requires a Unix endpoint.
Do not transfer hosted results to the Windows Desktop profile. A context label or
successful identity comparison alone does not establish containment or cleanup.
Local execution needs verified engine provenance before
its results can be compared with a hosted runner.
The authored pilot uses the real shared authority and WAL with frozen synthetic
reports for its ALLOW/DENY/CHALLENGE paths. Its loopback origin counts complete
received requests before writing an echo response; this is not a business-commit
witness or production browser provenance. The reload source now retains an
owned request-worker guard that joins on early return as well as explicit
completion, with bounded connect/read/write operations. The successful hosted tuple includes owned process-group checks before wait/reap
and pidfd-based normal signals. Its isolated prefork CLI observation exited and
reaped successfully without timeout, truncated output or forced cleanup. These
observations do not establish general CLI or arbitrary process-tree containment.

For Rust Gate integration for Go HTTP origins, source `c3211e27` passed the
bounded eight-case profile in [hosted run 42375310198](https://github.com/modootoday/humanymous/actions/runs/42375310198).
All eight ordered cases were evaluated and passed, with 40 known assertions,
zero failed or not-run cases and no unknown assertion counts. The owner exited
successfully, cleanup was observed and the source inventory remained unchanged.
The cases cover binary ALLOW, dependency failure, cancellation, incomplete body,
forged provenance, origin bypass, upgrade refusal and response semantics. This is
component qualification; full-product and release readiness remain false. It does
not qualify later source changes, browser provenance or every Go framework.

The earlier source `15f63fef`, [run 42369171124](https://github.com/modootoday/humanymous/actions/runs/42369171124),
remains failed evidence. Eight actor attempts failed at writer-mount setup before
semantic evaluation; assertion counts were unknown. The later success does not
rewrite that failure or turn those earlier attempts into measured passing cases.

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

A separate WSGI server harness and native factory have historical component
evidence. At source `7842b6a8`, an ordinary UID1000 WSL run used a private
Waitress/Flask/Django environment and passed eight tests: six actual groups
covering 14 rows and 28 cases, plus two framing tests. The earlier failed run is
retained. This is local component evidence with test-only frozen inputs, not
hosted qualification, production browser provenance or an installed wheel.

That result does not qualify the later shared WSGI/ASGI refactor. At source
`d68682e4`, all-target check and strict Clippy passed, followed by two receipt-parser
tests; two controller definitions were filtered. No new server, framework import
or package installation ran. The current controller still refuses execution
until its shared owned-lifetime contract is accepted. Current WSGI 28-case and
ASGI 42-case execution, hosted qualification, production-wheel clean installation
and Windows support remain separate pending gates. The historical incomplete-body
case measures idle closure, not a total upload deadline against a progressing body.

The current Ruby native-callable contract requires eight cases in four groups:
five liveness negatives and three dependency-unavailable framework-object cases.
Its four default pure Rust cases are separate. The contract records zero native
executions; required counts are not passing results. These are not an actual
server, successful production authority path, installed gem, GC-compaction or
GVL/cancellation qualification. Its native
extension is feature-gated: a default crate check can exclude the extension and
cannot replace an explicit native build and installed `require` test. Candidate
gem metadata is not a verified registry name, publisher or multi-Ruby loader.

The native source requires the trusted server environment callback
`humanymous.client_disconnected`. Only exact Ruby `false` means connected and
exact `true` means disconnected; missing, failing or nonboolean callbacks do not
grant admission. The source checks before processing, during authority waiting
and before the application handoff. A synthetic callback does not qualify a real
server's disconnect detection, input deadline or cancellation/GC/GVL behavior.

### Managed source follow-through

The naming contract at source `3962278c` selects the public product name
`humanymous`, with managed `humanymous.dll` and native `humanymous-native.dll`
as distinct filenames. This is a target contract. It does not establish that
the current package, imports, resolver or published artifacts have migrated;
the experimental interfaces below retain their current source meaning.

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

Selected dependency and input exceptions before application handoff become a
503 or connection abort. The downstream application runs outside that catch;
its exceptions retain the host application's normal handling. A `finally` block
restores the original request stream and disposes the replay buffer. A later
error does not prove that the application was never called or that its effects
rolled back. The candidate package requires
a supplied native artifact and runtime identifier; that check alone does not
verify its hash or package lineage. .NET 10 target metadata,
AOT-compatible settings and a pinned SDK do not qualify an installed package,
JIT/NativeAOT tuple, managed cancellation race or complete product.

The managed test host now tracks its GC worker and bounds its control-task join
before emitting a successful stopped message. A timed-out control join returns
failure instead. These are source checks, not observed shutdown: the supervisor
must still measure process exit/reaping and retained-resource release. Its
optional generic ABI roundtrip is a separate test profile, not a substitute for
middleware, JIT/NativeAOT, C++ or package qualification.
