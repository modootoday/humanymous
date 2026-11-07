# ASP.NET Core adapter candidate

The Rust request core lives in crates/dotnet-adapter. It owns bounded binary body
copies, EOF sealing, cancellation and one authenticated authority handoff.

ASP.NET Core middleware registration, PInvoke exports, NativeAOT linkage, managed
GC stress and NuGet packaging are not implemented or qualified. No managed host
tuple is advertised. Do not install this as a Kestrel protection package.

The shared Rust Gate can cover HTTP origins only under its separately qualified
deployment topology. It does not establish this adapter's host compatibility.
