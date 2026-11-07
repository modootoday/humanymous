# ASP.NET Core adapter candidate

The Rust request core lives in crates/dotnet-adapter. It owns bounded binary body
copies, EOF sealing, cancellation and one authenticated authority handoff.

An explicitly approved three-source C# candidate now supplies LibraryImport/
opaque SafeHandle binding, ASP.NET middleware and the actual Kestrel pilot.
The separate Rust export facade is pointer-free and consumes the shared native
registry and authority. Candidate NuGet project metadata is present. These new
managed sources are not yet compiled or executed; no managed host tuple is advertised.
Actual GC stress, NativeAOT linkage, clean package install and product gates are
NOT_RUN. Do not install this as a qualified Kestrel protection package.
The current Rust core/export component has 13 pure test passes plus all-target
check and Clippy; that result does not qualify the managed host.

The middleware must run first, before body readers, response writers and effects.
Complete bounded bytes are copied before one Rust authority decision. Replay is
copied before the final expiry and unchanged-route check, then next runs once.
No reverse managed callback or foreign pointer is retained. The candidate uses
the pinned .NET SDK in global.json and sends private configuration through stdin.
Run dotnet commands with adapters/dotnet as the working directory so that the
SDK resolver finds this exact global.json. Actual host execution requires an
explicit root allocation. The user authorized local Windows/WSL validation on
2026-11-07; the active benchmark quiet window still forbids new runtime commands.
The facade's Rust runtime is process-lived. Collectible assembly loading and
explicit native unload/reload are unsupported; the binding rejects collectible
contexts and does not manually load/free libraries. Zero request diagnostics do
not establish native worker termination or unload safety.

The shared Rust Gate can cover HTTP origins only under its separately qualified
deployment topology. It does not establish this adapter's host compatibility.
