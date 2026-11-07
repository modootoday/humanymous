---
id: plan:root/20261107140500
title: Shared retained direct-child capture ownership and honest deadlines
status: draft
created: 2026-11-07T14:05:00Z
updated: 2026-11-07T14:05:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/rust-migration]
---

# Source-only contract proposal v0.1

Root identified actual existing claim debt in test-support process.rs. No code,
new dependency, subprocess, Docker or account execution is authorized by this
proposal. Shared owner API belongs to native-windows; Delivery and Lab should
not fork the primitive or call current helpers bounded cleanup proof.

## Existing debt

capture_streams starts an initial until before spawn, but synchronous startup
write occurs before readers and wait(timeout) creates a NEW deadline after spawn.
stop calls Child::wait without a deadline. Reader JoinHandles are discarded.
ManagedChild Drop calls stop and can block. Output channel completion is not an
actual worker join. A scoped thread containing capture autojoins at scope exit,
so its outer135s branch cannot establish a hard135s return/cleanup bound.
Empty stdin removes startup write blocking for a Docker CLI command, but does
not solve spawn/wait/Drop/reader/descendant issues. Existing successful actual
fixture evidence stays scoped to its observed result, not universal deadlines.

## Proposed ownership contract

```rust
pub struct CaptureAttempt { /* actual child/start operation and all workers */ }
pub struct CaptureDeadlines { /* one absolute run and cleanup Instant */ }
pub enum CapturePoll { Pending, ExecutionFailed, DirectChildExited }
pub struct CaptureSettled { /* private actual status + completed joins + output */ }
pub enum CaptureFailure {
    NotStarted(/* bounded redacted error */),
    Retained { owner: CaptureAttempt, /* sticky error and observed facts */ },
}
```

No Clone/Debug/Deserialize/public native identity constructors. Caller provides
absolute original deadlines created ONCE before launch; nested operations borrow
them, never Duration->new Instant renewal. Caller also owns independent outer
supervision. Direct-child capture is not an owned-tree containment capability.

CaptureAttempt owns actual Child once spawn succeeds, stdin worker, stdout and
stderr workers, every JoinHandle, bounded channels, byte caps and sticky error.
Before spawn returns it owns the actual in-flight launch operation, not a fake
PID/child. Partial pipe/thread creation must preserve all already-created owners.
Use thread::Builder::spawn's fallible result and retain failed setup obligations.
No scoped thread autojoin on an early return; no detached handle on timeout.

Start stdout/stderr drainers before scheduling nonempty stdin; input<=4096 remains
a byte bound, NOT a write-time bound. Retain writer handle/result as well. A read
limit reached before EOF is failure, never truncated success. Every stream has
its own finite cap and aggregate memory/thread limits are checked before spawn.
On overflow retain enough ownership to request cleanup; dropping the receiver
does not cancel a blocked OS read. Separate bytes received, EOF observed, worker
result, actual JoinHandle::join completion and worker panic facts.

poll(&mut self, now) performs only observations and reports pending facts.
Execution failure is sticky: forced exit0 cannot turn timeout/overflow into a pass.
Request stop on the actual owned direct child; observe exit with try_wait, not an
unbounded Child::wait. Never stop a caller PID. A tree backend must separately
terminate/query its real Job/cgroup and prove descendant settlement. A child may
exit while descendants still hold inherited pipes: EOF and reader joins remain
Unknown. Do not infer tree-empty from status/EOF/pipe closure.

finish(self) consumes only if direct child status, writer completion and actual
reader joins are ALL observed and sticky execution result is preserved. Otherwise
return the SAME owner and missing facts; original cleanup deadline expiry cannot
upgrade it. CaptureSettled exposes direct-child status/output with separate
natural/forced classification and explicit tree coverage=Unknown. Native tree
supervision remains a separate owner/proof. No Drop deletion, unbounded wait,
recursive cleanup, cleanup deadline renewal or Drop=settlement claim.

## Honest limitation of standard threads and OS calls

Command::spawn, synchronous OS writes/reads and Child::kill are not documented
hard-wall-clock-bounded operations. A mere deadline loop around them does not
create that guarantee. Keep a dedicated retained operation/worker owner if the
calling controller must regain control while an operation is in flight. Do not
call a blocking operation from a supposedly nonblocking poll or hold a mutex
whose worker may block while owning it. Stop request and completed stop are
distinct facts. An in-flight spawn can complete AFTER run expiry; its owner must
then retain/stop the actual child under the ORIGINAL cleanup budget.

JoinHandle::is_finished is not a substitute for actual join completion and cannot
establish a hard deadline on the subsequent join call. Joining in another thread
also creates another thread ownership obligation; do not hide an infinite chain
or call channel-send-before-return proof of a joined thread. Exact final joins
may require the independent host/job ceiling, or an accepted native cancellable
IO backend that avoids worker threads. This proposal does not claim a fully
hard-bounded standard-library-only capture implementation. Any pending operation
at the cleanup deadline must remain retained/Unknown and fail qualification.
Root must choose the bounded native IO/outer supervisor contract before a
stronger public API guarantee is implemented.

## Delivery and Lab migration

Docker create/inspect/cp/rm captures the Docker CLI's direct process only. A
successful rm status alone is not proof of daemon-side object removal; independently
inspect the exact owned stopped container ID under the same original operation
budget. CLI exit does not settle daemon work, and stop/timeout must not trigger
unowned or broad deletion. Empty-input direct CLI is a narrower supported use,
still subject to retained readers and independent outer ceiling. No running
container or workload execution follows from this design.

Lab's scoped browser diagnostic remains explicitly limited: reported135s is an
observation/request threshold, while scope-exit autojoin and current helper can
outlive it. Only the separately observed outer job60min ceiling is the available
hard termination boundary, subject to Root acceptance; it is not cleanup proof.
Replace it with common retained-owner supervision after acceptance/validation,
not another timeout wrapper or a local process-helper fork.

## Verification before adoption

Pure state tests cover one conserved budget, failure stickiness, partial startup,
missing writer/reader joins, expired cleanup returning same owner and direct-child
versus descendant-Unknown outcomes. Actual allocated local fixtures separately
cover blocked stdin, stdout/stderr flood, no EOF after direct-child exit due to an
owned descendant, partial worker setup, forced exit0 and completed joins. Tests
must retain their own actual process tree; no orphan-inducing generic fixtures.
No ignored/filtered fixture entry is counted as an executed parent case. Native
OS cancellation, Docker daemon settlement and full product Docker e2e each need
their own actual venue and source-bound evidence. Existing helper consumers need
a source inventory before replacing legacy behavior; no silent global rewrite.

Adversarial verdict: needs-work. P0 current universal bounded-cleanup claims are
invalid; P0 std-thread deadline guarantees cannot be manufactured by signatures.
Required first step is honest consumer limitations plus an accepted retained-owner
and outer-supervisor/native-IO design, before implementation or stronger claims.

## Concrete safe reusable API shape for review v0.2

```rust
pub struct CaptureLimits { /* validated stdin/per-stream/aggregate caps */ }
pub struct CaptureProgress { /* child status, force flag, IO/EOF/join facts */ }
pub struct StopRequested; // channel request accepted, never termination proof
pub struct CapturedDirectChild { /* actual status, bounded output, sticky failure */ }
pub struct RetainedCaptureFailure { /* same CaptureAttempt + reason/facts */ }

impl CaptureDeadlines {
    pub fn from_original(run: Instant, cleanup: Instant) -> Result<Self, InvalidBudget>;
}
impl CaptureAttempt {
    pub fn begin(command: Command, input: Vec<u8>, limits: CaptureLimits,
                 deadlines: CaptureDeadlines) -> Result<Self, StartRefusal>;
    pub fn poll(&mut self, now: Instant) -> CaptureProgress;
    pub fn request_stop(&mut self) -> Result<StopRequested, ControlUnavailable>;
    pub fn observed_result(&self) -> Option<&CapturedDirectChild>;
    pub fn join_under_outer_supervisor(self)
        -> Result<CapturedDirectChild, RetainedCaptureFailure>;
}
```

begin starts a FALLIBLE retained supervisor thread, transferring owned Command
and bounded input into it before that thread calls potentially blocking spawn.
Its successful return owns the JoinHandle/control channels, not a fabricated
Child. Thread creation failure is StartRefusal with no launched child. Supervisor
retains actual child and all writer/reader workers immediately as they arise.
If an OS operation blocks, the attempt still owns that supervisor; public poll
only uses try_recv/atomics and never waits for a mutex held across OS calls.
request_stop uses a bounded try_send/control flag, no OS kill/wait on the calling
thread; supervisor action may be delayed and must not be reported complete.

observed_result borrows a delivered outcome; it does NOT consume the attempt or
prove supervisor join. IO worker joins may be completed by the supervisor, but
the supervisor's own JoinHandle remains owned by CaptureAttempt. Final reclaim
requires join_under_outer_supervisor, explicitly BLOCKING and allowed only under
the independent enforced host/job ceiling. This method performs actual joins,
checks returned observations against original deadlines, and preserves a failure
classification if work completed late. It has NO hard wall-clock return promise.
No is_finished-before-join trick upgrades that promise.

Drop must not call join/wait or manufacture proof. Safe standard-library Rust
cannot automatically preserve a dropped JoinHandle as a caller-owned obligation;
therefore mark CaptureAttempt and RetainedCaptureFailure must_use, expose explicit
handoff to the owning controller, and document drop as abandoned/Unknown (even
though OS cleanup requests may be attempted by the still-owned supervisor).
Root must accept that limitation for diagnostic-only use or require a native
service/tree owner that survives the caller. Never describe ordinary Rust Drop
as a linear-type guarantee. Long-lived registry/reaper ownership would itself
need finite capacity, explicit shutdown and actual joins, and is not implicit here.

This is concrete safe API design, not available source. Stronger hard-bounded
cleanup awaits native cancellable IO plus actual tree supervision. Delivery/Lab
may only consume this API after acceptance, implementation and allocated tests;
current helper must remain labelled with its existing limitations meanwhile.

## Accepted source implementation checkpoint v0.3

Root accepted the outer-supervisor contract for CI/Lab: absolute run/cleanup
thresholds are observations/requests under a responsive trusted OS; independently
enforced genuine job deadline is the hard ceiling, never cleanup proof. Root
authorized source implementation without dependencies/unsafe or actual fixtures.

Actual authored interface now uses synchronous CaptureAttempt::start(&mut Command,
Vec<u8>,CaptureLimits,CaptureDeadlines), borrowed request_stop(), nonblocking
poll(now), and consuming settle_joined(now). This refines v0.2's async-begin idea:
start/stop/final joins explicitly may block under the accepted outer supervisor.
No in-flight-spawn thread/registry or implicit deadline promise is introduced.
Shared private piped_child is extracted from existing ManagedChild::piped without
changing legacy flags/pipes. Existing capture helpers and ManagedChild remain
legacy behavior; no consumer migration or cleanup guarantee follows automatically.

New capture_attempt.rs owns actual Child and every started IO JoinHandle, with
separate worker.rs. Readers start before stdin writer. Partial setup returns
RetainedCaptureFailure::into_owner, keeping the same actual process/workers.
poll only observes try_wait/channels; no write/wait/join/kill or worker-held mutex.
request_stop may block in OS and records sticky ForcedStop before requesting it.
settle_joined returns same owner while child/workers remain pending, then performs
ACTUAL joins (potentially blocking even after is_finished) before private result
construction. Private result fields cannot be caller-constructed/mutated; getters
expose observed status and private bounded outputs, with sticky issues.

Run/cleanup thresholds are recorded as met ONLY when the required complete facts
are observed strictly before each original threshold; late completion cannot
renew or erase a missed deadline. Cleanup additionally requires closing any
remaining partial-setup pipes. A settled failed execution is distinct from an
unsettled retained owner, and no descendant/tree proof is represented.

Four authored PURE tests cover conserved original thresholds, late completion,
output-complete versus actual joins, and finite allocation/EOF boundary. These
tests have NOT_RUN; no process/worker actual fixture was executed. Partial worker
creation, actual retained-owner identity, blocked stdin/EOF, overflow/descendants
and actual joins remain next allocated execution gates. No test result is inferred
from rustfmt or source inspection.

Read-only consumer inventory and exact source hashes are in private
20261107141000-windows-runner-retained-capture/source-handoff.json. Inventory
includes Windows harness/service/pilot, Lab runner/runtime browser/Docker/Go,
Audit tests/child lifetime, Browser desktop/bootstrap, .NET package/hosted, and
Python component/WSGI support. No foreign consumer edits have been made. CLI
watchdog intentionally-unread-pipe fixtures cannot use auto-draining capture:
a common retained direct-child pipe-retention interface needs separate scope;
using this capture would destroy the intended backpressure experiment.

No timeout branch discards a JoinHandle. Owners/failures are must_use and require
caller retention/explicit settlement. Safe Rust does not enforce linear use;
abandoning the owner causes Unknown and is not a cleanup strategy. No Drop wait,
hidden reaper, leak, deliberate detach or claimed Drop settlement is implemented.

## Root clock critique addressed v0.4

Root identified P1: a caller-supplied past Instant to public poll/settle could
falsely mark deadline facts as timely. Public methods are now poll() and
settle_joined(), with no observation-time argument. poll samples Instant::now
INTERNALLY AFTER gathering actual child/channel facts; settlement samples again
after actual joins and closing remaining pipes. Caller-provided absolute deadline
values remain explicit unverified policy inputs, but callers cannot backdate the
observation clock. Only the private deadline ledger accepts an injected Instant
for pure state tests. Four pure tests remain authored NOT_RUN; no compiler or
runtime result follows from this source correction. Partial setup retains
SetupFailed and cannot claim successful IO when a worker was never started.

## Next actual direct-child parent cases and retained cleanup proposal v0.5

Root requested SOURCE preparation and exact execution proposal only; no actual
process fixture allocation has been granted. Reuse the ONE existing ignored
process::tests::subprocess_fixture entry point and its fixed known modes. Do not
add arbitrary executable/command inputs or count helper invocations as parent
test passes. Proposed new parent module process::capture_attempt::actual_tests
is ignored by default; future allocation selects EACH parent exact name with
--ignored. Existing pure filter should name ::capture_attempt::tests explicitly,
so no future default filter can accidentally execute subprocess cases.

Proposed exact parent names and observables:

1. retained_normal_stdout_stderr_and_actual_joins: existing both mode, cap4096 per
   stream, one original run3s/cleanup5s before spawn; actual status0, both known
   outputs, all three IO worker joins returned, no sticky issue. Direct child
   only; no descendant containment or arbitrary command claim.
2. retained_nonreading_child_expires_original_run: existing hang mode (finite5s),
   4096bytes input, run100ms/cleanup5s selected before spawn; observe original run
   expiry, request stop on borrowed actual child, retain same owner until actual
   exit and all started joins. This is a nonreading child + expiry test, NOT
   proof that stdin write blocked. Failure cannot renew either original threshold.
3. retained_stdout_cap_is_sticky: existing flood mode, cap128, observed StdoutFailed
   irrespective of whether actual child exits0 or broken-pipe nonzero; no
   truncated success. Stop if still running; require actual final joins.
4. retained_stderr_cap_is_sticky: existing stderr-flood mode, analogous exact
   StderrFailed proof, independently retain/drain stdout.
5. retained_stop_request_preserves_observed_exit_zero: existing finite normal mode,
   first OBSERVE actual status0, then request_stop; method must record ForcedStop
   and refuse execution_succeeded despite status0. No actual kill is claimed in
   this case because status was already observed. Case2 separately exercises an
   actual direct-child kill request and its real platform-specific exit status.

The default CaptureAttempt is an auto-draining capture and cannot demonstrate
CLI's intentionally unread stdout/stderr backpressure. A shared retained direct
pipe owner must use the same actual child/status/deadline backing while retaining
unread pipes in the owner; it must not return a fake CapturedDirectChild claiming
EOF/IO joins or call legacy ManagedChild cleanup authoritative. Its exact API is
a separate prerequisite for the CLI parent; no implementation fork is proposed.

### Blocked stdin is an explicit separate prerequisite

Public startup input<=4096 may fit fresh Windows/Linux pipe buffers completely.
hang/nonreading mode plus that input cannot establish an actual blocked writer.
Do not label a pending child or an unjoined worker as proof of pipe pressure.
Options for Root decision: a separately bounded synthetic-input profile allowing
enough fixed nonsecret bytes plus explicit writer progress observation, or a
native known-small pipe geometry/cancellable IO capability. Neither is silently
added to the production startup input policy or existing unsafe exception. A
cfg(test)-only bypass of public validation would test the private worker, not
claim the public 4096byte API actually blocked. Until a supported mechanism and
observable are accepted, actual blocked-stdin proof stays NOT_RUN.

### Failure handling and outer owner

Parent creates original absolute thresholds BEFORE spawn and keeps CaptureAttempt
in one explicit owner slot. A start failure retaining an owner is recovered with
into_owner; every failed settle returns it to that SAME slot before another
branch/assertion. Never unwrap/panic while an unsettled owner is temporarily
unbound. Assertions are deferred until actual child status and all started joins
are obtained, even when execution is already failed. No timeout branch discards
workers or relies on Drop wait. On expiry request stop, poll with real internal
clock, attempt final joins only when reported ready under the outer supervisor.

All helper modes have finite fixed behavior (hang body5s maximum under responsive
OS; floods finite8192bytes) and create no children. Tool session is retained until
terminal, and Root must provide the separately accepted independent outer
supervision/ceiling before execution. If cleanup deadline expires, keep failure
sticky and preserve the owner; convergence after expiry is reported late, never
a clean pass. If outer supervision terminates a stuck parent, actual child/worker
cleanup remains Unknown; finite helper body is an assumption/residual, not Job
empty proof. No spawned descendants, inherited-pipe-holder fixture, account, SCM
or native bridge API is included in this proposed allocation.

Actual worker-join evidence must come from returned JoinHandle::join outcomes
and private settled result construction, not a channel result/is_finished alone.
If exact joined-worker counts are exposed for the fixture receipt, record them
from real Worker joined states; do not add a constant-true getter. Partial worker
failure requires a separately reviewed deterministic injection seam and cannot
be inferred from these five parents. All five are proposed NOT_AUTHORED/NOT_RUN
until their full source and cleanup contract are reviewed for allocation.

## Accepted synthetic seam and authored actual parents v0.6

Root accepted a PRIVATE cfg(test) synthetic-input entry capped8MiB, sharing the
SAME start_inner, writer/drainers and actual owner; no public/release input cap
increase. Production CaptureAttempt::start still passes4096. Test-only atomic
writer observations mark not-entered/entered-before-write/write-returned; these
show a pending write operation, NOT kernel-blocked-versus-descheduled certainty.
No native pipe geometry/unsafe/dependency is introduced. CapturedDirectChild now
reports started/joined IO worker counts computed from actual Worker states,
including joined panic outcomes, never a constant-true fabricated receipt.

Source actual_tests.rs now authors FIVE ignored-by-default parents, reusing the
existing ignored subprocess fixture and its both/hang/flood/stderr-flood modes:

- process::capture_attempt::actual_tests::retained_normal_stdout_stderr_and_actual_joins
- process::capture_attempt::actual_tests::retained_synthetic_pending_stdin_expires_original_run
- process::capture_attempt::actual_tests::retained_stdout_cap_is_sticky
- process::capture_attempt::actual_tests::retained_stderr_cap_is_sticky
- process::capture_attempt::actual_tests::retained_stop_request_preserves_observed_exit_zero

This supersedes v0.5's nonreading public-payload case with the explicit8MiB
synthetic pending-writer parent. Its original run1s/cleanup7s precedes allocation
and spawn. Parent observes writer-entered and input-not-completed, then original
expiry, stop request, actual child status, write-returned and three actual joins.
Fixed hang helper sleeps5s and creates no children. Normal/flood/stop-classification
parents have original run3s/cleanup7s, finite helpers and1024byte flood caps above
ordinary harness headers. Stop-after-observed-exit0 is explicitly a REQUEST
classification case, not a claim that the OS killed a process with exit0.

Cleanup helper keeps owner in one slot, recovering every settle error via
into_owner before another branch. Assertions occur only after actual settlement;
partial-start failure is settled before assertion failure. It never invokes
Child::wait or resets thresholds. Potentially blocking stop/joins require the
independent outer supervisor; late cleanup remains a failure. All five actual
parents, their helper invocations, the new compile/check source and worker-count
observations are AUTHORED_NOT_RUN pending an explicit Root allocation. Prior
pure14 evidence remains attached to its earlier exact capture source.

## CLI unread-output common backing proposal v0.7

Source design only, not a second process lifecycle implementation. Extract ONE
private OwnedDirectChild backing from current CaptureAttempt to own actual Child,
PID/status, sticky stop/error state and original clock ledger. It uses the SAME
piped_child factory, internal real-clock observations, borrowed actual kill
request and retained failure semantics. CaptureAttempt composes that backing with
its existing drainer/writer workers; a distinct RetainedDirectPipeAttempt composes
it with intentionally held pipes. No legacy ManagedChild wrapper or PID constructor.
Actual backing extraction/API implementation requires Root's scoped review.

```rust
pub struct RetainedDirectPipeAttempt { /* one OwnedDirectChild, held actual pipes */ }
pub struct SettledClosedPipeChild { /* private status/close facts, NO output/EOF proof */ }
impl RetainedDirectPipeAttempt {
    pub fn start(command: &mut Command, deadlines: CaptureDeadlines)
        -> Result<Self, DirectPipeStartFailure>;
    pub fn close_input(&mut self);
    pub fn poll(&mut self) -> DirectChildProgress;
    pub fn request_stop(&mut self) -> io::Result<()>;
    pub fn read_stdout_prefix(&mut self, expected: &[u8]) -> io::Result<()>;
    pub fn read_stderr_prefix(&mut self, expected: &[u8]) -> io::Result<()>;
    pub fn settle_closing_pipes(self)
        -> Result<SettledClosedPipeChild, RetainedDirectPipeFailure>;
}
```

No output background reads or writer threads are started in the pipe-retention
profile. close_input closes only the owned input endpoint. Prefix methods admit
one fixed expected prefix<=256bytes per stream, allocate only that bound and
read_exact that many bytes from the retained endpoint, never drain beyond it.
These synchronous methods may BLOCK and explicitly require independent outer
supervision; they do not implement an internal read-time guarantee. Prefix failure
is sticky and retains the owner; no pipe ownership escapes or unjoined reader is
created. The CLI parent reads its controlled fixture's writer-entered marker,
then holds the remaining output unread. Marker observation demonstrates writer
entry, not certainty of kernel blocked state versus descheduling.

poll observes actual direct-child exit without reading outputs. request_stop is
an actual borrowed-child OS request with the same potentially blocking caveat.
settle_closing_pipes requires actual observed child exit, closes all held pipe
endpoints and returns a DISTINCT private result recording child status and closed
handles. It must not return CapturedDirectChild, label held/unread output EOF,
claim three IO joins, or infer descendant/process-tree settlement. If child exit
is pending it returns the SAME owner. All clocks are sampled internally and
threshold failures stay sticky; original run/cleanup thresholds are never reset.

CLI watchdog positive0 versus watchdog74 is read from actual child ExitStatus;
the fixed8MiB synthetic-pressure source, writer-entry marker and intentionally
held remainder are separately reported. A watchdog exit74 before an observed
marker fails the intended parent case. Closing parent pipe handles after child
exit is an actual local handle-close fact, NOT daemon/service/Job cleanup proof.
Public result ownership and Drop limitations remain explicit; no new dependencies,
unsafe, process creation path or automatic account/SCM authority is proposed.

## Approved common backing source checkpoint v0.8

Root approved the v0.7 SOURCE refactor after the five-parent source was frozen in
1e05b7ba0cd8a160443dfab45a81c5f2c34ea784. Actual source now has ONE private
process/owned_child.rs OwnedDirectChild holding Child/PID/status/stop/errors and
the original deadline ledger. CaptureAttempt and the separate direct_pipe.rs
RetainedDirectPipeAttempt both compose that owner and the same piped_child factory.
No new dependency, manifest, unsafe or actual execution is introduced.

The published direct-pipe API is start(&mut Command,CaptureDeadlines)->io::Result,
close_input(), poll(), request_stop(), read_stdout_prefix()/read_stderr_prefix()
and settle_closing_pipes(). No fallible setup follows successful spawn in that
profile. Prefixes are once per stream,1..256bytes, bounded read_exact without any
background reader/draining; synchronous reads/stop/release still require outer
supervision. Prefix error marks failure while retaining the borrowed owner and
remaining endpoints. Only complete exact match records prefix observation.

DirectChildProgress getters report actual status, sticky issues, retained endpoint
booleans and prefix observations. Settled result is named ClosedPipeChild with
PRIVATE fields and getters pid/status/released_pipe_ends/prefixes_observed/issues.
It is distinct from captured output and reports std-owned endpoint release ONLY,
not native CloseHandle acknowledgement, EOF, IO worker joins or tree cleanup.
Pending exit returns RetainedDirectPipeFailure::into_owner with same actual Child
and held pipes. No raw handle, Child getter or caller-PID constructor is exposed.

One new PURE prefix budget test is authored; the existing four pure clock/cap
tests and FIVE ignored actual parents are preserved with the stop-classification
parent updated to the same private backing. CLI owner was notified of exact API
and pressure-entry-versus-exit observations; its consumer and devdependency remain
separate owner/Root work. No old pure14 or five-parent source hash qualifies this
refactor. All new static/pure/actual direct-pipe and capture regression gates are
NOT_RUN pending an explicit Root allocation. No automatic retry is authorized.

## Common backing pure/static execution checkpoint v0.9

Root allocated ONLY the lower pure/doc/static batch on immutable173008f0, which
contains unchanged7a1 and3a0 source ancestors. Native Windows MSVC jobs1/inc0,
locked/offline: fmt FIRST, test catalog40, capture pure4 (36filtered), prefix pure1
(39filtered), bridge pure4 and separate positive1/compile_fail1 Rustdocs all pass;
both packages all-target check and strict Clippy pass. Selected input closure
68source/manifests/lock/toolchain +15registry/1217files +76selected tool files and
config/flags has zero differences. Compiler reservation returned at terminal.

No actual parent or native bridge call ran. The FIVE ignored parents are compiled
and listed, not executed; source ignore state and exact names are preserved in
common-owner-pure-summary.json. The older14pure receipt remains historical.
Passing negative Rustdoc has its positive import control but no separately captured
E0451 diagnostic output, so no stronger diagnostic-code claim is made. Captured
128.657GiB admission free space is DISK, not RAM; original physical availability
was not persisted and later observations do not backfill it. All L2 actual gates
and CLI consumer0/74 remain pending their explicit execution decisions.

## User-authorized actual-five controller and cooperative barrier v1.0

The user authorized stronger actual Windows testing on this terminal. Root
accepted a narrowly scoped independent Rust controller for exactly FIVE existing
capture-parent selectors. No new Win32 API, unsafe exception, manifest, account,
SCM or privilege action is required. Source reuses actor.rs create/configure/
assign/terminate/query Job operations through safe ControlledFixtureJob borrowing
the REAL std Child handle. std Child remains sole process-handle owner; no caller
PID or borrowed-handle ownership cast is admitted.

The runner controlled-capture mode admits fixed case IDs normal, pending-input,
stdout-cap, stderr-cap and stop-after-zero, the declared shared-test executable
name with expected SHA256, internally fixed selectors and absolute fresh work.
Private stdout/stderr logs are create-new regular files, not controller IO threads.
The nested lock adds ONLY the accepted bridge0.1 package/native-security edge;
registry versions/checksums and manifests remain unchanged. Root reviewed the
exact diff; locked offline metadata passes, with no compiler/execution claim.

Each actual test body FIRST waits at a five-second cooperative no-grant barrier,
BEFORE fixture-child spawn or its capture budget. Bounded closed local JSON
matches nonce, exact selector and current actual parent PID. Controller assigns
the real held parent to its own kill-on-close Job and observes nonempty assignment
BEFORE atomically publishing the grant. This is trusted-fixture synchronization,
NOT producer authentication or arbitrary/untrusted-launch authority. Preassignment
runtime startup and blocked local filesystem/kernel calls remain limitations.

Outer original run10s/cleanup15s precede process creation, include assignment/grant,
and are never renewed. Held Child::try_wait observes actual status; expiry is
checked AFTER observation. Cleanup terminates only the owned assigned Job and
separately observes parent exit, Job ActiveProcesses0 and the original deadline.
Unassigned failure uses only the actual held cooperative parent, never a PID;
empty unused Job is not descendant containment. Termination error stays failed.

Report natural exit/code, grant/assignment, run expiry, termination request/error,
actual parent exit/Job-empty and cleanup deadline separately. Known late cleanup
is not timely success. Unknown cleanup retains owner on its controller thread
BEFORE report IO; no unsafe Send or detach. Controller exit/Job closure requests
kill-on-close but cannot manufacture convergence. All five actual cases and two
controller pure tests remain NOT_RUN in this source checkpoint.

Root's separate CEF cold-build window holds new native compilation until explicit
terminal release. Existing user authorization for these five ordinary cases
persists afterward, with fresh commit<85percent, available-memory/disk guards and
exclusive target admission. No WSL/UNC, foreign kill, native bridge query/create,
account/SCM/privilege or producer activation is included.

## One shared cooperative barrier API v1.1

CLI requested the same verifier instead of copying nonce/file/PID policy. The
barrier codec/wait now lives once in test-support process/controlled_barrier.rs:
await_controlled_fixture_grant(expected_selector:&str)->io::Result<()> and
encode_unverified_controlled_fixture_grant(child:&Child,selector,nonce)->bytes.
Encoder takes an actual owned Child, not a caller PID. Typed closed metadata
schema is unchanged; selector grammar/256byte bound, nonzero lowercase32hex
nonce, pid,4096byte wire and exact field matches fail closed. Wait samples real
time after reading against one five-second threshold; local IO may still block.

These APIs expose NO verified producer/Job capability. Controller's independent
fixed-selector/executable hash/actual Job assignment gates still precede atomic
publication. Shared wait only synchronizes controlled trusted test code; arbitrary
or hostile preassignment execution is outside this profile. Existing FIVE parents
call it FIRST with literal exact selectors. The controller consumes the same codec.
Two pure codec/boundary cases are authored NOT_RUN. Current fixed controller
allowlist does NOT yet admit the CLI watchdog binary/selector; that exact consumer
extension follows provider-five proof and its separate accepted runtime scope.

Initial fmt encountered a transient Windows mapped-file1224 write error, then
the same owned-source formatting retry passed without foreign termination or
compiler execution. Source/fmt/spec and accepted nested-lock metadata are ready;
all new controller/barrier compiler/pure/actual gates remain held for the CEF
exclusive window. This extraction does not retroactively qualify older hashes.

## Actual-five terminal checkpoint v1.2

After Browser's exclusive window returned, Root released the existing grant on
immutable5cd2cdc9/controllerea024229. Fmt-first, barrier pure2 (40filtered),
controller pure2 (38filtered), both package all-target check/strict Clippy and
controller build all passed. No qualification command failed. Selected input
closure109source/31registry2042files/76tools/config/flags stayed equal. Both
executables were copied and hash-pinned before returning the compiler reservation.

Five actual cases ran ONE AT A TIME with fresh commit<85percent, recorded
physical availability>20GiB and disk>20GiB guards. Every outer controller exited0,
every selected parent reported1passed/0failed/0ignored/0measured/41filtered,
actual natural parent exit0 and Job-empty were observed under original10s/15s,
and the private actual-join result passed3started/3joined assertions. Pending
input8MiB is the private SYNTHETIC seam; raw observation records writer-entered/
pending/returned and inner exit1. Stop-after-zero is a request classification,
not a claim of forcing an OS exit0. Helpers are not extra parent test counts.

All five real Job assignments preceded cooperative grant publication. Runtime
source and copied-binary hashes remained equal. The actual supervisor lease ended
with unknown cleanup0 and no retained live owner; private work/log/grant files
remain intentional evidence. The48-attachment receipt preserves compile and each
runtime admission, statuses, outer budgets and actual native cleanup facts.
An initial receipt-path concatenation mistake was preserved and corrected without
any rerun or change to raw test output.

Outer live-parent timeout, missing-grant and unassigned-failure paths were NOT
fault-injected. No arbitrary hostile process/full-tree or universally bounded
kernel promise follows. Legacy capture debt and CLI0/74/native bridge/producer/
service/privileged/hosted gates remain open. This checkpoint unlocks Root review
of the exact next consumer/controller extension; it does not grant that extension
or any new runtime automatically.

## Accepted closed CLI consumer profile v1.3

After the five-provider receipt, Root accepted one additional fixed profile in
the SAME controller: cli-watchdog maps only to literal
final_delivery_watchdog_with_retained_unread_pipes and
watchdog_boundary-<16lowerhexCargoHash>.exe with independently expected SHA256.
The five capture mappings remain fixed; profile artifact names cannot cross.
No arbitrary selector/argv/PID, native call, manifest or lifecycle owner is added.
Two new pure mapping/name-negative cases are authored before qualification.

CLI owns the ONE actual parent invocation, with FOUR distinct0/74 subcase outcomes
counted separately from native provider5. Parent original8s/12s starts BEFORE its
shared barrier; outer original10s/15s remains independent, with no renewal. Native
L2 builds/freezes/hands off the controller and does not duplicate CLI execution.
Native5 evidence remains historical to its exact old controller hash.

CLI sourcec57cf0c6 bakes a separate service-fixture executable path, unlike the
native helpers' current_exe. CLI pins BOTH parent and fixture hashes and retains
an explicit artifact-write lease on the actual baked fixture path until runtime
terminal; copying only the parent is insufficient. Root was notified. This is
trusted component source/artifact closure, not producer or loaded-image attestation.
Actual CLI gate remains NOT_RUN until controller checks/build/hash handoff plus
fresh resource admission. SCM/accounts/privileges/native producer remain excluded.

## Closed CLI profile static terminal and handoff v1.4

On immutable24004321/controller49e488ea, four pure controller cases pass with
38filtered and0failed/ignored/measured. Fmt FIRST, all-target check, strict Clippy
and build pass; selected source/registry/config/tools/flags stay equal. Copied
controller SHA2563ff6cb8e35454571a682e4a2d224d62e4bd3a4852dbc398d81d07ff2c7d474c0
was handed to the CLI owner immediately after compiler target return. Native
owner executes no duplicate CLI parent; canonical build receipt is registered as
cli-controller-profile-summary.json.

Controller parent_test_passed is actual exit0 plus timely native cleanup, NOT a
Rust-harness selector/footer parser. CLI independently requires exactly one
selected parent and all four correct subcase observations, along with both
parent and baked fixture artifact pins. CLI reported its sole actual invocation
terminal with parent1/four subcases, but that actual durable receipt belongs to
the CLI owner and is not manufactured by this static build handoff.
