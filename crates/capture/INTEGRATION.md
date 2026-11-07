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
