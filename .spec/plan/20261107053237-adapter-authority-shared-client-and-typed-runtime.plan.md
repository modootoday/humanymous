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

## Native v0.4 nonfrozen Hello and probe review

Read exact nonfrozen draft SHA256
a98c20f2d0a2d96b3672a662d64a8492834511a4161a16ea99371b83b09ff0f6.
Independently constructed its listed compact ASCII Hello fields and counted
2653bytes:1938 artifact hex +83 grant JSON syntax +632 outside. Authority has no
objection to the closed ASCII/key-order/re-encode wire direction. This is NOT a
format freeze, golden-vector execution or channel implementation approval.

New probe requirement: the ordinary fixture controller must be named I/T, distinct
from ACTUAL publisher service P. Cross-SID query/pipe edges must execute under the
actual P/S/O service tokens with per-edge privilege, expected-role, native query,
ACL denial and cleanup facts. Ordinary controller/admin access cannot substitute
for service P access. Forwarded this ambiguity to native owner/root. The new
LaunchTicket/cursor and changed role_plan_digest binding are separate unresolved
format/replay/atomic-publication proposals; Hello agreement does not approve them.
Safe feature/dependency integration, cross-instance TEST credential handoff and
actual native probe gates remain. No source implementation or new execution.

## Shared observer epoch ownership proposal requested by Runtime

Runtime found no production verified epoch/revocation source and correctly refused
to treat ShutdownRequest/watch or check-then-async-write as proof. Authority B06 now
proposes owning the neutral gate core (Runtime already depends on Authority; no
reverse dependency), consuming native opaque verified epoch and separate W once.
Unique owner retains pins/monitor cancellation+join; restricted clones share the
SAME gate for secret release, Ready and origin effects. Drop synchronously revokes
but async drain/flush/join remain separate unknown facts. Bounded nonblocking
actual acceptance serializes with first loss; WouldBlock waits outside and rechecks.
No lock over await/token query/fsync, no public live/from-ID/activate(bool) factory.

Exact production native constructor, sealed Runtime readiness-evidence adapter
and concrete synchronous effect return types remain P0 joint review, not code.
Proposal SHA2566ebd5cbcbb65befade9da1bdfe92bbbda3f7fab0576482d36cbd3a7372292645
was sent to root/native/runtime for independent review. Required race/Drop/partial
write/blocked-worker negative gates are listed. No channel/gate implementation,
feature edge or runtime execution has been performed in this checkpoint.

## Concrete independent expiry slice and Ready interface decision request

Literal authority Rust was unchanged before this slice; historical frozen hosted
813 and current local217-input/13pure+13native receipts do not cover new source.
Authored private BoundResponse::consume_at(wall, monotonic), called by existing
public consume using real clocks, with no new public constructor/proof. Added
one deterministic independent authority expiry test: request deadline6000 and
live monotonic budget cannot extend earlier authority expiry5000;4999 accepts,
5000/5001 refuse authority_expired. No sleeps, production epoch or parent trust
fabricated. Scoped fmt passed; compile/unit/Clippy NOT_RUN pending allocation.

Concrete Ready interface decision sent to root/runtime: proposed Authority
announce_ready(RuntimeActivationEvidence) cannot seal a Runtime-private proof
without reverse dependency or a forgeable external trait. Recommend moving Ready
COMPOSITION into Runtime. Native-security must produce opaque AcceptedObserverEpoch
with actual pins and owned cancellation/join; Authority consumes it plus verified W
into PreparedObserverSession with receipt, same EffectGate and restricted bounded
transport. Runtime-private ServiceHost::activate_observed(self, pending) consumes
actual bound listener/capture/TLS/audit resources into RunningObservedService;
Runtime privately creates RuntimeReadyReceipt and emits ServiceReady under the
same gate, never passes that proof back into Authority. No public activate(bool),
parentUnverified conversion or callback identity proof. Exact transport ownership,
phase and native producer constructors require joint API decision before code.
This is a concrete proposed interface dependency, not an implemented proof source.

## Independent expiry actual warm validation and accepted Ready direction

Root accepted Ready composition in Runtime as architecture direction only. Removed
the draft Authority announce_ready(RuntimeActivationEvidence) reverse-proof API;
exact consuming session/transport/failure-retention and race types remain pending
joint acceptance. Native AcceptedObserverEpoch producer is not approved or present.

Root allocated A warm pure-only validation. At commit81percent/free31.92GiB,
locked/offline jobs1/inc0 target/validation-authority ran library14passed,
0failed/ignored/filtered, all-target check0, strict Clippy0, scoped fmt0. Independent
expiry named case passed. Source is literal base HEAD plus client.rs overlay
SHA2560e7c0d7ede66b79ce018143df79ea9f15a053c14a333ae7328f38e03b72fcd4c.
Real tree and resolved metadata bound95packages and4590source/registry/config
inputs, all unchanged before/after. Raw patch/snapshots/logs attached privately;
public independent-expiry.json binds scope and hashes. Warm Rust compilation
9.61s; no SDK/native cold graph, no actual handler/native/SCM/account execution.
Slot returned terminal; no owned validation process remains. Historical IPC proof
is not silently extended to this new client source; fresh hosted remains NOT_RUN.

## Runtime pre-unseal and failure-owner review reflected

Runtime identified that existing ServiceHost::bind opens/unseals audit writer
before activation; it cannot be reused as unverified TransportPrepared. Corrected
the earlier proposed input phases: VerifiedObserverPreparation carries native
publication/W/same gate/transport BEFORE unseal, with NO credential receipt yet.
Runtime private prepare_observed consumes that first. One ObservedStartupOwner
then retains resources through unseal, Offer and actual receipt; the later
PreparedObserverSession is receipt-bound. Requiring that later receipt before
unseal would create an invalid ACK cycle, so it is explicitly prohibited.

activate_observed consumes the same owner. Failure must own host/transport/secret/
pins/W/monitor, stage and ReadyDelivery(NotSubmitted/Partial/Submitted/Unknown),
revoke immediately and expose explicit bounded close/join/drain/flush facts.
Drop cannot certify those completions. Complete Ready enqueue and Active transition
serialize under the same loss gate; partial Ready permits no origin, resend or
watchdog reset. Historical Ready receipt cannot restore a revoked epoch. Sent
correction to root/runtime. No Rust change or new execution;689b95d2 pure14 receipt
is unchanged and does not prove these not-yet-accepted production APIs.

## Concrete sealed startup Rust ownership slice

Root requested concrete Rust advancement without a fabricated native producer.
Authored startup::observer in observer.rs/state.rs/tests.rs, exported by startup.rs.
Public VerifiedObserverPreparation, PreparedObserverSession and failure retain a
PRIVATE EMPTY NativeProof: uninhabitable even in tests, no Clone/Default/serde or
public factory. native_preparation returns Unsupported ONLY. Both phase types
expose restricted effect_gate inspection and consuming cancel(cause), producing
ObserverStartupFailure retaining the unique Authority guard and stage/delivery.
No production receipt transition, bounded transport implementation, activation,
Ready or origin-effect API is invented. Runtime retains its sole generic resource
owner R with an enum of these opaque Authority guards; Authority has no competing
resource owner or reverse dependency.

Internal Owner is unique; its Drop synchronously revokes all restricted clones.
First monotonic cause/instant stays immutable and close deadline uses min(original,
firstloss+drain). Public snapshots are diagnostics, never native proof or permission
followed by async effects. Private cfg(test) transitions model NativePrepared ->
WriterPrepared -> OfferPending -> ReceiptBound -> ReadyEnqueuePending -> Active;
terminal revocation cannot advance. That internal model NEVER creates NativeProof.
Seven authored cases cover Unsupported, ordered phases, illegal early ACK/Ready,
loss at every phase/first cause, retained clones on Drop, bounded close deadline,
and Ready-versus-loss serialization. Tests/compile/Clippy NOT_RUN; scoped fmt and
diff checks passed. Sent exact module/API checkpoint to Runtime/root for review
and requested an explicit warm pure21 allocation; no execution assumed.

Actual native/W constructors, owned monitor cancellation/join, concrete bounded
transport/receipt producer, atomic kernel effects and Runtime-private Ready remain
open. Existing expiry14 proof does not cover this newly authored source, and no
native/hosted/product qualification is claimed by the internal state model.

Root read observer.rs/state.rs and accepted this uninhabited/Unsupported/restricted
gate/consuming-cancel direction. Freeze checkpoint before explicit A warm pure21,
all-target check, strict Clippy and fmt allocation. No native/handler/service,
no new Cargo edge or production/test constructor is authorized.

## Frozen sealed ownership actual validation and Runtime API handoff

Authority source frozen/committed fecdb4d1 BEFORE execution. Root A warm admission
commit81percent/free31.56GiB, jobs1/inc0/offline/locked exclusive validation-authority.
Actual library21passed (7new internal cases),0failed/ignored/filtered; all-target
check0, strict Clippy0 and scoped fmt0. All4601 captured source/registry/config/tool
inputs across95resolved packages unchanged before/after. This binds the captured
dependency graph, not a clean whole-workspace assertion. Public
sealed-observer-ownership.json and hashed private metadata/snapshots/logs attached.
Slot terminal/returned; no owned validation process or actual IPC/native service.

Concrete source API checkpoint handed to Runtime: startup::observer opaque
VerifiedObserverPreparation and PreparedObserverSession, effect_gate diagnostic
clone, consuming cancel -> ObserverStartupFailure with retained unique Owner and
stage/delivery. native_preparation is Unsupported-only; private EMPTY NativeProof
has NO test constructor. Runtime confirmed sole ownership of its generic R and
AuthorityGuard enum; no reverse edge or second production gate. Actual receipt/
bounded transport/native monitor/atomic kernel effects/private Ready remain absent.
The21pure result is not production startup, actual native or hosted qualification.

## CA-SECRET-01 encoded inline-key lifetime source remediation

Root approved narrow source fix after independent audit triage. Audit configuration
hmac_key_hex/signing_seed_hex now own Zeroizing<String> from deserialization;
startup directly takes those wrappers after unchanged source/witness/private-dir
preflight, rather than creating nested wrappers. Retention before preconditions
still preserves the actual source and refuses mixed/required-witness inline mode.
No inline production promotion, secret reader/provider action or new dependency.
Audit remains Deserialize only, no Debug/Serialize. Public JSON string fields stay
compatible. Direct Rust field constructors must use Zeroizing<String>; repository
Rust search found consumers via JSON fixtures (.NET/Apache and authority tests),
no external direct Authority Audit literal/field mutation requiring foreign edits.

One authored regression covers invalid inline required-witness preflight, retained
zeroizing field types/bytes and subsequent same-allocation wrapper move with empty
default fields. No freed-memory inspection, native store read or RAM-erasure claim.
Caller original JSON, serde intermediates and transport/provider/allocator/kernel
copies remain separate lifetimes, explicitly recorded in owning SoT. Earlier21pure
proof does not cover this change. Expected22pure tests, compile/Clippy NOT_RUN;
scoped fmt/diff checks passed. No Cargo/native/runtime allocation assumed and
Performance target reservations remain honored. Raw encoded-key finding is P2
ownership debt, not observed exfiltration/log/provider exploit.

## Encoded-key first current-graph validation failure

Root admitted warm validation-authority at commit81percent/free32.86GiB, jobs1/
inc0/locked/offline. Authority source2ca6e4fa, observed HEAD5ea99fa9 with captured
96packages/4631source-registry-config-tool inputs, ZERO before/after drift. Library
command exited101 BEFORE any tests: Network correlation/registry.rs:3 and traffic/
consistency.rs:3 import absent signals::{Signal,Source,Verdict}. No Authority
diagnostic observed. Expected22tests remain NOT_RUN, package check/Clippy NOT_RUN;
fmt precheck passed. Preserve first raw log and closure; public
encoded-key-first-failure.json binds the exact failure. No native/store/account/
SCM execution. Reservation returned terminal, no owned process. Root and Network
owner received the concrete source dependency blocker; no foreign edit or guessed
export/overlay fallback. Corrected dependency source must be frozen and separately
revalidated; historical21pure cannot close CA-SECRET-01's validation gate.

## Encoded-key corrected-source retry terminal success

Root authorized retry of the SAME batch after Network canonical imports0429731c
and frozen doc-only successor306eedc8. No new approval loop, foreign edits or
signals export fallback. New admission81percent/free32.91GiB; Authority2ca6e4fa and
Network306eedc8 plus captured actual graph. Library22passed0failed/ignored/filtered,
all-target check0, strict Clippy0 and scoped fmt0. All4632source/registry/config/tool
inputs across96packages stayed unchanged. Public encoded-key-retry.json binds
exact source/head/raw hashes; private snapshots/logs attached. Prior failed
compile101/tests0 remains separately preserved in encoded-key-first-failure.json
and d3e03749. This closes CA-SECRET-01's scoped pure/check/lint validation gate;
no all-copy/RAM erasure claim. Native/store/IPC/SCM/account/producer activation and
new-source hosted/product verification remain NOT_RUN. Target returned terminal,
no owned validation process. Root notified before evidence commit for CI handoff.

## CA-HANDOFF L3/L4 source-only final-consume contract

Root assigned same-role source refinement after22terminal, no repeat workload.
Shared with Framework owner: existing BoundResponse API is sufficient for final
consumption; hold its actual non-Clone result across all awaits/host queues/GIL/GVL
and consume at actual trusted app invocation, no later await. verdict inspection
or an early copied status200 is not a live grant; Pending::decide is repeatable
and consumer must not re-decide/re-encode/recreate a bypass. check_handoff is only
local budget, never atomic native-epoch proof. Reviewed actual ASGI early consume
before tail poll/replay/call, Ruby before allocation/liveness and Node worker
status conversion; sent precise call sites to sole L4 writer, no foreign edits.
Required late-connect/queued expiry and cancellation tests must observe NO origin
invocation. Native same-gate effect authority remains missing, not filled by this
protocol0.1 contract. Owning SoT refined; no Rust behavior change/new API/workload.

## Root-assigned missing public publisher authorization kernel predecessor

Root assigned a concrete parallel R1 predecessor, not another blocker report.
Applied design-sot/panel requirements and inspected native draft1ee9095d, Runtime
R1 and actual crypto APIs. Authority has no direct dalek/base64 edge; Audit verify
is checkpoint-specific/private. Proposed Root integration of existing workspace
Ed25519/base64 only, no new external version or foreign API reuse. No graph edit.

Frozen draft Authority SoT SHA256
93c2c3373afba12744f9474344352e42159e17a9c11e349dec579f549a4103b6 adds
exact kernel v2: I-signed explicit P delegation and pair-bound P Prepared/Commit,
new per-kind domains/version, strict binary fields, complete-record commitments,
actual P/S/O claims, nonce/key/artifact/BASE-plan/expiry binding, no cursor fallback.
Maximum artifacts468/786/275; hex would overflow earlier Hello budget, so proposed
NEW /0.3 HMBP0003 authorization/2 compact canonical base64url Hello includes ALL3
records inside2771bytes/3072cap. Independent arithmetic performed, not Rust codec
execution or golden-vector qualification. Earlier /1 and /0.2 remain historical
proposals, not downgrade fallbacks. Windows SID profile is explicit; other actual
OS principal/provider backends remain required OPEN work.

Proposed Rust parsed data versus SignatureCheckedAuthorization are PUBLIC DATA
only, never NativeProof/Ready/healthy/management capability. Native owns original
protected I trust/P delegation/store, actual retained instances/cross-SID rights,
process-wide one-claim, independent W and cancel/join/close evidence. Clock data
is not a trusted clock proof. Signed acceptance mask is a publisher assertion,
not actual native ACK proof. Restart/no-key-reset/uncertain-Commit/expiry and
stable kill/control identity separation are explicit. Actual hosted ephemeral
P/S/O/provider/reboot/W/integration/product gates remain distinct.

Independent native/runtime/CLI adversarial critique requested on the exact frozen
hash; Root acceptance must precede implementation. No Rust codec, key, account,
SCM/elevation, native workload, shared Cargo/lock change or production activation
was performed. Next smallest accepted slice is bounded pure codec+strict signature
verification with golden/mutation vectors, then actual native/provider admission.

## Independent authorization v2 critique reflected and amended freeze

Native/CLI/Runtime independently critiqued frozen93c2c337. Reflected P0 I/P
Ed25519 public-key inequality (otherwise P could sign I delegation), mandatory
pairwise P/S/O SID and PID nonalias plus I/P principal distinction. All FOUR
principal SIDs distinct remains a PROPOSAL pending installer/backend/Root review,
not accepted policy or an implicit extra codec condition. Added closed bounded
ExpectedAuthorizationData and exact equality matrix covering EVERY signed field,
including original I SID commitment/P artifact/BASE plans and pair/time bindings.
No caller data becomes issuer/store/elevation/native proof.

Native actual grammar independently checked: humanymous-lab- prefix15 plus1..48
lowercase ASCII/digit/hyphen tail means16..63 total, correcting an independent
reviewer's62 arithmetic and replacing earlier conservative64 bound. Final valid
max bodies393/710/201, artifacts467/784/275, base64url623/1046/367, completeHello
2768bytes. Explicit nonzero epochs/grant/delegation/nonce parse rejection remains
separate from CSPRNG/one-claim proof. Checked time/cap equality/reference expiry
and canonical base64 unused-low-bit mutations are now explicit.

Amended exact SoT SHA256
09bc9d5506613dc9837b918fbe7e9fbd479db2a2b1b5e66400be446499149a19
sent to root/native/runtime/CLI for delta verification. All actual native/store/
W/claim/transport/Ready and other-OS provider gates remain OPEN. No Rust codec,
Cargo edge, private key/provider read or workload. Root must accept final packet
contract and existing-version graph edges before first pure implementation.
