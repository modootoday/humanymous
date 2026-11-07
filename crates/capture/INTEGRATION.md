# Capture integration requests and qualification

Owner: network-capture/rust-network-capture-goal. Design authority:
sot:root/network-capture. Existing network, integrity, authority and runtime are
read-only here. Capture is a new transport-lifecycle context, not a fingerprint
or security-engine fork. Root default-feature all-targets Clippy passed, followed
by21pure lib units passed/0failed/0ignored/0filtered. hosted-pilot compile and its
5actual cases remain NOT_RUN; runtime binding and product qualification remain0.

Root has integrated crates/capture as a workspace member and resolved the
dependency graph. Add a workspace humanymous-capture dependency when a consumer
needs it. The crate uses path dependency ../network and existing rustls0.23.46 with
default-features=false, features=[std,ring,tls12], plus pinned httlib-hpack0.1.3
for bounded HPACK. No unsafe/foreign authored glue is needed. Root owns all
future dependency/lock windows and execution allocations. This owner is not
running Cargo; root has returned pipelineA to another owner after validation.
Source tranche is now authored: strict ClientHello assembly, actual TcpStream/
rustls owner, complete H2 framing/HPACK per-stream observations and pure fixtures.
The non-default hosted-pilot feature adds optional pinned rcgen0.14.10 (ring,
crypto, no default features) to generate transient synthetic credentials in the
hosted test process; no certificate/private key is checked in or persisted.
Default-feature compilation/units/Clippy have passed; optional hosted-pilot
compilation remains NOT_RUN. Direct rustfmt ran only against owned source.
See evidence:root/20261107075403 for final root output, earlier5lint findings,
source-hash scope and the separate unexecuted hosted/product denominators.

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

## Runtime contract for checkpoint46822eb

This is the committed component API contract, not acceptance of a completed
runtime integration. Default-feature21pure units and all-targets Clippy passed.
Optional hosted-feature compile, actual5socket cases, Send-bound integration,
runtime lifecycle and Docker product tests remain separate unexecuted gates.
Retain required.raw_tls/original_h2=422 on the existing plain listener and all
adapter0.1 inputs until the actual ingress composition qualifies each plane.

| Boundary | Current component behavior | Runtime obligation or remaining seam |
|---|---|---|
| Socket custody | Connection::accept consumes one actual std::net::TcpStream; no public constructor from a parsed hello or receipt | Bind an accepted ingress socket to one worker/epoch; no independently opened or replayed connection may supply its evidence |
| Principal/topology | Successful rustls handshake on that socket; no OS peer-principal, user identity or original-hop attestation API | Own listener/configuration/peer policy and lab topology. A TLS-terminating proxy produces proxy-hop evidence; forwarded fields never upgrade it |
| TLS completeness | Exact bounded first ClientHello reassembly plus completed rustls handshake; no early data or HelloRetryRequest qualification | Use client_hello only from that same live owner. No fallback to tolerant parser success or offered-ALPN claims |
| Raw retention/digests | Raw handshake bytes exist transiently in the bounded assembler and are dropped after parsing; parsed fingerprint fields remain. No raw transcript export, digest receipt, zeroization guarantee or persisted capture | Do not treat JA3/JA4 or source hashes as authenticated transcript digests. Define any future retained digest separately with purpose/retention and request binding |
| Epoch identifier | No serialized or stable connection identifier is minted | Runtime allocates its own unguessable/local non-reused epoch handle and owns the mapping to the consumed socket. Client envelope IDs never select it |
| Stream identifier | h2_request(u32) accepts the actual HTTP/2 stream ID and returns the complete initial header block only from the negotiated-h2 owner | Preserve actual parser stream association, compare exact method/scheme/authority/target and header sequence, bind RIT/body/session to that same request, never choose an arbitrary successful stream |
| H2 headers | Original connection preface, first SETTINGS, WU/priority observations and complete contiguous HEADERS/CONTINUATION/HPACK. One decoding context includes trailers; reset/errors invalidate availability | Own HTTP serving, semantic/framing validation and final request inventory. Decoder limits assume the default4096table/16KiB frame profile; unsupported settings/profile changes must fail closed |
| H2 body/lifetime | DATA framing and END_STREAM/reset are observed internally; no complete-body flag, bytes, length/digest or body replay API is exported | Runtime's actual H2 serving parser must own DATA/body/trailers/cancellation and prove completion/bounds. A captured header block does not authorize an incomplete or different body |
| Abuse | h2_abuse exposes the unchanged network FrameMonitor signal even after capture failure, for audit | Consume the frozen signal once through the existing policy path; this value is never an allow capability and does not create a new score |
| Lifetime | One absolute budget>0and<=30seconds starts inside accept and covers handshake plus later I/O. Socket read/write timeouts are recomputed from remaining time; EOF/error/deadline removes evidence | Bound queue/admission time separately; recheck owner liveness/cancellation/deadline immediately before audited origin effects. Borrowed data is a snapshot, not a reusable authorization |
| Post-handshake cancellation | cancellation() is available only after accept returns. Its atomic revocation precedes shutdown(Both); owner Drop also shuts down the socket | Retain the supervisor handle, cancel on shutdown/request expiry and join the worker. Cancellation handle drop alone does not revoke. Do not detach or just time out the joining future |
| Handshake cancellation gap | No pre-handshake cancellation handle exists in46822eb | This is an unresolved integration seam. Before production use, add a capture-owned pending-handshake owner whose revocation handle exists before any blocking TLS I/O and survives into the same connection epoch, with close-on-error/drop semantics. Runtime must not hide this gap behind an async timeout |
| Reaping/Send | Capture spawns no thread/process and owns no join handle. Connection/Cancellation are intended to move to a supervised worker, not expose raw sockets | Runtime owns spawn/join/reaping and bounded shutdown. Assert Send at the actual consumer boundary and compile/execute it on supported tuples; pure parser success does not qualify worker behavior |

The pending-handshake seam above is a required next capture API slice, not an
already implemented API. Coordinate it before runtime consumes the component;
do not duplicate TLS framing, HPACK, crypto, raw-header canonicalization, audit
or scoring in runtime. Root alone approves a runtime dependency/lock window.
No source or manifest changed as part of documenting this contract.

## Historical resume handover (before root local validation)

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
