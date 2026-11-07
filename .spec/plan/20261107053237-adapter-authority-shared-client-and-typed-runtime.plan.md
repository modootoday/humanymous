---
id: plan:root/20261107053237
title: Adapter authority shared client and typed runtime integration
status: active
created: 2026-11-07T05:32:37Z
updated: 2026-11-07T05:32:37Z
rel:
  governs: [crates/authority/**]
  depends_on: [sot:root/adapter-authority]
---

# Adapter authority shared client and typed runtime integration

## Goal

Complete the assigned authority continuation through shared client/transport,
typed provenance/session/admin integration and actual negative native pilots.
Do not treat the first checkpoint as completion of the remaining runtime scope.

## Steps

- [x] Read roadmap v1.2, current source and all relevant canon; baseline native15
  passes on Rust1.99.1 while audit work is explicitly separate.
- [x] Wait read-only during audit ownership; accept d001ee6 and really claim gate-edge.
- [x] Record Node/nginx/runtime/root/runner accepted API contracts in authority SoT.
- [x] Implement credential-bound Node/Nginx profile and shared strict client.
- [x] Verify actual server peer/pipe ACL; remove unbounded Windows flush lifetime.
- [x] Implement generic LocalHandler/LocalContext IPC for runtime admin reuse.
- [x] Run pure/actual native and Docker component tests; retain failures/source bytes.
- [ ] Commit owned source, hand gate-edge to runtime with exact usable APIs.
- [ ] Reconcile runtime's actual management/session provider after its source exists.
- [ ] Implement and qualify typed one-use RIT/provenance join; v0.1 unsupported
  profiles remain 422 until the real producer/receipt contract exists.
- [ ] Resolve sealed audit lifetime owner dependency with audit/runtime, avoiding
  a duplicate startup implementation or crypto owner.
- [ ] Qualify actual Unix other-UID and Windows peer/ACL/remote client negatives.
- [ ] Source-bind public sanitized/private raw evidence; LF/Git bytes/spec checks.

## Dependencies and order

Root completed integrity parse_for/getters (fe432cc), explicit AdapterProfile /
parse_with_profile (64852c4) and browser pure codec (53abb0b). Runner provided
verify_windows_pipe_privacy safe actual-handle helper (26d9925). Authority adds
existing native-security path dependency only after claiming; root alone resolves
Cargo.lock. Audit d001ee6 provides durable witness client, signed ControlAuthority,
subject-scoped erasure and one-use ActionPermit. Shared sealed writer startup is
still a proposal, not a delivered API. Runtime first requested generic IPC; to
avoid its reverse dependency the accepted serial order is audit checkpoint →
authority common client/transport → runtime core/admin → authority receipt join.
Node/nginx accepted no duplicate private client. Their host callbacks own total
body deadline, body fidelity, cancellation/backpressure and origin invocation once.

Current remote-machine dependency: no independent Windows client host has been
provisioned by this chat. Local UNC path rejection or SMB-loopback is not proof
of a distinct machine's PIPE_REJECT_REMOTE_CLIENTS enforcement. Native runtime
and product Docker/full browser results remain different qualification contexts.

## Implementation checkpoint in progress

Common client and handler APIs are implemented in current working source and
accepted by Node/nginx call sites. Windows locked check/Clippy and actual native
client/profile/server-peer/DACL tests pass. Generic local-service conformance
proves separate credential/route/origin policy and detached-worker permit
retention after cancellation. The fixture is transport conformance, not runtime
admin qualification. A world-granted real pipe is refused before any credential
byte is written. 144 nonreading pipe connections exceed the previous128 blocking
worker threshold and a subsequent real decision still succeeds.

First full follow-up run failed: old native fixture read-to-EOF spent the request
deadline while the server bounded its Windows response lifetime. Retain
windows-initial-shared-client.txt privately. Corrected fixtures use the shared
strict protocol::read_response Content-Length decoder and close immediately;
scope/body/capture/expiry expectations were not weakened. New Response contains
actual status, observed headers and exact body, never a reconstructed fake frame.
The public reader is also reusable by runtime's real local IPC tests.

Windows client opens only exact local pipe names with identification SQOS,
validates safe pipe type/serverPID and actual shared owner/protected-DACL before
writing credentials. The stream wrapper deliberately avoids blocking native
flush and deferred-close limbo. Server waits for peer close inside the original
whole-connection deadline; cancellation closes and may discard unread bytes.
Client reads exactly one bounded response and drops the stream, no retry/cache.
Authority prerequisite raw capture/collector/RIT behavior remains unchanged422.

Final component results: Windows24passed/0failed/0ignored; isolated release
LocalHandler2passed; Linux Docker21passed/0failed with2explicit privileged/helper
ignores, followed by other-UID parent1passed (nested helper not another product
case). Current scope is not admin/RIT/full product qualification. Native release
runner caught a fixture race: capacity2 correctly returned429 while its third
policy request expected403 before prior peer-close events were reaped. Raw
interop fixture now uses immediate close and an explicit reactor turn after the
exact response; capacity and negative403 assertions are unchanged. Separate
capacity/cancellation and144-nonreader tests remain. Preserve the runner failure.

Other attempts: shared release executable was held by another actor (OS5), so
retest used target/authority-followup-release without killing shared processes.
First narrow Docker context omitted newly added non-crates workspace members;
owned Dockerfile now copies their actual Rust sources. Initial broad context
transfer was terminated without a test result. Root's changing lock temporarily
refused --locked; no lock bypass or owner edit occurred. The image has an explicit
tested source override for root integrity/browser/ack.rs, whose source advanced
after the build; exact old/new input hashes and reruns are retained privately.

Cargo audit:0known vulnerabilities in current workspace; retain unmaintained
ttf-parser RUSTSEC-2026-0192 in the separate UI closure (absent from authority's
dependency tree). No container OS CVE-clean claim. Scoped fmt/Clippy pass;
workspace fmt reported another owner's Node test formatting and was not repaired
outside lane. Canonical spec's existing binary-secret-scan warning is separate.

Runtime R15 review accepted: untracked listener tasks could outlive stop and
prevent honest custody-release qualification. Added serve_handler_until with
tracked JoinSet/global cancel/join/all-permit drain and bounded TimedOut error.
An actual controlled native worker proves stop remains pending until it ends
and cancellation prevents its effect. Existing serve_handler/drop does not claim
graceful drain. This extra tested source supersedes earlier24/21-count snapshots;
final denominator/source receipts below will identify the new checkpoint.

Final R15-inclusive current source: Windows25passed, optimized LocalHandler3,
Linux Docker22passed plus explicit other-UID parent1.0failures; default Linux
helper ignores2excluded. All final command exit codes0 and Docker exits0.
Source-bound sanitized summary is evidence:root/20261107053239. Private execution
bundle20261107053238 retains commands, source snapshots, original failures and
container/image/source-override identity.192 selected inputs were stable across
native validation and19 owned Rust/manifest/test inputs match LF/Git-filter bytes.
No combined canonical workspace, real admin/RIT or full product completion claim.

Next serial handoff: audit requested a short gate-edge repair of two inherited
child stdout status() calls found by strict Windows runner accounting. After
authority commits this usable client/transport checkpoint, audit reclaims and
fixes only its tests/canon, then releases to runtime. Runtime consumes actual
shared client/LocalHandler/serve_handler_until; authority remains read-only until
that owner releases. Remaining receipt/sealed-owner/remote-host requirements
above are pending, not closed by this checkpoint.

## Record

Read-only coordination history is preserved above from user-authorized existing
chats. No lane steal/force/fake claim, other-owner edit, branch switch, push,
main/tag/publish or scoring change. New code/tests are Rust only. Future state
changes update the accepted/implemented/verified distinctions and evidence here.

## Python/Ruby configuration follow-up (2026-11-07)

Root assigned configuration.rs only plus this plan and authority SoT, on free
release lane rust-authority-profile-config; runtime confirmed no file overlap.
Exact PythonAsgi/PythonWsgi/RubyRack contract comes from Python/Ruby adapter owner
and integrity implementation remains root-owned. Narrow serde/readiness source
adds explicit normalized profiles and removes nginx inference from non-Node.
No runtime/build test was run for this delta; hosted source-bound validation
remains required. Existing 25/22 results do not qualify this changed source.

Explicit user follow-through request created an active goal on this authority
chat: authenticated IPC, UID/SID/PID verification, typed authority/runtime
contracts and all producer profiles through source-bound hosted qualification.
A pure provisioning test now distinguishes the exact five config names from
producer IDs and wire mode strings. Execution remains NOT_RUN awaiting the
root-owned integrity enum and assigned single Cargo slot. No owned process or
container remains running. Existing runtime lane stays independent.

Root integrity enum is now present with the accepted exact IDs/modes. Windows
owner yielded release normally; authority reclaimed only its three paths. Added
a second pure test: each normalized producer enables only its own readiness key,
and each required TLS/H2/raw-header/collector/RIT plane makes it unavailable.
Direct rustfmt edition2024 and owned diff whitespace check passed. Public-only
spec check:0 errors,2 other-owner warnings (binary scan and missing verify).
Cargo compile/unit execution NOT_RUN: no assigned Cargo slot. Actual hosted
profile/native runtime evidence remains pending; old green is not reused.

## Shared credential injection slice

User active goal and runtime integration request require a single shared bearer
primitive. New actual authority-ipc lane claimed own authority scope, independent
of runtime. Added Zeroizing private injection constructor and entropy ownership,
canonical grammar refusal tests and exact Bearer negative cases. Own manifest
uses existing workspace zeroize; root alone must update shared lock package
dependency edge. Direct rustfmt/owned diff check passed. Cargo/unit/hosted tests
NOT_RUN pending root lock and single-Cargo-slot allocation. Source is not yet
qualified; prior component green is historical only.

Integration handover now pins historical25/22 results to e346834 and explicitly
excludes later source. New profiles and Zeroizing injection API are documented;
removed stale whoami fixture claim. Credential borrowing does not enforce once
delivery; supervisor owns count and epoch. Current runtime management Handler
now consumes shared LocalHandler/serve_handler_until with generated independent
credential; injected credential integration and actual hosted results remain open.
No Cargo slot/lock edge yet at this observation. Formatting/diff passed.

## Hosted normalized-producer native fixture

Added one native_process test with three producer iterations: actual child
authority startup and manifest, native server verified shared-client decision,
one-use consume, replay refusal, cross-profile wire-mode refusal, forged capture
refusal and missing raw-plane422. Existing startup helper now checks the entire
provisioned manifest rather than assuming every non-nginx producer is Node.
This is authored actual-IPC component coverage for hosted OS runners, NOT_RUN.
It does not run Python/Ruby interpreters or constitute their host/product e2e.
One Rust test with three iterations must not be reported as three passed tests.
Direct rustfmt and whitespace check passed; no local process/IPC test executed.

Public spec check at this checkpoint returned1 E1 from runtime's concurrently
changed source-commitments.json attachment and1 pre-existing binary U2 warning.
Owner notified; no modification or waiver of other-owner evidence. No pass claim.

## Reopened source-bound unit qualification

Root integrated the authority zeroize lock edge. Read private execution summary
shared-language-integration-38.json in evidence:root/20261106194410, execution
HEAD d3d5a839fb2621b0316ed1629e86aa861f0c5bae. Locked pure lib counts:
authority10, integrity22, framework6, failed/ignored/filtered0; scoped lib
Clippy-Dwarnings passed. Authority credentials/configuration SHA256 exactly
match the retained execution record at this observation. Raw console output was
not recovered as a file: summary/tool-output evidence only, explicitly limited.
Actual IPC and host pilot remain NOT_RUN. Do not count38 as authority tests.

Native fixture now owns the child in its Process RAII wrapper immediately after
spawn, before provisioning/readiness parsing; all panic paths kill and wait via
Drop, including startup timeout and malformed readiness. No extra process is
started to verify this locally. Direct rustfmt and whitespace check passed; this
changed integration fixture is NOT_RUN and not covered by the pure-lib result.
Audit confirms shared sealed startup owner is being authored, not yet available.

## Explicit Apache/ASP.NET Core delta

Root assigned exact profile names after shared enum landed; actual disk variants
verified. Config/readiness now includes apache/aspnetcore with all absent-plane
refusals and seven-profile isolation. Added one pure test for config-selected
wire identity/mode binding; existing hosted native test now has five normalized
producer iterations (still one test, NOT_RUN). Native client/IPC/scoring unchanged.
Direct rustfmt and owned whitespace checks passed. Cargo is allocated to wargame
and no command was started. Earlier authority10pure result does not cover this
changed configuration or new test; revalidation required. No blanket producer
inference or new Go producer was added.

## Shared SealedLog trait integration

Read actual audit29ee901 exported SealedLog API and append signature. Added
DurableAudit impl in authority decision.rs; delegates append.map to unit without
catch/fallback. Runtime can replace its separate wrapper with shared owner.
No manifest/lock/crypto/OS reader duplication. Direct rustfmt and whitespace
check passed; Cargo slot not assigned and trait compilation/hosted lifecycle
remain NOT_RUN. Authority binary inline-key startup remains explicitly unmigrated.
