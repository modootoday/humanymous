---
id: plan:root/20261107115350
title: Draft Windows ordinary actor prebuilt Kestrel profile
status: draft
created: 2026-11-07T11:53:50Z
updated: 2026-11-07T11:53:50Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/dotnet-adapter, sot:root/rust-migration]
---

# Prebuilt actor proposal v0.1

DESIGN ONLY. Root requested comparison and exact API/paths before implementation.
No local account/SCM mutation, execution grant, unsafe owner duplication, new
manifest edge or backend qualification follows. .NET owns Host/controller and
artifact lineage; Windows owns the existing account/token/Job and cleanup path.

## Narrow profile versus the SDK workload

Select candidate `standard-user-dotnet-prebuilt`, distinct from draft
standard-user-dotnet SDK/package matrix. It runs ONLY the existing hosted test
binary selector `actual_kestrel_engine_audit_body_gc_and_shutdown` with fixed
`--ignored --nocapture --exact ... --test-threads=1`. No Cargo, SDK host CLI,
restore, build, publish, Git, compiler, linker, NuGet or generic-probe flag inside
the actor. Framework-dependent apphost loads an explicitly pinned runtime.

The existing test runs production and synthetic cases sequentially; it is NOT
one apphost total. At most one known Kestrel child is active at a time, plus
in-process authority workers and witness. Source07967eae helper and managed
attempt2 artifacts have separate lineage; new abort fixture is not in that helper.
A mixed-lineage profile MUST enumerate cases actually present in the immutable
helper; new source cannot be retroactively counted. JIT component only: NativeAOT,
clean package, SDK matrix, Linux backend and full product remain NOT_RUN.

This removes SDK/compiler descendant variability, but does not remove Job,
private-root, reader-join, timeout, profile and cleanup gates. It is a smaller
candidate, not a waiver for current Host::start or existing actor failure paths.

## Proposed closed input and entrypoints

New strict bounded schema `humanymous.dotnet-prebuilt-actor-input/1`, <=256KiB:
fixed profile name; fresh run nonce; controller artifact; app closure; runtime
closure; native facade artifact; source/build provenance references; exact case
inventory; no caller-supplied command/args/environment or arbitrary cleanup path.
Each artifact is absolute local regular nonreparse file with size+SHA256. Closures
are bounded relative-name manifests with no traversal, duplicate, case collision,
symlink/reparse, device/network path or unexpected file. Limits and canonical
manifest digest require .NET agreement before freeze. Proposed <=4096 files,
<=1GiB total verified runtime, individual <=256MiB, two phase hash validation;
these are draft resource ceilings, not measurements of the existing runtime.

Required app closure: Humanymous.HostPilot.exe/.dll/.deps.json/.runtimeconfig.json,
Humanymous.Native.dll and the actual facade DLL. Runtime closure: resolved
host/fxr/10.0.14 plus Microsoft.NETCore.App and Microsoft.AspNetCore.App10.0.14
and every dependency actually loaded by the selected apphost profile. Runtime
closure is protected read-only for the actor; the installer's private SDK ACL
cannot stand in for verified actor RX. Fixed files and their original build
receipts remain separately identified. Hash verification is not native loaded
memory attestation; in-place mutation/replacement must be excluded while running.

Proposed interfaces (types are NOT implemented):

```rust
struct ValidatedPrebuiltInput; // closed profile and actual verified open artifacts
struct PrebuiltActorBudget;   // absolute 180s run, reserved 60s cleanup, no renewal
struct ActorAttemptOwner;     // same account/password/job implementation, unique owner
struct PrebuiltObservation;   // safe controller receipt; never native cleanup proof
struct ActorTerminalFacts;    // native Job/process/reader/handoff/cleanup facts

validate_prebuilt_input(...) -> Result<ValidatedPrebuiltInput>;
run_prebuilt_actor(input, budget) -> RetainedActorAttempt;
// All post-mutation failures retain the actual attempt owner, including partial start.
RetainedActorAttempt::finalize(&mut self) -> ActorTerminalFacts;
```

Parent dispatches fixed runner child `dotnet-prebuilt`, which rechecks actual
ordinary TokenUser/expected SID and input digests before launching the exact test
binary. .NET Host::start consumes a typed validated launch context, constructs
Command with env_clear, fixed apphost path and controlled private cwd. It cannot
accept arbitrary caller command text or invoke a shell/SDK. Parent, child and
controller use the same absolute remaining budget; two profiles do not each get
180s. Controller-specific setup/readiness deadlines are minima, never extensions.

## Environment and source/artifact protection

Explicit environment only: verified SystemRoot/WINDIR, minimal System32 PATH,
DOTNET_ROOT to the protected runtime, DOTNET_MULTILEVEL_LOOKUP=0, fixed
DOTNET_ROLL_FORWARD policy consistent with pinned10.0.14, and private TEMP/TMP/
USERPROFILE/runtime writable paths. Exact roll-forward/apphost resolution must be
verified by .NET; setting one variable is not proof of runtime selection. No
DOTNET_STARTUP_HOOKS, additional deps/store/probing path, profiler, runner token,
package config, inherited cwd, generic probe or ambient PATH search. System DLL
resolution and native assembly-directory search remain explicit dependencies.

Existing source/artifacts receive RX and write denial before launch; parent
holds verified artifacts against write/delete replacement through the run where
safe native APIs permit. Parent rehashes complete input closure afterward. No
unverified directory writable by another task may substitute for protected bytes.

## Job, readers, outcomes and cleanup

Reuse suspended CreateProcessWithLogonW -> existing kill-on-close Job -> resume;
never duplicate its unsafe account/token/Job code in .NET or shared test-support.
No BREAKAWAY/SILENT_BREAKAWAY; all child launches must remain in the owned Job.
Microsoft documents inherited child membership and nested-job breakaway rules;
actual actor image/nesting remains a required hosted probe:
[Job objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
This is trusted test supervision, not denial of every external privileged broker.

Current actor launch returns job-empty only on its success path; timeout closes
the Job without acknowledged post-kill emptiness. Current Account::Drop deletes
unconditionally and normal path deletes account before scratch. These MUST change
for this profile before it can qualify: retain actual Job/account on failure,
terminate/query exact Job until empty under cleanup deadline, then same-user
cleanup before account deletion. No fabricated job-empty from handle closure.
Existing other profiles need not receive stronger historical claims.

.NET must replace fire-and-forget stdout reader with retained join ownership and
bounded, cancellable IO. Queue sends cannot block forever on a stopped consumer;
stderr must have an explicit capture/suppression contract. Per-frame4096/queue64
is not a total output cap or joined-reader proof. Require exact actual host exit,
EOF, reader joins and final frame outcome; on timeout/malformed/overflow, keep
failure and retained cleanup ownership. No unbounded join or detached worker.
Parent can terminate the whole Job; if that kills controller before joins/report,
readers_joined is Unknown and execution cannot pass even if Job becomes empty.

Child creates/protects an owner-only private work parent; existing inherited
admin+actor TEMP alone does not satisfy the private-directory factory. Selected
results are published into a separate admin+actor handoff root BEFORE private
cleanup/controller exit. Reuse proposed limits: <=32 regular files, <=256KiB per
stream, <=8MiB aggregate, <=64KiB typed outcome, known relative names/hash/length.
No public raw log/credential/endpoint/PID/profile path publication.

On normal exit: bounded handoff -> private cleanup -> controller exit -> actual
Job-empty/direct-child wait -> verify/copy handoff -> delete account -> delete
scratch. On timeout/error: main Job-empty first -> fixed bounded cleanup child
under SAME user in a separate owned cleanup Job -> private cleanup -> cleanup
Job-empty/wait -> account/scratch deletion. No admin ACL repair/takeownership.
Incomplete cleanup keeps actual owner/recovery reservation; no deletion in a
Drop path that destroys the only cleanup identity. Recovery after parent death
is still a P0 contract, not supplied by the pure lifecycle model.

LOGON_WITH_PROFILE profile directory/HKCU removal remains explicit Unknown until
native deletion is separately reviewed/proved. USERPROFILE redirection does not
close that gate. Root decided residual profile state blocks this
narrow disposable hosted component; a .NET-only NoProfile design needs actual native proof;
no design assumption silently promotes account deletion to profile erasure.

## Ownership paths and acceptance gates

Windows writer only: scripts/ci/windows-native-runner/src/actor.rs plus new
actor/prebuilt.rs for pure DTO/dispatch (no second unsafe owner), runner main
mode/options and INTEGRATION.md; native-security only if an accepted safe private
root/artifact primitive is actually missing. .NET writer only: tests/hosted.rs,
tests/hosted/* or tests/support/*, own INTEGRATION.md/SoT/workflow. No workflow
change or new shared graph edge is requested by this proposal.

Before implementation: .NET agrees exact compiled helper case list, closure,
launch context/env, bounded reader/receipt API and host/runtime lineage; native
owner resolves retained Job/error/account/private cleanup/profile contract; Root
accepts exact API/paths and any residual profile boundary. Pure schema/transition
checks then source-bound genuine hosted ordinary actor probes; no local account
or SCM execution. Verdict NEEDS-WORK, source-only candidate.

## Concrete observed runtime and three compiled lineages

Read-only .NET runtime observation was independently rehashed: manifest
SHA256 d9bd271d558f07ad6b1438a27d6c8678b24915e26d03b5ed8cc5e3368529944d,
336 files,110309027 bytes, exact fxr/NETCore/AspNetCore10.0.14. This is current
bytes only: not original build provenance, actor RX, loaded-module binding or
runtime execution. No copy or invocation has been authorized.

The final DTO must carry THREE separately identified compiled lineage records:
(1) immutable hosted Rust controller/helper07967eae with its actual case inventory;
(2) managed apphost/assembly/support from successful attempt2 with original build
receipts; (3) actual .NET facade source2ffa/release artifact69b01501, explicitly
not generic ABI2475. Runtime observation is a separate dependency closure, never
an invented common atomic source SHA. Root supplies complete exact hashes rather
than truncated display prefixes before candidate freeze. Do not transfer SDK,
NuGet, build cache, credentials or unrelated installed frameworks.

Observed runtime size permits tighter proposed runtime-only ceilings of512files,
192MiB aggregate and64MiB perfile, with exact expected336-entry manifest checked
for this candidate. App/controller/native closures have their own named bounded
inventories. The earlier broad resource ceilings are provisional maxima, not a
choice to admit surplus runtime files. .NET must agree final limits/closed schema.

### Runtime staging boundary (proposal, no copy grant)

Prefer a new administrator-owned per-attempt artifact root under the existing
fixed local ProgramData allocation, with actor RX only, populated solely from
closed managed/native/runtime manifests. Do not grant traversal into another
user's private SDK/profile or change its inherited ACLs. Stage before actor launch,
verify regular files, protected ancestors, exact manifest equality and hash both
source and staged bytes; retain mutation-excluding handles for the selected run.
Writable cwd/TEMP/audit state remain separate actor-owned paths. Final artifact
root removal is distinct from actor private-state cleanup; failures stay separate.
Root must approve this bounded copy and lifecycle before any staging operation.

## Root decisions and pure implementation candidate v0.2

Root accepted runtime exact336 entries with512/192MiB/64MiB ceilings, six fixed
app roles and separately bounded hashed harness; no SDK/generic ABI. Caller DTO
is closed schema humanymous.dotnet-prebuilt-actor-input/1 with profile, nonce,
harness, app, runtime and named managed/native/harness provenance. Runtime carries
new normalized files-only manifest plus explicit parent_observation_sha256=d9bd;
normalization sorts relative paths, compact serde JSON+LF. The original runtime
observation float summary is immutable and informational; per-file u64 totals are
recomputed. Expected manifest/harness/provenance digests come from independently
bound evidence, never the DTO's own advertisement. Native byte/ACL verification
is not implemented by this pure parser.

Parent-generated LaunchContext is separate from caller DTO. It binds apphost to
app_root/Humanymous.HostPilot.exe, disjoint app/runtime/work/handoff/system roots,
private audit child, SID syntax and named lineage. Budget wire format uses u64
issued_utc_ms, run_expires_utc_ms=issued+180000,
cleanup_expires_utc_ms=run+60000, parent_elapsed_ms_at_dispatch. A single receipt
maps min(wall-expiry remaining, conserved monotonic remainder) to local Instant
run/cleanup deadlines; rollback, elapsed/expiry overflow and extension reject.
Consumers borrow this one mapping across both sequential profiles. No original
180s budget is recreated by each Host/readiness call. Actual authenticated
parent handoff, one-owner receipt consumption and native clocks remain OS adapter
gates; pure parsing itself cannot prove the origin or freshness of metadata.

Fixed environment includes only verified SystemRoot/WINDIR/System32 PATH,
DOTNET_ROOT/X64, multilevel=0/LatestPatch, and private TEMP/TMP/USERPROFILE.
Private runtime root contains only10.0.14; runtime selection must still be actually
observed. No custom caller env/argv/deadline fields. Tests cover closed schemas,
traversal/ADS/reserved names, role/count/resource/digest mismatch, named lineages,
original/new manifest separation, fixed env/selector, original clock conservation,
cleanup ordering and failed/unknown terminal outcomes. Seven tests passed in Root-allocated native Windows validation,
with all-target check and strict Clippy. Runner module is cfg(test) ONLY;
no mode dispatch, OS launch, copy, account operation or native file proof is wired.

Root decided profile residual BLOCKS a clean pass. A .NET-only logonflags0
NoProfile design may proceed only with actual before/after ProfileList and profile
directory absence, not USERPROFILE/account-delete inference. Other profiles retain
LOGON_WITH_PROFILE. Parent death can remain Unknown/residual and never fabricate
synchronous cleanup; no unbounded recovery promise. Retained Job/account failure
owner API and joint bounded reader context still need review BEFORE OS wiring.

### Actual pure validation and cleanup-only budget

Final cfg(test)-only source passed7tests/0failed/0ignored/30filtered, all-target
check and strict Clippy;51source inputs and32registry packages/2079files had zero
drift. No OS mode exists. Evidence is registered in dotnet-prebuilt-pure-summary.json.
A separate trusted-parent CleanupOnlyContext has no apphost/harness or workload
API. Expired run is allowed only while the ORIGINAL cleanup reserve is positive:
180000/180001/239999ms permit cleanup only,240000 rejects, with no renewed240s.
Run context rejects expired run. Fixed env adds DOTNET_EnableDiagnostics=0.
Original parent's actual monotonic watchdog stays authoritative: UTC+dispatch
elapsed alone cannot prove every transit/rollback. Native adapter must retain
that same watchdog/owner; parse calls cannot become renewal or attestation.

## Retained native owner implementation slice and next Job API review

First production refactor (not yet compiled/OS-qualified): OwnedActor owns the
existing Account and Zeroizing password. It is prepared with an uncreated Account
BEFORE NetUserAdd. Successful creation marks the existing account state before
any fallible membership checks. provision(self) returns OwnedFailure containing
the SAME owner on every error. Retained policy suppresses automatic account
deletion; legacy adapter deliberately drops its returned owner using old policy,
preserving existing profiles/receipts. No new Win32 call, flag, feature or unsafe
handle owner is added. Password never appears in Debug/error/argv/environment.
New profile cannot use the lossy legacy conversion or choose its own policy.

This is the account half only. Proposed next native API, BEFORE wiring/OS run:

```rust
struct OwnedProcessAttempt; // existing unique child/thread/Job Handle guards
struct OwnedProcessFailure { owner: OwnedProcessAttempt, error: io::Error }
enum StartFailure { NotStarted(io::Error), Retained(OwnedProcessFailure) }

start_fixed_runner(...) -> Result<OwnedProcessAttempt, StartFailure>;
OwnedProcessAttempt::wait_run(&mut self, original_deadline) -> Result<ChildExit>;
OwnedProcessAttempt::contain(&mut self, original_cleanup_deadline) -> Result<JobEmptyProof>;
```

Store actual child/thread handles immediately after successful CreateProcess,
BEFORE Job creation/configuration/assignment/resume can fail. Post-start errors
retain them and the optional exact Job guard; no resource-free error. Contain
uses existing TerminateJobObject/WaitForSingleObject/QueryInformationJobObject
only, retains handles on every failed/unknown observation, and separates actual
child wait from Job active0. Legacy launch adapter preserves20minute/5second
behavior and old failure scope; it does not gain stronger cleanup facts.

A new prebuilt attempt will compose OwnedActor plus OwnedProcessAttempt under
one owner, retain account on unknown main/cleanup Job state, invoke ONLY typed
same-user cleanup after main Job-empty, and permit account deletion only after
private/profile outcomes. Drop may close exact Job and leave account/state as
Unknown; parent death cannot promise synchronous successful cleanup. No numeric
PID/foreign process constructor, arbitrary command, new unsafe owner or new Win32
API is proposed. Exact refactor critical diff and original deadline integration
need Root review; NoProfile logonflags0 and ProfileList/directory API remain
separately reviewed prerequisites, not implementation acceptance from pure tests.

## Shared unverified parent wire owner v0.3 (fresh validation pending)

Root selected test-support::parent_context as the single wire/fixed-environment/
budget codec (existing serde/serde_json only). Public API:
- decode(bytes<=64KiB) -> UnverifiedParentContext::{Workload, CleanupOnly};
- encode(&context) -> deterministic compact serde JSON+LF;
- typed UnverifiedLaunch/UnverifiedCleanup, named UnverifiedProvenance;
- context.fixed_environment() -> closed literal keys plus unverified path text;
- context.map_budget(wall_utc_ms, receiptInstant) -> UnverifiedMappedBudget,
  borrowed getters run_deadline(Option)/cleanup_deadline, no Clone/Deserialize.

Workload wire schema remains humanymous.dotnet-prebuilt-launch/1 with original
accepted fields. Cleanup schema humanymous.dotnet-prebuilt-cleanup/1 contains
ONLY system_root/private_work/handoff_root/actor_sid/provenance/budget; it has no
apphost/harness/workload data. Unknown/duplicate fields and malformed metadata
reject. Golden cleanup JSON plus workload roundtrip and original cleanup budget
boundaries are tested ONCE in shared code. No private raw payload enters errors.

Shared types are UNVERIFIED syntax and cannot mint native authority, proof of
parent, VerifiedArtifact, protected root, actor identity, Ready or authenticated
original deadline. Runner retains manifest hashing, native filesystem/Job/account,
actual TokenUser, fixed apphost binding/disjoint-root policy and trusted construction.
.NET must not treat successful decode as launch admission. Actual original parent
monotonic watchdog, one trusted receipt/owner and bounded reader contract remain
prerequisites. No new Cargo edge is needed; .NET already depends on test-support.

Runner now consumes this shared codec; its duplicate clock/environment/provenance
codec was removed. Seven old pure cases at12ab3335 stay historical; new runner7
and shared4 are authored, format-checked and NOT_RUN pending allocation. First
production OwnedActor/OwnedFailure account refactor is also format-only, with old
profile behavior intentionally preserved. No new Win32 calls/flags or OS launch.

### Fresh shared/account source qualification

Immutableb482d5be +seven overlays passed shared4/0failed/0ignored/23filtered and
runner7/0failed/0ignored/30filtered. BOTH all-target check/strict Clippy passed.
Snapshot2417files and32registry source packages/2079files plus observed Cargo
config/compiler fingerprints had zero drift. Actual compiler version was not
retrieved and no complete cache-key provenance is asserted. This supersedes the
NOT_RUN marker above only for these exact new source hashes, registered in
parent-context-joint-summary.json. Account state is explicitly Uncreated/Present/
Deleted. Retained-policy owners cannot call the legacy account-delete adapter.
No actual account, Job, profile, Kestrel or retained Job API execution occurred.

## Retained Job source implementation candidate (not yet compiled)

OwnedProcessAttempt now wraps ONLY existing Handle owners for child/thread and
optional Job. Successful native CreateProcess stores both handles BEFORE Job
creation/configuration/assignment/resume. StartFailure distinguishes pre-spawn
NotStarted from Retained(OwnedProcessFailure with actual owner). wait_run and
contain borrow the owner on every error. JobEmptyProof borrows the exact owner,
combining actual direct-child exit observation with native active-process0;
no numeric-ID, clone, serialization or external proof constructor is exposed.

The existing legacy adapter remains the only caller: same fixed runner executable,
LOGON_WITH_PROFILE,20minute run/5second Job budget and legacy failure Drop10second
wait policy. Retained internal policy must be selected BEFORE creation and never
renews a synchronous Drop wait; Drop requests exact owned direct termination
when needed and closes exact Job, but cleanup remains Unknown. If Job setup or
assignment failed and child was unassigned, contain may fail/timeout and retain
the owner; no Job-empty/all-descendants proof is manufactured. New prebuilt mode
is still not wired. No new Win32 API, flag, feature edge or separate unsafe handle
owner exists. Native APIs remain in actor.rs's existing quarantined boundary.
Critical diff must pass Root review and fresh compiler/pure checks; actual native
Job/account/profile outcomes remain unexecuted and unqualified.

## Additive duplex stderr API (source-only, actual IO review pending)

ManagedChild::spawn_duplex_with_stderr(&mut Command) returns one ManagedChild,
ChildStdin, ChildStdout, ChildStderr. Existing piped(command,true) supplies pipes;
partial extraction drops the existing direct-child guard. No reader thread,
startup write, protocol, implicit deadline or descendant supervisor is created.
Original spawn_duplex remains stderr=null and does not delegate through a dropped
stderr pipe. Caller owns all streams/join handles and explicit output bounds.

Separate startup is one-shot<=64KiB AFTER actual captured child PID validation;
subsequent controls<=1024bytes/queue64. Parent-context wire contains NO authority
credential. EOF, joined readers and outer Job emptiness remain independent actual
facts. Existing duplex parent fixture was extended for concurrent returned stdout/
stderr readers, with no new harness helper-parent inventory. That actual process
case is authored/NOT_RUN until Root explicitly allocates it; formatting alone is
not pipe/EOF/deadline or Kestrel qualification. Shared context4629133c/fa436a2a
remains bytefrozen for .NET's independent D compile.

## Critical partial-setup correction before validation

Independent .NET review found that a stored Job handle can still be null,
unconfigured or unassigned while the child remains suspended. Explicit native
phases now advance ONLY after each successful operation: Created -> JobCreated
-> Configured -> Assigned -> Resumed. Created/JobCreated/Configured select exact
owned child-HANDLE TerminateProcess plus bounded wait under ORIGINAL cleanup
expiry; Assigned/Resumed select exact owned Job termination/query. No PID fallback,
new Win32 API, fresh cleanup budget or dropped failure owner is used.

ConvergenceProof records scope (unassigned direct child versus assigned Job) and
independent exit observation with natural/forced provenance. It borrows the exact
owner; unassigned convergence never claims Job-empty or descendants-empty.
force_requested becomes irreversible BEFORE termination request. A previously
observed natural exit stays distinct from exit observed after forced cleanup.
Legacy return accepts only its prior natural exit plus assigned-Job convergence;
forced exit0/Job-empty alone cannot become natural success or reader-settled proof.
Reader/EOF/handoff/private/profile outcomes remain separate unknown native gates.
Pure partial-phase failure matrix regression is authored, NOT_RUN pending Root
allocation. No actual process/account/SCM/SDK/Kestrel execution is authorized here.

### Partial setup and duplex actual narrow qualification

Source actor0372733b/process6b1 on immutable77ca7d9f passed shared4,prebuilt7,
partial-phase1 pure cases plus existing duplex parent1 (two nested helpers NOT
counted separately), all0failed/ignored/measured. BOTH all-target check/strict
Clippy passed. Snapshot2427files and32registry packages/2079files/config/flags had
zero drift. This supersedes NOT_RUN above ONLY for that code and those selectors.
Actual Job/account/partial-fault/profile/staging/Kestrel and authenticated parent
transport remain unqualified. Exact footers/hashes are registered in the cross-OS
runner partial-setup-duplex-summary.json. C returned, no background handle held.
