---
id: sot:root/native-abi
title: Shared native ABI ownership and buffer contract
status: active
created: 2026-11-07T07:25:00Z
updated: 2026-11-07T07:43:08Z
verify: manual
rel:
  governs: [crates/native-abi/**, adapters/native/**]
  depends_on: [sot:root/rust-migration, sot:root/native-embedding, sot:root/python-ruby-adapters]
---

# Shared native ABI ownership and buffer contract

Version0.2. Root authorized the safe Rust ownership foundation and subsequently
accepted one narrow export exception on2026-11-07: #[unsafe(no_mangle)] only on
the pointer-free by-value Command->Response entrypoint in a separate tiny export
crate. Raw-pointer shim, unsafe blocks/functions, dynamic-loader unsafe and
authored foreign host glue are not authorized within this owned slice; separately
approved managed host glue belongs to the .NET owner. Shared scoring, authority,
cryptography and policy stay owned
by their existing crates; this crate must not duplicate their implementations.

## Decision rules

ABI-01: Negotiate exact major1/minor0 and fixed-width repr(C) request size and
reserved fields before creating a context. Unknown versions, size or flags reject.
No Rust enum/String/Vec/usize layout belongs to the wire. Status codes are stable
u32 constants; failure returns zero value, never an uninitialized output.

ABI-02: A Registry owns contexts and buffers. Nonzero monotonically increasing
u64 handles are never reused during that Registry's lifetime, even after close.
Different resource types share one ID sequence. Wrong-kind, zero, stale and
already-freed handles reject. Handles are process-local lifetime identifiers,
not authenticated capabilities against a compromised in-process host. A future
export facade must use one process-lifetime Registry, never recreate it on init.

ABI-03: At most16contexts,64buffers/context,1MiB/buffer and8MiB total live buffer
bytes. Validate before allocation. Buffers are initialized zeroed and bound to
one live context. Bounds checks use checked arithmetic. Read/write borrow host
Rust slices only for the call; no pointer retention or callbacks. Free and close
zeroize owned bytes before releasing allocation. A free requires the exact owning
context and buffer; foreign allocators never free Rust memory. Close revokes every
owned buffer before returning. Repeated close/free is an explicit error.

ABI-04: Context close is synchronous for this in-memory foundation: no workers,
IO, callbacks, OS handles or origin effects are created. Future deadline/cancel/
worker state cannot claim cleanup complete merely because authority was revoked.
Calls require exclusive mutable Registry access; a future global facade must
serialize calls and fail closed after lock poisoning. No C++ exception crosses
the boundary. No DLL unload while calls or future workers remain live.

ABI-05: A panic boundary maps unwind panics to PANIC with zero value and revokes
all resources, dropping owned zeroizing buffers. If mutation
panics while owning a future Registry lock, poison must make all future protected
operations unavailable. Panic-abort/OOM/native memory faults are not contained by
catch_unwind. Do not install/replace the host's panic hook or log input data.
Static error descriptions contain no caller buffers, credentials or body content.

ABI-06: This foundation grants no security verdict, native observation, collector
qualification, origin authorization, human identity or new producer profile.
Required signal absence remains fail-closed in the owning authority. Detection
weights/predicates/bands, a11y, dual control and T4 remain unchanged.

ABI-07: Safe Rust dispatch takes one88byte repr(C) Command and returns one48byte
Response, containing only u32/u64 and fixed byte arrays. Operations1..7 are open,
close, allocate, free, length, write, read. Every command repeats negotiation;
reserved flags, unused fields and WRITE tail padding must be zero. Read/write
chunks are at most32bytes; no foreign addresses exist. Every error has zero data
and scalar; successful READ has zero tail. Command/Response have no Debug to avoid
accidental buffer logging. Host-owned inline command/response copies remain the
host's zeroization responsibility; freeing the registry cannot erase those copies.
Owned-allocation zeroization cannot prove erasure of compiler-generated temporary
stack/register copies, OS pages, process dumps or host backups.
The safe Rust dispatch is separate from the candidate exported symbol and does
not alone establish actual ABI compatibility evidence.

ABI-08: crates/native-abi retains workspace unsafe forbid. The separate minimal
crates/native-abi/export crate uses deny(unsafe_code); only the function carrying
#[unsafe(no_mangle)] has expect(unsafe_code). No unsafe block, unsafe function,
foreign-authored template, pointer input or dereference is permitted. Export
humanymous_native_abi_v1 uses extern C, Command and Response; every field is a
fully initialized fixed-width integer or byte array with all bit patterns valid.
Root alone registers its manifest and shared lock dependencies. Static inspection
must confirm that the export attribute is the sole unsafe lint exception, and
strict Clippy must reject unfulfilled expectations. Generated headers need exact
upstream cbindgen tool/source provenance, not handwritten foreign declarations.

ABI-09: One library-lifetime LazyLock<Mutex<Boundary>> serializes the exported
entrypoint. Atomic permanent revocation is checked before and after taking the
lock; poisoned lock, inner panic or outer unwind never resumes protected work.
On lock poison, take the poisoned guard only to revoke/drop zeroizing resources;
never clear the Mutex poison flag or reset the Boundary. Explicit Boundary revoke
is idempotent cleanup only, never a restart operation.
The outer catch includes lazy initialization and locking. Close linearizes under
that same lock: calls ordered before close finish, close revokes/frees, calls
ordered after close get HANDLE. Mutex fairness or invocation-order scheduling is
not promised.
No callback/worker exists; reentrant/signal-handler calls and post-fork use without
exec are unsupported. Hosts must finish calls, close all contexts and stop callers
before unload; retain no handles across unload/reload or module copies. Handles
are not unique across independent library instances.

ABI-10: Rust panic hook executes before catch and may log secrets/abort according
to host policy. This library does not replace the process-global hook, inspect or
format panic payloads. Abort/OOM/native fault remain outside containment. Each
outer boundary leaks at most one caught payload after permanent revocation to
avoid a destructor panic; in-flight threads may each already be unwinding. The
global symbol must have exactly one definition in a link image: versioned prefix
does not prevent a hostile/duplicate library from defining the same name. Host
loader must select the intended absolute artifact; symbol presence is not trust.
By-value source declarations do not remove the compiler's possible hidden return
address or the caller's duty to use the exact calling convention and initialized
arguments. Corrupted foreign calling frames are outside safe Rust containment.

ABI-11: The Rust-authored header_conformance example invokes Clang only on an
upstream-generated C header, as C11 and C++11. It compares compiler-reported record
size/alignment/all named field offsets against Rust size_of/align_of/offset_of
for Version, Reply, Command and Response. Missing, duplicate, malformed or changed
layouts reject. No authored foreign source, script or executable template is
generated. It preserves actual compiler version/target/stdout/stderr and refuses
an existing evidence directory. The current execution profile is standard free
GitHub-hosted Linux x86_64 only; environment guards prevent accidental local
execution, not a hostile host forging environment variables. Root CI must bind
source/header/tool hashes and enforce job timeout. This is header syntax/layout
conformance only, not actual dynamic loading, calling-convention execution,
C++ RAII/exception behavior, allocator interoperability or full product success.

ABI-12: Typed host facades reuse Registry::validate_context and buffer_bytes.
They reuse the public Version::validate for exact common negotiation instead of
copying major/minor/size/reserved rules into each host namespace.
The latter returns an owner-checked immutable Rust borrow only, with no detached
allocation or new allocator domain. Rust lifetimes prevent free/mutation while
the view is live. A host-specific facade may compose its request state with the
same Registry through Boundary::call, but generic core must not depend on host
adapters. Validate context before every host-specific effect. A facade must
cancel/drop its mapped replay/request state when intercepting generic CLOSE,
then close Registry; a freed Registry alone cannot erase host-owned copies.
On PANIC/POISONED or facade lock poison it must cancel/drop all mapped host state;
generic Boundary has no access to host-specific resources or destructors.
Each separately loaded library has its own namespace; handles cannot cross
libraries/reload. Host extensions use their own versioned symbol/operation
namespace and must not redefine generic1..7 or invent shared scoring policy.

ABI-13: The hosted delivery coordinator consumes the owning header example's
exit status and fresh closed evidence directory, without a second layout parser.
Success requires exit0, the exact success stdout and scope.txt produced by the
owner, plus thirteen regular files: for each c/cpp prefix, version.txt,
version-stderr.txt, target.txt, target-stderr.txt, stdout.txt and stderr.txt;
scope.txt is the thirteenth. The generated header is a separate artifact.
The coordinator binds all file hashes, actual cbindgen0.29.4 binary/version,
compiler metadata, source/lock/toolchain/config/owner code and runner identity.
It may emit a machine-readable outer receipt only after those checks. No extra
owner JSON certificate is required: scope.txt is written only after both actual
compiler exits and both layout verifications succeed. Each language checks four
records, four sizes, four alignments and seventeen named field offsets. These
counts are separate from the25pure unit denominator. Partial files or a generated
header alone never authorize success. Actual dynamic loading and C++ runtime
behavior stay separate gates. Root alone pushes/dispatches the next candidate;
the currently implemented header profile is hosted and no local heavy tool
installation has run. Later user authorization permits actual local Windows/WSL
validation under root allocation, but the benchmark quiet window currently
forbids new execution. A future local header profile must identify the actual
local runner and cannot spoof GitHub environment variables to bypass the current
hosted guard. Root's measured execution-policy decision and explicit allocation
precede new local commands.

## Closed original proof return

ABI-15: adapters/native/header-proof-profile.json is the sole owning candidate
for the closed original header-proof payload inventory and observed R3 tuple.
It defines sixteen flat-path file IDs, exact-byte header/compiler metadata/AST
digests and finite bounds. Its current candidate status is not production
admission or dispatch authority. Delivery owns strict Rust validation, transport
and reconstruction; no duplicate profile constants or permissive unknown-output
fallback may replace this owning data. Full source-before/after originals need
the exact immutable candidate's canonical public path set and measured count/
size acceptance before dispatch. All sixteen payloads must validate before any
payload chunk is emitted; duplicate/unknown/reordered/truncated/mismatched data
cannot complete a proof. Exact byte privacy admission does not replace the
owning semantic layout checks or establish a hosted compiler tuple by itself.
Local R1/R2 failures, R3 Docker grammar observation and future hosted execution
remain separate source-bound records. No dynamic ABI scope is added here.

## C++ contract

ABI-14: The separately authorized local Windows x64 profile uses the explicit
`--local-msvc` example, absolute tool/input paths and a fresh output directory.
It compiles the actual upstream-generated header directly with MSVC `/TC
/std:c11` and `/TP /std:c++14`, requiring successful nonempty object outputs.
No handwritten C/C++ caller or loader is introduced. Passive dumpbin output is
artifact inspection, not a dynamic call or layout-equivalence assertion. This
profile never qualifies C++11, C/C++ callers, RAII or exception behavior, and
does not bypass the separate genuine hosted Clang C11/C++11 layout gate.
Actual generic DLL calls reuse the .NET owner's existing approved Program.cs
`generic_probe_only` path: a bounded owner launcher must verify actual child
exit0, its starting PID and exactly one generic ABI receipt. That is a C# caller
only. Header/tool/DLL/source hashes and actual local venue bind each receipt;
all compilation and managed publish require root's serialized allocation.

The generated header is declarations only, generated from Rust layouts
by upstream cbindgen with exact tool/artifact provenance. Never author C/C++
implementation or executable templates here. Host glue must provide move-only
RAII owners with one context close and buffer free, explicit nonthrowing destructors,
no destructor calls after library unload, and no shared allocator assumption.
Finalizers/destructors cannot silently retry an uncertain protected operation.
Host compiler/runtime/framework support requires actual hosted compiler and
dynamic ABI evidence. A Rust unit test cannot qualify C++ exception/RAII behavior.

## Critique reflected

P0 invalid foreign pointers cannot be validated -> no raw pointer shim in the
approved slice. P0 stale ID alias after close -> one never-reused shared sequence.
P1 wrong-owner free -> context association checked before any removal. P1 memory
exhaustion -> context/count/byte caps and fallible allocation. P1 close looks like
worker cleanup -> current zero-worker scope explicit. P1 panic payload destructor
repanics -> boundary forgets the caught payload at the poisoned boundary rather
than allowing a second unwind; this does not promise a global payload memory cap.
P1 overclaim -> ABI-06 and
separate unexecuted qualification denominator.

## Verification and residuals

The latest local pure receipt is evidence:root/20261107081545: 25 tests passed
(16 core, 6 export, 3 header diagnostics), with zero failed, ignored, measured or
filtered. Locked all-target tests and strict all-target Clippy ran on Windows
x86_64, Rust 1.99.1/edition 2024, in root-assigned slot B. Eighteen recursively
captured owned/shared files and HEAD stayed unchanged. Formatting checks passed.
Actual allocator failure injection remains not_run. These are component units;
the header example's main program did not execute. Header generation, hosted
C/C++ compiler/layout execution, actual dynamic ABI, C++ RAII/exception/allocator
host behavior, cross-OS runtime and full product Docker e2e are not_run.
Actual local validation was subsequently user-authorized, but this receipt ran
pure units only. New actual execution awaits the benchmark quiet-window release
and explicit root allocation; no earlier result is relabeled as a local pilot.

The later local A allocation produced an actual cbindgen0.29.4 header and built
the DLL. MSVC C11 and C++14 direct header compilation passed, alongside passive
x64/export inspection and example strict Clippy. This adds artifact evidence,
not dynamic-call or layout-equivalence proof. The first framework-dependent JIT
managed restore passed, but pilot build failed with two SYSLIB1051 aggregate
marshalling diagnostics. No managed probe ran; failed evidence remains separate.
After the .NET-owned same-file correction, the second JIT build passed. A later
single root-approved generic-only caller run passed with actual PID-bound frames,
process exit0, empty stderr and removed fixture. Seven artifact after-hashes and
the strict input hash matched. Evidence:root/20261107081545 keeps this actual
managed caller result separate from header syntax, historical failures and units.
It does not qualify full .NET, AOT, C/C++, hosted layout or general process trees.

Primary references checked2026-11-07: Rust FFI safety
https://doc.rust-lang.org/nomicon/ffi.html ; panic limits
https://doc.rust-lang.org/std/panic/fn.catch_unwind.html ; header generation
https://github.com/mozilla/cbindgen . References are contracts, not test evidence.
Upstream releases list cbindgen0.29.4 at https://github.com/mozilla/cbindgen/releases ;
that tool version is a verified candidate, not an installed or executed tool here.
