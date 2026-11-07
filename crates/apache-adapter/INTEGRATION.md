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

Root must add crates/apache-adapter to workspace (no new external dependencies
in current safe-only slice). Native linking needs separately approved local
unsafe boundary and pinned generated httpd/APR bindings. No handwritten C/C++.
Current Rust forbids unsafe; no hidden feature bypass is provided.

Hosted pilot interface: adapters/apache/pilot-contract.json. Product-lab owns
Compose orchestration; delivery owns workflow dispatch. Both currently not_run.
Public docs delta for docs owner: Apache is planned native support with safe
lifecycle source only, not a qualified proxy/module/MPM tuple.
