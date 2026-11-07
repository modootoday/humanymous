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
authored foreign host glue are not authorized. Shared scoring, authority,
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

## C++ contract

The eventual generated header is declarations only, generated from Rust layouts
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
repanics -> boundary forgets caught payload, retaining bounded-process residual
panic leakage per poisoned boundary rather than allowing a second unwind. P1 overclaim -> ABI-06 and
separate unexecuted qualification denominator.

## Verification and residuals

Pure Rust units cover negotiation, stale/wrong-kind/wrong-owner handles, caps,
overflow/bounds, all-or-nothing invalid writes, close, ID exhaustion and panic
status. Actual allocator failure injection is not_run. Scoped locked Cargo/fmt/Clippy await root's one-slot
schedule and workspace registration. Exports/header generation, dynamic ABI,
C/C++ compiler/RAII/exception/allocator tests, cross-OS and full product Docker
e2e are not_run. No local process/Docker/browser execution is authorized here.

Primary references checked2026-11-07: Rust FFI safety
https://doc.rust-lang.org/nomicon/ffi.html ; panic limits
https://doc.rust-lang.org/std/panic/fn.catch_unwind.html ; header generation
https://github.com/mozilla/cbindgen . References are contracts, not test evidence.
Upstream releases list cbindgen0.29.4 at https://github.com/mozilla/cbindgen/releases ;
that tool version is a verified candidate, not an installed or executed tool here.
