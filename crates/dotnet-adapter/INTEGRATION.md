# Integration requests

Root owns Cargo workspace and lock. Register humanymous-dotnet-adapter with
framework-adapter, authority and integrity path dependencies and serde_json dev
dependency. No new third-party dependency is requested.

Integrity owner: add DotnetAspNetCore provisioned profile, exact ID
humanymous.dotnet, version0.1.0-experimental.1, aspnetcore-normalized only,
rawHeaderOrder=false, peer/connectionId/capture/stream null. Framework owner:
encode selected profile using that mode; preserve existing parser validation.

Native ABI owner: request/payload handles and C exports belong to your crate.
This crate owns safe body/lifecycle behavior and accepts no pointers. No unsafe
or foreign host glue exception is approved. Actual Kestrel/NativeAOT package is
blocked on an authorized binding mechanism, not replaced by a mock.

CI owner: schedule scoped units/fmt/Clippy and future actual Windows/Linux host
pilot only on standard free GitHub runners. Root dispatches. Qualification must
bind source and artifact hashes, runtime/OS/arch/link mode and preserve ALLOW,
CHALLENGE,DENY,error, failed, ignored, unsupported and origin-effect counts.

Public docs delta for docs owner: .NET currently has safe Rust lifecycle source;
no installed middleware, managed GC, PInvoke/NativeAOT, NuGet or qualified tuple.
No full-product protection claim follows. Reference dotnet-adapter SoT.
