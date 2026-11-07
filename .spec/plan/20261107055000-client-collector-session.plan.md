---
id: plan:root/20261107055000
title: Browser client session and actual transport integration
status: active
created: 2026-11-07T05:50:00Z
updated: 2026-11-07T05:50:00Z
rel:
  depends_on: [sot:root/client-collector, sot:root/runtime-application]
---

# Client session and transport

## Accepted actual-browser runtime pilot

Native-only runtime_pilot_contract integration test independently checks the
fixture/WebDriver shared loopback tuples, distinct nonzero ports, proxy mode,
absence of bypass/extra browser arguments, route uniqueness and the required
422/noACK/zero-effect expectations. It passed1test/0failed/0ignored/0filtered
under locked buildjobs1/testthreads1/incremental0. This is static cross-file
consistency only: configured localhost proxy preferences do not prove real
browser interception, namespace isolation or actual runtime outcomes.

The dedicated WASM pilot selects exactly one fully named product test; its66
ordinary component tests are intentionally filtered, not passed by this run.
fixture.json declares the selection and expected filtered denominator. The lab
must verify one terminal summary, the exact selected case, zero failed/ignored
and the declared filtered count. Merely finding a substring containing1passed
is insufficient. Network-negative timeouts are unknown, not proof of refusal.

The separate runtime-pilot feature and crates/collector/pilot/fixture.json define
one shared lab contract. Read that file rather than copying chat messages: the
scope is install/tenant/origin/r1 at127.0.0.1:31111; browser proxy is31112.
POST/effect before report must return403CHALLENGE with a valid consumed ACK;
actual unmodified BrowserCollector report ingestion returns200 with valid ACK.
POST/requires-collector requires unqualified fresh provenance and returns422
before consumption, without ACK; the client must close and never retry.
The separate lab coordinator proves actual proxy hits, isolated browser/gate
namespaces, inaccessible backend loopback, no egress and real witness/audit
outcomes. Same-namespace component results cannot replace these product gates.
Typed WebDriver JSON configures only a manual owned HTTP proxy and Firefox's
localhost proxy preference; it never disables sandbox. Existing66component tests
keep their default configuration. Select the dedicated actual-runtime test by
its full name; do not run the missing-owned-route negative in this live profile.
Current product pilot source and fixture are authored, not yet executed. Initial
cargo fmt was blocked by a concurrently added lab tokio.workspace reference;
root added the already-used Tokio1.54.1 workspace declaration, without upgrading
dependencies. Preserve this failure as integration lineage, not a test failure.
After the manifest correction, scoped WASM all-target Clippy with runtime-pilot
enabled passed under buildjobs1/incremental0. This compiles the new feature test
and embedded fixture, but does not execute HTTP/browser/runtime code. The lab
coordinator must separately observe required-capability422 with no ACK; the
client's closed/error result alone cannot distinguish that from network failure.

- [x] Accept exact scope/bootstrap/one-flight/clock/expiry/invalidation contract.
- [x] Accept shared response ACK with runtime owner; bind actual status and consumed
  counter to pending request MAC. ACK is not ALLOW/body/collector attestation.
- [x] Implement strict bootstrap, shared request binding, zeroizing session/token
  buffers, one-flight reservation, authenticated completion and ambiguous invalidation.
- [x] Pin ACK SHA256/HMAC vector independently checked with .NET.
- [x] Native collector26/integrity20+legacyparity3 and actualDockerFirefox58 pass;
  host/WASM all-target Clippy pass after owning helper fixes its WASM boundary.
- [x] Own browser Fetch Request/Headers/AbortController and a single whole-request
  deadline through complete buffered response; same-origin/omit/error/no retry.
- [x] Verify real final Request representation and existing app Authorization;
  reject forbidden/reserved headers, cross-origin, streaming and ambiguous paths.
- [x] Verify ACK only after complete bounded body, cancellation/drop cleanup and
  reentrancy/concurrency; invalidate on partial body/network/timeout/redirect.
- [ ] Join real runtime bootstrap/browser profile/ACK and actual owned Docker lab.
- [ ] Bind actual collector version/epoch/coverage to fresh server-issued context;
  keep required.collector422 until accepted provenance and actual browser pilot.
- [ ] Complete route-scoped cookie/identity profile, distribution/bootstrap and
  full product/human/keyboard/AT qualification; initial cookie-free is not final scope.

The initial runtime body limit64KiB is independent of the shared codec1MiB bound.
Bootstrap has exactly10 fields; no hidden budget negotiation. A pre-consumption
413 has no ACK and invalidates the client epoch. Server/client integration is not
proven by synthetic ACK unit tests or native/Browsers' pure lifecycle tests.

## Fetch implementation checkpoint under resource hold

The working tree now contains authored-Rust BrowserSession.connect/fetch/close,
bounded BrowserResponse, shared Abort ownership and a reusable whole-request Timer.
It constructs browser Request/Headers, signs the final serialized URL (including
an explicitly empty query), preserves application Authorization, and verifies ACK
only after the complete bounded response body. Request limit is64KiB; buffered
response limit is1MiB. Cookie, streaming and full collector provenance remain
required follow-up profiles, not supported capabilities of this checkpoint.

Existing private logs were re-read during the resource hold:

- fetch-transport-browser-final.log: actual Docker Firefox65passed/0failed/0ignored.
- fetch-transport-native-tests.log: collector26, integrity21 and3parity tests passed.
- Positive transport cases use Rust-authored Fetch/Response fixtures exercising
  browser APIs. The owned-route missing-bootstrap negative performs actual Fetch.
  These results do not prove a successful real runtime HTTP/bootstrap/ACK join.

Formatting occurred after the browser run. Therefore that run is historical
implementation evidence, not an exact-byte receipt for the current working tree.
The three implementation checkboxes above remain open until the source checkpoint
and exact-artifact verification are complete. No new build or test was started
during memory triage. Required next steps: preserve logs/source hashes, coordinate
the runtime bootstrap/profile/ACK seam, then acquire the single heavy validation
slot for exact-source browser verification and the actual owned product lab.

Resource policy: allow parallel source/spec edits within claimed paths; serialize
heavy validation across chats. Do not restart completed REA research or kill shared
MCP/WSL processes. Remote Docker execution may reduce local pressure but cannot
substitute component tests for the existing product isolation acceptance gates.

Bootstrap cancellation coverage delta: authored a browser test that captures the
actual constructed Request.signal and checks both dropping a pending connect
future and waiting for its timeout. Each must abort that signal rather than leave
bootstrap work running. This new test is not compiled or executed under the
resource hold; the historical65browser pass count does not include it.

Fixture lifetime correction: its Rust Fetch closure captures the fixture global,
which in turn holds that closure in its fetch property. Fixture Drop now clears
that property to break the cross-language reference cycle between browser test
cases. This is a test-fixture ownership issue; it does not establish the cause of
desktop-wide REA/MCP memory pressure. Compilation/browser validation is pending.

## Resource-gated verification update

After available physical memory recovered to31GiB (commit51/69GiB) and no active
cargo/rustc was observed, root used the single heavy slot with CARGO_BUILD_JOBS=1.
Current WASM tests compiled successfully. The first isolated Firefox container
exited139 with thread-creation EAGAIN under an explicitly configured PID cap256;
the original log is retained. Repeating the same WASM/image/network-none/UID65532,
2GiB-memory/one-CPU setup with PID cap1024 completed66passed/0failed/0ignored.
The new bootstrap cancellation/timeout test passed. No sandbox-disabling flags
were added. Both logs and a current-source/WASM SHA256 receipt are registered under
evidence:root/20261106194410 as fetch-transport-bootstrap-cleanup-* artifacts.
This qualifies the bounded browser component, not a positive real runtime join or
full product e2e. Source commit and Git-canonical byte proof remain pending.

Source checkpoint667e98f subsequently committed the implementation. Native50,
scoped WASM all-target Clippy and fmt passed. All14changed Rust source files were
compared via Git's path-aware clean filter against667e98f blobs; every normalized
working-source blob matches. Raw CRLF/LF bytes remain separately identified; this
is not a claim of raw-byte identity. The binding receipt is registered privately
as fetch-transport-667e98f-git-binding.json. The first three transport tasks are
therefore complete in browser-component scope; runtime join/full collector and
product/human/AT tasks remain unchecked. Further actual runtime execution now
belongs to the accepted standard-hosted validation policy, not routine local work.

## Actual browser request boundary extension

Extend the existing single selected runtime BrowserSession test with a local
65537-byte request rejection before session begin, followed by a real65536-byte
request to the existing effect route. The local rejection must leave the epoch
open and is not evidence of server HTTP413. The full-size accepted request must
verify its consumed ACK and receive the same pre-report CHALLENGE as the existing
small request. No scoring, request limit, route or identity policy changes.
The shared fixture expected.effect_requests becomes2; Lab owns exact actor and
report proxy-count consumption. Both successful effect requests precede collection,
so origin effects remain zero. Keep one selected test and66 filtered components.
Source authoring does not establish actual browser execution; require WASM
compilation and genuine owned Lab browser-product evidence for qualification.

The boundary source now passes native fixture consistency1, WASM all-target
runtime-pilot check, strict Clippy and scoped fmt. Final105 captured files did
not change during validation. Prior post-test fmt failure is retained separately;
final formatted-byte checks reran. Registry/SDK/compiler input closure was not
measured. Actual browser/product boundary execution remains NOT_RUN and the
Lab's exact count consumer update is separately owned and required.
