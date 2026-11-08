---
id: plan:root/20261108043000
title: Concrete B2 Windows owned tree original budget and recovery backend
status: draft
created: 2026-11-08T04:30:00Z
updated: 2026-11-08T04:30:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/structural-development, sot:root/native-security, plan:root/20261107223000, plan:root/20261108023000]
---

# One selected backend, still unapproved

Select Windows2025/x64 native JOB_LIST-at-creation containment, private std
OwnedHandle ownership, and ONE safe owning lifecycle above the Platform bridge.
No new crate, WinSafe dependency edge, nightly, foreign implementation, arbitrary
Command/Read/Write provider or module-wide unsafe allowance. B2 remains an explicit
Root+X review proposal; no source/feature/unsafe/compiler/native/actor grant follows.
B1 is independent: Root's conditional decision6aea9fcc accepts ONLY its exact
seventh low-level non-authority creation scope. Root subsequently implemented it
in isolated Platform source and reported scoped pure/compile/doc checks; actual
four native fixtures remain ignored/unrun and provider admission Unsupported.
This is isolated component source implementation, not native/provider/whole-source
qualification; original04110973 candidate remains immutable. B2 remains UNAPPROVED.
This is a concrete refinement of the
prior ten-scope B2 proposal, not permission to add those scopes.

First primitive supports fixed source-bound Rust child images with NO inherited
stdio/credentials/Job handles and no pipe workers. It qualifies containment/status/
original deadline/retained recovery only. Required captured-output/provider/W
consumers remain unavailable until their real transport/worker ownership is wired;
zero workers cannot count as passing a requested nonempty capture profile.

# Safe-facing contracts and exact owner placement

Proposed Platform module windows_supervisor, feature windows-supervisor default-off,
using existing windows-sys0.61.2 plus Win32_System_Threading/JobObjects features.
Platform dependency currently has only Foundation/FileSystem/Security; Root alone
can change features after approval. Existing six windows_object wrappers unchanged.
Native-security/Authority/Runtime remain unsafe-forbid. No testing feature in
production and no public SDK proof/host/provider constructor.

```rust
// Proposed bridge types: private fields, no Clone/Debug/serde/raw getters.
struct NewJob;
struct ConfiguredJob;
struct CreatedProcess;
struct RetainedLaunchFailure;
struct JobPolicyObservation;
struct JobPopulationObservation;
enum ProcessPoll { Pending, Exited(ObservedExit) }

fn create_job() -> Result<NewJob, JobCreateFailure>;
fn configure_job(job: NewJob) -> Result<ConfiguredJob, RetainedJobFailure>;
fn spawn_in_job(job: ConfiguredJob, launch: PreparedLaunchData)
    -> Result<OwnedTree, RetainedLaunchFailure>;
impl OwnedTree {
    fn poll(&mut self) -> Result<TreeObservations, BorrowedObservationFailure<'_>>;
    fn request_stop(&mut self) -> Result<StopRequested, BorrowedStopFailure<'_>>;
    fn settle(self) -> Result<TerminalTree, RetainedTree>;
}
```

No bool/native claim input. PreparedLaunchData is bounded validated DATA, never
host/source/process authority. Platform owns actual Job/process/thread handles;
the safe Runtime private supervisor adapter owns that tree, original clock pair,
registered reaps, real worker owners and invocation/source admission. No actual
native observation can be reconstructed from wire fields or imported raw handles.

Root chooses a SINGLE common owning controller by extracting the applicable
safe retention/clock/worker bookkeeping out of Platform testing into a normal
private module, then using it from testing and the native adapter. Do not copy
OwnedDirectChild or make production depend on testing. Replace only its concrete
child/status backend with a closed INTERNAL enum StdChild/WindowsTree, constructed
by their own actual spawn operations; no externally implementable backend trait
or callback. WindowsTree cannot be fabricated by filling the StdChild PID field.
Shared generic lifecycle is only a conjunction checker; concrete adapter alone
can record actual completion. Do not export symbolic observe(fact) as native proof.

Required supervisor composition input is the original owning InvocationOwner:
actual source/artifact/actor admission plus a first-I/O captured run/cleanup pair
and independent outer controller ownership. This manufacturer remains absent for
the final product. A CI environment label, current_exe, PID, local QueryClock or
Lifecycle<()> cannot supply it. Bridge create/spawn does NOT manufacture O,
original-I delegation, NativeProof or an admitted budget capability.

# Pre-execution membership and fixed nonescape profile

Before CreateProcessW: create unnamed noninherited Job; set
JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE ONLY; query extended limit information and check
exact accepted policy, including absence of BREAKAWAY_OK/SILENT_BREAKAWAY_OK. Hold
the sole Job handle privately. No OpenJobObject(name), duplicate/inherited Job
handle, named-job collision, parent-process substitution or foreign Job attachment.

Build one attribute entry PROC_THREAD_ATTRIBUTE_JOB_LIST containing that actual
Job handle. The minimum QUALIFICATION image is a fixed admitted Rust Windows
CONSOLE PE executable (IMAGE_SUBSYSTEM_WINDOWS_CUI=3), not a GUI/service/arbitrary
executable profile. Check the actual held source-bound image/PE subsystem before
launch; argv or a reported subsystem cannot establish it. Use explicit executable
path, closed bounded argv/environment/cwd, EXACT flags
EXTENDED_STARTUPINFO_PRESENT|CREATE_UNICODE_ENVIRONMENT|DETACHED_PROCESS
(0x00080408), and bInheritHandles=false. Initialize STARTUPINFOEX fully, cb matching
the extended structure, NO STARTF_USESTDHANDLES, hStdInput/hStdOutput/hStdError=NULL,
no reserved/desktop/title/stdio attributes. Neither CREATE_NEW_CONSOLE nor
CREATE_NO_WINDOW is combined with DETACHED_PROCESS. inherit=false ALONE was not
enough to justify the previous no-console/no-stdio claim; this explicit CUI profile
supersedes that incomplete statement and still needs actual qualification.
NO CREATE_BREAKAWAY_FROM_JOB, parent-process or token substitution. JOB_LIST
associates the new process before its first execution; no running-child
AssignProcessToJobObject race and no cooperative grant accepted as containment.
This minimum profile is NOT CREATE_SUSPENDED: it needs no ResumeThread exception.
If Root instead requires suspended preflight, that is a separate exact API review,
not an unstated extra call in this allowlist.

Immediately adopt BOTH returned process/thread handles into private owners BEFORE
fallible work. Query IsProcessInJob with the same process/Job and requery Job policy.
Failed/unsupported attributes/nesting/membership refuse, retaining actual owners;
never retry without JOB_LIST or with breakaway. A process may run before the
post-create confirmation, but it was already kernel-associated at creation. The
membership query is corroboration, not a retroactive assignment proof.

Nested enclosing Job restrictions apply; no escape from a genuine hosted outer
Job is requested. Children created directly through CreateProcess inherit job
association under these limits; child nested jobs remain in the hierarchy.
IMPORTANT ceiling: Job objects are NOT a sandbox against external brokers.
Windows documents that WMI Win32_Process.Create children are not associated like
ordinary CreateProcess descendants. SCM/task/WMI/foreign broker spawning cannot
be called contained by this backend. The first profile is fixed trusted shipped
Rust work with no broker delegation, under existing same-principal/privileged-OS
ceiling. A requirement to contain arbitrary hostile broker-capable workloads is
Unsupported; satisfying it needs a separate reviewed token/sandbox architecture.
No broad global noescape claim follows from two Job flags.

# Original deadlines and state ownership

InvocationOwner captures Instant BEFORE first Job/allocation/native I/O; computes
checked run and cleanup limits from accepted bounded profile. It never accepts a
wire Instant or derives a new cleanup budget after failure. Every partial owner,
worker, stop request, retained failure and recovery transfer keeps this SAME pair.
Repeated requests/inspection/polling cannot extend either deadline. At first loss,
cleanup limit is min(original cleanup, first-loss instant + accepted drain cap).
This is a scheduling/observation bound, not proof synchronous Windows calls return
by that time. The independent outer process/controller must remain armed across
create/query/stop/final join. Missing outer-owner association is Unsupported.

State owner progression:

| State | Actual resources retained | Permitted next action / refusal |
|---|---|---|
| Preflight | Original invocation/clock, admitted fixed image/argv | Allocate one Job only while original start budget remains |
| JobCreated | Same actual Job handle, original owner | Configure and query policy; failure returns it, no launch |
| Configured | Job + policy + live attribute backing | One CreateProcessW attempt; no named reopen/fallback |
| CreateCallEntered | Same Job/attribute/original owner | Outcome uncertainty remains retained; native false does not by itself prove no possible effect |
| Created | Same Job + actual process/thread handles | Membership/policy check, then retain running tree; any failure closes admission and requests OWN stop |
| Running | Tree + registered direct waits + actual workers | Poll/observe or request stop, never change identity/clock |
| Closing | Same owners, no new launch/admission/stop worker | Poll direct signal and Job population; join only owned completed workers |
| RetainedUnknown | Same owners/first cause/original deadlines | In-process transfer to recovery, never spawn/retry/renew |
| Terminal | Actual signal/Job0/reaps/joins/stop settled conjunction | Release allocation with separate execution outcome |

Unknown failure after actual spawn must not become an io::Error that drops the
only tree owner. RetainedTree is non-Clone/non-Debug/non-Serialize; inspection
does not give raw handles or replace identity. One recovery owner receives the
actual value by MOVE. No recover-by-PID/name, duplicating Job handle to a generic
reaper, background detached thread, generic scheduler callback or leaked owner.

# Kill on owner loss versus retained recovery

Returning RetainedTree keeps the Job handle alive, so kill-on-close does NOT fire
just because an operation returned an error. Recovery can issue a stop request
on the SAME Job and poll under original deadlines. Only in-process ownership
transfer is admitted; no cold reconstruction of Instant or successful cleanup.

On actual last-handle loss/process death, Windows kill-on-job-close requests tree
termination. This is containment policy, NOT a receipt that descendants exited,
pipes reached EOF, workers joined or custody erased. No duplicate Job handle may
silently keep the job alive after its alleged owner dies. The outer controller
retains its OWN enclosing Job/actual O child handle, not a duplicate of the inner
Job; it separately observes O exit and outer population. If proof is unavailable,
report Unknown/unproven74 and preserve durable journal reservation/tombstones.
Cold O restart remains journal RecoveryOnly/inert: no resumed old attempt, new
clock, create/delete, counter reset or inference that owner death cleaned roots.

Drop closes owned kernel handles through std ownership and may trigger kernel
kill-on-close, but it never blocks waiting or reports terminal. Existing actor.rs
legacy_drop_cleanup's relative10000ms wait is NOT reused as original-budget
production recovery. No foreign/SCM-managed service is treated as an owned child;
normal S lifecycle stop still needs separate manager authorization and dual-control
mutations retain their accepted boundaries.

# Concrete completion evidence, including descendants and pipe workers

Poll WaitForSingleObject(process,0). Only WAIT_OBJECT_0 permits GetExitCodeProcess;
WAIT_TIMEOUT is Pending; WAIT_FAILED/unexpected result is Unknown. Exit code259
may be a REAL exit code after signal, never a sole STILL_ACTIVE test. Retain the
same actual direct process handle until its terminal observation is recorded.
Windows has no POSIX waitpid adoption here: required reaps mean observed terminal
waits for EVERY handle this adapter actually owns/registered, then safe handle
release; do not claim direct wait handles for arbitrary descendant PIDs.

Job emptiness is a separate successful QueryInformationJobObject basic-accounting
ActiveProcesses==0 on SAME Job AFTER closing new launches. Direct exit/Job handle
Drop/TerminateJobObject success/termination timestamp/ACTIVE_PROCESS_ZERO event
alone cannot set PopulationEmpty. No future process may be admitted by this owner
once Closing; private noninherited Job handle prevents other ordinary owners from
assigning new processes under this fixed profile. Actual Job0 plus registered
direct reaps are separate conjunctions; unknown descendants remain unknown if
containment assumptions were violated.

Reuse existing CaptureAttempt/Worker behavior where valid: bounded per-stream
buffers, retained JoinHandles, outcome channels, is_finished followed by ACTUAL
join, joined panic reported as execution failure; no detach on timeout. Output
arrival, EOF, helper exit, receiver disconnection, a bool or Command::output is
not worker completion. Retain all already started workers on partial setup/error.
StopOperationsSettled requires no outstanding launched stop/query operation and
no path able to issue another mutation for this invocation. AllocationReleaseReady
requires real completion/handle/worker owner transfer, never a symbolic counter.

Minimum B2 creates NO pipe/IO workers: actual registry is empty because no pipe
operation is permitted. A requested nonempty capture profile MUST return
Unsupported before launch, not set ReadersJoined=true. Existing std blocking
Read/Write workers have no proved bounded safe cancellation; is_finished/join can
still block and require outer supervision. Keeping that real owner under a Job
does not magically make host pipe threads interruptible.

Required next capture decision is separate B2-P: ONE actual transport owner using
existing safe interprocess/Tokio native pipes (or a separately reviewed explicit
handle-list/overlapped bridge), with cancellation/close completion and ACTUAL task/
worker joins. Anonymous inherited stdio would need exact HANDLE_LIST attributes,
bInheritHandles=true with explicit allowed inheritable pipe handles only, and
reviewed cancellation; neither inherit-all nor HANDLE_LIST is in this B2 allowlist.
Do NOT add CancelIoEx/ReadFile/WriteFile/CreatePipe/SetHandleInformation/ResumeThread
under a broad exception. No complete retained-pipe/lint/native-pilot success can
be claimed until its selected actual path and failures qualify. D4 cross-SID
Authority channel remains separate and unapproved.

# Exact ten proposed private unsafe scopes, no broad exemption

Keep raw calls in Platform windows_supervisor/native.rs, each function-local
allow(unsafe_code) ONLY after Root+X accepts exact body. All facade/state/Runtime
code forbid unsafe. Existing six Platform wrappers/B1/actor's two allows unchanged.
No generic buffer/class/PID importer or unsafe Send/Sync. Selected backend uses
std OwnedHandle throughout; no new WinSafe feature/dependency edge.

| Private function | ONLY native operations and ownership obligations |
|---|---|
| create_job_owned | CreateJobObjectW(NULL,NULL), validate nonnull, ONE OwnedHandle::from_raw_handle adoption. No name/open-existing/duplicate. |
| configure_job | SetInformationJobObject with initialized fixed JOBOBJECT_EXTENDED_LIMIT_INFORMATION and ONLY KILL_ON_JOB_CLOSE policy. Borrow same Job. |
| query_job | QueryInformationJobObject with ONLY extended-limit policy or basic-accounting fixed classes; exact initialized output sizes/returned lengths. No arbitrary class or output pointer. |
| terminate_owned_job | TerminateJobObject SAME owned Job; only request observation. Never terminal proof. |
| is_owned_process_in_job | IsProcessInJob SAME owned process and Job; output internal, no caller bool factory. |
| make_job_attribute_list | InitializeProcThreadAttributeList bounded expected-size probe/init and UpdateProcThreadAttribute JOB_LIST ONLY. No parent/token/handle-list attributes. |
| drop_job_attribute_list | DeleteProcThreadAttributeList exactly once after successful initialization and last create use. No broad destructor allowance. |
| create_process_in_job | CreateProcessW with fixed valid STARTUPINFOEX/PROCESS_INFORMATION and flags above; on success immediately adopt BOTH handles via OwnedHandle::from_raw_handle. No arbitrary attributes/token/foreign parent or assign-after-start. |
| poll_owned_process | WaitForSingleObject(handle,0), GetExitCodeProcess ONLY after signaled; fixed borrowed actual handle. |
| terminate_owned_process | TerminateProcess on SAME returned actual process only for partial create/membership cleanup; no OpenProcess(PID). Record stop request, never completion. |

Attribute memory is owned, bounded and suitably aligned BEFORE creation: expected
size probe must report insufficient-buffer and finite accepted size; initialized
list uses stable16-byte-aligned allocation, actual Job HANDLE array has stable
backing, Job and backing outlive every create call and Delete. All UTF16 argv/
environment/cwd and result-owner storage prepared before CreateProcessW, no
fallible post-create allocation that could lose actual handles. Failed creation
does not interpret uninitialized PROCESS_INFORMATION or adopt arbitrary values;
retain Job and classify uncertain effect until real Job0 observation. No manual
CloseHandle unsafe call: std owned guards close exactly once.

Already safe APIs cover checked Instant arithmetic, bounded allocations/encoding,
std OwnedHandle close/move, actual std Child::try_wait/kill for existing std-owned
children, Worker JoinHandles/results, WinSafe observation leaves and File privacy.
Those WinSafe methods cannot operate on arbitrary std raw handles without a
reviewed ownership adapter, so they do not remove the selected Platform polling
scopes by magic. Stable Rust's raw process attribute API is not qualified here;
no nightly or unsafe constructor hidden in Runtime is an alternative. Existing
actor native calls are an oracle, not permission to extend module-wide unsafe.

# Exact hosted test matrix and remaining acceptance

After Root+X approval/source integration and fresh resource allocation:
fmt FIRST/scoped locked check/strict Clippy/negative construction checks, then
genuine Windows2025 x64 source-bound Rust fixture under original outer owner.

1. Actual JOB_LIST membership before any fixture work marker; one real ordinary
   CreateProcess descendant under same Job. No parent assignment barrier credited.
   Independently test both a console-attached parent and a parent with redirected
   stdin/stdout/stderr: exact detached CUI helper must not acquire the parent's
   console/standard handles or deliver a marker into redirected parent streams.
   Use separately reviewed observation APIs; no new console unsafe calls enter
   B2's ten scopes. Unsupported observation cannot count as a passing negative.
2. Explicit child breakaway request denied/contained; nested host Job incompatibility
   refuses without fallback. Broker escape is outside fixed profile, never probed
   against third-party services or reported prevented by Job-only evidence.
3. Force post-create query/membership refusal: actual process/thread/Job retained,
   stop requested, direct signal and real Job0 observed separately; no PID reopen.
4. Real child exits while descendant keeps running: direct Exited cannot finish;
   original deadline stops OWN Job, waits actual population0 plus registered reaps.
5. Error moves retained owner to recovery WITHOUT closing Job or renewing limits;
   duplicate stop retains first cause. No cold restart reconstruction.
6. Actual supervisor death while its own child remains: separate outer controller
   observes kill-on-last-close behavior and actual outer population; owner death
   alone never passes cleanup. Never give observer a duplicate inner Job handle.
7. Exit259 after actual signal recognized terminal; wait/query failure stays Unknown.
8. Requested captured IO refuses BEFORE launch in minimum profile. Later B2-P
   required retained-unread pipes, pending write, overflow, worker panic, lost
   channel, partial setup and true joins are separate required cases, not waived.
9. Run/cleanup deadline expiry before/after each state: same original pair, no
   detached worker, no extra launch/stop after release; unresolved retains74 and
   durable journal Unknown, never success, erase or allocation release.

Root+X must accept exact scopes/ABI/ownership and common lifecycle extraction;
Root retains manifests/feature edges and source loans. Actual hosted actor/source
admission and outer-most budget manufacturer still require their own proof.
No NativeProof, provider/O identity, original trust, private-root durability,
ServiceHost Ready or full product qualification follows. All tests/native/compiler
work here NOT_RUN. Original provider/journal/Ready goal remains active and OPEN.

Primary references checked:
[JOB_LIST and explicit inheritance](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute),
[Job association, nesting and broker ceiling](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
[termination request semantics](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-terminatejobobject).

# B2-P qualification helper alternative, no added unsafe proposal

Root requested evaluation, NOT source/manifest/helper acceptance. Select as a
viable QUALIFICATION-ONLY next candidate: JOB_LIST-spawn the exact admitted
humanymous-delivery CONSOLE artifact in a fixed capture-helper mode, detached as
above with NO inherited handles. Inside that process, ordinary safe Rust
std::process::Command creates one fixed admitted Cargo child with piped stdio;
normal descendants inherit the actual Job association. No CreateProcess raw
HANDLE_LIST, inherited parent stdio or new unsafe scope is required by this
architecture. It does not cover SCM/WMI/external-broker process trees.

Existing Delivery command.rs/workspace.rs provide command/artifact context, but
their output()/status() shortcuts are NOT the bounded retained helper contract.
No such new fixed helper mode is claimed implemented by this proposal. Delivery
owner must separately author/accept that exact mode in the existing binary;
Root alone can grant its source and compiler resource window. No new crate/helper
executable or source write is authorized here.

Proposed closed mode takes only source-bound recipe ID and explicit admitted
artifact/output locator DATA, never arbitrary program/argv/shell text. Cargo
executable must be selected by original trusted toolchain/artifact association,
absolute path/digest; no PATH search/wildcard or report-selected Cargo. Fixed
manifest/package/target/features/operation are Root-frozen recipe, no generic
command passthrough. The helper may not run a build while Root has not allocated
the corresponding compiler lease. Normal runner self-rebuild hazards stay open.

Before Job launch, parent reserves finite output storage in its actual owned
namespace and retains that allocation/creation obligations through all outcomes.
Helper opens only explicit NEW named stdout/stderr/control files, no overwrite,
append-existing or random retry. Choose stdout/stderr8MiB EACH with overflow
sentinels and finite aggregate16MiB plus fixed bounded control metadata; final
limits are profile constants recorded before launch, not supplied unbounded
lengths. Memory buffers/queues and stored bytes share the same finite caps. Any
file-create/ACL/provider/budget ambiguity refuses; B1 primitive alone does not
manufacture that output namespace. No inherited file or credential handles.

Reuse actual CaptureAttempt/Worker code after the single common-controller
extraction: retain real Child/stdin/stdout/stderr/JoinHandles/results; start readers
before large writes, bound per-stream/aggregate allocation, stop on overflow,
actual join every started worker and classify panic/disconnect/setup/write/read
failures independently. Cargo exit0 is not EOF or worker join, and an unbounded
Command::output is forbidden. No caller bool sets joined/success. Retain failure
owner and output obligations; write only bounded nonsecret DATA observations.

The parent keeps the ORIGINAL invocation clock/Job/actual helper process and
supervises the entire helper+Cargotree. Rust Instant is not serialized/reconstructed.
Any helper-local work cap is subordinate scheduling DATA, NEVER a replacement
original deadline or a parent budget renewal. Late startup/blocked helper cannot
extend parent's run/cleanup limits. Parent can request OWN Job stop on failure or
original expiry; repeated stop retains first cause. Source-bound helper reports
are DATA only, never native source/admitted parent/NativeProof/Ready capability.

For successful component observation, helper must actually join workers, record
bounded footer/counts/issues through the admitted output plan and exit naturally;
parent verifies same source/artifact/recipe/output association, actual helper exit,
Job0 and original deadline. It records reported worker facts as source-bound
qualification DATA, not an independent kernel/production worker proof. A boolean
footer, exit0 or symbolic lifecycle cannot create a native terminal receipt.

If parent kills helper while readers/writer/join are pending, worker processes
may be gone at Job0 but Rust joins were NOT observed. Never convert Job kill,
EOF or OS thread reclamation into ReadersJoined or successful capture. Retain
failure/join issues and output reservation; report Unknown/unproven74 as needed,
no automatic fresh attempt/storage release/cleanup success. This qualification
mode therefore does not solve production retained-worker recovery by itself.

Required actual cases: finite normal stdout/stderr+real joins; cap overflow on
each stream; descendant retains unread pipe; pending large input; partial worker/
file setup; helper panic/lost footer; original deadline killing blocked helper;
Job0 with missing joins still failure; output replacement/oversize/stale recipe
refusal. Console-attached and redirected-parent cases above remain separate.
No helper/source/manifest/unsafe/native grant or product/provider/Ready acceptance
is implied. Root/X review and actual original-owner manufacturer remain pending.

Additional primary references: [process flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags)
and [STARTUPINFO standard-handle semantics](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/ns-processthreadsapi-startupinfow).
