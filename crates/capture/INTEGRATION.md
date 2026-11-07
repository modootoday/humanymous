# Capture integration requests and qualification

Owner: network-capture/rust-network-capture-goal. Design authority:
sot:root/network-capture. Existing network, integrity, authority and runtime are
read-only here. Capture is a new transport-lifecycle context, not a fingerprint
or security-engine fork. All current source/tests are NOT_RUN; hosted/product0.

Root requested changes (not made by this owner): add crates/capture as workspace
member and workspace humanymous-capture dependency when needed. Crate manifests
will use path dependency ../network; reuse existing rustls0.23.46 with
default-features=false, features=[std,ring,tls12], plus pinned httlib-hpack0.1.3
for bounded HPACK (new dependency closure requires root review/lock update).
No unsafe/foreign authored glue is needed. Please allocate the global Cargo
slot once the source tranche is ready; none is currently being consumed here.
Source tranche is now authored: strict ClientHello assembly, actual TcpStream/
rustls owner, complete H2 framing/HPACK per-stream observations and pure fixtures.
The non-default hosted-pilot feature adds optional pinned rcgen0.14.10 (ring,
crypto, no default features) to generate transient synthetic credentials in the
hosted test process; no certificate/private key is checked in or persisted.
All dependency closure/compilation remains NOT_RUN pending root's workspace/lock
and Cargo slot. Direct rustfmt has run only against owned source.

Runtime seam request: capture owns a same-transport rustls connection and
produces borrowed connection/stream evidence only after handshake/header-block
completion. Runtime must consume that owner directly, bind exact request and
stream, then call the existing network report builder once. No public boolean
handshake attestation or deserialized capture envelope is proposed. Existing
plain HTTP1 listener and adapter0.1 must retain422 for required TLS/H2. Their
current unconditional rejection is correct until actual integration; replacing
it with trust in forwarded headers/capability bits is prohibited.
Exact report seam: network::report::build currently renders negotiated_alpn from
the first offered ClientHello name. Connection::negotiated_alpn returns rustls'
actual negotiation. Do not conflate these in a capture-qualified response; the
runtime/network owner should coordinate that output field while preserving all
existing frozen heuristic predicates (which inspect offered protocols).
The initial actual socket owner is synchronous. Tokio integration requires a
supervised joined blocking worker, absolute lifetime<=30seconds, and use of its
revocation-only Cancellation handle; no detached spawn_blocking task. The handle
atomically revokes evidence before shutting down the same socket, and owner Drop
shuts it down even when a supervisor handle survives. A native async serving
loop/profile remains an explicit owning-runtime integration task.

Post-checkpoint review (after ab4e8ee): every HPACK representation now receives
safe integer/string-span validation before the pinned decoder and is decoded
alone. Upstream decoder truncated integer paths may index past their input;
calling it directly on an unvalidated block is prohibited. Regression fixtures
cover each incomplete integer/literal family. The shared network FrameMonitor
is reused unchanged; Connection::h2_abuse exposes its frozen audit signal even
after a terminal capture refusal, never as an allow capability.

Hosted request: free standard GitHub runner must execute an owned Rust TLS/h2
client/server pilot with synthetic credential fixture. Delivery/product-lab own
workflow/Compose changes and product isolation/effect witnesses. No dispatch,
push, local socket or Docker work will be performed here.
Authored invocation: cargo test --locked -p humanymous-capture
--features hosted-pilot --test hosted_transport -- --ignored --test-threads=1.
The actual tests assert GITHUB_ACTIONS=true and RUNNER_ENVIRONMENT=github-hosted
before opening a socket. Five denominators: actual TLS/H1 without H2 claim,
actual TLS/H2 continuation/order/stream association, actual wrong-continuation
terminal refusal, actual invalid TLS refusal, and actual supervisor cancellation
revocation. These are component tests only.

Public docs delta for Pages owner: this is original-hop capture only when the
deployment actually terminates the original client; reterminated proxy traffic
describes the proxy hop. Unit parsing and a successful TLS pilot do not qualify
request integrity, collector execution, T4/human identity or full product.

## Resume handover (2026-11-07)

Mission remains full original TLS/H2 producer, owning runtime binding,
fail-closed required-plane handling, pure units, actual owned hosted traffic and
Docker product qualification; source-only support does not complete it.

Owned checkpoints: ab4e8ee (producer source),3e836ea (HPACK span safety,
cancellation and exact historical source receipt),bccecba (TLS extension and
hostile-compression fixtures, future-idle WU refusal). Current authored counts:
21pure units,5hosted component tests; all compile/unit/Clippy/actual runs0.
Direct owned rustfmt passed. Scoped public spec passed0errors/0warnings; last
staged check had0errors/2missing-change-record warnings. No policy/scoring or
other owner's source changed. Follow AGENTS.md and shared lessons on resume.

The same external blocker was observed in the initial goal turn and both
continuations: root has not added capture to Cargo.toml/Cargo.lock, and has not
allocated its explicit validation pipeline or hosted dispatch. This owner may
not bypass those rules. No running capture process/job exists to wait on.
No local socket, Docker, browser, paid infrastructure or publication was used.

Next authorized owner actions:

1. Root reviews pinned dependency closure and adds workspace membership/lock in
   its integration window. No dependency/feature/manifest change by this owner
   until root approves that window. Existing manifest is already committed.
2. Root assigns A or B target explicitly. Run scoped locked lib units, owned fmt
   and Clippy with that exact target/profile/jobs/incremental policy. Compile
   hosted-pilot without running socket tests locally. Preserve every failure.
3. Root/delivery dispatches the five ignored actual hosted component tests using
   the command above. Record exact source, OS/arch, runner,5case denominators and
   actual failure/recovery evidence; ignored tests alone are not successes.
4. Runtime owner accepts the same-Connection/stream join and supervised worker
   contract, corrects the old runtime SoT's network/src/capture path proposal to
   the root-authorized crates/capture context, and keeps plain ingress422.
   Runtime must own request/body binding and late liveness checks; no capture
   upload, forwarded identity bit or independent bool receipt is a substitute.
5. Product-lab validates actual topology, denied-origin effects and missing,
   malformed, reset, expired, cancelled and reterminated-plane cases in Docker.
   Public docs belong to Pages; submit deltas rather than editing docs here.

Read .spec/sot/network-capture.sot.md and
.spec/plan/20261107072531-network-capture.plan.md. Historical attachment under
.spec/evidence/20261107073810-network-capture identifies the3e836ea source,
not a successful run or the later21unit source. Reclaim network-capture normally
before resuming writes; never force another owner or change shared Cargo/lock.
