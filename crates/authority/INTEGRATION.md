# Local authority integration contract

Package/binary: `humanymous-authority` (publish=false). Build with
`cargo build --locked --release -p humanymous-authority`; executable at
`target/release/humanymous-authority[.exe]`. No TCP, admin, collector or browser
recovery endpoints are added to this binary. Node and explicitly provisioned
nginx use `humanymous.adapter/0.1` POST `/adapter/v0.1/decision`.
The old nginx mock projection is not this native contract.

## Current follow-up shared interfaces

Historical checkpoint e346834, accepted by Node/nginx/runtime, passed
Windows25 scoped tests, release LocalHandler3, Linux network-none Docker22
and explicit other-UID parent1. Linux default2helper ignores are not successes.
Scoped check/Clippy/format pass. Source-bound evidence is in public Specfold
evidence20261107053239 with canonical source proof. These results do not qualify
the later Python/Ruby profile or private-credential changes; their Cargo and
hosted execution remains NOT_RUN until source-bound results are retained.
This is component qualification; runtime admin/RIT and full product remain open.
All new code/tests use Rust1.99.1/edition2024; no authored foreign-language glue.

Startup JSON accepts `node`, `nginx`, `python-asgi`, `python-wsgi`, `ruby-rack`
in the optional `adapter` field (omitted means Node).
Configuration.adapter is the shared integrity::adapter::AdapterProfile. It is
credential scope, never selected from wire contents. Native nginx identity is
humanymous.nginx/0.1.0-experimental.1 and headerMode nginx-raw/nginx-normalized;
connectionId is null, normalized peer is null. Existing Node vocabulary remains.
Both are application observations; capture/stream null and missing raw TLS/H2/
collector/RIT capability422 remain. Shared parse_with_profile is the only parser.

```
use humanymous_authority::client::{ClientOptions,ExpectedBinding,decide,Verdict};
// ClientOptions<'a>: endpoint:&str, credential:&str, server:&Peer,
// timeout_ms:u64, adapter:humanymous_integrity::adapter::AdapterProfile.
// ExpectedBinding<'a>: id/tenant/origin/revision:&str, deadline:u64.
// decide(&ClientOptions, &[u8], ExpectedBinding) -> Result<BoundResponse,Rejection>.
```

BoundResponse is non-Clone; verdict()/expires() inspect and consume(self) checks
wall rollback, authority expiry and monotonic budget before yielding Verdict.
The host must still check cancellation/expiry immediately before invoking the
original bound handler exactly once. HTTP/data errors become static503; no retry,
redirect, cache or TCP fallback. server is the actual authority UID or SID/PID
from a trusted supervisor, not the declared original application peer. Client
validates native server identity and owner/protected pipe DACL before sending
the bearer. Windows identification SQOS prevents impersonation privilege; Drop
closes immediately without blocking FlushFileBuffers or deferred-close limbo.
The strict response reader completes at exact Content-Length, then client closes.

Runtime local management reuses these native server interfaces:

```
use humanymous_authority::transport::{
    Listener,Limits,LocalHandler,LocalRequest,LocalContext,LocalResponse,
};
// Listener::bind_ipc(endpoint:&str, peer:&Peer) -> io::Result<Listener>.
// listener.serve_handler(handler, Limits{capacity,rate_per_second,timeout_ms}).await.
// listener.serve_handler_until(handler, limits, shutdown_future, drain_timeout_ms).await.
// trait LocalHandler: Send+Sync+'static {
//   fn handle(&self, LocalRequest, LocalContext)
//       -> impl Future<Output=Result<LocalResponse,Rejection>>+Send;
// }
```

LocalRequest owns method,target,ordered headers,authorization,body. Headers are
service framing metadata, not original-client evidence. Duplicated framing/auth
singletons fail; ordinary duplicates are preserved for handler Origin/CSRF checks.
LocalContext.peer() is actual OS identity; check() returns trusted Unix millis
only within cancellation/monotonic/rollback limits; remaining() is Duration.
Clone the context into detached workers so admission remains held until they end.
LocalResponse has status/body, or json(status,&Serialize); transport revalidates
JSON/65KiB and alone writes HTTP. A partial write closes without appending503.
Management requires a separate listener, credential, strict route/CSRF/scope
policy and shared audit/control issuers; generic locality is not admin authority.
Existing decision serve refuses every admin path. protocol::read_response returns
actual status/ordered headers/body for common native tests, without fake frames.

Explicit shutdown uses serve_handler_until, which stops acceptance, signals
global cancellation, aborts/joins tracked connections and waits for every
admission-owning context/worker. Ok means drained; TimedOut is incomplete shutdown,
not graceful success. Dropping serve_handler cancels but does not acknowledge
drain. Runtime must retain custody/state while remaining workers own it and only
report graceful shutdown after explicit until/flush completion. Native controlled
blocking-work tests prove no early success and no canceled effect.

Current actual negative pilots: server PID/scope/profile/replay; foreign pipe
grant rejected before credential disclosure; 144 nonreaders then healthy service;
separate fixture-management credential/route/Origin checks; canceled blocking
worker holds capacity and cannot act. Fixture-management is not runtime admin
qualification. Historical15/14 results below are not results for this new source.

Dependencies still open: runtime's actual LocalHandler management/session join;
typed one-use provenance/RIT grant (v0.1 has no receipt field); shared audit-owned
Log+Custody lifetime owner (do not duplicate runtime startup::SealedAudit);
sealed-only authority startup; Windows different-machine remote denial and full
OS tuple/product e2e. No actual raw TLS/H2/capture producer exists in this crate.
Node/nginx observed application-http CHALLENGE is not ALLOW or human acceptance.

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
protocol is incompatible; the new native profile uses its explicit nginx
identity. Authenticated capture/stream binding still requires a future version.

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

Windows uses the native shared safe owner/DACL verifier for fail-closed audit
directory ACL inspection. It creates a protected SID-only pipe DACL with the
interprocess 2.4.7 safe API, rejects remote clients and checks actual client PID.
It does not impersonate the client or claim an independently queried client
token SID; the DACL and PID are the two enforced boundaries. A trusted supervisor
must control process creation, SID/configuration and secret pipe inheritance.
Same-principal hostile processes, PID/handle reuse and independent remote-host
testing remain explicit deployment concerns. Audit parent ancestry is trusted
installation state; hostile administrator replacement is outside this profile.


## Native Windows privacy follow-through

Storage directory verification no longer launches PowerShell. It opens the
non-reparse directory handle, reuses audit storage's shared safe SID/ACE verifier,
and requires owner-only protected DACLs. Custody uses the same implementation
with a distinct child-inheritance policy. Unknown ACE types, null/empty DACLs and
foreign owners/grants fail closed. No repository unsafe block or production
ACL mutation was added. Native fixtures now provision descriptors through the
safe Windows dependency and current_identity().sid() for the test token SID.
Existing Unix ancestor checks remain. Exact new SHA remote qualification is
required; older passing native reports do not cover these changes.

## Private credential injection and normalized producers

Credential::from_private_provisioning(Zeroizing<String>) -> Result<Credential,Rejection>
consumes exactly64 lowercase hex bytes. Shared storage and generation buffers
are zeroizing; borrowed/wire copies are caller-owned. No Debug/Clone/Serialize.
Management requires an independent epoch and scope; grammar is not entropy or
role validation. Runtime reuses the shared authenticator; injection integration
and actual management hosted validation are still its owned work.

PythonAsgi/PythonWsgi use humanymous.python; RubyRack uses humanymous.ruby, all
version0.1.0-experimental.1. Modes are asgi-normalized/wsgi-normalized/rack-normalized.
Readiness keys asgi_normalized/wsgi_normalized/rack_normalized are exact-profile
only and false whenever required raw TLS/H2/header order/collector/RIT is absent.
ASGI ordered duplicate pairs do not claim original casing; CGI map order and
duplicate loss remain explicit. Native client/server peer checks are unchanged.

## Later pure-library result

Root's source-bound shared-language-integration-38.json (evidence:root/20261106194410)
records authority10 pure lib tests passed, failed/ignored/filtered0 and scoped
lib Clippy passed. Its credentials.rs/configuration.rs SHA256 match the current
source at observation; execution HEAD d3d5a839fb2621b0316ed1629e86aa861f0c5bae.
Raw console was tool-output only, not recovered as a file. This supersedes the
pure-library NOT_RUN statement for those exact inputs only. Native process/IPC,
new producer host pilots and the later startup child-guard change remain NOT_RUN.

Apache/ASP.NET Core source additionally accepts config apache/aspnetcore, binding
Apache/DotnetAspNetCore to humanymous.apache/apache-normalized and
humanymous.dotnet/aspnetcore-normalized at0.1.0-experimental.1. Exact readiness
keys apache_normalized/aspnetcore_normalized are normalized-only. Configuration
changed after the authority10pure record: new compile/unit/native execution is
NOT_RUN; original host ABI/MPM/framework qualification belongs to host owners.

Audit-owned log::SealedLog now implements authority::decision::DurableAudit
directly; append errors propagate before response. Construct through audit
SealedLogOptions/UnlockCredential/PinnedWitness, retaining sealed ownership
through connection drain and flush. Runtime owns wrapper migration; authority
main still uses legacy inline-key configuration. Trait delta compile and hosted
startup/drain are NOT_RUN pending assigned gates.
