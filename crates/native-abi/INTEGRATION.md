# Native ABI integration frontier

Owner: native-abi / rust-native-abi-goal. Canon: sot:root/native-abi.

Root registered crates/native-abi and reported14pure units passing. The core
reuses existing zeroize and inherits Rust1.99.1/edition2024/workspace unsafe forbid.
No shared Cargo/lock edits here. Root source-bound log/evidence reference awaited.
After revoke cleanup was added, root reported latest15pure units passing with
zero failures/ignored/filtered. The cache observation JSON records only five
top-level source hashes and is not a complete validation receipt. A subsequent
typed host seam adds one core case; current core16 is pending rerun.

Root Cargo slot requested: cargo test --locked -p humanymous-native-abi --lib;
cargo clippy --locked -p humanymous-native-abi --all-targets -- -D warnings;
cargo fmt --manifest-path crates/native-abi/Cargo.toml -- --check.

Implemented source only until tested: safe context/never-reused handle registry,
bounded zero-initialized owned buffers, zeroizing free/close, exact version/size,
fixed-width status/reply and permanently poisoned unwind boundary. No security
engine duplicated. No authority verdict, producer profile or scoring API invented.
Safe pointer-free Command/Response dispatch adds bounded32byte inline transfers;
its candidate layout is88/48bytes, without Debug on payload-bearing structures.
No foreign addresses, unmangled symbols or native library exports are added.

Root subsequently accepted no_mangle attribute only on a pointer-free by-value
Command->Response export. Separate minimal export/src/lib.rs consumes the same
Boundary/Registry, with one LazyLock<Mutex<Boundary>>, AtomicBool revocation,
outer unwind containment including initialization and lock poison denial.
Core forbid remains. Root opened its next integration window, authorized the
minimal wrapper manifest, registered member36 and resolved the lock. Root reports
six wrapper pure units passing with zero failures/ignored/filtered. The broader
all-target Clippy command failed in unrelated capture code; lint closure and exact
root source-bound evidence are pending. No actual dynamic loading occurred.

Root wrapper integration contract: member crates/native-abi/export; package
humanymous-native-abi-export with inherited package metadata and publish=false;
crate-type cdylib+rlib; dependency humanymous-native-abi path=..; local Rust lint
unsafe_code=deny,missing_docs=warn and Clippy all=warn. Do not inherit workspace
forbid in wrapper, because only its export attribute needs expect(unsafe_code).

Raw pointers, unsafe blocks/functions, foreign authored glue and dynamic-loader
unsafe remain unapproved. No actual dynamic ABI, generated header, .dll/.so,
C++ RAII/exception/compiler/allocator or hosted compatibility result exists.

Hosted header compiler gate is now ready as Rust source only:
cargo run --locked -p humanymous-native-abi-export --example header_conformance
-- <generated-header> <fresh-evidence-directory>. Standard free GitHub-hosted
Linux x86_64 only; never run the compiler/process example locally. Pure example
parser tests may run locally in the assigned Cargo slot. Add --all-targets to
tests/Clippy so its three cases are included. The runner compiles the generated
header directly as C11/C++11 and compares actual Clang record layouts with Rust,
without creating authored foreign glue. Root must bind exact source/header/tool
hashes, preserve raw output and enforce a job timeout. This gate does not load
the dynamic library or exercise C++ RAII/exceptions/allocator interoperability.

.NET facade seam, coordinated with its owner and root: validate_context and
buffer_bytes are now authored in Registry; immutable borrowed bytes remain tied
to its lifetime and ownership. No take_buffer/detached allocation was added.
Facades validate before every custom effect, drop mapped host request state on
generic CLOSE, and drop all mapped state on panic/poison. Do not copy the handle
registry or pass handles across independently loaded libraries. Latest authored
pure scope is core16/export6/header-example3; prior root passes are not a new
combined25case validation receipt.

Latest direct receipt now supersedes that pending note: evidence:root/20261107081545
records core16/export6/example3 passing and strict all-target Clippy passing, with
zero failed/ignored/measured/filtered. Root assigned B, target/validation-agents,
jobs1/incremental0; all18 captured files and HEAD were unchanged. B returned.
Raw private bundle20261107081250-native-abi-local-pure-verification retains the
original metadata empty-array encoding issue and independent verification sidecar.
The example's compiler main program did not run. Header generation, hosted Clang
layout execution and actual native loading remain pending. The .NET owner agreed
to a separate optional generic_probe profile within its approved pilot file;
supply the generic native library separately and do not mix its case denominator
with the .NET middleware/facade profile or C++ qualification.

Public docs delta for docs owner: safe ownership foundation is being implemented;
C/C++ integration remains unqualified/not_run. Do not advertise native SDK,
human detection, callback/worker cleanup, allocator compatibility or ready bindings.

## Accepted-family migration handoff, 2026-11-08

The accepted eight-public-package baseline assigns safe/export native ABI roles
to humanymous-adapters. Root alone freezes the concrete slice and changes paths,
package graph, role features and refs. No relocation or speculative shim is made
here. The owning plan records the exact current-path handoff and hazards.

Later actual pure validation is29 passed, zero failed/ignored/filtered, with
locked offline all-target check and strict Clippy passing. Evidence:root/20261107183956
retains the source/config/proxy snapshot and explicit unresolved registry/resolved
compiler prehash limits. This supersedes the earlier pending29 note, without
rewriting the historical25-case result. No local compiler main ran in that slot.

Hosted source9586f7e4/job128509489237 subsequently reports header generation and
owning C11/C++11 conformance exit0, execution_passed=true and source_changed=false.
The actual step failed at compiler identity admission; original16-file proof was
not exported/reconstructed. Evidence:root/20261107214000 retains captured API body
and separately ANSI-stripped derivatives. This is source-reported hosted semantic
execution, not a complete independently reconstructed header proof. The general
native job versus standalone header-profile route mismatch also remains pending.

No compiler, native process, Docker container or target lease is held by this
owner. Native ABI current implementation paths were clean before this handoff
metadata update. Current-path safety and historical source-specific evidence
must survive the Root-controlled integration.
