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

## User-requested sealed executable continuation

Re-read latest root AGENTS, Rust conventions and existing implement-sot-slice.
Actual authority-ipc claim verified. Root approved explicit inherited lab mode
versus protected OS service store; implementation follows the contract above.
Added startup module delegating custody/WAL/unlock/witness to shared SealedLog,
main opens audit before IPC and uses Zeroizing raw input. Added two pure source
selection/witness refusal tests and one hosted actual sealed startup fixture:
parent provisions fixture keys, two child generations verify live custody
exclusion, post-exit release and unchanged sealed key bytes. Fixture is one test,
not two successes; no interpreter/framework or full product claim.

Initial rustfmt encountered Windows mapped-file OS1224, retried narrow files
with skip_children after no process termination; later format/diff passed.
Cargo and actual IPC execution NOT_RUN awaiting root A/B slot and hosted source
closure. No new dependency/manifest/shared lock change. Latest goal tool still
returns blocked despite user resume; API has no resume operation, no replacement
goal or false completion issued. Implementation continues under user direction.

Hosted sealed fixture additionally authors bounded wrong-unlock and missing
witness subprocess refusals, requiring nonzero exit and zero readiness stdout.
Both retain identical sealed key bytes. This is source-only coverage, NOT_RUN.
The fixture ends child processes by kill+wait and proves ownership release; it
does not claim graceful service shutdown/flush acknowledgement. Existing explicit
serve_handler_until drain tests remain separate. Root slot queue still owns
local compile allocation and hosted source dispatch; no duplicate run started.

## Root-assigned A local verification

Actual locked lib13passed/0failed/ignored/filtered; all five lib/bin/integration
harnesses compiled with --tests --no-run, never executed. All-target Clippy
-Dwarnings passed, all exits0, jobs1/incremental0. Raw logs recovered and attached
with before109input hashes/results under private evidence20261107053238.
Authority source40d0bdd stable. Three audit inputs changed concurrently across
commands (control_operations/custody/rotation), so combined immutable graph is
NOT qualified. Retain exact before/after hashes and HEAD27d5bbc to66ed85c rather
than retroactively assigning results to latest source. Root notified; A returned
immediately. Native IPC/custody/process tests remain NOT_RUN, latest hosted result
42348518353/63f69ae failed and does not cover these changes. Public sanitized
sealed-local-results.json records this distinction.

## Audit drift revalidation boundary

Inspected actual diff against pre-run27d5bbc. custody.rs changes read_envelope
used by load_or_create/read in SealedLog startup: revalidate frozen-source
compile plus hosted unlock/wrong-unlock, custody lifetime/release and restart.
rotation.rs is an audit library compile dependency, but authority startup does
not execute rotation; dual-control/recovery tests remain audit-owned.
control_operations.rs is audit-test-only, outside authority test denominator.
Local13passing observed runs remain evidence, not final frozen-graph qualification.
Public sealed-revalidation-scope.json records exact current hashes and required
gates, without assigning execution to source-only requirements. Root is preparing
a fixed hosted candidate; no duplicate dispatch or local runtime execution.

## First fixed-candidate hosted observation

Remote branch now points to a26d1c3f08b365ca93545c6a51343095868edb45, which
contains audit20b57d2 and the same eight authority source blobs as40d0bdd.
Actual run42354873001/attempt1 (CI Validate, ci.yml, push) completed with
startup_failure and jobs.total_count0. Authority executed0tests: sealed startup,
IPC, cancellation/drain and otherUID all remain NOT_RUN, not failed test cases.
Full run/jobs metadata recovered privately and hash-linked to sanitized public
hosted-42354873001.json. CI owner reports reusable caller permission-chain issue
and is fixing owned workflow/policy; authority does not bypass or duplicate dispatch.

Public spec rechecks observed concurrent other-owner attachment windows: initially
two wargame P2 registrations, then CLI summary E1. Own attachments remain valid;
no foreign evidence mutation or waiver. Root confirms new failure cause is the
ci caller actions:none ceiling versus nested actions:read. CI owner implements
minimal caller permission fix/regression; root remains sole push/dispatch owner.

## Corrected candidate8132117 actual execution (partial)

Run42355168729 passed policy and reached real Linux native actor and Docker.
Linux workspace --all-targets compilation passed on the frozen graph. Actor
failed strict publication after15of88harnesses: observed51passed tests,0failed
tests,1actor error. Unique authority decisions target observed7passed/0failed;
this is synthetic/pure decision coverage, not production capture or sealed IPC.
Last host_process row had2passed/3ignored/exit0/validatedfalse. Source inspection
and Node owner confirm these3ignores are actual Node/Bun host pilots, NOT helper
subprocesses. Runner/Node/CI owners notified; do not relabel or waive them.
Authority library/native_process/local_service were not reached. Sealed startup,
normalized native producer assertions, cancellation/drain and actual UID denial
therefore remain NOT_RUN in Linux. macOS/Windows results pending at this record.

Recovered full public job logs privately. Docker public summary interleaved one
stderr diagnostic line, so raw extract is invalid JSON; retain it unchanged and
separately label the derived summary that removes that single diagnostic. It is
not the private owning report. Docker failed/full_product_qualified=false and
no authority otherUID receipt was observed. Sanitized partial result is attached;
original raw execution report remains unrecovered under branch no-upload policy.
Initial gh log retrieval refused escape sequences; explicit file-only retrieval
succeeded without displaying untrusted control sequences. No local runtime run.

Audit owner additionally committed future sourcea04603e (not pushed into813):
recover_erasure_pending gains trusted Scope and rejects foreign-scope permit
before I/O; no authority/runtime callsites exist. This requires next-candidate
audit compilation plus audit-owned regression, not retrospective qualification
from813control tests or local13. Direct sealed read_envelope revalidation remains
separate. No new authority source change or local Cargo allocation was inferred.

## Ignored qualification classification correction

Ordinary Linux authority native_process has two ignored entries: required
privileged Docker parent unix_other_user_is_denied_by_actual_socket_acl and its
child helper unix_unprivileged_connect_probe. Earlier shorthand '2helpers' was
imprecise. Preserve original measured22passed/2ignored plus separately executed
parent1 at e346834; do not alter historic raw files/hashes. The ordinary parent
is required_not_run until an actual isolated Docker parent receipt proves child
UID65534/PermissionDenied. Only the child is a subprocess helper. Runner/CI
owners were notified not to waive/reclassify the parent. Current813 actor will
otherwise reject ignored authority coverage when it eventually reaches this
harness; that is incomplete qualification, not failure of executed sealed cases.

## User-authorized local comparison and benchmark quiet window

The user explicitly authorized actual local Windows/WSL validation for measured
comparison with hosted execution, superseding the former blanket hosted-only
restriction. Root declared a benchmark quiet window: no new Cargo, native pilot,
runtime, model or Docker command until an explicit allocation follows measurement.
Authority owns no local validation process, Cargo slot, model/GPU server or Docker
container. Existing lightweight remote status watcher session29096 remains the
only owned operational process, observing run42355168729 without a new dispatch.
Do not infer local execution from this authorization; no such pilot has run.
Shared execution policy remains root-owned and is not redefined by this plan.
Local comparison receipts will remain labelled local; they do not establish the
explicit hosted execution requirements of this authority goal.

## Independent Linux hosted execution in the MSRV job

Exact source8132117, run42355168729, successful job128359709010 executed
`cargo test --workspace --all-targets --locked` after the native actor failures.
Recovered raw job log shows authority lib13/main0/decisions7/local_service3/
native_process7 passed, total30passed/0failed/2ignored/0filtered. The two ignored
entries are the required privileged Docker parent and its child helper, not two
validated helper successes. Named sealed_startup_retains_custody_and_reuses_existing_identity,
normalized_producers_are_bound_by_native_authority_credentials, real IPC/replay,
capacity, native server checks and explicit cancellation/drain tests all show ok.
This independently closes the observed Linux sealed read/unlock/guard/restart
revalidation for audit20b57d2; it does not retroactively change the failed native
actor steps. Windows/macOS sealed IPC remains unexecuted in those actor steps.

The source-controlled parent passes an inherited fixture secret through private
pipes; this is not OS-store attestation or independent witness qualification.
Sealed process termination is kill/reap, not graceful service flush; generic
explicit drain tests are separate. Strict workspace/full product qualification
is still false. Future audit a04603e is outside this source and needs its own
compile/regression receipts. Raw log and derived authority extract are attached
privately; public hosted-42355168729-linux-authority.json lists exact named
results. Extraction first failed on ANSI-prefixed Cargo headers; the corrected
derived extract explicitly strips timestamps/ANSI CSI while preserving raw bytes.

Node owner confirmed that exact813 job128361689495 executes native build and
inspection but reaches the missing qualification.json gate before any host-test
execution. Those successful component commands do not establish actual Node/Bun
framework/authority qualification. No replacement qualification receipt is minted.
Global quality job128360847587 fails audit retention.rs items-after-test-module;
audit owner already has a separately recorded future fix c34c3e91 without lint
allowance. That fix is not in813, so this quality result stays failed.

## Terminal hosted run and remaining authority work

Run42355168729/attempt1/source8132117 completed failure: actual API inventory
19jobs,5success/14failure. Ready failed. Status watcher29096 exited1 and was reaped;
no authority-owned Cargo/runtime/Docker/model process or server remains. Final
run/jobs metadata and status transcript are retained privately with public hash
links. The independent successful Linux MSRV authority30passed/2ignored receipt
remains valid for this frozen source; no actor, product or whole-run green follows.

Pending: isolated Docker UID parent/probe proof; Windows/macOS sealed/native IPC;
protected OS-store authority unlock; independent remote Windows denial; accepted
typed one-shot private broker provisioning; and integration after post813 audit
fixes. Current authority source had no failing assertion to correct in the observed
Linux run. Foreign-scope CI/runner/audit failures were routed to their actual owners.
Local actual validation is user-authorized but remains unexecuted during root's
benchmark quiet window. Preserve the full goal; this checkpoint is not completion.

Root's new Docker canon selects Windows Docker Desktop desktop-linux explicitly
through the Windows docker.exe path. Historical local authority Docker evidence
used the different WSL-native29.1.3 engine, not Desktop29.9.2; preserve its original
identity and results without relabelling. No automatic WSL fallback is allowed.
Future local Docker needs context endpoint/server ID/version/OSType preflight and
an allocated window. Hosted Linux runner Docker remains a separate profile.
No Docker command was started during this quiet window.

## Current-source audit revalidation and bootstrap review checkpoint

The resumed authority owner verified its existing authority-ipc claim and current
AGENTS/rule94. No idle slot is treated as an allocation. Root was asked for the
current local/hosted execution assignment; no Cargo, IPC, SCM or Docker execution
was started by this checkpoint. This is source review, not a passing gate.

The historical hosted baseline remains source8132117, run42355168729 attempt1:
Linux authority30passed/2ignored; whole run19jobs,5success/14failure. Windows/macOS
actor steps did not reach sealed/native authority harnesses. No new hosted
candidate or authority job was allocated in this owner at this checkpoint.

Compared with8132117, current audit production changes include control ledger,
policy, retention and new state/retained-backup/erasure modules; custody now binds
pending erasure recovery to trusted Scope, and vault uses that shared predicate.
Authority startup does not call recovery/retirement control operations. They are
compiled dependencies and require fixed-source compile/Clippy plus audit-owner
control/recovery/retirement tests; authority startup alone cannot qualify them.
The WAL delta is inside its tests: bounded actual child capture, explicit ignored
helper invocation and required refusal marker. It requires audit-owned parent
execution; an ignored/no-op helper is not a standalone passing case. Cargo.toml
and Cargo.lock also differ from8132117, so a fresh complete dependency closure
must be captured before and after validation; unchanged authority Rust files do
not make the current graph identical to the historical passing graph.

Required authority revalidation on the frozen current graph: locked compile and
all-target no-run, pure library tests, scoped fmt/Clippy; then allocated actual
native sealed startup, wrong unlock/missing witness refusal, custody exclusion and
release, restart key-byte identity, native peer/credential/replay/body/capacity and
normalized producer IPC cases. Keep actual OS results separate. Windows/macOS
sealed IPC, protected OS-store unlock, independent remote Windows denial and the
isolated Docker UID parent/probe remain NOT_RUN here. Whole product Docker e2e,
SCM cross-principal bootstrap and actual graceful sealed S flush/stop remain
separate unqualified gates. Forced child kill/reap is not graceful sealed flush.

Bootstrap draft now reflects root-approved R2 two own-process SCM roles under
distinct service principals and accepted R3 irreversible first-loss semantics.
observer-status grants no service stop or management mutation; independently
verified native lifecycle authority is required. O retains W through actual S
flush/instance-stop proof before its own cleanup. No universal24hour lease default
or renewal policy is accepted. Native R1 noncircular grant publisher/discovery,
both process-instance/principal pins and cross-SID access remain P0 pending an
exact native-security backend and independent review. Typed activation/admission
and bounded join implementation remain unqualified. No secret channel source,
manifest change or local account/SCM mutation is authorized by this draft alone.

Root subsequently reserved local slot C: target/validation-authority, jobs1,
incremental0, start ONLY after explicit root manifest-window-closed notification.
Allowed: all-target compile/no-run, reviewed pure13 library tests and Clippy.
Excluded: actual network/process IPC, account/SCM/privileged profiles. Reviewed
pure inventory: configuration3, credentials2, client3, protocol1, transport2,
startup2. Framing uses byte slices; missing-witness startup refuses before secret
or filesystem activation. This reservation is not an execution result. A later
actual IPC/hosted candidate allocation remains required.
CLI independently reviewed draft SHA256
b1e9cd5d51b8e37e26350ec00b37690b9f477299fd167a9088e7be4e3b685f14:
no remaining topology/permission objection in these corrections; overall verdict
still needs-work for R1 backend and actual typed bounded R3 implementation/tests.
This is consumer alignment, not channel implementation approval or native proof.

## Current Windows C execution result

After root explicitly closed the manifest window at e83d5e55, C admitted at
commit79percent/free33.26GiB. Actual local rustc1.99.1 Windows x86_64 compiled all
five authority harnesses (--all-targets --no-run), passed13pure library tests
(0failed/ignored/filtered), and passed all-target Clippy -Dwarnings. Scoped
cargo fmt -p humanymous-authority --check passed. The initial erroneous --all fmt
invocation inspected foreign workspace packages and exited1; raw output is
preserved separately, with no foreign source edits or whole-workspace fmt claim.

Real locked/offline Windows cargo tree identified nine local package dependencies.
All217 tracked closure inputs (including Cargo/lock/toolchain) were unchanged
before/after. The broader533-input observation changed only an unrelated .NET
package test outside that closure. Source at start332d79c0; this checkpoint does
not assert a clean whole workspace. Public current-windows-pure.json records
exact scope and raw hashes; private snapshots/logs are attached to execution
evidence. No authority Cargo/process remains running after terminal collection.

This closes current graph compile/pure/Clippy/fmt revalidation only. Actual sealed
startup with custody/restart and native IPC integration harnesses were compiled,
NOT executed. No new hosted result exists here. Root was asked for a separate
actual IPC/hosted allocation; historical Linux813 results remain source-bound.

## Renewed C actual Windows IPC result and R1 review

Root explicitly renewed C for warm ordinary owned Windows process/pipe tests.
At commit79percent/free33.4GiB, directly executed the already-built exact harness
EXEs after reviewing actual --list and prerequisites: native_process10/10 and
local_service3/3 passed, both0failed/ignored/filtered, exits0,11.41s/0.18s. No Cargo
or cold compilation, no account/SCM/privilege change, no hosted-marker spoofing.
All217 tracked compiled closure inputs stayed identical through execution. Actual
local actor SID and three binary hashes are private; exact lists, terminal footers
and post-run process observation are attached. No validation-target process remains.
Public current-windows-ipc.json binds this local proof and its limits.

Sealed custody/exclusion/release/restart identity and wrong-unlock/missing-witness
refusal now have actual CURRENT Windows component proof. This uses controlled
inherited provisioning, not OS-store attestation or production R1 bootstrap.
Child termination is kill/reap, not graceful sealed service final flush. Generic
local handler stop/drain has its own3-case proof; never merge these claims.
Fresh hosted candidate IPC, current macOS, OS-store unlock, remote Windows denial,
isolated Docker UID parent/probe, SCM/bootstrap and whole product remain NOT_RUN.

Independently reviewed native R1 plan20261107103828 exact SHA256
6b97bda59859527a7e9735f93bca34b68ab215514afba5ddc64faf99bf2b8778:
NEEDS-WORK. Public signed targeted authorization could explicitly replace initial
bearer Hello, subject to root decision; it is not a protocol fallback. Proposed
SCM_RUNNING TransportPrepared also needs approval and real cross-SID primary-token
query feasibility remains P0. Added requirements: authenticated typed P acceptance
from BOTH live roles without making P management peer; one-use burn across repeated
context constructors/process-wide concurrency; trusted clock/rollback and conserved
absolute startup deadline; independent W key/capability/lifetime proof before
unseal. Sent critique to native owner/root. No R1 channel implementation follows.

## Root signed-grant and TransportPrepared architecture decisions

Root accepted signed PUBLIC targeted authorization as the replacement for initial
shared-secret Hello, requiring a NEW explicit protocol version/domain and no
fallback. Observer management credential remains secret; BOTH actual native
instance proofs remain mandatory. Root also accepted TransportPrepared:
SCM_RUNNING is native process/discovery readiness ONLY, while AppReady/ingress/
unseal/writer remain false, independent absolute startup watchdog and Stop handler
armed. Existing Running/publicReady publisher cannot be reused early; all status,
doctor and installation consumers must preserve these separate facts.

Authority updated its draft to v0.2 with proposed exact protocol
humanymous.observer-bootstrap/0.2, domain
humanymous.observer-bootstrap.signed-grant/1 and magic HMBP0002. These exact bytes
and canonical signed-record encoding require review; architecture acceptance is
not wire-format qualification. Reject legacy HMBP0001/bearer Hello/mixed version
and no fallback. Native opaque grant types remain the proof boundary.

Remaining P0s are explicit: actual cross-SID primary TokenUser/retained handle
query rights without SeDebug/admin fallback; create-new installation actual
SCM/root-owner binding and existing-key no-reset; authenticated typed P acceptance
without management authority; process-wide one-claim/clock rollback/conserved
absolute deadline; actual loaded-artifact binding; atomic release/epoch gate;
concrete unattended restart/republication policy. P's account-private WinCred
availability after logout/restart is NOT assumed. Linux/macOS backends remain
required/open. No universal24hour policy or implicit renewal accepted.

Authorization now covers SPEC and native primitive probe preparation only.
Native-security owns reviewed safe primitive/API/feature gap reporting; no local
SCM/account mutation, channel implementation, manifest edge or new execution was
started by this authority checkpoint. Existing Windows IPC13 receipt remains
local component proof and does not qualify this new production architecture.

## CLI wire-bound review reflected

CLI reviewed exact6171afd6 and had no objection to proposed /0.2/HMBP0002/domain
names and hard legacy/mixed refusal, without approving R1/channel implementation.
Reflected its required delta: FULL signed authorization including signature and
encoding overhead fits inside Hello3072byte metadata, no extra/unbounded grant
attachment; phase table permits exactly64lowerhex secret bytes ONLY in Offer,
all other phases zero. Fixed header12bytes and phase slot determine exact length,
subject also to frame4096/startup32768 limits. Reject lengths before allocation or
secret release, unknown phase and trailing bytes; subsequent frames require an
admitted state transition. Native owner still must freeze exact domain separator/
length, canonical field order/encoding/numeric rules, signature/fingerprint bytes
and duplicate rejection. Domain naming is not signed-byte contract completion.
No implementation or new execution in this source-only review correction.

## Independent native v0.2 review

Read exact native R1 plan SHA256
93d73d7d7003e8f27ab3382c85721835bedaa2301c9ff63f52c5ad4365e8675d.
P's separate native-authenticated nonsecret acceptance listener and Prepared /
both claims / signed Commit phases now address earlier acceptance-path ambiguity
at design level. OS-backed actual-instance claim marker addresses repeat-context
burn bypass in the specification; conserved monotonic/wall budgets and typed W
prerequisite are explicit. None of this is actual native proof or code approval.
Added required seam: both signed Prepared and Commit records plus linkage and
encoding fit the SINGLE Hello3072metadata cap; distinct signed record-kind/domain
prevents Prepared-as-Commit, wrong-kind and cross-grant substitution. Exact format,
actual native rights/owner/image/time and unattended publisher policy remain open.
Post-Commit uncertainty must not be called revocation or trigger automatic retry.
Critique sent to native owner/root; no implementation or execution started.

## Independent native v0.3 exact-format review

Reviewed frozen native R1 plan SHA256
c934d71a23546eed83f4e597e3be6817b9aeed29c5b9bf902208cae0218bd8f6.
Independent arithmetic confirms Prepared body652/artifact726 and Commit
body169/artifact243; full lowercase hex1938 plus grant overhead128 and other
Hello896 totals2962, below3072. Separate exact signature domains, prepared artifact
commitment including signature, repeated grant/key/epoch bindings and strict EOF
address the prior record-kind/linkage concern at specification level. No new
blocker found in the listed binary field ordering; this is not cryptographic code
execution, test-vector qualification or acceptance of native identity.

Required before format freeze: normative exact Hello DTO/format literal and strict
unknown/duplicate fields, every non-grant JSON syntax byte included in896; golden
byte/signature/commitment vectors for min/max names/SIDs and numeric boundaries;
mutations of each bound field, Prepared-as-Commit, cross-grant records, trailing
bytes, uppercase hex and oversize/predecode refusal. Overall NEEDS-WORK remains:
actual native query/owner proof, loaded-image limits, unattended publisher policy,
independent W and atomic epoch/secret gate. Sent exact review to root/native owner.
No new source implementation, feature edge or runtime execution was performed.

## One-shot unattended publisher architecture direction

Root accepted a finite one-shot registered Windows P publisher service, alongside
persistent S/O: three registered roles, two persistent processes. Original
installer/controller authorization and stable publisher service principal are
DISTINCT. Protected original installer/root/SCM-owner record binds exact publisher
principal/artifact/original public key; the old interactive-P==root-owner premise
cannot be reused unchanged. No self-issued key trust/reset or production key
creation. P may start/wait exact S/O TransportPrepared then publish; no dependency
cycle where S/O waits for P stop while P waits for S/O acceptance. Concrete boot/
recovery orchestration, fresh instances/grants, conserved deadlines and failure
cleanup remain required. Service-principal store access over stop/start/reboot
needs actual genuine hosted probe; no interactive-profile/machine-DPAPI fallback.

Authority reflected this architecture direction only. Native-security owns the
narrow safe API/query/retained-handle/Prepared-discovery/test-credential probe;
report dependency/feature gaps before root manifest integration. No local SCM or
account action, production credential, full issuer/channel implementation or
additional runtime test was performed. Full implementation still requires accepted
API, exact golden vectors and actual probe proof; remaining P0s are not waived.
