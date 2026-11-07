# Apache adapter integration frontier

Safe std-only lifecycle crate authored; workspace registration belongs to root.
No native ABI/module, authority ALLOW handoff or httpd support is implemented yet.
Pure unit corpus is authored NOT_RUN pending root's single Cargo slot.

Required shared seam (owners only): AdapterProfile::Apache with trusted config
name apache; humanymous.apache/0.1.0-experimental.1; exact apache-normalized;
peer/connectionId/capture/stream null, rawTls/originalH2/rawHeaderOrder false.
framework-adapter::encode match needs Apache => apache-normalized, and current
Pending/Configuration/Admission plus authority::BoundResponse remain sole
decision/IPC owners. Apache must never use Node identity as fallback.

Root registered safe crate in workspace and reports the initial six pure units
passed. Exact command/hash receipt is pending. Replay adds three unrun tests.
Root accepted isolated native ABI; see adapters/apache/ABI-SAFETY.md. Safe crate
still forbids unsafe. No handwritten C/C++ or executable foreign templates.

Next graph window request: separate crates/apache-adapter/native package
humanymous-apache-native, cdylib/rlib, opt-in httpd feature, default empty so
non-Linux workspace checks remain safe-only. Dependencies: Apache safe crate,
framework-adapter, authority, integrity, tokio(workspace), zeroize(workspace),
rustix1.1(fs,process). Build dependencies bindgen=0.73.2, sha2(workspace),
serde_json(workspace). Build script runs generation only for Linux httpd feature.
Manifest edits are not made until root authorizes that graph window.

Hosted pilot interface: adapters/apache/pilot-contract.json. Product-lab owns
Compose orchestration; delivery owns workflow dispatch. Both currently not_run.
Public docs delta for docs owner: Apache is planned native support with safe
lifecycle source only, not a qualified proxy/module/MPM tuple.
