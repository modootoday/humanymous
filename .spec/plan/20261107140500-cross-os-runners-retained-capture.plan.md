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
