# ASP.NET Core adapter candidate

**Diataxis quadrant:** Reference. **Audience:** adapter developers and reviewers.

This candidate has no qualified Kestrel/JIT or NativeAOT host tuple. Actual GC
stress, clean package installation and product gates remain NOT_RUN. Do not
install it as a qualified Kestrel protection package.

The Rust request core lives in crates/dotnet-adapter. It owns bounded binary body
copies, EOF sealing, cancellation and one authenticated authority handoff.

An explicitly approved three-source C# candidate now supplies LibraryImport/
opaque SafeHandle binding, ASP.NET middleware and the actual Kestrel pilot.
The separate Rust export facade is pointer-free and consumes the shared native
registry and authority. Candidate NuGet project metadata is present. These new
managed sources compiled in the ABI owner's Windows FDD/JIT build after an
actual marshalling error was corrected. A separately supervised generic ABI-only
managed process passed; Kestrel and middleware have not executed and no managed
host tuple is advertised. That build disabled AOT analysis/publishing
and does not establish NativeAOT compatibility.
The earlier Rust core/export snapshot has 13 pure test passes plus all-target
check and Clippy; that result does not qualify the managed host.

The middleware must run first, before body readers, response writers and effects.
Complete bounded bytes are copied before one Rust authority decision. Replay is
copied before the final expiry and unchanged-route check, then next runs once.
No reverse managed callback or foreign pointer is retained. The candidate uses
the pinned .NET SDK in global.json and sends private configuration through stdin.
Run dotnet commands with adapters/dotnet as the working directory so that the
SDK resolver finds this exact global.json. Actual host execution requires an
explicit root allocation. Every actual SDK/host stage needs a separate allocation;
the ended benchmark quiet window does not allocate one. Local container work
uses the explicitly selected Windows Docker Desktop Linux engine and has no
automatic WSL fallback. The projection/generic caller helpers passed eight
pure tests and release all-target Clippy at source07967eae. One non-build context
document changed; the other267captured inputs were unchanged. Later metadata
edits are separate from that source. Generic caller success does not qualify the adapter.

The later bounded host/control protocol passed eight pure tests, and the exact
three-source C# inventory passed one test. All-target check and strict Clippy
passed on immutable base0a1bb36e with the exact owned test overlays recorded in
314a9dac. The new authority-abort/down fixtures compiled but did not execute.
These results do not establish joined IO workers, ordinary-actor cleanup or any
managed host tuple. The shared parent context is unverified syntax; actual file,
token, Job and cleanup proof belongs to the native runner.
The facade's Rust runtime is process-lived. Collectible assembly loading and
explicit native unload/reload are unsupported; the binding rejects collectible
contexts and does not manually load/free libraries. Zero request diagnostics do
not establish native worker termination or unload safety.

The shared Rust Gate can cover HTTP origins only under its separately qualified
deployment topology. It does not establish this adapter's host compatibility.
