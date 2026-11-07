---
id: sot:root/python-ruby-adapters
title: Rust native Python and Ruby framework adapters
status: active
verify: manual
created: 2026-11-07T07:08:28Z
updated: 2026-11-07T07:36:39Z
rel:
  governs: [crates/framework-adapter/**, crates/python-adapter/**, crates/ruby-adapter/**, adapters/python/**, adapters/ruby/**]
  depends_on: [sot:root/adapter-service-contract, sot:root/server-adapter-expansion, sot:root/rust-migration, sot:root/program-roadmap, sot:root/delivery]
---

# Rust native Python and Ruby framework adapters

Version0.1 implementation contract, authorized 2026-11-07. Framework names below
are target pilots, not support claims. No tuple is qualified yet.

## Ownership and registration

New authored implementation/tests are Rust1.99.1/edition2024 only. Python native
module humanymous_python exposes Wsgi(app,options_json) and Asgi(app,options_json).
Ruby native module Humanymous exposes wrap(app,options_json), returning a real
Rack callable. No handwritten Python/Ruby loaders, gemspec, extconf or executable
foreign-language strings. Host app registration belongs to the user's app.
Upstream PyO3/Magnus generated ABI glue is recorded separately from authored Rust.
Package registry names are undecided; no publish/token/tag action is authorized.
Root accepted optional native host dependencies: Python native enables PyO3 and
async-runtimes, Ruby native enables Magnus/rb-sys. extension-module, host-pilot
and package-tool explicitly enable native. Default zero tests/empty host library
are never native or product success. Mandatory shared pure gate and real native
feature compile+pilot+installed-package gate must all pass; none substitutes for
another. Package binary requires package-tool; shipping wheel uses extension-module,
shipping Ruby native gem uses native without fixture/package features.

Reuse authenticated authority::client, native-security transport identity,
integrity schema, frozen scoring and durable audit. No duplicated decision,
crypto, IPC framing, browser recovery or management backend. Public admin routes
remain rejected; localhost management is a separate existing Rust service.
Common workspace Cargo/lock and integrity profile registry are root-owned.
Root conditionally accepted framework-adapter for shared config/envelope/admission.
Node must consume this context through a small followup slice preserving its
schema/provenance and rerunning actual regressions, rather than retain duplicate
validation as the final architecture.

## Profile delta

Proposed exact producer IDs humanymous.python and humanymous.ruby, version
0.1.0-experimental.1. Profile names/modes PythonAsgi/asgi-normalized,
PythonWsgi/wsgi-normalized, RubyRack/rack-normalized require integrity owner
acceptance. Credential provisioning selects profile independently of wire data.
Root accepted these exact profiles; authority configuration uses python-asgi,
python-wsgi and ruby-rack. Source registration is not actual runtime qualification.
ASGI headers preserve received list order/duplicates, but original casing is
unavailable. WSGI/Rack CGI maps lose original order/duplicates/casing. These modes
always report rawHeaderOrder=false, rawTls=false, originalH2=false, peer=null,
connectionId=null, capture=null, stream=null. Never reuse Node/Fetch identity to
evade the registry. Parsed client addresses are not original capture evidence.

Complete transfer-decoded/content-encoded bytes only, at most1MiB and64 in-flight
body/decision admissions per process. Acquire admission before input allocation.
Exact method, raw origin-form target including query, host, bytes and configured
scope bind to the one-shot decision. Reject missing raw target rather than
reconstruct a lossy decoded path. Explicit configured routes include all protected
paths; unknown paths and /_humanymous (including children) fail closed.
Strip internal/forwarding trust headers in both evidence and app-visible copies.
Reject CONNECT, Upgrade, trailers, request streaming, body-length disagreement,
decompression/reconstruction, and unsupported protocol versions before origin.

## Host lifecycle

WSGI middleware receives (environ,start_response), copies sanitized environ,
requires server-provided RAW_URI or REQUEST_URI, CONTENT_LENGTH and unconsumed
wsgi.input; reads exactly bounded entity bytes and replaces input with a native
host BytesIO for the application. Server-enforced input deadlines are a deployment
prerequisite: synchronous read cannot safely be interrupted by a Rust timer.
Never call app/start_response before decision. After ALLOW return the app iterable
unchanged, preserving close(), write(), duplicate response headers and errors.
No claim that middleware alone controls an unrelated server upgrade handler.

Rack receives env, copies/sanitizes it, requires REQUEST_URI, CONTENT_LENGTH and
rack.input. Reject rack.hijack? and upgrade/stream extensions. Native callback
reads bounded binary input, replays via upstream StringIO and returns the original
[status,headers,body] without rewriting streaming/close behavior. Ruby objects
must remain on the Ruby stack or in VM-managed ivars; no unmarked Value in Rust
Vec/Box/Arc. Hold GVL for Ruby calls. GVL scheduling/timeout and GC.compact are
mandatory actual runtime probes; source safety annotations are insufficient.

ASGI is a distinct async callable (scope,receive,send). HTTP1 only initially;
require raw_path. Trusted-server lifespan forwards the original scope/state and
receive/send to the app without an HTTP decision; startup/shutdown effects are
deployment lifecycle, not protected HTTP origin effects. Unsupported WebSocket
and other request protocols reject. Bound http.request chunks before
allocation; reject disconnect/incomplete body. Copy scope and replay one complete
http.request followed by the real receive for disconnect ownership. Await real
app/send callbacks with task-local context, cancellation drops Rust IPC future.
No sync Tokio block_on on the Python event loop. App is invoked once only after
consuming BoundResponse and immediately checking expiry/deadline/cancellation.
Application response send/backpressure and exception semantics remain unchanged.

## Dependencies and packaging

Candidate pins: PyO3=0.30.1, pyo3-async-runtimes=0.30.0 with tokio-runtime;
Magnus=0.9.3 with transitive rb-sys resolved/audited in root lock. Maturin=1.16.0
build backend. Initial Python per-interpreter CPython wheels; abi3 is an explicit
later qualification, not automatic free-threaded/subinterpreter support.
Free-threaded CPython, PyPy, JRuby, TruffleRuby and Ractors remain unsupported.
Ruby native platform gems must match actual Ruby ABI/OS/arch/libc; Rust packager
must use upstream RubyGems Specification/Package APIs to generate gem metadata,
not write a handwritten Ruby gemspec. Source gem installation hooks remain an
unresolved packaging boundary; do not ship a broken source gem as qualified.

Candidate hosted matrix: CPython3.12/3.13/3.14 regular-GIL and CRuby3.3/3.4,
Linux GNU x64 and Windows x64 first; macOS/ARM64/musl are unqualified separate
tuples. Windows Ruby UCRT/MinGW target is distinct from Rust MSVC. Wheel manylinux
tag/audit and gem platform naming require actual clean installation and import.
No success from a label, cross-compilation or symbol listing alone.

## Verification and critique reflected

Common local/hosted policy canon: sot:root/program-roadmap and sot:root/delivery.
Local pure units cover config/route/body/provenance/admission only. Actual host
callbacks/framework processes, native package install and crossOS IPC/ACL execute
on hosted, product acceptance Docker-only. No new local runtime pilots.

P0 findings incorporated: do not disguise CGI maps as Node raw evidence; a blocking
host read requires a server deadline; do not invoke application on expired result;
Ruby GC must root callback state; public admin is not middleware; abi3 does not
establish free-threaded support; source gem tooling cannot be replaced with a
hidden Ruby string. Residual verdict needs-work until actual host pilots pass.

Required target pilots: FastAPI/Starlette ASGI, Django ASGI+WSGI, Flask WSGI,
Rack with Sinatra and Rails integration. For each exact tuple, record actual
denominator/failed/ignored/filtered, framework/runtime/dependency/source hashes,
production nonALLOW versus synthetic test-only ALLOW, audit record and independent
origin effect counter. Negative corpus covers malformed/missing peer, credential,
scope/target/body/provenance, down/timeout/cancel, replay/expiry, oversize/aborted
body, unsupported upgrade/hijack/stream, and audit failure with zero origin effect.
Compatibility includes binary/empty bodies, duplicate Set-Cookie, HEAD/204/304,
redirect, exception, iterable close and SSE backpressure. No motor/speed gates,
new scoring or third-party probing. T4 ceiling remains unchanged.

Current source/package/runtime/whole-product status: NOT_RUN; qualified tuples=[];
no current dependency closure audit, registry artifact or publisher verification.

## Explicit hosted smoke source

adapters/python/pilot-contract.json describes2required Rust test groups,5target
framework tuples (Flask/Django WSGI and Starlette/FastAPI/Django ASGI). They deliberately
use an unavailable authority fixture, expect503 and independent native spy count0.
adapters/ruby/pilot-contract.json describes3required Rack/Sinatra/Rails object
callbacks with the same dependency-down expectation. These are source-defined
denominators, not executed counts. All8cases remain NOT_RUN. They do not qualify
production IPC/ALLOW/audit/server/wheel/gem/streaming/cancellation-race behavior.
Actual acceptance still requires the full earlier verification corpus.

Python candidate directpins from official PyPI JSON API on2026-11-07:
FastAPI0.162.0, Starlette1.10.0, Django6.1.3, Flask3.1.3.
RubyGems official v1/gems JSON API candidate pins Rack3.2.8/Sinatra4.2.1/
Rails8.1.4.1 and registry artifact SHA256 are in its contract. These retrieved
metadata values are not compatibility/security findings; hosted must retain the
actual transitive installed closure and current audit. No local interpreter ran.

Ruby package-tool native init can read a bounded private JSON request path from
HUMANYMOUS_NATIVE_GEM_PACKAGE_REQUEST and call upstream packaging APIs, allowing
an unchanged ruby --require native-extension invocation with no custom Ruby
entrypoint. The feature is absent from production. Actual archive/install remains
NOT_RUN, and self-generated metadata must never state runtime qualification.

## Dedicated context handoffs

Root split Apache/C ABI/Go/.NET into dedicated owners. This owner now exclusively
implements framework-adapter/Node/Python/Ruby in adapter-hosts lane. Later-language
research below is handoff context, not a competing implementation canon. Apache
owner requests one shared encode arm only after root's exact integrity profile
exists; unknown profiles currently fail closed. No new language source paths here.

## Go integration extension (explicit user scope)

Go net/http plus chi/Gin/Echo and Fiber are required followup targets. Existing
legacy Go product code stays a read-only oracle, not permission for new authored
Go implementation/tests. net/http.Handler requires an actual Go method set
ServeHTTP(ResponseWriter,*Request). cbindgen emits C declarations, and cmd/cgo
generates interoperability glue for Go source that already calls C; neither is
evidence that Rust alone constructs a correct Go Handler. No known pinned generic
generator has yet been verified to generate the complete request lifecycle bridge
from declarative input without an authored Go middleware template. Go-side glue
feasibility is unresolved; do not call a Rust reverse proxy idiomatic Go middleware.

If no suitable unchanged upstream generator exists, root must request a specific
user decision: allow the minimum Go Handler lifecycle adapter (bounded request
body copy/replay, context cancellation, call Rust decision once, delegate same
ResponseWriter and Request after consumed ALLOW), or accept explicit HTTP-origin
proxy coverage while native Go middleware remains unavailable. No exception is
assumed and no authored Go/script string or new C pointer unsafe export is added.

net/http header maps lose original casing/order and may coalesce values; RequestURI
and exact entity bytes remain separately available. Context cancellation, body
Close, request trailers, optional Flusher/Hijacker/Pusher/ResponseController and
response streaming require real Go host tests. chi/Gin/Echo adapt net/http with
their own outermost middleware ordering. Fiber/fasthttp has a distinct pooled
request lifetime/body/cancellation contract; never pass retained pooled pointers
to an async Rust task or claim net/http parity from its adapter conversion.
No Go producer profile is registered until the bridge contract is accepted.

Go distribution is a module/version/source-tag and proxy/checksum-database path,
not npm-style publish/OIDC. No tag/proxy publication/token exchange is authorized.
CGO toolchain/libc/crossOS linking and pointer retention/ABI evidence are separate
from module availability. All Go implementation/package/runtime cases NOT_RUN.

Primary Go references: https://pkg.go.dev/net/http#Handler ;
https://pkg.go.dev/cmd/cgo ; https://github.com/mozilla/cbindgen ;
https://go.dev/ref/mod#module-proxy ; https://github.com/go-chi/chi ;
https://gin-gonic.com/en/docs/ ; https://echo.labstack.com/docs/middleware ;
https://docs.gofiber.io/next/extra/faq/ . Findings are feasibility constraints,
not claims of a generated Go bridge already working.

## C, C++ and .NET extension (explicit user scope)

Add one versioned C ABI contract over the shared Rust core, not one detector per
language. Candidate Rust cdylib/staticlib exports must specify opaque handle
ownership, fixed-width repr(C) layouts/size/version, exact allocator/free pairing,
buffer-length maximums, error/status codes, panic containment, secret zeroization,
deadline and cancel ownership. Cbindgen can generate headers from Rust declarations;
it does not prove foreign pointer validity or make unsafe pointer dereference safe.
Core unsafe_code=forbid remains intact. No authored unsafe C ABI shim is silently
waived: isolate a reviewed generated upstream boundary or request a narrow explicit
exception for the necessary pointer/lifetime/export boundary before implementing it.

C++ RAII, exception translation, thread/callback lifetime and allocator behavior
are separate host glue. .NET LibraryImport's source generator produces P/Invoke
marshalling from C# declarations; it does not author an ASP.NETCore middleware
InvokeAsync(HttpContext) registration/lifecycle. Kestrel RequestAborted, bounded
body replay, headers/trailers, Response.BodyWriter/backpressure, managed GC rooting,
NativeAOT/trimming and SafeHandle disposal require actual managed-host evidence.
No new authored C/C++/C#/Go implementation or executable template is authorized
by listing these targets. Rust reverse proxy coverage does not complete middleware.

Initial feasibility status: generated header/marshaller tools exist; full native
host callback bridge and exception decision unresolved. Report the minimal missing
host declarations/middleware glue to root rather than fabricate support. No new
crate/workspace member or producer profile until ownership/contract is accepted.
NuGet trust/distribution and native runtime asset selection are separate CI-owner
work; no environment, tag, token exchange or publish action is authorized here.

Primary references: https://doc.rust-lang.org/nomicon/ffi.html ;
https://github.com/mozilla/cbindgen ;
https://learn.microsoft.com/en-us/dotnet/standard/native-interop/pinvoke-source-generation ;
https://learn.microsoft.com/en-us/aspnet/core/fundamentals/middleware/write?view=aspnetcore-10.0 ;
https://learn.microsoft.com/en-us/nuget/nuget-org/trusted-publishing .
All C/C++/.NET compiler/package/runtime/ABI cases are NOT_RUN.

## Apache httpd extension (explicit user scope)

Apache HTTP Server is a distinct native target. A reverse proxy to Rust Gate
provides HTTP-origin topology coverage, not a loaded native httpd module.
Qualification binds exact httpd/APR ABI, build flags, OS/arch, request pool and
MPM. Interception must precede application effects and correctly replay bounded
input through filters. A content handler alone cannot gate arbitrary later proxy
handlers. Never retain request_rec/pool/bucket pointers past cleanup or transfer
them to an unowned asynchronous thread.

event/worker require thread-safe lifetimes; prefork requires fork-safe per-process
client/admission initialization. Graceful reload/shutdown, internal redirects,
subrequests, handler re-entry, timeout/disconnect/filter failures, upgrades and
once-only decisions require real probes. MPM names do not establish safety.
Reuse shared authority/integrity/admission and coordinate nginx seams; no scoring
copy. No binding/version is selected before maintained ABI/unsafe/CVE research.
Core forbid-unsafe remains; hook/pool-pointer boundary needs reviewed upstream
binding or an explicit narrow exception, never an implicit waiver.

Hosted isolated Docker must verify module load/reload/MPM/filter/product cases.
Compose is deployment orchestration, distinct from adapter implementation.
Product-lab owner must agree exact image/module/config, isolation, bypass denial
and audit/effect witness. Priority follows compiling current Node/Python/Ruby
commonization. Implementation=false; all Apache tests NOT_RUN.
Primary: https://httpd.apache.org/docs/2.4/developer/modguide.html ;
https://httpd.apache.org/docs/2.4/mpm.html ; https://httpd.apache.org/docs/2.4/mod/event.html .

## Primary research (2026-11-07)

- https://pyo3.rs/main/class/call and https://docs.rs/pyo3/latest/pyo3/
- https://docs.rs/pyo3-async-runtimes/latest/pyo3_async_runtimes/tokio/fn.future_into_py.html
  (cancellation propagation; async contextvars versus synchronous callback caveat)
- https://pyo3.rs/main/free-threading and https://www.maturin.rs/bindings
- https://www.maturin.rs/distribution.html and https://pypi.org/project/maturin/
- https://raw.githubusercontent.com/matsadler/magnus/main/src/lib.rs (GC safety)
- https://github.com/oxidize-rb/rb-sys
- https://guides.rubygems.org/c-extensions/ and https://guides.rubygems.org/platforms/
- https://asgi.readthedocs.io/en/latest/specs/www.html
- https://peps.python.org/pep-3333/ and https://github.com/rack/rack/blob/main/SPEC.rdoc
- https://rustsec.org/advisories/RUSTSEC-2026-0176.html: PyO3 iterator overflow
  patched>=0.29.0; candidate0.30.1 is outside this advisory's affected range.
  This is not a current full-closure security audit or a no-vulnerabilities claim.
