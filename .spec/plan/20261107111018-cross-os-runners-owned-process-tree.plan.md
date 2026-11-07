---
id: plan:root/20261107111018
title: Draft shared bounded owned process-tree lifetime
status: draft
created: 2026-11-07T11:10:18Z
updated: 2026-11-07T11:10:18Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/rust-migration]
---

# Shared owned-tree lifetime proposal v0.2

DESIGN ONLY. Root requested one shared test-support lifecycle for SDK/compiler/
Python controller descendants. No source, dependency, privilege, runtime workload
or new unsafe exception is approved here. Python currently refuses before setup;
that refusal remains until the shared contract is accepted and implemented.
Existing ManagedChild owns only the direct child and stays labelled accordingly.

## Ownership invariant

Never authorize a negative-PGID signal by procfs UID, environment/module marker,
path, same UID or a PID saved AFTER reaping. Those observations are diagnostics.
A signalable group capability must own a still-live OR deliberately UNREAPED
original group-leader process object for the entire signal interval. No public
constructor accepts a numeric PID/PGID as that capability. Ordinary wait/try_wait
that reaps the leader is forbidden until group/cgroup finalization ends. A global
waitpid(-1) reaper cannot share these children. No foreign group/process is killed.

If a reviewed backend needs a companion guardian, that executable must be a declared, digest-pinned Cargo
artifact from test-support, not a shell/script/ignored no-op test fixture. Parent
spawns and retains that direct child. On Unix the guardian establishes its own
fresh session/group BEFORE any SDK child; process_group(0) alone is insufficient.
Guardian reports only bounded control facts. Its private inherited control channel
is ownership plumbing for an actual owned child, NOT native peer attestation and
must never become a bootstrap authorization primitive.

The SDK/controller is a CHILD of the guardian. Its exit is separately reported;
it is not the guardian/group-leader exit. The guardian remains alive while group
ownership is needed. Parent observes leader state with a nonreaping native wait
(e.g. Linux waitid WNOWAIT), not ManagedChild::wait/try_wait. If the guardian dies,
retain its unreaped identity until all final signals/observations finish. A group
ID cannot be borrowed from an already reaped child. Unknown pin state forbids
further group signaling and yields Unknown, not best-effort foreign PID killing.

## Containment strength is an explicit prerequisite

Windows target: existing owned Job Object kill-on-close plus actual job-empty and
owned child wait facts; nested SDK jobs must be qualified. Reuse the Windows
owner boundary; no account/token logic is copied into test-support.

Linux selected candidate: a SYSTEM manager transient service with a fresh,
controller-owned unit name, cgroup-v2 allocation and ordinary SDK identity. A raw
PGID guardian is no longer the selected full-tree backend. Retain a live D-Bus
manager connection, unique manager owner/boot identity, unit reference,
InvocationID, cgroup native handles, IO endpoints and stop-job ownership. A unit
name alone is not a capability; reconnect/restart or mismatched invocation is
Unknown and never authorizes stopping a replacement invocation.

Propose RemainAfterExit=yes so SDK main exit does not discard the unit before
cleanup proof. Do not combine this with systemd-run --pipe/--wait. Prefer reviewed
D-Bus transient-unit FD properties and bounded Rust readers. Failed units remain
retained; no --collect, ResetFailedUnit or release before terminal evidence.
RuntimeMaxSec behavior after active-exited is NOT an assumed cleanup watchdog;
actual qualification must establish an independent absolute cleanup mechanism.
Exact D-Bus methods/properties and safe dependency edges remain unaccepted.

User=runner alone is trusted native-test supervision, not a hostile-child sandbox:
sudo, Docker, system bus launch brokers and writable cgroup ancestors can escape
it. Full hostile-child containment requires actual denial of ancestor/sibling
migration, allocator/guardian capability reopening and external privileged
launchers. If the authorized host cannot establish the requested profile, return
Unsupported before creating any process/unit. No hidden sudo/account expansion.
After a unit or child exists, all failures retain the actual recovery owner.

Qualification must inspect actual Ubuntu PID1, system bus, unified cgroup mount,
manager authorization, nonroot SDK identity, native allocation and unit retention.
Exercise only owned setsid/double-fork, ignore-TERM, migration and broker escape
attempts. Group-only supervision cannot silently qualify a full-tree consumer.
macOS/other Unix full containment stays Unsupported pending a reviewed backend.

cgroup.events populated=0 proves no live population, not descendant reaping.
Report SDK reap, guardian reap (if used), adopted descendant reaps and native
population separately. Require a concrete reviewed subreaper/adoption policy or
report descendant_reap=Unknown; consumers requiring no leftover processes reject
Unknown. Never infer reaping from cgroup.procs absence or successful removal.

## Lifecycle, cancellation and IO

Nominal states: Prepared -> NativeOwnerRetained -> NativeContainmentAttached ->
SdkRunning -> SdkExitObserved -> Stopping -> ContainmentEmptyObserved ->
RequiredReapsObserved -> ReadersJoined -> Completed. Any started state may instead
enter CleanupRequired or Unknown while retaining its actual owner. A guardian is
optional backend machinery, not a substitute for the selected systemd capability.
SDK exit0 alone never advances to Completed. Parent/guardian process exits and
SDK exit codes are separate fields, including when final kill terminates guardian.

Use one absolute run deadline plus a separate bounded cleanup reserve; repeated
stop/cancel/TERM cannot reset either. On normal SDK exit, timeout, output limit,
caller cancellation or malformed control frame, stop the owned group/tree before
joining stdout/stderr readers. Do not wait indefinitely for pipes inherited by
surviving grandchildren. No successful truncation or detached reader threads.

Guardian must survive the TERM grace phase so it can escalate after parent death;
any signal handling uses a reviewed safe primitive, not new ad-hoc unsafe handlers.
SDK exec must not inherit guardian-only control writers or signal dispositions.
Control EOF means parent loss: guardian stops only its own still-anchored tree and
uses bounded escalation. Parent-death cleanup has no surviving reporter, so it is
not a fabricated acknowledged success. A guardian dying before cleanup cannot
promise later escalation; full containment policy must account for that failure.

Parent can TERM/KILL the retained owned group while the leader remains pinned;
Linux cgroup.kill is preferred for the full allocated subtree. Never signal ID1,
zero or an unvalidated process group. Repeated finalization stays within the same
owned capability. Only after native empty observation and all intended signals
are complete may the leader be reaped. Then never signal that former numeric ID.
Empty cgroup and reaped child are separate: zombies/readers and escaped processes
are not silently represented by cgroup.procs absence.

IO uses bounded nonblocking read/write/poll over stdout, stderr and control, or an
actually cancellable owned equivalent. Closing an FD in another thread is not
assumed to unblock its syscall. No unbounded blocking BufRead worker can survive
Unknown while its lifetime is forgotten. If cancel/join cannot be proved by the
deadline, report Unknown with incomplete capture while retaining the ACTUAL owner,
including native handles, unreaped processes, readers and resource reservation.
No stdout/stderr/control payload or environment enters Debug/error text.

## Proposed shared API

```rust
struct VerifiedExecutableObject; // retained opened object and execution binding
struct NativeTreeAllocation;     // verified Job or retained systemd invocation
struct OwnedTreeCommand;         // bounded program/args/cwd/explicit environment
struct OwnedProcessTree;         // unique native lifetime and reader ownership
struct TreeObservation;          // independent facts; never resource ownership
struct CompletedTree;            // bounded private bytes, terminal proof
struct TreeDeadlines;            // absolute run plus cleanup reserve
struct TreeOwnerFailure;         // owns partial native allocation on spawn failure

enum SpawnResult {
    NotStarted(PreSpawnError),
    Started(OwnedProcessTree),
    Partial(TreeOwnerFailure),
}
enum Progress { Running, CleanupRequired, Unknown, Terminal }
OwnedProcessTree::start(...) -> SpawnResult;
OwnedProcessTree::capture(&mut self, input, limits) -> TreeObservation;
OwnedProcessTree::cancel(&mut self, reason) -> TreeObservation;
OwnedProcessTree::advance_cleanup(&mut self) -> TreeObservation;
OwnedProcessTree::finish(self) -> Result<CompletedTree, OwnedProcessTree>;
TreeOwnerFailure::recover(self) -> OwnedProcessTree;
```

Post-spawn errors cannot be resource-free errors. Observation borrows the owner;
Unknown retains it. finish returns the owner unless all required terminal facts
hold. Caller cancellation/Drop must transfer unfinished ownership to an explicitly
owned bounded supervisor before releasing its reservation; that supervisor's
startup, capacity, parent-death and terminal acknowledgement are API gates, not an
implemented promise. No Drop-only proof, detached forgotten reader or consumed
Unknown result. A caller unable to provide this retention contract cannot start.

Every started state can enter CleanupRequired, including malformed startup,
attachment refusal, cancellation, output overflow and SDK-never-started. Unsupported
is resource-free only before ANY unit/guardian/process exists. SDK nonzero with
complete cleanup remains a captured failure, never qualification success.

Executable verification must bind the ACTUAL executed object, not a pathname
hashed earlier. A retained open object alone does not prevent in-place mutation.
Each backend must establish safe native execution/object binding, immutable
publication or denied mutation/replacement, loader dependency and environment
constraints, with actual replacement and in-place-write negative probes. Same-UID
readonly mode alone is insufficient. Missing safe primitive means Unsupported;
no authored unsafe exception is implied.

No numeric PID constructor or kill-by-label API. Facts separately report SDK and
supervisor exits, invocation identity, native population, adopted descendant reap,
reader joins, output completeness and cleanup outcome. Private raw streams never
enter Debug/errors. Source hashes and exact test selectors remain caller inputs.

## Existing primitives, required edges and verification

Existing test-support has direct-child bounded capture and strict JSON only.
Potential safe Unix primitives: std Command, rustix process setsid/waitid/kill group,
owned FDs and nonblocking event/poll. Process/filesystem/event features and a safe
signal handler facility may need new declared edges. Windows owned job adapter and
guardian binary build selection also need exact review. No manifests changed and
no authored unsafe exception is implied. Check the PINNED dependency version,
not only latest documentation, before freezing signatures.

Pure: impossible state transitions; absolute deadlines; SDK exit versus guardian
exit; unsupported downgrade; output/control bounds; duplicate/unknown frames;
no secret diagnostics; no reaping-before-last-signal; explicit environment.
Actual allocated native: SDK exits0 while grandchild keeps stdout alive; TERM-
ignoring child; double fork/setsid attempt; guardian crash; parent EOF/death;
output overflow; cgroup attach denied; migration denied/observed; group leader
exit with remaining child; partial cleanup timeout; no foreign group signal;
no leftover owned processes/pipes and actual wait/join facts. Each OS/backend is
qualified separately. No process_tree implementation or native result is claimed.

Self-critique NEEDS-WORK: exact safe primitive/version coverage, full cgroup
allocation/runner migration policy, guardian TERM survival/parent-death failure,
nonblocking IO/reader joins and Windows nested-job adapter remain design gates.
Root and independent .NET/Python reviewers must accept the profile/API before code.
The earlier raw-group guardian discussion describes constraints only if such a
subordinate exists; it does not replace the selected systemd Linux backend.

References: [rustix waitid](https://docs.rs/rustix/latest/rustix/process/fn.waitid.html),
[rustix group signal](https://docs.rs/rustix/latest/rustix/process/fn.kill_process_group.html),
[Linux cgroup v2](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html).

## Independent critique disposition v0.2

Root and Python independently BLOCKED v0.1 implementation: consumed-owner errors
(P0), pathname-to-execution race (P0), descendant reaping (P1), concrete allocation
authority (P1), and abnormal startup transitions (P1). These are requirements above,
not resolved native results. Root selected systemd as the Linux candidate. Exact
safe D-Bus/IO/recovery-supervisor APIs and actual host probes remain required before
implementation acceptance. Windows Job and .NET actor review remain separate.
