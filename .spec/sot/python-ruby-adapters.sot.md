---
id: sot:root/python-ruby-adapters
title: Rust native Python and Ruby framework adapters
status: active
verify: manual
created: 2026-11-07T07:08:28Z
updated: 2026-11-07T07:36:39Z
rel:
  governs: [crates/framework-adapter/**, crates/python-adapter/**, crates/ruby-adapter/**, adapters/python/**, adapters/ruby/**]
  depends_on: [sot:root/adapter-service-contract, sot:root/server-adapter-expansion, sot:root/rust-migration, sot:root/program-roadmap, sot:root/delivery, sot:root/structural-development]
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
Require a trusted server disconnect callable, humanymous.client_disconnected or
waitress.client_disconnected, before input and immediately before app handoff.
Absence is unknown and rejects, not a fabricated false. Waitress qualification
must enable channel_request_lookahead so its callable can observe disconnects.
Never call app/start_response before decision. After ALLOW return the app iterable
unchanged, preserving close(), write(), duplicate response headers and errors.
No claim that middleware alone controls an unrelated server upgrade handler.

Rack receives env, copies/sanitizes it, requires REQUEST_URI, CONTENT_LENGTH and
rack.input. rack.hijack? advertises a capability, not a request action. Disable
it and remove rack.hijack from the app-visible environment; reject actual upgrade
and unsupported request streams. Native callback
reads bounded binary input, replays via upstream StringIO and returns the original
[status,headers,body] without rewriting streaming/close behavior. Ruby objects
must remain on the Ruby stack or in VM-managed ivars; no unmarked Value in Rust
Vec/Box/Arc. Hold GVL for Ruby calls. GVL scheduling/timeout and GC.compact are
mandatory actual runtime probes; source safety annotations are insufficient.
Rack has no universal standard request-disconnect callable. Require a trusted,
server-owned humanymous.client_disconnected callback before input, during IPC
wait and immediately before application handoff. Only exact Ruby false means
connected; true rejects, and absence/nil/truthy values/callback errors fail closed.
Never derive it from a request header or assume IO#closed? attests remote liveness.
Until a real server supplies and validates this seam, generic Rack HTTP support
is unqualified. Request-owned Rust worker Drop closes cancellation and joins
without Ruby/GVL calls; no detached task is accepted as cleanup proof.

ASGI is a distinct async callable (scope,receive,send). HTTP1 only initially;
require raw_path. Trusted-server lifespan forwards the original scope/state and
receive/send to the app without an HTTP decision; startup/shutdown effects are
deployment lifecycle, not protected HTTP origin effects. Unsupported WebSocket
and other request protocols reject. Bound http.request chunks before
allocation; reject disconnect/incomplete body. Copy scope and replay one complete
http.request followed by the real receive for disconnect ownership. Await real
app/send callbacks with task-local context, cancellation drops Rust IPC future.
Keep one receive future outstanding while awaiting authority, prioritize an
observed disconnect/unexpected post-body event and poll it before origin handoff.
After ALLOW, move that same pending receive into replay ownership; never drop an
already-consumed server event and start another receive as if none arrived.
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
Local Docker engine selection follows sot:root/rust-migration: explicit Windows
docker.exe --context desktop-linux, using Windows Docker Desktop. No automatic
WSL Docker fallback. Retain observed context endpoint, server ID/version and
OSType in private run evidence before allocated build/run; SDK clients such as
act receive explicit Desktop DOCKER_HOST. WSL-native Rust and hosted Linux remain
distinct surfaces. Do not retrofit Desktop provenance onto an older receipt whose
actual engine identity was not retained.
User's later explicit direction authorizes actual local Windows/WSL build/test/
runtime measurements as well as hosted runs to compare execution policy. Root
allocates execution; no new Cargo/native/runtime/model/Docker command starts
during the benchmark quiet window. Historical hosted-only restrictions remain
historical evidence context, not a current prohibition. Product acceptance
remains isolated Docker-only. Each receipt states its actual execution surface;
the historical HOSTED opt-in variable does not establish hosted provenance.

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

Current Node shared-source check/Clippy passed (evidence:root/20261107075243).
Python native first check failed E0277 on opaque Rejection conversion; retry
check passed and Clippy passed after two unsuppressed style fixes. This excludes
the package-tool binary and all actual execution. Ruby native/compiler and actual package/runtime/whole-product
gates remain NOT_RUN; qualified tuples=[]; no current closure audit, registry
artifact or publisher verification. Compilation never establishes native runtime.

## Explicit hosted smoke source

adapters/python/pilot-contract.json describes3required Rust test groups,7target
cases: Flask/Django WSGI and Starlette/FastAPI/Django ASGI dependency failures,
plus Starlette/FastAPI trusted-server lifespan startup/shutdown. HTTP cases deliberately
use an unavailable authority fixture, expect503 and independent native spy count0.
adapters/ruby/pilot-contract.json describes3required Rack/Sinatra/Rails object
callbacks with the same dependency-down expectation. These are source-defined
denominators, not executed counts. All10cases remain NOT_RUN. They do not qualify
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
The Rust gem driver invokes this entrypoint and unchanged RubyGems local install,
owns/reaps only its children, caps output1MiB per stream and applies120s deadline.
Upstream Ruby Digest computes archive SHA256. Clean require uses fresh GEM_HOME/
GEM_PATH and both fixture feature bits false. This standard Rust tooling alone
does not establish Ruby ABI, actual Rack or generation provenance. No remote
install, publish or token operation exists.

Both native modules expose capabilities JSON with actual host-pilot/package-tool
feature bits. Installed production wheel inspection must reject either bit true.
Node's common packaging boundary likewise exposes actual hostPilotEnabled. A
single native module initializer handles read-only capability receipts and the
optional host fixture; production-load mode rejects the host-pilot feature before
fixture code. Generation/inspection loads both upstream-generated CJS/ESM entry
points under Node/Bun with closed stdin, fixture/loader override variables cleared,
and requires exactly one actual receipt with hostPilotEnabled=false. Missing,
duplicate, non-boolean and oversized receipt values cannot qualify a package.
Successful loader exit alone does not establish this property. No authored JS
wrapper is introduced; native feature/import gates remain separate from actual
framework/authority and full-product qualification.
Node clean-install acceptance is a separate Rust CLI install-test ARCHIVE gate:
copy bounded tarball bytes to a fresh owned directory, retain their SHA256, invoke
upstream npm offline with lifecycle scripts disabled, and verify installed package
identity/exports. Import installed generated CJS/ESM under Node/Bun, then resolve
the public package through Node --require and --import with closed stdin. Every
load must supply the unique production capability receipt. These six imports
prove neither generation lineage nor actual framework/authority behavior. No
authored JS entrypoint, installation hook or publishing action is permitted.
Ruby has an explicit read-only native capability receipt environment mode for
upstream ruby --require, without an authored Ruby script. Empty qualified tuples
remain explicit; code feature detection is not ABI/runtime qualification.

## Actual WSGI server factory source

host-pilot-only pilot_wsgi reads a bounded private fixture request whose
configuration member is the original JSON string, preserving strict-parser
duplicate detection. It constructs actual Flask or Django URL routes through
public object APIs, with Rust-native views that echo binary201, set two cookies
and append independent body-length/SHA256 effect receipts in the owned lab.
No foreign executable source or code-string factory exists. The production module
must exclude this feature, enforced by installed capability checks.

Root approved and integrated three existing Rust dev path dependencies and the
separate Waitress3.0.2 official wheel-hash recipe. No new external Rust package
version was added; installation and installed-closure audit remain NOT_RUN.
Pinned upstream source exposes REQUEST_URI and a disconnect callable.
Unix-only wsgi_server authors six mandatory actual groups/28 planned cases and
two pure framing groups. Separate host/authority support owns process/listener
cleanup and reuses shared test-only evidence/audit construction. Gated-disconnect
cases include a matched connected control, an early evidence timing assertion
and exact ALLOW audit records; no-effect alone is insufficient. Accepted IPC
timeout proves a request reached the stalled peer, not merely an absent socket.
Live-authority invalid requests must preserve the original effect/audit counts.
Incomplete input must close within the separately configured server deadline.
Existing object smoke counts do not include these unrun HTTP-server cases.
adapters/python/server-pilot-contract.json preserves this mandatory gate with
harnessSourceImplemented=true and a successful Unix check/Clippy against source
8eb66811 recorded in summary007. Summary008 later records a linked test executable,
native fixture extension and private dependency setup; executedCases remains0
because resource admission denied launch. No prepared artifact is runtime proof.
Eight groups must execute with zero ignored/
filtered cases. No source/object-smoke result qualifies actual runtime or Windows
ABI/IPC; no server group is silently skipped green.

Later execution evidence supersedes only the historical WSGI NOT_RUN state:
summary009 retains the first immutable8eb66811 actual failure (6passed/2failed,
10rows/22cases, exit101). Fixture-only correction7842b6a8 sets the stalled
peer's required0600 permissions while retaining accepted request-byte proof,
and compares exact fresh owned empty WAL bytes and file identities before/after
shutdown without reopening empty history. Production history validation remains
unchanged. Summary010 records new immutable7842b6a8 coherent artifact builds,
all-targets check/Clippy and local UID1000 WSL actual success:8passed,0failed,
0ignored/filtered,6actualgroups+2pure,14rows/28cases. Source2254-file and reused
dependency manifests stayed unchanged; owned cleanup was empty. This establishes
local WSGI component behavior only. Hosted execution, ASGI realserver, production
wheel clean install, closure audit and Windows qualification remain independent.

The dedicated python-validation.yml path is a component workflow with one
standard Ubuntu job, exact github.sha checkout, Rust1.99.1, jobs1/incremental0,
contents:read and no automatic artifact upload/cache write/publication/OIDC.
Only codex/python-validation push and manual dispatch trigger it; root alone
pushes/dispatches reviewed source. Rust tests/component_controller.rs reuses the
sole wsgi_server cases. It prepares an ordinary-actor private venv, check/Clippy
and required pure framing tests before coherent artifact builds and execution.
Required discovery8, exact passed8/failed0/ignored0/filtered0,14distinct rows and
28cases are independently parsed; missing/duplicate/unknown cases fail closed.
Source, dependencies and copied artifacts bind to before/after hash closures.
Safe bounded stdout exposes only status/counts/hashes; private output stays local.
The shared helper bounds/reaps the direct harness; a recorded process group,
current UID and exact private task marker govern cleanup of known children.
Missing identity or a group escape fails cleanup and qualification. This is not
arbitrary process-tree containment. Controller/policy/hosted execution remains
NOT_RUN until separate compiled and actual receipts exist; the earlier local
28-case receipt does not qualify newly authored controller code.

Root static review rejected the draft's check-then-signal group cleanup: the
direct leader may already be reaped, so procfs UID/group/task-marker observations
do not atomically exclude identifier reuse. Source checkpoint535f879f is
NOT_ACCEPTED and was never executed. Primary code removes raw group signals;
the coordinator unconditionally refuses before provisioning or actual execution.
Cleanup observations cannot return qualification success. Execution requires
the separately reviewed shared owned-lifetime contract from test-support/native
security owners; no duplicate ad hoc containment may replace that contract.
Receipt/provenance pure verification remains a separate possible work item.

The next ASGI server slice uses a host-pilot-only Rust factory through an upstream
server's factory interface. It reuses the same bounded private factory request,
strict original configuration string and minimized effect writer as WSGI.
Starlette/FastAPI mount a native ASGI binary echo origin through their public
mount API; this exercises framework ASGI routing/middleware, not FastAPI parameter
validation or dependency injection. Django uses its standard ASGI application
with the shared native Django view. The outer production Asgi adapter remains
the only policy/authority boundary. Native async origin callbacks reuse the
owning-loop scheduler, with bounded buffered input and binary201/two-cookie
responses. No Python source/eval/template is authored. A candidate server must
be pinned and its actual HTTP, lifespan, partial-body and disconnect behavior
verified separately under the reviewed lifecycle contract. Keep-alive timeout
is not a total request-body deadline. No source-only factory qualifies any of
these runtime requirements; framework-mount scope stays explicit in receipts.
Primary references: https://uvicorn.dev/settings/ ;
https://uvicorn.dev/server-behavior/ ;
https://fastapi.tiangolo.com/advanced/sub-applications/ .

The WSGI six actual case bodies now live once in
tests/support/framework_server_cases.rs. WSGI retains its six named wrappers,
two framing units and28 planned cases. ASGI adds matching wrappers over
Starlette/FastAPI/Django:42planned cases/21rows plus the same two framing units.
The suite selects the authenticated PythonAsgi/PythonWsgi authority profile and
asserts the corresponding normalized observation mode; scoring/evidence/audit
remain shared. Existing WSGI refusal expectations stay intact. ASGI unsupported
upgrade and receive-deadline exceptions expect server500 with no authority/origin
effects, rather than claiming they return the synchronous WSGI503 response.
Exact empty owned WAL verification, accepted-byte stalled-peer proof and matched
connected/disconnected ALLOW controls remain in the common implementation.

The ASGI candidate is Uvicorn0.54.0 on Linux CPython3.12 with asyncio/h11,
one worker, no reload/WebSocket/proxy headers, and stripped UVICORN_* overrides.
Official PyPI wheel hashes pin Uvicorn/h11/Click in uvicorn-requirement.txt;
this is not installation or an advisory/closure audit. A bounded version check
must match the selected profile before a server can start. Starlette/FastAPI
enable lifespan; Django6.1.3's HTTP-only ASGI handler uses lifespan off, explicitly
not a Django lifespan-support claim. All new ASGI wrappers and its host launcher
call the same unconditional component_lifecycle refusal before fixtures/processes.
The dedicated controller uses that same refusal; no signal/group workaround is
introduced. All refactored source is uncompiled/unexecuted; historical7842b6a8
WSGI evidence cannot qualify these changed bytes. A reviewed shared lifetime
contract and allocated immutable compile/runtime receipts remain required.
Primary source: https://raw.githubusercontent.com/django/django/6.1.3/django/core/handlers/asgi.py .

Later summary012 supersedes only the typecheck-NOT_RUN state on exact5655b965:
host-pilot all-targets check passed3.01s for the ASGI factory/shared suites and
controller definitions. Clippy failed one receipt-parser byte-slice idiom;
pure tests did not run. No linking or runtime/package/lifecycle qualification
follows. The equivalent primary idiom fix has no suppression and needs its own
immutable retry. Source2313-file closure stayed unchanged; B returned terminal101.

Summary013 subsequently records immutable d68682e4 all-targets check and strict
Clippy passes, plus exactly2receipt pure passes/0failed/0ignored/2filtered.
Filtered definitions are blocked controller/cleanup entry points, not server
case waivers. Source2325-file hashes stayed unchanged. No new actual framework,
package, hosted or lifecycle result is inferred; NOT_ACCEPTED remains unchanged.

CA-API-01: the shared process-wide admission guard must be constructible only
after a successful acquire. Its payload/constructor is private; safe external
Rust must not manufacture a value and run Drop without owning an increment.
The bound remains64 and each owned guard releases exactly its acquired slot.
An external compile-fail rustdoc example rejects direct construction and an
external positive no-run example checks the supported acquisition API; the scoped
unit retains the full-capacity refusal and verifies one dropped guard releases
one reusable slot while the other63 guards remain owned. This repairs a public
Rust API ownership invariant shared by Node/Python/Ruby; no remote HTTP exploit,
score change, threshold change or new signal is claimed. Actual scoped unit,
doctest and Clippy receipts are required before marking the correction verified.

CA-API-01 verification is now recorded in summary014 on exactac7dc4a9:6units,
external positive/no-run and direct-constructor/compile-fail controls, scoped
check and strict Clippy all passed. Source/registry/config ledgers were stable;
the independent auditor reread the actual saved manifests/output and accepted
this narrow source invariant/regression closure. Capacity64, scoring and policy
remain unchanged. Native SDK/HTTP/lifecycle/product qualification is separate.

The L4 role consumes one common contract documented in
crates/framework-adapter/CONSUMER-CONTRACT.md version0.3, draft for independent
review. It consolidates the accepted existing request/admission/deadline/
Authority semantics, lower-provider ownership, per-language ABI/server/GC/GVL
obligations and separate L0-L5 promotion evidence. It grants no new native API,
readiness flag, privilege, source ownership or execution. Symbolic owner models,
path inventories, direct-child capture, full-tree cleanup and production secret
ingress stay distinct. Public configuration cannot mint a qualified provider.
Actual production/host qualification requires the applicable genuine lower
contract and source-bound tuple; upper consumers can prepare contract/negative
tests while those providers mature. The original complete Node/Bun/Python/Ruby/
package/hosted objective is preserved, not reduced to pure units or historical
WSGI component success. No foreign owner paths are changed by this draft.

Authority review clarifies that BoundResponse consumption is result ownership,
not nonce replay or a type-level at-most-once host invocation. L4 owns one
decision attempt/handoff per request and cannot retry/recreate tickets.
Pending's synchronous handoff check is not atomic epoch admission across an
await/queue/GVL boundary. Current UID-only Unix and SID/DACL/server-PID Windows
verification does not establish cross-principal R1 attestation. Actual delayed
handoff/expiry/cancellation evidence and each required lower capability remain
separate gates. Native review confirms a dev codec/source-ready process API is
not a production proof or an automatically required framework edge. Only the
pilot's explicitly accepted capability/profile and genuine receipt may unblock
its actual controller; the existing NOT_ACCEPTED refusal is unchanged.

CA-HANDOFF-01 requires retaining the actual BoundResponse through all host
preparation, trusted liveness callbacks and VM/owning-loop delays. Inspecting
verdict is branch selection only. Check the original Pending and consume only
immediately before the trusted application invocation, without another await,
queue or callback. The WSGI/Rack source correction implements this ordering;
its SDK check and actual late-connected/no-origin plus timely-connected control
remain NOT_RUN. ASGI's source correction moves the real result and original
Pending into the owning-loop callback and consumes immediately before app
invocation after cancellation/one-shot inspection. It retains the permit through
task completion; queued expiry/cancel and timely controls remain NOT_RUN.
Node's new source correction reuses original shared Pending and retains the
opaque result until final VM invocation; check/Clippy, clock negatives and actual
late/queued/abort controls on those changed bytes remain NOT_RUN. No current
source-only path is qualified shared behavior.
The genuine ALLOW must precede the delayed connected callback in the regression;
an earlier dependency timeout is insufficient. No native epoch atomicity,
actual controller admission or broader host qualification follows from this fix.

The new host-pilot Unix lib tests exercise the actual embedded CPython WSGI
callable with the existing real component Authority/audit and frozen synthetic
evidence. They explicitly observe both trusted callbacks and one audited ALLOW;
only the final callback delays2100ms against the original2000ms request budget.
Late connected must refuse with zero origin calls, while timely connected must
invoke exactly once and preserve binary bytes. This is not Waitress/server input
or production evidence, and execution remains NOT_RUN. The existing private
directory/configuration fixture is extracted once for SDK and server reuse;
no child controller, transport/peer clone or new native provider is introduced.

ASGI setup ownership is a separate open gate: retain a created task and its
permit before fallible completion-callback registration/cancellation. An Err
completion must not disable Guard cancellation. The narrow source correction
does this, but failed callback registration, failed cancellation, a custom task
factory creating a task before raising, and cancellation-suppressing application
code do not provide terminal settlement. Such outcomes are cleanup-uncertain,
not successful qualification. Supported-loop/task-factory admission and bounded
fault-injection controls must observe actual task settlement and permit lifetime;
no source-only cancellation request closes that obligation.

Windows compile/link receipt summary016 binds exact252ad259 to CPython3.12.10,
Rust1.99.1/MSVC and PyO3 0.30.1: all-target host-pilot check/strict Clippy and
lib-test SDK linkage passed; source2742/SDK2614/selected registry5639/config1/
selected tool7 ledgers were unchanged. One actual linked executable is preserved.
No tests executed: the two SDK WSGI controls remain Unix-only/unrun. This closes
the corrected-source Windows compile/link gate, not CA-HANDOFF or task settlement.
Actual Windows peer/private-parent fixture admission and every required host/
fault/queued/runtime/package gate remain separate. No WSL/UNC use occurred.

### Lowest-dependency Node clock/final-consume slice

The Node source now starts the common Pending before host metadata/input reads,
uses remaining time for its input timer and body checks, and calls common decide
without resetting the IPC timeout. Immutable Arc body/observation/budget values
cross the Rust worker boundary; N-API leases remain in the VM callback. The actual
BoundResponse survives scheduling and replay/function/argument preparation.
After final trusted liveness checks, original Pending plus actual consume gate
Express next or Fetch.apply without another host lookup/callback in between.
DENY/CHALLENGE still consume and reject. This is source-only, not executed proof.

Three deterministic shared-Pending units cover repeated stage reads without a
budget reset, wall rollback bounded by the original monotonic deadline, and
expiry of either clock while the other has time. Production clock acquisition
is unchanged; the private arithmetic seam supplies exact clock samples only in
unit tests. No OS clock change or fake Authority result is introduced.

A Rust-only host-pilot Hono fetch-property probe provides timely and1100ms late
lookup against an exact1000ms fixture budget. It writes distinct private entry/
return receipts and returns the original function; the JS timer cannot run while
the VM getter is blocked. Paired actual cases must establish a genuine audited
ALLOW before that lookup, both receipts, and origin0/503 late versus origin1/
binary/cookie-preserving timely. A delay before ALLOW or a failed dependency is
not the negative. Probe source is prepared; its actual driver/lifecycle profile,
Node/Bun/Express tuple controls and execution remain pending. It is not exported
in production and cannot mint a trusted provider or change scoring/ABI names.

Python ASGI's registered-success task ownership is source-implemented and had
Windows252 compile/link evidence only. Failed task creation/registration/cancel,
suppressed cancellation and late completion remain cleanup-uncertain. A future
bounded actual loop fixture must observe permit retention until real terminal
settlement or retained uncertain ownership, not infer it from cancel/Err/status.
Windows WSGI still needs the qualified native private-parent/peer fixture; no
fabricated SID/DACL, arbitrary TEMP fallback or WSL restart fills that dependency.

Summary017 records actual frozen0a1 Windows units: all9 shared framework units
(including the3 original-clock controls) and4 default Node units passed with
zero failures/ignored/filtered. The subsequent host-pilot check failed on the
new Hono getter return lifetime, so strict Clippy and allocated fmt did not run.
Source2804/selected registry5059/config1/missing-config14/tools8 remained stable;
execution120.73s/cleanup0.936s ended101/closure0 and the exact owned target lease
was removed. The source fix uses the existing safe host-holder return pattern;
its host-pilot check remains NOT_RUN until a separate exact-source allocation.
Pure clock success does not establish actual VM callback or host qualification.

Summary018 binds corrected4f to a successful Windows host-pilot all-target check;
strict Clippy then failed only on two redundant Fetch.apply borrows. The source
correction passes the same app Object directly, preserving final-consume order.
Work218.98s/cleanup1.66s stayed within original360+60; unchanged source2815/
registry5059/config15/Rust-tools4 ledgers, terminal101/closure0 and actual lease
removal are retained. No unit/host/SDK execution or reuse of historical13 passes.

### Registered ASGI completion owner

Done previously retained only Weak<State>, allowing observer/Cancel release to
drop the task/permit owner before a still-pending registered task completed.
The narrow source correction makes registered Done retain Arc<State>, checks
Python identity against the held task and actual done()==true without a Rust
lock held, then rechecks ownership and takes task/permit/result under the lock.
References and results are dropped/sent only after unlocking. This breaks the
deliberate retention cycle only after matching terminal observation. Done is
allocated before final decision consumption/application invocation.

Three Rust-only actual CPython Future controls are authored, ignored by default:
late terminal callback after observer drop, cancellation before its registered
terminal callback, and wrong/nonterminal callback refusing permit release. Each
uses real acquired permits and real asyncio Futures, not a minted bound decision;
they qualify registered completion ownership only if explicitly run and passed.
They are currently NOT_RUN and not full ASGI/TaskFactory/cancellation-suppression
or Authority proof. The strong cycle may intentionally persist if task/loop never
completes: bounded admission exhaustion is fail-closed, not successful cleanup.
Registration failure still lacks a guaranteed anchor if no callback attached;
partial creation/factory failure needs a separate retained-failure keeper/profile.
No current Windows private-parent/peer or interactive host dependency is waived.

CA-ASGI-DROP-01: Guard::drop is best-effort cancellation scheduling, not a bounded
or nonblocking teardown path. The initial State Mutex::lock can also block.
Python::attach can wait for the GIL and can panic
in unsupported uninitialized/shutdown/GC-traversal states; loop method calls can
execute Python and a stopped/closed loop or failed scheduling can prevent Cancel
from running. No hard deadline encloses this Drop path. try_attach is not a
timeout or a nonblocking guarantee. Registered Future ownership controls above
do not test this path or prove interpreter/loop/task/permit cleanup.

Future admitted teardown controls are named asgi_drop_contended_state_mutex,
asgi_drop_contended_gil,
asgi_drop_interpreter_shutdown, asgi_drop_gc_traversal,
asgi_drop_stopped_loop and asgi_drop_schedule_failure, each with a timely control,
retained owner/permit observations, actual terminal or unknown outcome, and an
independently bounded supervision/cleanup contract. They remain NOT_RUN and need
an exact root-accepted lifecycle/failure-retention design and supported SDK
allocation. No API swap, new unsafe, authored foreign harness or full ASGI cleanup
claim follows from correcting this source overstatement.

### Next public facade candidates, pending acceptance

The owned framework INTEGRATION now records proposal0.1 for adapter-framework,
adapter-node, adapter-python and deferred adapter-ruby modules under humanymous.
Current root manifest still admits only []/[adapter-go]. Shared and Node Rust
surfaces already exist; Python classes are hidden by private modules/constructors
and Ruby has no public Rust binding API, so empty crate aliases cannot be called
usable adapters. Root must accept any existing-type visibility change first.
The proposal separates SDK-free-host Rust, host-qualified Rust, Node cdylib,
Python extension/embedding and Ruby extension products, explicit feature sets,
generated loaders, host init names and restored-package gates. Additive Cargo
features require separate extension/embedding build closures and no production
fixture/tool exports. No root Cargo/features/deps, leaf API or runtime changed.
Registry availability, current namespace and historical SDK checks do not qualify
the proposed profiles. Independent critique and root acceptance precede wiring.

### Eight-package baseline: exact owner handoff

The user confirmed structural-development baseline v0 on2026-11-08. Framework,
Node, Python and Ruby responsibilities move into the intentionally public
humanymous-adapters package only through root's sequenced slice. This supersedes
treating the earlier per-context Cargo package candidates as required permanent
packages. It does not make all internal modules/constructors public or qualify a
host role. No independent move, rename, manifest change or ref synchronization
is authorized by this owner handoff; root must first freeze this specific group.

Snapshot adapter-migration-handoff-001.json in evidence:root/20261107075243 binds
baseline commitd6da3e9c9d0b7709d184fe122729329545057c78 to87 tracked files using
relative paths, byte sizes, exact worktree SHA256 and canonical Git blob SHA1.
Counts: framework7, Node16, Python24, Ruby9, node assets18, Python assets8, Ruby
assets4, Python workflow1. Moving roots were clean at capture; active owned
compiler/process handles and target leases were empty. Historical immutable
materializations/targets, raw evidence, ignored native outputs and node_modules/
SDK caches are excluded and must not be pruned or relabelled as moved proof.
The handoff metadata/this SoT update are outside those moving roots. It is a
snapshot, not a live freeze: any subsequent edit to the87 inputs requires a new
versioned hash handoff, never silent reuse or overwrite of this receipt.

Root must resolve these hazards in its exact destination/path/API matrix:

- One library target must keep distinct Node/Python/Ruby/core/.NET host-role
  artifacts and entrypoints through separate admitted feature/build/staging
  profiles. Current leaf native/host-pilot/package-tool features have different
  meanings; merging them under one undifferentiated native switch could activate
  both Python and Ruby or ship fixture exports. No all-features host product.
- Preserve module/context-restricted custody. Admission's private permit and
  actual Authority bound-result ownership cannot become sibling-constructible.
  Keep Configuration/error types in their original contexts rather than flatten
  same-named framework/runtime configs. Python's private Rust constructors and
  Ruby's private binding API need curated visibility, not blanket pub(crate).
- Node N-API, PyO3 and Magnus build/init/SDK boundaries and reviewed unsafe macro/
  export scopes must survive module moves; do not broaden unsafe into safe core.
  Generated CJS/ESM/init/export/header identities need final artifact tests, not
  namespace inference. Python extension and embedded products remain separate.
- Packaging drivers currently assume CARGO_MANIFEST_DIR.parent().parent() is the
  repository root. Nested branded paths invalidate that assumption. Update owned
  package recipes/manifest paths, explicit library/bin targets, relative shared
  fixture/include_str paths, pilot discovery and Python workflow Cargo selectors
  together. Root decides whether packaging tool targets belong in adapters or CLI;
  they must not become default installed product commands by accident.
- Replace external context imports only against accepted core/runtime/platform
  APIs; reuse one canonical implementation. Test-only frozen evidence and process
  helpers need explicit testing capability and real provider contracts, with no
  readiness bypass. Private parent/peer/interactive ownership is still unqualified
  for the pending actual Node/Windows WSGI pilots; no DACL/TEMP/PID workaround.

Required slice tests: scoped fmt/check/strict Clippy; shared original-clock and
admission/forged-constructor controls; Node pure and actual VM late/queued/abort/
body/cookie cases; Python SDK linkage plus3 registered-completion controls and
separate WSGI/ASGI/GuardDrop teardown cases; Ruby native ABI/GC/compaction/GVL/
worker settlement and real framework cases. Validate unknown/incompatible/fixture
features, exact type/visibility identity, extracted-package consumers and final
restored npm/wheel/gem imports for each role/tuple. No zero-selected, ignored or
old-source result counts as migrated proof. Preserve summaries010/014/016/017/018
and every original source/failed denominator; current pending SDK/runtime work
is not closed by this handoff. Root owns common graph/locks and final Cross-OS/
Docker/product gates. No publication or main integration authorization follows.

#### CORE-SLICE-01: separately authorized consumer references

Root subsequently assigned exactly the four current adapter Rust source/test
trees for mechanical core consumer edits, superseding the no-import-change hold
for this slice only. Eight files had integrity/signals references; none had the
other five old core crate references. They now use humanymous_core::integrity or
humanymous_core::signals, with no behavior, weights, visibility, SDK/API or path
move. Root alone edits all Cargo manifests/lock and relocates the seven contexts;
its concurrently dirty adapter Cargo manifests are foreign and excluded here.

core-consumer-handoff-001.json records exact eight before/after source hashes.
The earlier87-file snapshot is historical before these assigned edits and root's
manifest updates; it is NOT the current move freeze. The shared fixture include
now points to the actual sdk/humanymous-core/tests/scoring/fixtures path. Canonical
filtered Git blobcbf3e6c6 matches the original. The raw moved worktree blobd2cb0146
has preexisting CRLF: failed raw-versus-normalized equality is not migration
drift. Root separately verified all26 moved non-Rust rawSHA256 against its
pre-move worktree map, differences0. No byte restoration/normalization is required
or performed; both observations remain distinct provenance. The include remains
a temporary current-tree path, not a permanent published-package escape.

Direct pinned rustfmt and whitespace/reference scans pass. No old qualified core
crate references remain in these four Rust trees. No Cargo build, Clippy, units,
CI or runtime was launched while root's workspace integration is incomplete.
Compiler/actual-host/SDK gates remain pending explicit coherent-source allocation;
no original receipt is promoted to the new core graph. Root must reconcile
manifests, dependency visibility/type identity and fixtures before that gate.

Primary references: https://flask.palletsprojects.com/en/stable/api/ ;
https://docs.djangoproject.com/en/6.1/ref/urls/ ;
https://docs.pylonsproject.org/projects/waitress/en/stable/runner.html ;
https://raw.githubusercontent.com/Pylons/waitress/v3.0.2/src/waitress/task.py ;
https://raw.githubusercontent.com/rack/rack/v3.2.8/SPEC.rdoc .

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
- https://raw.githubusercontent.com/matsadler/magnus/0.9.3/src/value.rs (exact Qfalse/Qtrue conversion)
- https://guides.rubygems.org/c-extensions/ and https://guides.rubygems.org/platforms/
- https://asgi.readthedocs.io/en/latest/specs/www.html
- https://peps.python.org/pep-3333/ and https://github.com/rack/rack/blob/main/SPEC.rdoc
- https://rustsec.org/advisories/RUSTSEC-2026-0176.html: PyO3 iterator overflow
  patched>=0.29.0; candidate0.30.1 is outside this advisory's affected range.
  This is not a current full-closure security audit or a no-vulnerabilities claim.
#### PLATFORM-SLICE-02: authorized test consumer references

Root authorized four mechanical Python Rust test-source replacements from
humanymous_test_support to humanymous_platform::testing. The exact paths and
before/after hashes are recorded in platform-consumer-handoff-002.json in the
shared compilation evidence bundle. Three process consumers and one strict JSON
receipt parser retain their existing suffixes and behavior. No matching references
exist in the owned Node, Ruby or framework Rust sources; no windows-object bridge
references were found. The Rust parser source named wsgi_receipt.rs is distinct
from immutable historical execution receipts, which remain untouched.

Root alone owns platform file moves, manifests, lockfile and feature/dependency
categories. No alias, shim, authority change, unsafe change or host activation was
introduced. This checkpoint is source-only. Compiler, unit tests, Clippy, CI and
Cargo formatting await Root's coherent allocation; no runtime completion or cleanup
qualification follows from these substitutions. No owned execution handle or
target lease is active. The four consumer files freeze after handoff.
#### RUNTIME-SLICE-02: authorized runtime consumer references

Root authorized mechanical references in fourteen owned Rust source/test files.
Existing runtime prefixes were replaced first, then authority, audit, capture,
network and native_security package prefixes mapped to their Runtime contexts.
Actual matches cover authority, audit and native_security; the other prefixes
have no matches in these owned sources. Exact before/after hashes and paths are
recorded in runtime-consumer-handoff-002.json in the shared evidence bundle.
No behavior, scoring, authority decision or unsafe boundary changed. No aliases
or compatibility shims were added. Root retains all manifests, feature selection,
lockfile and six-context destination files. Source-only verification does not
qualify an actual host. Compiler, tests, Clippy and formatting are not run pending
Root allocation. No owned execution handle or target lease is active; consumer
files freeze after source handoff. Historical runtime receipts remain unchanged.
## Python/Ruby artifact-role candidate 003 (source only)

Root assigned Python/Ruby-only versioned role metadata, artifact selectors and
package lifecycle deltas in uniquely owned evidence paths. Candidate003 contains
six full Rust candidate files with exact main-worktree preimage SHA256 and final
candidate SHA256 in preimages.json. Root alone applies these after comparing every
preimage; stale preimages require rejection and a new reviewed delta. No shared
manifest, Node source or read-only Tools integration source was changed.

The candidate adds host-artifact-role/1 metadata: exact Python/Ruby context,
ABI major1 and production/host-pilot/package-tool role. Combined pilot/package
features produce unsupported-combined, rejected by package validators. A shared
package ID or two false flags alone cannot admit an artifact. Missing, wrong-major,
wrong-schema, cross-language, feature-role mismatches and combined roles fail closed.

The Python Cargo harness selector joins exact package ID obtained via Cargo pkgid,
target source, test kind, bin crate type, test profile, exact feature set and one
absolute executable below the allocated target. Malformed streams, ambiguous rows,
wrong sources and path escapes are rejected. Staging checks selected and copied
executable digests before/after copy. This is an observation across staging, not an
atomic custody/TOCTOU proof. The controller's existing lifecycle refusal remains.
Root must remap selectors when moving these adapter contexts; old literal paths
are bound to the recorded preimages and are not a compatibility guarantee.

Python wheel inspection hashes its sole native member. A future allocated clean
import joins the installed relative member path, byte count and SHA256 with that
inspection. Wheel .data relocation layouts are unsupported by this initial exact
path join. Verification is post-import observed bytes, not authorization before
native code execution, loader custody or generator provenance. Framework/product
qualification remains false. Ruby records loadedArtifactJoinVerified=false: no
Ruby loaded-extension byte/custody join has been observed or fabricated.

Both package drivers refuse build/install before process or staging effects until
an allocated shared owned-process lifecycle is integrated. Ruby run also refuses
before spawn, retaining the old implementation only as non-authoritative source.
Its old reader joins and Python unbounded package subprocesses must not become
completion authority. An environment opt-in is insufficient to authorize them.
The added refusal is deliberate incomplete functionality, not a completed lifecycle.

Six Rust tests are authored, NOT_RUN: two role-contract suites, Python exact Cargo
selector negatives, Python archive/import byte-join negatives and two lifecycle
refusal guards. Candidate-only Rust1.99.1/edition2024 rustfmt passes. No compiler,
Clippy, SDK, host, Docker, browser, hosted run or runtime tuple was executed.
Root allocation is required for actual scoped tests and package/host qualification.
