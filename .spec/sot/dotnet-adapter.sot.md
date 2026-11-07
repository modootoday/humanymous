---
id: sot:root/dotnet-adapter
title: ASP.NET Core Rust adapter lifecycle
status: active
created: 2026-11-07T07:30:00Z
updated: 2026-11-07T07:30:00Z
verify: manual
rel:
  governs: [crates/dotnet-adapter/**, adapters/dotnet/**]
  depends_on: [sot:root/rust-migration, sot:root/adapter-service-contract, sot:root/adapter-authority, sot:root/server-adapter-expansion]
---

# ASP.NET Core Rust adapter lifecycle

## Source-bound release compilation (2026-11-07T10:07Z)

Separately allocated C compiled the release Rust export DLL and hosted Rust test
executable from standalone clean source2ffa4335df49c111685a764a81f8e4b414685cd4.
Both actual commands returned0 (76.032s/12.616s),261recursive input hashes did
not drift, and the source checkout remained clean. Cargo JSON selected the exact
artifacts for source/lock/compiler/target/native/harness hash provenance. Evidence:
evidence:root/20261107100735, private evidence:root/20261107100408. C returned.
The harness used --no-run. No actual test, SDK build, PInvoke, Kestrel, managed GC,
NativeAOT or package installation executed; next stages require allocation.

## SDK preparation only (2026-11-07T09:48Z)

Root separately allocated A for private official Windowsx64 SDK10.0.403
preparation. The downloaded ZIP matches its official channel metadata SHA512.
Only --version and --info executed, both exit0, reporting SDK10.0.403 and
runtime10.0.14/win-x64. A fresh owner-only protected directory holds the SDK,
CLI home and package cache; no global PATH/admin installer/SCM/account operation
occurred. ASP.NET certificate generation was suppressed and telemetry opted out.
A returned. Evidence: evidence:root/20261107094859, private evidence:root/20261107094726.
This establishes tool readiness only, not managed compilation or host behavior.

## Allocated revalidation after quiet ended (2026-11-07T09:42Z)

Root explicitly ended the benchmark quiet window, resumed this work and
allocated A with the default target, jobs1/incremental0 for Rust validation only.
The latest package/support/host fixture source type-checks and passes all-target
Clippy. Core12/export1pure tests pass,0failed/ignored/measured/filtered;259selected
recursive source/configuration hashes are unchanged before/after. A was returned.
Evidence: evidence:root/20261107094251; private raw evidence:root/20261107094127.
No managed scope test execution, SDK build, Kestrel/GC/NativeAOT/package install
or product pilot is implied. These actual stages require the next explicit root
allocation. Earlier quiet/blocker records below describe their recorded time.

## Execution authorization update (2026-11-07)

The human authorized actual local Windows/WSL timing/resource comparisons,
superseding the earlier hosted-only blanket. Root's benchmark quiet window
currently forbids new Cargo/native/runtime/model/Docker commands; no local SDK
or host run has started here. Future actual tests require explicit root allocation
or a standard hosted job and record execution_environment plus OS/arch. The local
allocation environment marker prevents accidental runs; it is not an approval
or security token and cannot itself replace root scheduling/user authorization.
Earlier immutable hosted-only records remain historical. The root roadmap is
the execution-policy authority; this adapter does not allocate itself a slot.

The ignored Rust package orchestrator accepts prebuilt native/harness artifacts
and strict source/lock/toolchain/target/hash provenance, checks committed clean
source and SDK10.0.403, then packs and publishes source/clean-consumer JIT/AOT
fixtures. It never recursively invokes Cargo, never manually injects the native
asset into the packed consumer, and compares installed bytes/hash to the supplied
artifact. It copies only the approved pilot source/project/props unchanged.
All SDK outputs and NuGet lockfiles are directed into the actor-owned private
build root. It uses existing OS hash tools, no new hash implementation, dependency
or C# source. Per-stage real exit/timing logs and separate host profiles are
required. Source is currently NOT_COMPILED/NOT_RUN, including these new tests.

## Approved managed binding continuation (2026-11-07)

The human explicitly approved the minimum C# binding, middleware and actual-host
pilot exception in this chat and root confirmed exactly three authored sources:
adapters/dotnet/src/Humanymous.Native/NativeBinding.cs,
adapters/dotnet/src/Humanymous.Native/HumanymousMiddleware.cs and
adapters/dotnet/pilot/Program.cs. No fourth authored C# file, executable foreign
template or copied security engine is approved. Project/XML/package configuration
is configuration only. managed_scope.rs automatically verifies the exact source
inventory and rejects symlinks. Compiler-generated LibraryImport glue is upstream
generated output in target/dotnet, not an additional authored source. All comments
and documentation remain English. The prior blocker below is historical.

Bridge owns one shared native-abi Boundary/Registry instance, not a new generic
allocator. The native facade's only Rust exception is its pointer-free by-value
unmangled C export attribute; raw pointers/unsafe blocks/dynamic-loader code are
absent. Fixed Command/Response88/48bytes use exact ABI1/0 on64bit little-endian
Windows/Linux. .NET LibraryImport uses Cdecl and fixed integer fields, no arrays,
delegates, pointers or managed references. SafeHandle owns opaque context IDs;
DangerousAddRef leases span every synchronous call and the entire scheduled
Task.Run decision, including scheduling delay. No reverse callback is used.
The native Tokio runtime is process-lived. The binding rejects collectible
AssemblyLoadContext before opening native resources and never calls a manual
native loader/free API. Explicit DLL unloading/reloading is unsupported and
unqualified; zero request/flight/buffer metrics do not prove runtime worker
termination or unload safety. The generic native-ABI library has no workers and
its separate roundtrip result cannot establish this facade's unload safety.

Custom operations100..109 configure, begin, append, EOF, decide, cancel, final
handoff, classify trust headers via the shared function, copy replay and measure
bounded active-request/decision-flight/initialized-buffer diagnostics. Generic
operations1..7 reuse the registry. Generic CLOSE also cancels/drops owned children
and their buffers. PANIC/POISONED/Mutex poison revokes all bridge resources and
never resets the boundary. Context IDs are never reused. Library instances have
independent domains; handles cannot cross them. Rust-only Ticket/Completion also
carry an unforgeable per-Bridge Arc identity to reject cross-bridge completion.

Native DECIDE takes the request out of the global lock, then runs the shared
authority future on one Rust runtime. Cancellation is observed on a5ms timer and
drops the actual IPC future; timer latency is scheduler-dependent, not a realtime
guarantee. The original deadline remains mandatory. A replay copy is read into
private managed bytes before final HANDOFF rechecks authority expiry, deadline,
host abort/response-start/upgrade, method/raw target/host/headers and the exact
PathBase/Path/QueryString triple. This catches routing rewrites that leave
IHttpRequestFeature.RawTarget unchanged. No await occurs between final validation
and installing the owned replay and invoking the app. Admission is not an ALLOW.

The host installs first, before Body/BodyReader consumers or response/effect
middleware. It strips forwarded/capture authority header names only by querying
the Rust classifier, reads actual EOF, clears rented arrays, and restores the
original stream on completion. DENY maps403; CHALLENGE maps503 without pretending
to implement recovery. Missing dependency/abort/error refuses origin. No signals,
policy, scores, credentials or cryptographic decisions are minted by C#.

Candidate source exists; all new bridge/export/managed compilation, actual host,
GC, package, NativeAOT and product results remain NOT_RUN until recorded.
Native Registry limits16contexts/64buffers/8MiB initialized bytes apply in addition
to shared framework admission64 and body1MiB; no concurrency eviction is used.

## Packed host and private fixture inputs

The hosted fixture receives an existing verified private parent. Windows creates
new production/synthetic children through native-security's ordinary-user
initializer; service-SID factories are unchanged. Unix creates new0700children.
Existing children reject; each source/packed JIT/AOT invocation needs a distinct
parent. No ambient TEMP/admin trust or inspection-time ACL repair is introduced.

Candidate pack requires an existing explicit native artifact and accepts only
win-x64/linux-x64. It includes project LICENSE, owned candidate NOTICE/README
and the source-tree audit compatibility notice, without claiming the historical
Go engine dependency inventory applies to this Rust DLL. Complete resolved
license and vulnerability checks remain qualification gates.

Clean packed consumption must use NuGet.Packed.Config, an explicit local feed
through HUMANYMOUS_NUGET_FEED and a new empty NUGET_PACKAGES directory. Only the
exact candidate ID maps to the local feed; SDK package prefixes map upstream.
Existing global packages bypass source lookup/mapping, so a reused package cache
cannot prove installation of the measured nupkg. Record package and installed
native hashes, SDK/runtime/RID and source lineage. No publish/token operation is
authorized. These configuration and fixture changes after058a014 are NOT_RUN
until their next compile/hosted check; earlier13pure component results are not
transferred to new managed/package behavior.

Version0.1 is a safe Rust request core, not installed ASP.NET middleware.
PInvoke, NativeAOT, actual Kestrel, GC stress and package qualification are not_run.
No release-qualified tuple or NuGet package is advertised. Native ABI ownership
belongs to crates/native-abi; this crate must not duplicate handles or crypto.

## Ownership and decision rules

Use shared framework Configuration, process Admission, Pending deadline and
authority client. The authenticated DotnetAspNetCore producer uses identity
humanymous.dotnet/0.1.0-experimental.1, headerMode aspnetcore-normalized,
rawHeaderOrder=false, peer/connectionId/capture/stream=null. Header maps do not
establish raw TLS/H2 or original-client capture. Never relabel Node or Fetch.
Shared integrity owns this profile and parser. Frozen scoring is unchanged.

Begin before host body input and effects. One original wall/monotonic deadline
includes input, authority exchange and final handoff. Copy exact bounded binary
entity bytes; no UTF8 decoding, form parsing, JSON reconstruction, decompression
or content re-encoding. No managed buffer/pointer survives append. The host
must observe successful actual EOF; ContentLength alone never seals input.
Length mismatch, overflow, allocation failure, timeout or cancellation closes
the lifecycle. Unknown length is bounded and requires EOF. Admission is held
until handoff or request drop. Logical body length is at most configured1MiB
and shared process admission64; allocator overhead and authority encoding are
additional memory. Exact reservation avoids geometric Vec growth; no temp-file
spill is used by the Rust core.

Exactly one shared decision attempt is permitted, including when its future is
dropped before completion.
The owned DecisionAttempt guard cancels a request when a polled decision future
is dropped before successful completion or unwinds; no host callback is needed
for that native Rust cleanup. A never-polled future has not started an attempt.
Shared authority authenticates peer/scope/framing, exact request nonce/body/
binding, replay and durable audit. An external response
cannot be attached to a different Request. No Rust adapter score or synthetic
ALLOW capability exists. Only consuming the internal BoundResponse, rechecking
its expiry and original Pending, and ALLOW permits transfer of the owned body.
CHALLENGE/DENY/error never invoke the protected application. Recovery UX is a
separate capability; a status code does not implement a Pass challenge.

Cancellation tokens are per-request Arc state, never recycled integer handles.
Mutex order linearizes cancellation against the one handoff. Cancellation
winning before handoff refuses it. Cancellation after transfer belongs to the
application's running cancellation token; this core cannot undo origin effects.
Poisoned locks fail closed. Host binding must cancel and drop the decision future
on RequestAborted, timeout, dispose or app shutdown. Final handoff is synchronous
on the host request execution path after checking RequestAborted=false,
HasStarted=false, exact authorized method/raw target/authority/headers/route and
unchanged input. Rust retains the authenticated observation and compares the
current method/target/authority/sanitized ordered header values itself. The
HostState aborted/response-started/upgrade flags come from actual host features,
never client data. Original internal forwarding/capture headers are stripped by
the shared encoder and must also be stripped from the downstream host request.
No parallel application delegate may run before this linearization point.

## Managed host and ABI gate

Outside the exact three-source C# exception above, new foreign host glue remains
disallowed. Do not embed scripts or generate foreign templates to evade it.
The core remains safe Rust; the separate native facade permits only its reviewed
pointer-free unmangled export attribute. NativeAOT direct and dynamic linkage, architecture,
calling convention, fixed-width status/length types and loader paths need exact
tuple qualification. No callbacks or managed references are retained here.

The approved binding must use SafeHandle-style deterministic disposal,
retain managed delegates for the full callback lifetime if any, avoid async
borrows of pinned pooled arrays, keep HttpContext on its request execution path,
and never allow native access after release. Finalizer fallback is cleanup only,
not successful request completion. Actual managed GC/NativeAOT pilots are required.
Replay must install the exact returned bytes as the downstream body at offset0,
preserve content metadata, leave application response streaming untouched, and
restore/dispose host-owned wrappers on every terminal path. Request.Body and
BodyReader competing consumers, pre-read bodies, upgrade/CONNECT and request
streaming remain unsupported until an explicit host contract qualifies them.

## Critique reflected and verification

P0 response substitution: external BoundResponse injection is forbidden; Request
owns the sole attempt and result. P0 stale callback: request-specific cancellation
cannot target a replacement. P1 early EOF: declared length is only a constraint,
not completion proof. P1 disposed await: decision_started is set before await.
P1 GC proof: pure Rust units never count as managed host qualification.

Pure units cover binary ownership, exact EOF/length, cap, cancellation, terminal
failure, host mutation, expiry, DENY/CHALLENGE and one-owner transfer. Positive
private finish tests exercise only lifecycle mechanics, not authenticated ALLOW.
Scoped locked Cargo tests/fmt/Clippy need root Cargo scheduling. Real Kestrel
Windows/Linux JIT and NativeAOT GC/cancellation/replay/package tests run only on
free standard hosted runners. Real engine/audit and isolated Docker product
effects, restart/replay/peer failure and protected-origin counter are separate
gates. Exact source/artifact/configuration hashes and all failed/ignored/not_run
denominators are retained. No local process/Docker substitute is authorized.

## Sources

- https://learn.microsoft.com/en-us/dotnet/standard/native-interop/best-practices
- https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/interop
- https://learn.microsoft.com/en-us/aspnet/core/fundamentals/use-http-context?view=aspnetcore-10.0
- https://learn.microsoft.com/en-us/aspnet/core/fundamentals/middleware/request-response?view=aspnetcore-10.0
- https://devblogs.microsoft.com/dotnet/re-reading-asp-net-core-request-bodies-with-enablebuffering/
- https://builds.dotnet.microsoft.com/dotnet/release-metadata/10.0/releases.json
- https://learn.microsoft.com/en-us/nuget/consume-packages/package-source-mapping
- https://learn.microsoft.com/en-us/nuget/reference/nuget-config-file

SDK pin researched2026-11-07: official channel metadata reports SDK10.0.403 and
runtime10.0.14. global.json disables roll-forward/prerelease. This is version
provenance, not a vulnerability-clean or executed-artifact statement.
