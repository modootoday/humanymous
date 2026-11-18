---
id: sot:root/dotnet-protected-host-launch
title: Protected .NET host launch admission and B2 custody
status: draft
created: 2026-11-18T00:00:00Z
updated: 2026-11-18T00:00:00Z
verify: manual
rel:
  governs: [crates/dotnet-adapter/tests/hosted/retained.rs]
  depends_on: [sot:root/dotnet-adapter, sot:root/structural-development, sot:root/delivery, sot:root/cross-os-runners, sot:root/native-security, plan:root/20261107115350]
---

# Protected .NET host launch admission and B2 custody

Document version: 0.1.0. Status: **proposed**. The owner chose the SoT design
path under decision D1(a) on 2026-11-18 (issue #75 comment 6968092014). D1(a)
approves *drafting* this SoT. It does not accept the native operations listed
below. Every item marked UNAPPROVED here needs the review that the structural
B2 rule asks for before anyone writes source for it. This document authorizes
no OS execution, account, Job, SCM, SDK or Kestrel run.

Scope: how `Host::start()` in `crates/dotnet-adapter/tests/hosted/retained.rs`
gets protected host admission; who holds custody of the process tree, account,
private root and failure handoff; how the consumer associates its launch
context with the exact producer; and which evidence would turn the dotnet
coverage-accounting rows green on Linux and Windows.

Non-coverage:
- macOS. It is unsupported under D2 and handled in PR #108.
- Scoring. The detection freeze is untouched.
- L5 Docker product qualification, NativeAOT unless that mode actually ran,
  and package publication.
- Any new process, worker, object-bridge or authentication provider. Structural
  rule 5 allows one owner for each shared security primitive.

## 1. Problem

At afa5514, `Host::start()` returns `Unsupported` before it builds a
`Command`, and it does so on purpose. Both native
`require_exact_producer_identity()` gates (publisher and inspector, in
`native_security/storage/actor_context.rs`) return `Unsupported("producer
handle association unqualified")`. The structural SoT still lists the B2
process-tree supervisor operations as UNAPPROVED. The retained interactive
transport (`humanymous_platform::testing::process::InteractiveAttempt`) and the
L4 consumer `Host::from_retained` already exist; `from_retained` is
`#[expect(dead_code)]` because no admitted provider calls it.

The native coverage accounting therefore fails on two harness rows. Indices
below are the positional ones from run 44154451544. They are not identities.

| Row | Harness | Ignored tests |
|-----|---------|---------------|
| 24 (every native OS) | `crates/dotnet-adapter/tests/hosted.rs` | `actual_kestrel_engine_audit_body_gc_and_shutdown`, `generic::actual_generic_abi_only_process` |
| 72 (Linux) / 73 (Windows) | `crates/dotnet-adapter/tests/package.rs` | `sdk_package_and_real_host_matrix` |

## 2. Vocabulary

- **Custody provider**: the single existing actor profile
  `standard-user-dotnet-prebuilt` (plan 20261107115350). This SoT gives it two
  OS backends, defined in section 4. It is one provider, not two.
- **Producer**: the trusted parent that publishes the launch context. On
  Windows that is the elevated runner parent (`publish_actor_context`). On
  Linux it is the delivery executor that runs as the hosted runner user.
- **Consumer**: the hosted test harness process that calls `Host::start()`.
- **Creator channel**: the stdin pipe of a process. Its creator makes the pipe
  and installs the read end as the child's standard input at creation time.
  The creator keeps the only write end and closes it after writing.
- **Admission envelope** `humanymous.dotnet-host-admission/1`: one bounded
  frame sent over the creator channel. It carries the exact context bytes and
  binding bytes, run nonce and profile. Maximum 96 KiB: 64 KiB context, 16 KiB
  binding, plus framing.
- **Admission**: a private, `!Clone`, process-wide value that the consumer
  builds once from the envelope. It is the only input `Host::start()` accepts.
- **Derived admission**: an admission that a same-actor controller (the package
  matrix) narrows for one mode's apphost. It can never widen the original.
- **Terminal facts**: the provider's native observations. On Windows these are
  a `ConvergenceProof` with assigned-Job scope. On Linux they are scope
  `populated 0`. Both also cover reader joins, handoff, private and profile
  cleanup. Only these facts can make a run clean.

## 3. Decision rules

### 3.1 Producer identity association (replaces the `Unsupported` gate)

1. The association comes from a **handle, not a path**. The consumer reads the
   context only through the creator channel it inherited at creation. Reopening
   a path, matching a hash, reading an environment locator or decoding a DTO
   never satisfies the gate. This keeps the rule already written in
   `actor_context.rs`.
2. Hops. Each creator writes the envelope into the stdin of the child it just
   created, then closes the write end:
   - Windows: the admin parent creates the actor runner child (`dotnet-prebuilt`
     mode). That child creates the harness.
   - Linux: the executor creates the harness.
   - Package mode only: the package controller creates the per-mode harness.
3. On Windows, the actor runner child also inspects the published
   `context.json`/`binding.json` with the existing exact-DACL inspector. The
   bytes it gets that way must equal the envelope bytes exactly. The file pins
   still serve as the admin's retained audit copy. They are never the
   association source.
4. The consumer accepts the envelope only if all of these hold:
   - Its stdin object is an anonymous pipe. On Linux, `fstat` through a safe
     clone of fd 0 must report a FIFO. On Windows, `GetFileType` must return
     `FILE_TYPE_PIPE` (native query N2).
   - Exactly one envelope arrives, followed by EOF, inside the original run
     budget.
   - The schema, nonce and profile match, and the binding digests match the
     context bytes (the existing `decode_bound_claims` rules).
   - The actor is the current actual identity: TokenUser SID on Windows, a
     real non-zero uid on Linux.
   A second read, trailing bytes, a console or file stdin, or a timeout all
   fail closed.
5. What this association proves: the bytes came from the process that created
   the consumer, or from a process that creator gave the write end to. On
   Windows the per-run reserved account `hmrun<nonce>` makes every same-user
   process a member of the provider's Job. On Linux the runner uid is shared
   with the GitHub runner agent, so same-uid peers count as trusted hosted
   infrastructure. This SoT makes no claim against a same-uid adversary on
   Linux, and receipts must disclose that.
6. The existing `require_exact_producer_identity()` returns `Ok` only on an
   `InspectedActorContext` that was bound to a creator-channel envelope by rule
   4. The publisher side keeps returning `Unsupported` (it never consumes).

### 3.2 Launch API shape

```rust
// crates/dotnet-adapter/tests/hosted/retained.rs (L4, safe Rust)
impl Host {
    /// Borrows the one process-wide Admission (built on first call from the
    /// creator channel) and refuses BEFORE Command on any missing gate.
    pub fn start() -> io::Result<Self>;
}
```

- `start()` keeps its zero-argument signature so the existing two-profile loop
  does not change. The two sequential profiles borrow one admission and one
  original budget mapping. Nothing renews a 180 s budget per call.
- Command construction:
  - `Command::new(apphost)` with the apphost taken from the admitted context.
  - `env_clear()`, then exactly `context.fixed_environment()`.
  - `current_dir(private_work)`.
  - No shell, SDK, `DOTNET_STARTUP_HOOKS`, probing path or inherited `PATH`.
- `HUMANYMOUS_DOTNET_PILOT` and the other path environment variables are not
  admission. When present, they must equal the admitted apphost, or start
  refuses.
- Sequence: admission, then the fixed `Command`, then
  `InteractiveAttempt::start(command, deadlines_from_admission, limits)`, then
  `Host::from_retained`. `InteractiveStartFailure::NotStarted` maps to `Err`.
  `Retained` keeps its owner: it is converted into a `Host` and returned
  through the existing failure path, never dropped.
- Limits are the INTEGRATION.md contract: 64 KiB startup, 1024 B × 64
  controls, 4096 B frames, 256 KiB per stream.

### 3.3 Fail-closed refusals (all before `Command`)

| Condition | Result |
|-----------|--------|
| No creator-channel envelope, a stdin that is not a pipe, a malformed, duplicate, trailing or oversize envelope, or no EOF | `Unsupported` |
| Nonce, profile, binding digest or provenance commitment mismatch | `PermissionDenied` |
| Actual actor ≠ envelope actor; Unix uid 0; an `ACT` marker; an unclassified venue | `PermissionDenied` |
| Run budget exhausted or zero; cleanup reserve not positive | `TimedOut` |
| Cleanup-only context, or a derived admission that widens its parent | `PermissionDenied` |
| Apphost outside `app_root` (prebuilt) or outside `private_work` (derived) | `PermissionDenied` |
| OS backend not qualified (macOS, or a Linux scope prerequisite not observed) | `Unsupported` |

No path in the table above falls back to the current user, ambient `TEMP`,
an environment-variable apphost or a direct `Command`.

### 3.4 Custody and failure handoff

- **L4 Host** owns the direct apphost child and its IO workers until
  `settle_joined()`. On any failure it returns the same `Box<Host>`. The test
  then returns `Err(Box<Host>)` and the harness exits nonzero. Host never claims
  Job, tree or account cleanup.
- **Failure handoff**: when the harness exits, its residual direct and
  descendant processes are still inside the provider's containment, because
  breakaway is forbidden. Custody passes to the provider implicitly through
  that containment. The provider's `contain()` under the original cleanup
  deadline is the only cleanup authority.
- **Bounded outcome**: before exiting, the harness may publish one typed
  outcome of at most 64 KiB. Publication goes to `handoff_root`, uses known
  names, and stays within the plan's limits (≤32 files, ≤8 MiB). The outcome is
  DATA. It cannot mint terminal facts.
- **Provider order**: the existing `CleanupOrder` applies unchanged. A clean
  pass needs every one of these facts. Any Unknown fact is a failure, even with
  test exit 0.
  - Normal exit: bounded handoff, then private cleanup, then controller exit,
    then actual Job-empty or scope-unpopulated, then verify and copy the
    handoff, then delete the account, then delete scratch.
  - Timeout or error: main containment empty, then a same-user cleanup child in
    its own containment, then private cleanup, then cleanup containment empty,
    then account and scratch deletion.
- **Parent death**: a kill-on-close Job (Windows) or scope teardown (Linux)
  stops the tree. Account, profile and private state are recorded as
  residual/Unknown. They are never reported clean, and no recovery is promised.

### 3.5 Derived admission (package matrix)

The package controller builds four apphosts under its own `private_work`.
Each per-mode harness gets a derived envelope through the controller's creator
channel. The derived envelope must keep:
- the same actor, parent nonce and original budget, borrowed and not renewed;
- an apphost and audit root under the original `private_work`;
- a fixed mode label.

A derived admission cannot become an Admin-staged closure claim. Package
receipts keep `component_qualified=false` and `cleanup=null` until the
provider's terminal facts are joined.

## 4. Wiring

### 4.1 Windows backend (windows-native-runner, existing actor)

These reuse code that already exists: `OwnedActor`/`OwnedFailure`,
`OwnedProcessAttempt` with explicit phases (Created through Resumed),
`ConvergenceProof`, `CleanupOrder`, the shared `parent_context` codec, and
`publish_actor_context`/`inspect_actor_context`.

| ID | Change | Status |
|----|--------|--------|
| N1 | Set `STARTF_USESTDHANDLES` on the existing `CreateProcessWithLogonW` `STARTUPINFOW`. `hStdInput` is the read end of a `std::io::pipe()`. Std output and error use bounded parent readers. No new Win32 function. | UNAPPROVED (changes an existing unsafe call) |
| N2 | One read-only `GetFileType(stdin)` query in the native-security boundary. | UNAPPROVED (new unsafe scope) |
| N3 | Bind `InspectedActorContext` to the envelope and make `require_exact_producer_identity` pass (rule 3.1.6). Safe Rust. | proposed |
| N4 | Wire the `dotnet-prebuilt` runner mode: take `prebuilt.rs` out of `cfg(test)`. Allowlist two fixed selectors: the existing Kestrel selector and `generic::actual_generic_abi_only_process`. | proposed (amends the plan's single `SELECTOR`) |
| N5 | NoProfile `logonflags 0` with actual ProfileList and directory absence before and after. | UNAPPROVED (prerequisite the plan already named) |

N1 needs a real fixture. The fixture must show that the secondary-logon
service delivers the std handles to the actor child, that the child sees a
pipe, and that the parent's handle leaves no inheritable copy anywhere else.
If the fixture refutes this, N1 falls back to a `DuplicateHandle` design,
which needs a fresh review. This SoT does not pre-approve that fallback.

### 4.2 Linux backend (delivery executor, same provider profile)

| ID | Change | Status |
|----|--------|--------|
| L1 | `delivery component dotnet-host` (following the `node-host` pattern) runs as the non-root hosted runner user and creates the harness with a piped creator channel. | proposed |
| L2 | Containment is a transient systemd scope (cgroup v2), created through `sudo -n systemd-run --scope --uid=<runner uid>`. Termination goes through the scope. The terminal fact is the scope's `cgroup.events` showing `populated 0`. The existing `linux_tree_probe` prerequisites (systemd PID 1, unified cgroup, system bus) must be Observed first. | UNAPPROVED (the Linux B2 backend is unqualified) |
| L3 | The `parent_context` wire gets an OS-tagged actor identity: a uid on Linux, a SID on Windows. This is a `/2` schema owned by the platform codec owner; `/1` bytes stay frozen. | proposed |

The dotnet SoT already rejects process-group kill and post-reap PID kill as
proof. If L2 is not observed, Linux refuses with `Unsupported`. A process
group never stands in for the scope.

### 4.3 CI and accounting declarations (after source exists)

These steps come after the source exists:
- `scripts/ci/native-pilots.json`:
  - `dotnet-hosted` for the hosted target, tests
    `[actual_kestrel_engine_audit_body_gc_and_shutdown,
    generic::actual_generic_abi_only_process]`;
  - `dotnet-package` for the package target, test
    `[sdk_package_and_real_host_matrix]`;
  - both with `os: [linux, windows]` and `executor: job`.
- The job named by those rows must be one that merge-ready requires. It pins
  `actions/setup-dotnet` at `26b0ec14cb23fa6904739307f278c14f94c95bf1` with
  `cache: false`, SDK 10.0.403 and x64, and runs `dotnet --info` into the
  receipt. Workflow edits are human-gated paths, so they go to owner review.
- The rows are declared only in the same change that adds the named job step.
  Delivery tests reject a `job` row whose step does not exist.

## 5. Verification: what turns the rows green, and nothing less

Common to every pilot:
- Exact source SHA and lock, compiler 1.99.1, harness, native and app closure
  digests.
- A genuine hosted, standard runner. `ACT` and local runs are not
  qualification.
- Every declared test run alone with `--exact --ignored`, giving one
  single-test success footer.
- Provider terminal facts all present (section 3.4). A deadline kill, Unknown
  cleanup or a missing reader join is a failure.

**Row 24 (hosted harness), per OS:**
1. `actual_kestrel_engine_audit_body_gc_and_shutdown` passes under the
   provider. Both profiles emit `humanymous.dotnet-host-observation/1` with
   `failed 0` and `ignored 0`. `Host::finish` returns `SettledInteractiveChild`
   with observed child exit, stdout and stderr EOF, and joined workers.
2. `generic::actual_generic_abi_only_process` passes under the same provider
   with its existing exact two-frame, PID-bound receipt.
3. Unit evidence in the same change: pure tests for every row of the 3.3
   refusal table, the envelope codec bounds, derived-admission narrowing and
   budget non-renewal. Plus `cargo test --locked -p humanymous-dotnet-adapter`,
   fmt and strict Clippy.

**Rows 72/73 (package harness), per OS:** row 24's facts, plus all of these.
- Four modes run: jit-source, jit-packed, aot-source, aot-packed.
- Package and installed native bytes hash-equal to the measured artifact.
- `public["host_contract_complete"] == true`. That needs all 21
  `adapters/dotnet/pilot-contract.json` host cases covered except
  `package-install`, which is evidenced by packed mode.

At afa5514 the hosted fixture covers only 12 cases. These 8 are still not
authored:
- `drop-pending-decision`
- `expired-result`
- `response-started`
- `request-rewrite`
- `wrong-peer`
- `authority-restart`
- `dispose-race`
- `shutdown-active`

So **`Host::start()` alone cannot turn rows 72/73 green**. The package test
would still fail on its final assertion, and it must keep failing until those
fixtures exist. Nobody may weaken that assertion.

Non-claims, which hold even with every row green:
- no L5 product pass;
- no protection claim for the full product;
- no released NuGet tuple;
- no macOS coverage;
- no defense against a same-uid adversary on Linux;
- no native runtime unload safety.

Receipts keep `full_product=false`.

## 6. Critique reflected

- **P0 path substitution.** A same-user writer could replace `context.json`
  between publish and read. Rule 3.1.1 makes the creator-channel handle the
  only association, and file bytes must equal envelope bytes.
- **P0 forged admission.** A caller DTO, an environment locator or a decoded
  binding could each look like an admission. Admission is a private `!Clone`
  value with no public constructor, built only from rule 3.1.4.
- **P0 lost owner.** A partial `InteractiveAttempt` start could lose the child.
  `Retained` maps into `Host`, and a failure returns the same `Box<Host>`.
- **P1 fresh budgets.** Each profile, mode or derived admission could otherwise
  get a new 180 s. All of them borrow the original mapping.
- **P1 process-group cleanup on Linux.** A process group can leak descendants.
  Rule L2 requires the scope, and absence means `Unsupported`.
- **P1 false green.** Declaring rows before the job step exists, or before the
  8 missing cases exist, would forge coverage. Section 4.3 and the rows 72/73
  rule forbid both.
- **P2 Linux peer trust.** The same-uid runner agent is trusted infrastructure,
  stated as a residual risk and disclosed in receipts.

## 7. Residual risks

- N1, N2, N5 and L2 are UNAPPROVED native operations. Until they are reviewed
  and pass fixtures, `Host::start()` stays `Unsupported` on that OS.
- If the N1 fixture refutes std-handle delivery through secondary logon, the
  Windows design needs a fresh review.
- Parent death leaves account, profile and private residue as Unknown.
- The prebuilt plan already blocks a clean pass on profile residue (N5).
- The 8 missing host cases are separate L4 fixture work, and they gate rows
  72/73.
