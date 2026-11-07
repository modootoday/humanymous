# Runtime integration request

## Current follow-through v0.3 (source/contract work, validation held)

Owner gate-edge/rust-runtime-followthrough normally claimed after authority
e346834/e8ddb4c and audit5528d74 released. Current normative contracts live in
runtime-application.sot.md and plan:root/20261107061820 (R01-R17). Historical native
slice results below remain historical; none qualifies this follow-through.

Current validation policy is the single program-roadmap/delivery canon: local
coding/fmt/bounded pure unit with Cargo concurrency1; actual process/port/IPC/
ACL/SCM/browser/Docker/CrossOS uses free standard hosted runners. No new local
runtime/browser/Docker/global-private scan is permitted. This chat has no running
owned process/container/session. Interactive human/AT/physical/independent
authority evidence cannot be inferred from a hosted label.
The earlier --locked baseline at1ee2c928 exited1 before tests because the shared
lock needed update; actual tests0, retained as failure. Do not bypass --locked.

Accepted shared browser API is integrity::request::browser::{Binding,canonical,
PROTOCOL,MAX_SAFE_INTEGER,acknowledgement,verify_acknowledgement}; root commits
53abb0b/fabed20 and client667e98f. Consumers import them, never copy bytes/crypto. Actual field,
error, lifetime, profile and ACK contracts are in owning SoT v0.3. Server profile
source is implemented; actual browser wire join is NOT_RUN. Initial profile rejects every Cookie,
preserves application Authorization and explicitly opts routes into browser RIT.
Required collector/TLS/H2 stay422 until actual evidence qualifies each plane.

Authority transport exact API: Listener::bind_ipc(endpoint,&Peer),
serve_handler_until(handler,Limits,shutdown_future,drain_timeout_ms). Handler
gets owned LocalRequest{method,target,headers,authorization,body} and actual
LocalContext{peer()/check()/remaining()}; returns bounded LocalResponse::json.
It owns no socket. Use context.check after durable work and before effects.
Explicit until/flush completion is required for graceful Stopped; drop/timeout
is not success. Runtime management credentials/allowlist/RBAC remain separate.

Audit supplies ControlAuthority/ActionPermit and CheckpointWitness/
WitnessServiceClient/Log::open_with_external_witness/flush. Use the real shared
issuer and transport, not synthetic booleans or a copied SealedAudit owner.
Shared audit-owned sealed Log+Custody lifetime owner is a later audit task.
rotate_approved currently binds the old ciphertext generation only, not a new
secret/destination; full destination-bound management rotation is unsupported.

Pending Cargo delta: add runtime dev-dependency
`humanymous-collector = { path = "../collector" }` to exercise the actual shared
client Session/ACK state machine against native TCP, not a copied algorithm.
This reuses an existing workspace package/version; root must refresh runtime's
Cargo.lock dependency list before --locked tests (root reports this is done).
No production dependency added.
New dependency/membership/feature requirements
will be recorded here before implementation; only root changes shared manifests
and lock. Collector/integrity remain root-owned. Native capture expansion is
narrowly authorized in future but not yet claimed/edited. All R01-R17 remaining
requirements and external ownership are tracked in the plan, including cookie
compatibility, typed adapter0.2 receipts, full H2, Pass, actual service credential
provider and whole-product human/AT/Docker gates. Product readiness remains false.

Source checkpoint entrypoints for hosted verification:

```
cargo test -p humanymous-runtime --locked --lib tests::browser
cargo test -p humanymous-runtime --locked
cargo clippy -p humanymous-runtime --all-targets --locked -- -D warnings
```

The browser module contains two authored fixture tests using the real
collector::session::Session against runtime native TCP/shared ACK and a separate
origin counter. Latest source execution is NOT_RUN. It checks bootstrap10fields,
no bootstrap ACK, CHALLENGE+ACK progress without origin, empty-sessionId report
projection, fixture ALLOW audit-before-origin, original application Authorization,
duplicate Set-Cookie, forged origin ACK stripping, replay and Cookie refusal.
Native helpers remain synthetic controls, not ordinary browser/human/AT success.
Root's actual BrowserSession API can connect to the new reserved bootstrap and
fetch collect/protected with matching exact origin/authority and caller scope.
required.collector remains422 pending fresh provenance, and there is no public
admin, servicehost, Pass or TLS/H2 support in this checkpoint. R15 and R05/R06
remaining work continue in later commits; this is not whole-plan completion.

Intermediate production-only cargo check passed before final report/test edits;
it is not validation of final bytes. Formatter and public-only spec check are
the only checks bound to this source checkpoint. No new local runtime/Docker/
browser execution was performed. Free hosted join/negative pilots belong to
CI/Compose/root; no paid resources are requested or created.

Owner: detection-core / rust-runtime-next. Status: source implemented and runtime
package integrated; full workspace/CI/Docker qualification belongs to root. No further source changes are
planned unless validation finds a defect.

Integrated lock delta: runtime now also directly depends on `socket2 = "0.6.5"`
(already in the current Tokio closure). Safe native MSG_PEEK is needed to detect
client FIN/RST after synchronous durable append; Tokio cached try_read missed
that abort in the regression test. The runtime dependency list is now reflected
in root's shared Cargo.lock. No new upstream version required.

Root added `crates/runtime` to workspace.members and updated Cargo.lock. No
shared manifest, lockfile, CI or lab file is edited by this lane. Package remains publish=false.
Dependencies reuse current workspace closure: authority, audit, integrity,
network, scoring, signals, serde, serde_json, sha2, hex, base64, getrandom,
zeroize and tokio 1.54.1 (net/io-util/time/rt-multi-thread/sync/macros).

The first native binary is explicitly loopback-only HTTP/1.1, complete bounded
body, exact route inventory and loopback owned origin. It owns volatile session
epochs, client-asserted report ingestion and RIT binding. No TLS/H2 capture,
Rust browser collector, browser recovery, admin or full product qualification.
Required absent planes reject. Docker product integration is requested only
after the native source/tests are ready; native fixture tests are separate.

## Interfaces and execution

- Binary `humanymous-runtime`: no args, bounded JSON through inherited stdin
  (close after writing), inherited piped stdout emits one readiness JSON line.
  Source of the exact JSON fixture is `tests/native_process.rs`. Audit paths
  must be canonical and owner-private; audit HMAC/signing keys are stable
  provisioned values, never automatically replaced on malformed recovery.
- POST `/_humanymous/session`: empty complete body, exact configured Host;
  returns fresh session/key/counter0/expiry. No browser cookie or admin authority.
- POST `/_humanymous/collect`: exact session-bound ClientReport JSON plus RIT.
  This is client-asserted ingestion. It does not qualify a browser collector.
- Configured protected exact method/path routes: RIT mandatory; query bytes,
  all ordered headers (except signature), body, tenant, installation, origin, revision,
  authority and deadline bind through `session::canonical`, with the shared
  integrity seed/token implementation. Consumers MUST call that API, not copy
  the binding algorithm. Counter increments on authenticated attempts even if
  later audit/collection/origin work fails. Seed counter is previous accepted n.
- Application Authorization is reserved for runtime capability and stripped
  before origin. A separate upstream Authorization header/stream/upgrade profile
  is unsupported; do not install this slice over an arbitrary existing app.
- All responses close; complete request body64KiB, origin response64KiB plus
  headers16KiB. No compressed body rewriting, TE/chunked/upgrade, duplicate
  request headers or encoded/dot-segment paths. Responses preserve Set-Cookie
  duplicates in their exact validated opaque origin frame.
- ALLOW is consumed directly by the original application call after durable
  Log.append and deadline/liveness checks. Errors/unsupported/CHALLENGE do not
  dispatch. Origin failures after request bytes may be ambiguous and never retry.
- TLS/H2 and browser collector required routes are422/unready. Pass, browser
  recovery, admin/dual-control actions, remote workload auth, origin bypass
  isolation, independent witness and full Core/Gate parity remain absent.

Commands with root's integrated runtime socket2 lock dependency:

```
cargo test -p humanymous-runtime --locked
cargo clippy -p humanymous-runtime --all-targets --locked -- -D warnings
rustfmt --edition 2024 --check crates/runtime/src/lib.rs crates/runtime/src/main.rs crates/runtime/tests/native_process.rs
```

## Measured and pending

Windows x64 Rust1.99.1 actual-source detached harness:12unit/native-TCP tests
plus1native child-process pilot,13passed/0failed/0ignored; Clippy-Dwarnings passed.
The native process pilot really runs session issue, collect, fixture ALLOW to
an independently counted origin, replay refusal, authenticated WAL restart,
credential invalidation and wrong-key startup refusal. The TCP abort test uses
an independent OS thread to deliver FIN during synchronous audit. Client report
fixtures with zero motor/typing richness do not claim a real human/browser ran.

Root had already added runtime membership and an earlier source snapshot passed
11unit+1process workspace tests. The final direct socket2 dependency subsequently
was integrated by root; runtime-only --locked workspace tests now pass13/13.
Raw logs/private harness metadata: evidence:root/20261107002939. Sanitized summary
and exact source commitments: evidence:root/20261107003506.

Docker product runner request: use actual binary and session::canonical with
complete literal request headers in final order; no handwritten JS or copied
token algorithm. This loopback-only listener needs client/runtime/owned-origin
in one isolated container/network namespace. Do not widen listen/origin binding
to public IPs to fit existing adapter lab topology. Docker executable is absent
from this chat's PowerShell PATH; no Docker success is claimed here. Root may use
its qualified Docker environment. Full browser/control outcomes remain not_run.

Native storage privacy follow-through: authority::transport::private_directory
now delegates Windows owner/DACL verification to audit::storage::permissions.
It queries the actual opened non-reparse handle and requires protected owner-only
DACLs, without PowerShell. Custody consumes the same native descriptor verifier
with its distinct inheritance requirement. No ACL repair occurs during startup.
Windows CLI still has separate inline script debt; this change does not claim
all authored product sources are Rust-only yet.

## Prepared control matrix

| Case | Expected observation | Execution |
| --- | --- | --- |
| Coherent ordinary Chrome + real collector/RIT | Actual human control, separate origin/audit counter | not_run; Rust collector absent |
| Keyboard-only with no mouse richness | Same policy path, no motor/speed condition | browser not_run; zero-motor fixture wiring passed |
| Real screen reader / AT | No required event speed/richness; record actual UX/recovery | not_run; browser recovery absent |
| Missing collector-required/TLS/H2 |422and origin0 | native unit passed; Docker pending |
| Body/target/method/tenant/revision/cookie/header-order mutation | Reject before origin | native binding tests passed |
| Replay/expired/aborted/audit unavailable | No protected origin dispatch | native/TCP tests passed; Docker pending |

Final-product readiness stays false. No scoring policy, threshold, band or rule
predicate was changed. No shared Cargo/CI/lab files were edited by this lane.

See sot:root/runtime-application and its implementation plan for acceptance.


## Root follow-through: sealed startup v0.2

The runtime now consumes audit.keys.directory and audit.keys.passphrase through
private inherited JSON. Inline hmac_key_hex/signing_seed_hex are rejected. Keys
must already exist in native sealed custody; startup never provisions or repairs
identity. SealedAudit owns both Log and lifetime Custody, so application code
cannot accidentally discard the key lock while retaining the log. Passphrase
buffers are consumed and zeroized before bind. CLI provisioning/rotation,
independent witness, service integration and full product qualification remain.
The exact updated fixture is tests/native_process.rs; that test preprovisions
synthetic sealed material, then validates real child-process restart and refusal.
Updated Windows native tests:13unit/TCP +1process,14passed. Root Docker process
pilot passes with sealed wrong-password/missing/tampered/concurrent-writer
refusals, actual origin2/audit4 and restart invalidation. Source/image commitments:
evidence:root/20261106235519, sealed-runtime-custody-results.json. Earlier source
snapshots above remain historical; new Cross-OS qualification is required.

R15 source checkpoint adds service::{read_configuration,ShutdownRequest,Readiness,
ExitReport,ServiceHost}. ServiceHost::bind authenticates existing sealed custody
and binds ingress before readiness; run takes watch::Receiver<Option<ShutdownRequest>>.
The serialized application stops accepting on observed shutdown, checks the watch
before origin connect/write, bounds active asynchronous drain, and flushes the
retained audit before reporting stopped. Closed controllers and incomplete drains
return errors. No detached workers are introduced. Synchronous private input,
custody recovery and WAL flush require supervisor deadlines; this source cannot
promise their preemption. Main reuses this lifecycle with legacy inherited pipes;
actual SCM/SIGTERM controller, management listener and unattended credentials are
still R09/R16 work. This implementation and all lifecycle tests are NOT_RUN.