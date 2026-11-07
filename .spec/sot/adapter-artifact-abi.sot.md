---
id: sot:root/adapter-artifact-abi
title: Unified humanymous public identity and host ABI contracts
status: active
created: 2026-11-07T16:33:34Z
updated: 2026-11-07T16:33:34Z
verify: manual
rel:
  depends_on: [sot:root/structural-development, sot:root/adapter-ecosystem, sot:root/native-abi, sot:root/adapter-authority]
---

# Unified public identity and host ABI canon

Document version: 0.1. Root records the user's explicit decision: consumers see
one product, humanymous. Internal decomposition preserves single responsibility;
it must not become a collection of differently named public products. These are
target contracts, not claims that current exports or packages already conform.
No publication, namespace ownership, new unsafe boundary or foreign glue is
authorized by this document. Independent critique precedes implementation.

## Identity and facade rules

| Surface | Target consumer identity | Implementation consequence |
| --- | --- | --- |
| Rust | package/crate humanymous; humanymous::adapters::<context> | One public library facade; internal humanymous-adapter-* packages remain distinct |
| Product CLI | humanymous | Exactly one product binary; internal CLI implementation becomes a delegated Rust library |
| Node | npm package humanymous; require/import humanymous | Native module selected by package metadata; no public language-suffixed product |
| Python | distribution humanymous; import humanymous | Native extension/module initialization must match; tags identify interpreter/ABI/platform |
| Ruby | gem humanymous; require humanymous; module Humanymous | Native initialization Init_humanymous matches the installed extension |
| .NET | package humanymous; namespace Humanymous; managed humanymous.dll | Explicit lowercase assembly filename; native resolver identity humanymous-native |
| C/C++ | include humanymous.h; dynamic link/load humanymous-native | The native suffix identifies the common FFI binary, not a separate consumer product |
| Apache/nginx | humanymous product/module installation | Required module structures, directives and host export symbols remain explicit host contracts |
| Go origins | humanymous HTTP integration | Current Rust-owned HTTP Gate is not a Go module, cgo library or native Go SDK |

Registry ownership is unverified until the actual registry confirms it. An
unavailable name is a release blocker to resolve, not permission to silently
publish humanymous-python, humanymous-node or another public alias. Exact
language-required domains/casing/tags are recorded without inventing support.

The public Rust facade is library-first with explicit optional adapter features;
default use must not link all host SDKs or start workers/listeners. A public
product binary, if shipped by this Cargo package, is explicitly feature-gated.
Do not introduce two workspace binaries that both write the same humanymous
output. Implementation/CLI test rewiring belongs to the coordinated graph change.

## Artifact layout and collision rules

The user explicitly chooses distinct lowercase managed/native names:
managed humanymous.dll, common native FFI humanymous-native.dll. The canonical
product spelling is humanymous; the user's shorthand humanyous does not rename
the product. NuGet package ID and AssemblyName are humanymous; RootNamespace
remains idiomatic Humanymous. This supersedes the earlier case-only filename plan.

Use target/host directories and manifests for variants. For example:
native/<target>/<host-role>/abi-v1/humanymous-native.dll,
libhumanymous-native.so or libhumanymous-native.dylib. Node uses humanymous.node;
Python uses its actual tagged
humanymous extension; Ruby uses the extension expected by its loader. Archive
basenames carry release/target metadata without changing the import identity.
Exact install layouts must be generated from one versioned artifact manifest.

Internal Cargo library target names remain unique in a shared target directory.
Do not set every leaf crate's lib.name to humanymous: that can clobber rlib,
metadata and dynamic outputs before packaging. The facade/staging pipeline maps
unique internal outputs to the public host-specific destination. Any SONAME,
Mach-O install-name, relocation or signature transformation is explicit and
verified against the FINAL artifact bytes, not inferred from a filename rename.

Do not flatten two same-basename variants into one install directory. Distinct
humanymous.dll and humanymous-native.dll resolve the managed/native case collision,
but role/target/ABI variants still retain distinct directories and the package-owned
exact native resolver. A bare-name search through current directory or arbitrary PATH
is forbidden. Test final build AND publish/install layouts, including the .NET
managed/native collision and actual native symbol identity. No tested path may
silently load a different already-present library.

The initial native v1 strategy is SINGLE ROLE per supported process/profile.
The generic C role is core-v1 and the managed host role is dotnet-v1; they are NOT
composed and co-loading their different role/major binaries is Unsupported in the
initial profile. The supported loader must refuse that configuration before the
second load; separate paths alone do not establish loader namespace isolation.
Arbitrary manual loading by host code is outside this supported-loader guarantee.
No two leaf export crates may accidentally overwrite a common artifact.
Host-specific entry symbols
such as humanymous_native_abi_v1 and humanymous_dotnet_v1 are meaningful product
API functions, not competing product/package identities.

| Initial role | Unique internal target | Installed native path below trusted application root | Required export |
| --- | --- | --- | --- |
| core-v1 | humanymous_native_abi_export | humanymous/native/<rid>/core/abi-v1/<platform humanymous-native library> | humanymous_native_abi_v1 |
| dotnet-v1 | humanymous_adapter_dotnet_export | humanymous/native/<rid>/dotnet/abi-v1/<platform humanymous-native library> | humanymous_dotnet_v1 |

The .NET managed asset is lib/<tfm>/humanymous.dll in the package and
<app-base>/humanymous.dll after deployment. Its native asset remains in the exact
dotnet path above for BOTH build and publish outputs; never flatten it beside
the managed assembly. The resolver belongs to the existing approved
NativeBinding.cs and is installed once before any LibraryImport call. It accepts
the logical native library name humanymous-native only, maps the declared RID/role/ABI to the fixed
manifest-relative path, verifies the exact expected artifact/exports and retains
the resulting library-instance lease. No PATH/current-directory/legacy-name or
other-role fallback. Failed/missing manifest, tuple, digest or load refuses.
Manifest trust must come from a verified package or explicitly pinned component
input; an attacker-replaceable neighboring JSON file does not authenticate a DLL.
Actual JIT/build/publish/AOT and second-role rejection controls are required for
every claimed mode. A role-composed facade needs a later explicit domain plan.

Private dependencies may retain build identities that consumers never import.
Every packaged binary/header/metadata entry binds source, role, target, ABI,
runtime constraints, byte size and cryptographic digest. Verify actual restored
bytes and reject missing, duplicate, traversal, symlink and wrong-target members.
Only declared packaging transformations may change bytes or installed names.

## C ABI, version and wire separation

Rust's native ABI and default struct/enum layout are not a stable foreign contract.
Use the declared target C calling convention with repr(C) fixed-width by-value
records; never pass Rust String, Vec, references, trait objects, native enums,
usize-dependent layouts or Rust-owned pointers across this common public ABI.
The current pointer-free ABI remains the baseline; a pointer/callback interface
requires a separate accepted safety contract, not a convenience exception.

Current native source defines ABI major1/minor0 with exact version checks:
Command88 bytes, Response48, Version/Reply16, fixed32-byte initialized data.
Version.struct_size=16 describes the Version header, NOT the88-byte Command.
Reserved/unused fields remain zero. Actual size/alignment/offset/endian/calling
convention is qualified per compiler/architecture tuple, not inferred globally
from repr(C) or a successful Rust build. Generate the header from canonical Rust
types with pinned cbindgen; C/C++ host compilation and actual calls are separate
gates. Do not serialize/hash raw padded native structs as a wire format.

Current common C ABI status values are fixed:0 OK,1 VERSION,2 ARGUMENT,3 HANDLE,
4 CAPACITY,5 ALLOCATION,6 PANIC,7 POISONED. Error scalar/data/reserved outputs
remain initialized and zeroed according to the current schema. Unknown status
must never become success; host exception/Result mappings preserve refusal and
redaction without pretending every host error uses this C status namespace.

Cargo/package SemVer, C ABI version, Node-API version, interpreter ABI and the
authenticated adapter wire protocol are distinct identities. Rename alone
changes none of the wire fields, provenance IDs, security rules or evidence
schemas. The current C ABI accepts exact minor0; same-major is not implicit
minor negotiation. Layout, symbol, ownership, status or calling-convention
changes require an explicit compatibility/version migration before release.
Unknown version, operation, field, capability or unsupported tuple refuses before
effect; never reinterpret reserved bytes as new meaning without a versioned plan.

The current native product is a dynamically loaded cdylib C ABI. A target import
library supports that dynamic product; a Rust rlib is not a C static library.
Static linkage requires a separately accepted symbol/owner/allocator profile.

## Ownership, lifetime, threads and failure

- Allocate and release through the same Rust owner/API domain. Current scalar
  u64 IDs are unique only within one Registry/library lifetime; they DO NOT
  encode library-instance or exported-role identity. A private host lease binds
  the retained actual loaded library, selected entrypoint/domain and context,
  rejecting cross-domain use BEFORE dispatch. No implicit generic/.NET handle
  sharing or same-scalar provenance guarantee is permitted. Stronger wire-side
  domain rejection needs separately versioned accepted encoding. Retain current
  context/kind/liveness validation. A handle is not authentication against
  malicious code already executing in the process.
  Supported SDK objects retain that private owning lease and never reconstruct
  it from a serialized/scalar ID. Raw C ABI v1 callers must retain the same
  library/entrypoint owner themselves: arbitrary scalar exchange is Unsupported,
  and callee-side cross-library rejection is NOT provided by this baseline.
  Wrapper collision/reload negatives must prove the chosen wrapper guarantee,
  not falsely claim the existing wire token encodes instance provenance.
- Copy bounded initialized bytes or retain a precisely declared borrow only for
  its accepted lifetime. Never retain caller stack, Python/Ruby/Node object or
  host request/pool pointer across an asynchronous boundary without its actual
  owning lease. No foreign free of Rust allocations or allocator assumptions.
- Bound contexts, buffers, bytes, pending requests, workers and cancellation
  state. Numeric logical limits are not RSS/FD/kernel cleanup guarantees.
  Poison/uncertain failures remain fail-closed; no silent eviction or reset.
- Node values stay in their owning environment and permitted thread; Python
  interpreter/GIL or supported free-thread profile and Ruby VM/GVL requirements
  are explicit. .NET queues retain SafeHandle/permit ownership before fallible
  task setup; Apache/nginx worker/request/pool and fork generations remain bound.
- Validate original deadline/cancellation and consume the actual opaque decision
  immediately before the host effect after all fallible preparation/liveness
  callbacks. Queued execution rechecks on the actual owning loop. A status peek,
  label, configured timeout or earlier callback does not authorize the effect.
  For asynchronous hosts this boundary is protected application INVOCATION on
  the actual owning loop, not every later side effect of an awaited application.
  Effects requiring atomic epoch/lease admission need the separate native
  provider; invocation success does not prove task settlement or effect lifetime.
- Rust panics must not unwind through the C boundary. Existing bounded panic
  translation revokes/poisons its owner; catch_unwind does not recover abort,
  OOM, segmentation faults, arbitrary host exceptions or OS termination.
  C++ exceptions must not enter Rust. Error shapes are stable/redacted, never
  credentials, raw request bodies or sensitive native paths.
- Close/revoke/drain owned work before unloading or VM teardown. No outstanding
  callbacks, handles, borrowed host values or pending native tasks may survive
  DLL reload. Failed cancellation does not prove settlement. Actual exit,
  joined workers, Job/cgroup/daemon state and unknown cleanup remain distinct.

Every profile declares process-lived/non-unloadable or separately qualified
unload support. Initial .NET is process-lived: reject collectible ALC and reload
before context creation, and retain the library/runtime through process exit.
Context/resource counters do not prove the process-lived Tokio worker stopped.
For this profile test refusal plus actual process-exit cleanup, not a fictitious
successful-unload path. Any later unload profile needs actual pending-task,
callback, GC, worker and library-instance settlement evidence.

Exactly the existing approved three C# glue files and reviewed native exceptions
remain allowed. This canon does not authorize authored C/C++/Go/Python/Ruby/JS
glue or new unsafe calls. Needed exceptions require a concrete scoped decision;
generated upstream binding output is not an independently verified ABI.

## Supported tuples and discovery

### Build-product and loader profiles

| Profile | Allowed product | Required separation |
| --- | --- | --- |
| rust-library | public humanymous Rust facade/configuration APIs | No automatic host SDK initialization; feature exposure must have an explicit SDK-free or host-qualified closure |
| cli | humanymous executable | One entrypoint; no implicit host extension profile |
| core-v1 | dynamic pointer-free C facade | Single process role; not a C static archive |
| dotnet-v1 | process-lived .NET native bridge | Default/non-collectible context; fixed resolver; no generic role co-load |
| node-extension | humanymous Node package | Pinned Node-API minimum and approved generated CJS/ESM loaders |
| python-extension | import humanymous | Selected extension-module/limited-API/GIL profile and actual wheel tags |
| python-embedded-pilot | explicit CPython embedding component | Not the wheel/extension product; never infer import from a linked test executable |
| ruby-extension | require humanymous | Selected Ruby VM/GVL/platform/compiler ABI and generated entrypoint |
| apache-module / nginx-module | actual host module | Exact host headers/module signatures/worker lifecycle; distinct role directories |

These products are separate builds/staging profiles with one public identity,
not an arbitrary all-features composition. Before facade wiring, freeze exact
feature sets and incompatible pairs in a machine-checked build-product manifest.
Reject incompatible SDK/extension/embedding combinations. Do not use a universal
all-features build as proof that every host product can coexist. Current optional
feature defaults do not prove the future facade feature graph already exists.

The Node public exports map require/import to the approved pinned upstream
generated native/index.cjs and native/index.mjs, with declarations and exact
native payloads. Current generator identity is @napi-rs/cli3.10.12; record its
lock/digest/source provenance and actual generated bytes. The public package
becomes humanymous; private native filenames may carry generator-required target
tags under the humanymous stem, never a separate language-branded public import.
Native .node files are not directly imported as ESM: the approved generated
loader performs supported native loading. No authored JS shim, script-string
substitute, install-time network fetch or unverified custom loader is granted.
Actual restored-package require AND ESM import tests, wrong/missing payload,
ABI/architecture and manifest/loader tamper negatives are mandatory.

### Release identity and API semantics

One release manifest binds canonical product release ID, source commit and each
ecosystem's legal version spelling. SemVer/PEP440/gem/NuGet spellings may differ;
no independent component version is silently presented as the same binary.
The exact version mapping is frozen before packaging/publication. A mapping
never changes the independent ABI/wire negotiation requirements above.

Common public lifecycle concepts keep consistent meaning across languages, while
idiomatic casing and return types remain host-specific. Declare UTF-8/binary
ownership, units, optional/absent values, error categories and async completion
explicitly; no implicit truthiness, string conversion or header normalization
may change signed/provenance semantics. Metadata/capabilities are observations,
not proof of human identity or a newly trusted native producer.

Record OS, architecture, endianness, libc/CRT/compiler, host runtime/SDK ABI,
binding features, minimum versions, dependency/native headers and exact source.
MSVC is not MinGW/UCRT; glibc is not musl. A .NET JIT caller is not AOT/Kestrel;
an embedded CPython link is not an extension import or WSGI/ASGI host proof.
Node-API compatibility is limited to the used stable API and declared minimum;
direct V8 APIs do not inherit it. Python abi3 requires the selected limited-API
and ABI features; free-threaded Python separately requires an explicit interpreter
and extension synchronization profile. Both require actual tuple tests.
Ruby/version/platform compatibility is
qualified, not assumed. Apache/nginx module signatures and actual host builds
remain specific tuples. macOS signing/loader/sandbox and ARM/32-bit are separate.

A compatibility descriptor/manifest reports available API, ABI and tuple facts
without claiming native admission, trusted producer, humanity or readiness.
Reject wrong architecture/runtime/version and dependency load failures. Dynamic
loading is restricted to the validated package-owned artifact path and approved
exports. No network fetch, installation or remote activation occurs at import.

## Migration and qualification gates

1. Freeze the old/new matrix: internal Cargo identities, public package/import,
   lib/init/export names, loader/install paths, manifests, features and every
   consumer. Existing humanymous_python, humanymous_ruby,
   humanymous_native_abi_export and humanymous_dotnet_adapter_export are current
   implementation names, not exemptions from the user's unified target.
2. Root coordinates Cargo/locks/imports and facade wiring; each existing owner
   edits only claimed sources. M11 projects conventions; CI updates commands,
   package/archive policy; Pages updates current public examples. Preserve
   original test receipts, commands, payloads, hashes and source IDs.
3. L1: unit/layout/error/ownership/old-new mapping and header controls, fmt,
   target compilation and strict Clippy. Positive and reason-specific negatives
   are mandatory; zero selected tests, ignored cases and build-only results are
   not host execution.
4. L2/L4: actual restored-artifact import/load/call in every claimed tuple,
   minimum/unsupported versions, Windows basename collision, wrong library and
   stale/cross-domain handles, allocator/GC/VM/task/cancel/unload failures and
   host body/effect controls. Use Rust-authored tests within approved glue.
5. L5: genuine Cross-OS/host matrix, product Docker topology and human/keyboard/AT
   controls, clean source/artifact provenance and current public docs. Native
   Windows supplementary tests and Docker Linux profiles retain venue limits.
   Detection freeze, fail-closed, dual-control, a11y and defense-only remain.

No renamed package inherits an old receipt as execution proof. A byte-preserving
mapping can explain history, but affected actual imports/loaders/artifacts need
fresh qualification. No release, tag, OIDC exchange, publisher trust or final main
integration is authorized by completing a name table.

## Review and remaining implementation

### First public Rust facade slice under review

Root owns the proposed `crates/humanymous` library package and its common graph.
Its default feature set is empty. The first optional `adapter-go` feature exposes
the existing leaf namespace as `humanymous::adapters::go`, preserving type identity
through a crate re-export rather than copied wrappers. Existing `scenario` and
`probe` module paths remain: `scenario::evaluate`, `scenario::CaseObservation`
and `scenario::EvaluationError` must not be silently moved to the namespace root.
Deployment, Host, Configuration and Rejection retain the actual shared runtime
types and constructor/configuration compatibility. Receipt wire literals and the
eight-case qualification profile remain unchanged.

A colocated closed build-product manifest and Rust validator must precede feature
wiring. Initially admit only the rust-library profile with no features or exactly
adapter-go; reject unknown profiles, features and combinations. This manifest is
checked against the actual Cargo-declared feature set. Define adapter-go using
`dep:humanymous-go-adapter` so an implicit alternate dependency feature cannot
bypass the closed feature vocabulary. Duplicate feature requests are invalid.
This manifest is an admission boundary to be wired into supported build entrypoints, not a claim
that Cargo cannot be invoked directly. The first slice includes no host extension
initialization, dynamic artifact, dependency upgrade, leaf package rename or
publication. Default-off and enabled public-consumer tests are separate gates.
Feature-gate Go consumer tests; do not add an unconditional Go dev dependency
that hides the default graph. The enabled closure includes the existing runtime,
authority, audit, capture, network, TLS and async dependencies; no small or wholly
native-free closure is promised. Actual build receipts must bind effective
resolved features, including Cargo feature unification, rather than only manifest
declarations.

Go, framework, .NET, Native and adversarial bounded reviews accepted the concrete
library-only design with the explicit feature/closure constraints above.
The first source slice now implements the facade, closed manifest and Rust
admission validator. Locked offline default-only library tests passed2 and its
feature-gated consumer ran0 tests; enabled adapter-go passed2 library and2 public
consumer tests. These are4 distinct tests across6 executions, not6 distinct
cases. Enabled all-target strict Clippy and scoped format checks passed. The
default normal dependency tree contained only humanymous; no unconditional Go
dev dependency was added. Only the new internal package lock entry was added,
without external version changes. No full transitive source/tool attestation is
claimed by this local validation.

The admission API is not yet connected to Delivery build entrypoints. Actual
resolved-profile artifact receipts, Lab consumer migration and fresh hosted
product qualification remain required. Later adapter package renames require a separate
atomic owner-coordinated graph batch and fresh product evidence. The immutable
pre-fix e70 browser diagnostic must not be changed by this facade work.

Root synthesized architecture, abuse, operations, data handling and evaluator
constraints: prevent loader substitution/domain confusion, retained work after
failure, privacy leaks, falsely broad ABI promises and cosmetic-only rename.
Independent X/native/framework/.NET review must confirm the concrete facade,
collision handling and migration matrix before implementation. Any P0/P1 is
reflected here rather than waived by successful unit tests.

Four bounded reviews accepted the refined target contract after resolving the
registry-local handle/domain and process-lived unload P1 findings; Node generated
loaders, exact async invocation and feature profiles were also reflected. The
user then explicitly selected lowercase humanymous.dll versus humanymous-native.dll;
that filename refinement removes case-only collision without relaxing role,
instance, trusted-path or actual import gates. Reviews are design evidence, not
renamed implementation or runtime qualification.

The initial Go Rust facade is implemented and locally checked as described above;
other public imports, restored artifacts and changed host init symbols remain
unimplemented or unqualified. Current support remains
exact-source evidence in the owning SoTs. Public namespace ownership, unresolved
host tuples, additional glue and actual GUI/AT remain explicit release gates.

Primary references: [Cargo library targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#library),
[Rust C layout](https://doc.rust-lang.org/reference/type-layout.html#the-c-representation),
[Rust FFI unwinding](https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-unwinding),
[Node-API](https://nodejs.org/api/n-api.html#implications-of-abi-stability),
[PyO3 0.30.1](https://pyo3.rs/v0.30.1/building-and-distribution.html),
[CPython ABI](https://docs.python.org/3/c-api/stable.html),
[Ruby extensions](https://docs.ruby-lang.org/en/master/extension_rdoc.html),
[.NET interop](https://learn.microsoft.com/en-us/dotnet/standard/native-interop/best-practices).

