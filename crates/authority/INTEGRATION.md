# Local authority integration contract

Package/binary: `humanymous-authority` (publish=false). Build with
`cargo build --locked --release -p humanymous-authority`; executable at
`target/release/humanymous-authority[.exe]`. No TCP, admin, nginx pilot protocol,
collector or browser recovery endpoints. Node's exact
`humanymous.adapter/0.1` POST `/adapter/v0.1/decision` is the sole endpoint.

Startup reads one bounded JSON configuration from inherited stdin until EOF.
The supervisor must keep stdin/stdout private, never log readiness, and drain
stderr. stdout must be a pipe, not a terminal. After binding, one JSON readiness
line contains `protocol`, `endpoint`, `profile`, `credential` (fresh random bearer, scoped
to this process), and `qualification: experimental-local-authority`. The
supervisor delivers the credential to the configured adapter only. Restart
always rotates it; do not cache/reuse readiness across process lifetimes.
Secrets are not command-line arguments, environment variables or public files.
The profile reports each route's node-raw/fetch-normalized readiness, current
revision, absent capture/browser/admin capabilities and complete-body support.
An experimental listener's readiness does not qualify its blocked routes.

Configuration schema and executable/native process tests live in this crate.
One process serves one installation/tenant/origin/revision and actual IPC peer.
Use a private existing directory for the Unix socket and audit WAL. Unix peers
are authenticated by OS UID; Windows uses a generated protected DACL restricted
to the configured SID and checks actual connected client PID. Unix socket is
filesystem-only; Windows endpoint is exactly `\\.\pipe\NAME`, never UNC.
No listener starts before verified audit recovery succeeds.

Compose should supervise the process with private pipes and mount the socket
directory only to the selected adapter. Run as the same non-root UID with a 0700
socket parent; socket mode 0600. Audit state is separately writable. Image
packaging belongs to the Compose owner. Do not put bearer values in Compose
logs/config or publish any port. `cargo test -p humanymous-authority --locked`
includes native process protocol/security pilots; those are not Docker product
qualification. Explicitly missing raw TLS/H2, collector and RIT readiness causes
422 on routes requiring those capabilities. The observable application HTTP
profile uses the shared network report and frozen scoring engine; it never
turns application User-Agent headers into collector evidence.

ALLOW requires the existing durable audit Log append/checkpoint before the
seven-field response. A test-only injected evidence provider can exercise that
ordering, but cannot qualify a browser/capture profile. nginx's current mock
protocol is incompatible; a future protocol version needs authenticated
capture/stream binding rather than impersonating Node.

Admin and dual-control operations are unavailable; decision credentials cannot
invoke them. Qualification remains incomplete until remaining native multi-user/remote
denials, independent audit anchoring, real Node/Bun tuple qualification, and isolated
Docker browser/Gate/Pass/lab evidence are available. Documentation owners should
carry these limits into public documentation; this slice does not edit docs/**.

Measured in this slice: Windows x64 Rust 1.88 tests (15 passed), including actual
pipe/PID/auth/replay/restart/body/audit/capacity/timeouts. Linux x64 Rust 1.88
network-none Docker tests (14 passed; two explicitly ignored privileged/helper
tests). The privileged other-user test was then explicitly run in a separate
network-none container with only SETUID/SETGID and passed; its child actually ran
as UID 65534 and received PermissionDenied. These counts are authority evidence,
not whole-product acceptance. Cargo advisory database commit
309392b18f6b17022f82c701f300433126901068 reported zero findings on 2026-11-07 KST.

Unix paths are canonical filesystem paths in private 0700 directories. Exact
route inventory currently supports ASCII paths without percent escapes; query
bytes remain in the bound request. After abrupt termination, a supervisor must
verify process exit/socket ownership before removing its own stale socket;
startup never unlinks an existing path. No live configuration reload exists:
revision changes require supervised restart and new credential delivery.

Windows currently requires inbox Windows PowerShell/.NET for fail-closed audit
directory ACL inspection. It creates a protected SID-only pipe DACL with the
interprocess 2.4.7 safe API, rejects remote clients and checks actual client PID.
It does not impersonate the client or claim an independently queried client
token SID; the DACL and PID are the two enforced boundaries. A trusted supervisor
must control process creation, SID/configuration and secret pipe inheritance.
Same-principal hostile processes, PID/handle reuse and independent remote-host
testing remain explicit deployment concerns. Audit parent ancestry is trusted
installation state; hostile administrator replacement is outside this profile.
