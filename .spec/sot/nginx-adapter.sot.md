---
id: sot:root/nginx-adapter
title: Rust nginx body-bound native adapter
status: active
created: 2026-11-07T00:40:00Z
updated: 2026-11-07T05:25:00Z
verify: manual
rel:
  governs: [adapters/nginx/**, crates/nginx-adapter/**]
  depends_on: [sot:root/adapter-service-contract, sot:root/adapter-authority]
---

# Rust nginx body-bound native adapter

## Current qualified component checkpoint

Exact source8132117e7bf1445939d7dee9066fcf45ccf0915b passed hosted nginx
job128358355202 in run42355168729:35 scenario groups passed, zero failed/ignored/
not_run, cleanup exit0, unchanged source and immutable image identity. Evidence
is evidence:root/20261107052501, hosted-813-execution-summary.json and
hosted-813-source-proof.json. The pinned Docker build enforces separate native,
pilot and package feature builds/Clippy plus library tests. Latest separate local
pure verification is14 passed plus package check/scoped Clippy (7f3dc93).

This qualifies the owned Linux HTTP1.1 component, including JSON keepalive,
SSE upstream cancellation and unsigned package binding assertions. Positive
ALLOW remains explicit trusted synthetic evidence through actual authority/WAL.
ApplicationHttp409, private-pipe executable startup and same-WAL reload assertions
do not qualify sealed ServiceHost startup or human/browser/product protection.
Full product remains false. Future worktree/API/profile changes inherit no result.
Earlier NOT_RUN notes below describe historical source-only checkpoints.

## Rust native replacement contract v0.2

The accepted current implementation target is Rust1.99.1/edition2024, package
humanymous-nginx-adapter, Linux HTTP1.1 only initially. The njs rule below is the
historical b1e10ae mock pilot, not the current native contract or proof of it.
Preserve those source files separately without adding foreign-language code.

Shared contracts, agreed with Node/authority/root owners on 2026-11-07:

| Contract | State and owner |
| --- | --- |
| integrity::adapter::{AdapterProfile::Nginx,parse_with_profile} | implemented/16 checks, root64852c4; plugin producer still unverified |
| protocol humanymous.adapter/0.1; id humanymous.nginx; version0.1.0-experimental.1 | accepted; shared contract owned by Node |
| nginx-raw / nginx-normalized | parsed application HTTP1 ordered casing/duplicates vs normalized; connectionId/capture/stream null, rawTls/originalH2 false |
| authority::client::decide(&ClientOptions,&[u8],ExpectedBinding) | implemented in authority owner's current working source; owner negative/native validation pending |
| ClientOptions endpoint/credential/server:&Peer/timeout_ms/adapter:AdapterProfile | explicit server UID and Nginx profile; no producer selected from envelope |
| BoundResponse + Verdict::{Allow,Deny,Challenge} | exact7field response; verdict()/expires() inspect only, consume(self) rechecks wall/monotonic expiry immediately before one origin continuation |

The module owns ingress projection, complete body, total monotonic body+decision
deadline, cancellation and a single proxy continuation. Shared authority owns
peer/ACL/credentials/replay/scoring/audit; no clone of those mechanisms. Client
completion is not authority qualification until actual peer/replay/audit tests.
No missing client fallback or fake ALLOW. Application-http results and injected
trusted synthetic fixture evidence must be reported separately from human ALLOW.

Use official nginx/ngx-rust rev ebbcd514ee7a336fa4b622232170d8a16763a9ca (ngx0.5.0)
and nginx1.30.6 upstream source SHA256
3cf46f5ed7504d460125866b7bea99fb357f31d887e2e63b35503310e9b8b6c8.
Build nginx and its generated bindings with the same configure/ABI inputs.
Official sources: https://github.com/nginx/ngx-rust/tree/ebbcd514ee7a336fa4b622232170d8a16763a9ca,
https://nginx.org/download/nginx-1.30.6.tar.gz,
https://nginx.org/en/docs/dev/development_guide.html,
https://nginx.org/en/docs/configure.html (consulted2026-11-07).
Upstream C/headers/generated bindings are pinned external dependencies, not
new authored C. Core request modules forbid unsafe; native.rs alone permits
reviewed FFI under crate deny, as required to register nginx callbacks/exports,
read nginx-owned buffers, rebuild the sanitized generic header list and manage pool/timer
lifetimes. This exception grants no arbitrary memory or thread access. Never
send a nginx pointer to another thread. Catch Rust unwinds at callback boundaries.

Register before access in Preaccess, so satisfy-any cannot bypass denial. Read
the complete request body through nginx's buffered body API, max1MiB; file-backed,
incomplete or unsupported bodies fail closed. One total deadline includes slow
body reception. Reject upgrades, CONNECT, gRPC, trailers, Transfer-Encoding,
non-HTTP1.1 and unsafe target/authority/framing. Preserve binary/compressed entity
bytes without decoding; ordinary duplicate request headers survive in order.
Remove internal/forwarded spoof headers before both envelope and proxy. Require
exact configured method/path inventory, Host and unchanged original target;
reserved /_humanymous subtree is never an application/admin endpoint.

Only owned Rust values cross into the worker-local bounded Tokio tasks. nginx
timers receive results on the nginx thread. Pool cleanup cancels the task and
removes its timer. Recheck expiry and unchanged request bytes before ALLOW;
no retry/cache. Unknown/reserved routes reject before origin. DENY403,
CHALLENGE409(no recovery), transport/malformed/mismatch/expiry503 no-store.
CHALLENGE must carry application/json and a fixed challenge_unavailable error,
with no authority-provided redirect or script. This body-format requirement was
not verified by16c18f3's status-only case. Its source-only follow-up is awaiting
native compile/runtime qualification under root's resource scheduling.
The source-only follow-up uses ngx_http_send_response with a constant zero-script
complex value, then one-shot pool-owned timer finalization after the SDK wrapper
returns. No cell borrow may cross finalization; origin phases must not resume.
These source-only ABI/lifetime and JSON checks were subsequently qualified by the
exact8132117 hosted component above; earlier35 status-only evidence stays separate.
These are module/IPC errors; nginx parser rejects malformed wire framing before
module invocation and retains its native400 response/error headers.

Standard proxy performs the authorized request once. No response interception,
HTML/CSP insertion or Set-Cookie coalescing. Disable proxy retries, caching and
X-Accel-Redirect processing; response buffering off permits SSE. Configuration
must have no post-authorization rewrite or alternate public origin route.
Management absent, IPC fixed local-only, no TCP fallback. Supervisor reads actual
authority readiness privately and provisions the nginx credential file with
owner-only0600 in canonical0700 parent; plugin reads no-follow, verifies owner,
and zeroizes its owned credential on drop. Actual shared client also verifies
the server peer. Same-user hostile processes, mount/ancestor trust and resource
qualification remain explicit deployment assumptions.

Critique reflected: fail-closed Preaccess prevents auth bypass; full-body and
final unchanged snapshot prevent pre-body/status-only authorization; total
deadline prevents trickle extension; no cross-thread nginx pointer; native ABI
and fixture vs production evidence remain separate. Unsupported streaming upload,
H2/H3/TLS capture, browser recovery, AT and non-Linux module tuples remain open.

## Native server module and hosted component boundary

nginx worker/request pools and Apache connection/request pools and MPM workers
have different ownership rules. This owner implements only nginx lifecycle;
Apache owns its corresponding native boundary. Share integrity profile parsing,
authority decision binding and capability vocabulary, not native pointers or
worker lifecycle machinery. No server-owned pointer crosses a worker thread.
Connection identifiers and normalized server headers cannot establish original
TLS/H2 provenance. Raw application HTTP projection and normalized projection
must remain explicit; missing required capture signals fail closed in authority.

Product-lab owns Docker Compose integration; CI owns workflow invocation and
strict aggregation. adapters/nginx/pilot-contract.json defines the fixed owned
Compose interface and exact35 scenario IDs for report humanymous.nginx.pilot/0.2.
Only exit0 plus exactly one source-bound complete success report qualifies this
component. Unknown/missing/duplicate groups, nonzero failed/ignored/not_run,
unbound revision/manifest/lock or changed image identity cannot count as passed.
Full product qualification remains false. No duplicate host/lab implementation.

The package schema humanymous.nginx.package/0.1 binds exact module and engine
bytes/lengths, actual engine -V configure identity, Cargo.lock, pinned Rust1.99.1,
x86_64-unknown-linux-gnu, nginx1.30.6 and SDK revision plus frozen source closure.
The writer uses a separate package feature build after the native .so is saved.
It accepts bounded regular files in trusted hosted build directories and creates
a new manifest without overwrite. It is not a secure installer or signature.
abi_load_qualified and signed_release must remain false in metadata; actual
hosted load/runtime evidence is an independent gate. No credentials or body data
enter provenance. The8132117 hosted component qualifies the report/package/
keepalive assertions; metadata itself still grants no signature or ABI permit.
The SSE scenario must also observe actual upstream EOF/reset after a downstream
client closes following its first event. Successful buffered writes alone do not
prove cancellation. This owned-origin witness is bounded to2s observation and
does not qualify sustained backpressure, load or independent audit witnessing.

## Current execution and handoff

Node actually acquired pass first despite planned nginx-first ordering. nginx
performed read-only ABI/contract research until Node voluntarily released the
lane for dependency waiting; nginx then claimed pass for adapter paths only.
No actual Pass/scoring edits. Native implementation and Rust driver are authored;
new source-bound Docker qualification is measured below. Default pure tests7/7
and strict Clippy pass in actual root working workspace on Windows. Pinned Linux
native module and Rust pilot both pass strict Clippy/build with the unaltered
27-member root Cargo/lock snapshot; unit7/7, Compose pilot35groups/0failed/0ignored.
The validated dependency snapshot is authoritative through its file manifest,
not a claim that every concurrently changing Git commit was validated.
njs24/mock28 remains historical and is not shipped in the native image.
Cargo/lock integration belongs to root. Accepted APIs above were reconciled with
the current authority client source, not yet a final canonical owner checkpoint.

## Source-bound native verification

Private bundle20261107052500-nginx-adapter-native retains commands, frozen
sources, image inspection and all failed/retried logs. Sanitized results belong
to evidence:root/20261107052501, attached through Specfold.

Actual Linux65532/network-none/read-only/cap-drop-ALL Compose execution includes
full1MiB binary/content-encoded bodies, ordinary duplicate request/response
headers, Set-Cookie, HEAD/204/ETag, SSE first event10ms, spoof stripping and no
origin before bound ALLOW. Missing capture, IPC outage/delay, client cancellation,
replay, UID mismatch and injected DurableAudit dependency failure cause zero
origin effects. Real malformed-wire duplicate Host/CL, TE+CL and dot-route cases
reject before origin; native parser400 can precede module response headers.

The real production authority executable starts through private stdin/stdout,
accepts explicit nginx scope, and yields CHALLENGE409 without origin for its
application-http profile. The test-only synthetic provider is the separate
positive ALLOW numerator with actual shared authority and WAL. nginx reload
transitions include observed503 before409. Actual executable restart authenticates
the same WAL and a locally retained verified signed checkpoint, rotates bearer,
rejects stale worker credentials, then continues records1->2 after reload. This
local anchor is not independent witness/rollback protection.

The final bounded32-request concurrency observation admitted16 and rejected16
with503, with exactly16 origin effects. Earlier root30-group snapshot admitted20
and rejected12; neither is a throughput/SLA claim or a configured16 total-request
limit. The implementation bounds simultaneously outstanding tasks per worker.
Sustained load, long-stream backpressure, cross-user matrix and independent
witness remain qualification gates. Full product/browser/Pass/human/AT absent.

First native pilot found real spoof-header leakage because nginx proxy ignores
hash=0 as a removal mechanism. Rebuilding the generic list fixed it; keep that
failure. Native strict Clippy initially rejected deprecated fetch_update and
redundant casts; fixed without lint suppression. A malformed-wire test exposed
the Rust fixture client's read-to-EOF reset behavior; reading exact response
Content-Length fixed the client, without changing the plugin to hide failure.

## Execution boundary under the confirmed common policy

User direction: local coding and bounded unit checks; actual runtime/Docker/
browser/Cross-OS on eligible free standard hosted runners. Common policy and
the confirmed DAG stay in [[sot:root/delivery]] / [[sot:root/program-roadmap]].
Those documents alone own quotas, scheduling and collection policy. nginx adds
only the scope boundaries below, not a second implementation or manual dispatch.

Local default-feature unit/fmt/narrow Clippy do not qualify Linux .so ABI,
pool/timer lifetime, peer/WAL, body/header fidelity, SSE or CHALLENGE JSON.
Those require actual pinned nginx/authority Docker tests on hosted Linux.
Source5ef5dd3 JSON/runtime remains NOT_RUN until scheduled. Other-UID privileged
helpers, true remote peer, interactive human/AT, non-Linux nginx and independent
witness/power-loss claims remain separate; no hosted green substitutes for them.

Shared authority/integrity/audit/scoring/network, Cargo/lock/toolchain, producer
contracts and nginx ABI/Docker inputs affect nginx. This mapping is advisory
until the common impact classifier exists; no required gate is removed by path
filters. Semantic contract changes are not ordinary docs. The confirmed2lane
cap is within its workflow, not a global manual-workflow mutex; do not create a
second nginx dispatch while root observes the common source-closed execution.

Native and standalone pilot have different feature/link graphs: --all-features
is not a valid deduplication strategy. Preserve .so before pilot build. Artifact
reuse requires matching libc/target/Rust/features/nginx configure signature.
Reuse within-job pinned engine/builder layers and permitted read-only caches;
the free branch profile disables cache writes and artifact uploads. Do not
mistake ephemeral BuildKit target mounts for permission to export a shared cache.

For the supplied14GiB fresh-VM budget, exclude generated target/cache inputs and
whole-image/workspace/target uploads. Authority readiness credentials, bodies,
cookies and private IPC traces never belong in public logs/artifacts. Recover
sanitized summary/log/hash as the common policy allows; explicitly record any
necessary private raw not recovered. A later .spec.local download is not privacy
for a public upload. No local native/runtime run resumes from this policy note.

## Historical njs pilot rule (superseded for new implementation)

Implement an njs js_content gate followed by a named internal proxy location.
No C/FFI or scoring. This is an experimental Linux nginx compatibility slice,
not whole humanymous protection. Shared adapter/0.1 currently permits Node only;
use distinct `humanymous.nginx-pilot/0.1`, never impersonate Node. Integration
requires authority-owner approval/schema extension, authenticated peer/ACL,
replay/audit and required-signal enforcement. Mock ALLOW is not Rust validation.

Bound complete transfer-decoded content-encoded body at 64 KiB; unavailable
body fails closed. Only GET/POST on exact /echo, /deny, /challenge, /bad,
/timeout, /cookies, /stream pilot inventory. Unknown and reserved /_humanymous
routes reject. Reject upgrades, CONNECT, trailers, transfer-encoding, duplicate
headers, Content-Encoding uploads, non-ASCII header values, unsafe/ambiguous target and mismatched Host. Fixed configured authority
is example.test. No rewrites after decision; proxy_pass without URI retains
original request target and body. No request retries or decision cache.

Use an internal IPC-only location, fixed Unix socket, fixed decision path,
credential injected from protected nginx configuration (fixture credential only
in pilot). No TCP authority fallback. No management methods or public admin.
Production socket ownership/mode, credential provisioning and authority peer
verification are open blockers. No client header can establish local identity.

Forward only explicit content-type, authorization, cookie, accept and user-agent
headers plus configured Host and generated framing. Decision binds this exact
header projection, method, untouched target, authority, body length/base64/SHA256,
nonce, deadline and configured tenant/origin/revision. Other headers including
forwarded/x-forwarded and x-humanymous are discarded on both legs. Projection is
an explicit capability loss: arbitrary request header passthrough is unsupported.
Raw TLS, original H2/H3 and original client provenance are unavailable.

Match exact bounded response fields and request identity, expiry and verdict.
Non-200, timeout, malformed/mismatched response or body error => 503 no-store.
DENY =>403; CHALLENGE=>409 with no origin contact. Recovery endpoints are absent.
Only ALLOW redirects once to a private named origin location. The subrequest
response cannot mutate origin headers. Origin responses use standard nginx
proxy semantics; preserve repeated Set-Cookie, no HTML/CSP transformations,
proxy buffering off for response streams. Upgrade/gRPC/uploads streaming are
unsupported; SSE response support is limited to normal proxy behavior.

## Historical njs rationale

Official njs requestBuffer/rawHeadersIn/subrequest/internalRedirect APIs and
nginx proxy_pass_request_headers permit a small auditable boundary. Bounded
js_content avoids pretending an auth_request status alone binds an unread body.
Sources consulted 2026-11-07: https://nginx.org/en/docs/njs/reference.html,
https://nginx.org/en/docs/http/ngx_http_js_module.html,
https://nginx.org/en/docs/http/ngx_http_proxy_module.html.

## Historical njs critique

Blue: authority alone owns verdict; no duplicated detector. Red: public direct
origin location, spoofed peer, body/target reuse and response mismatch must fail.
SRE: finite body/header/subrequest/time limits and no proxy retry. DPO: no access
log or envelope persistence, fixture token only. Evaluator: pilot protocol cannot
claim shared-contract or Rust compatibility; real browser/a11y flow remains open.
P0 production blockers: real authority integration, peer ACL/replay/audit and
required capture signals. P1: runtime matrix, disconnect/load/resource tests,
response streaming cancellation/backpressure, HTTP2 framing and browser recovery.

## Historical njs verification

Docker internal network only; real nginx njs plus mock Unix authority and origin.
Unit boundary cases and negative network cases must check zero origin effects.
No Go edits; full product Docker gate remains unrun by this scoped pilot.

