---
id: sot:root/dotnet-protected-host-launch
title: Protected .NET host launch admission and B2 custody
status: draft
created: 2026-11-18T00:00:00Z
updated: 2026-11-19T12:00:00Z
verify: manual
rel:
  governs: [crates/dotnet-adapter/tests/hosted/retained.rs]
  depends_on: [sot:root/dotnet-adapter, sot:root/structural-development, sot:root/delivery, sot:root/cross-os-runners, sot:root/native-security, plan:root/20261107115350]
---

# Protected .NET host launch admission and B2 custody

Document version: 0.3.0. Status: **proposed**. The owner chose the SoT design
path under decision D1(a) on 2026-11-18 (issue #75 comment 6968092014). D1(a)
approves *drafting* this SoT. It does not accept the native operations listed
below. Every item marked UNAPPROVED here needs the review that the structural
B2 rule asks for before anyone writes source for it. This document authorizes
no OS execution, account, Job, SCM, SDK or Kestrel run.

Version 0.2.0 update. On 2026-11-18 the owner approved D3 and chose D4
option (b) (issue #75 comment 6979694428):
- D3 approves N1, N2, N5 and L2 below.
- D4(b) lets a test-only authority issue a decision expiry shorter than the
  request deadline, so `expired-result` is distinguishable from the request
  deadline. The production authority contract is unchanged. The test
  authority is the `humanymous-runtime` feature `test-authority`
  (`authority::decision::TestExpiry`). Only the dotnet-adapter dev-dependency
  enables it, and a `compile_error!` refuses it in any build without debug
  assertions, so release and production builds cannot contain it.

Version 0.2.1 update (2026-11-18). No new decision. The package controller
is wired to the derived admission (3.5), all four package modes ran on Linux
(5), and `dotnet-package-linux` is declared (4.3). N3 and N4 stay proposed.

Version 0.3.0 update (2026-11-19). The owner approved N3 and N4 under
decision D5(a) (issue #75 comment 6998447599). D5(a) keeps these safeguards:
a Windows pilot declaration counts only after the pilot actually ran and
passed (no `unsupported` or `test = false` row stands in for it), and the
change needs an independent agent review and all five required checks before
merge. D5(a) does not approve the `DuplicateHandle` fallback, a direct
parent-to-harness shortcut, running the harness as the admin user, or any
admin ACL repair or ownership change. State after this version:
- N1, N2 and N5 ran and passed on a windows-2025 runner (4.1).
- N3 is implemented and its unit test passed on windows-2025 (4.1).
- N4 is implemented but blocked before launch by the carrier owner rule
  (4.1, 7). Windows hosted and Windows package stay undeclared.

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
- **Linux provider child**: on every exit path, an early error included, the
  provider kills the scope and its `sudo systemd-run` child and reaps that
  child within a bounded grace. `child_reaped` is a terminal fact; a child
  still running at the cleanup deadline never reports an exit code.
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

Implemented 2026-11-18. `tests/package.rs` includes the consumer
`hosted/launch.rs` and refuses before any `Command` without its own original,
non-derived admission. `Admission::derive` re-sends the exact context and
binding bytes and the nonce, and narrows only the apphost and audit root under
the original `private_work`; a derived admission never derives again. Each
per-mode harness timeout is the remaining original run deadline. The whole
matrix (pack, four restores and publishes, four host runs) therefore has to
fit the one 180 s original budget. Observed on Linux: 108-113 s.

## 4. Wiring

### 4.1 Windows backend (windows-native-runner, existing actor)

These reuse code that already exists: `OwnedActor`/`OwnedFailure`,
`OwnedProcessAttempt` with explicit phases (Created through Resumed),
`ConvergenceProof`, `CleanupOrder`, the shared `parent_context` codec, and
`publish_actor_context`/`inspect_actor_context`.

| ID | Change | Status |
|----|--------|--------|
| N1 | Set `STARTF_USESTDHANDLES` on the existing `CreateProcessWithLogonW` `STARTUPINFOW`. `hStdInput` is the read end of a `std::io::pipe()`. Std output and error use bounded parent readers. No new Win32 function. | APPROVED (D3). Implemented as an opt-in launch variant; existing modes are unchanged. **Ran and passed** on windows-2025 through `delivery component dotnet-stdio` (Validate run 44450072313, job 135312398686): the secondary-logon service delivered the piped stdin and bounded stdout/stderr to the NoProfile actor child, which read exactly one frame and then EOF. The `DuplicateHandle` fallback is not needed. |
| N2 | One read-only `GetFileType(stdin)` query in the native-security boundary. | APPROVED (D3). Implemented as `humanymous_platform::windows_object::stdin_is_pipe`, because the runtime crate forbids unsafe code and the platform object bridge already owns the deny-with-local-allow Win32 scope. Ran in the N1 fixture child (same run): stdin observed as a pipe. |
| N3 | Bind `InspectedActorContext` to the envelope and make `require_exact_producer_identity` pass (rule 3.1.6). Safe Rust. | APPROVED (D5(a)). Implemented as `InspectedActorContext::bind_creator_channel(CreatorChannelFrame)`: it binds once, refuses a non-pipe stdin or a frame not followed by EOF as Unsupported, and refuses a nonce or context/binding byte mismatch against the carrier as PermissionDenied. `require_exact_producer_identity` returns Ok only after that binding. The cfg(windows) unit test ran and passed on windows-2025 inside `component dotnet-stdio` (Validate run 44453094364, job 135322965663). |
| N4 | Wire the `dotnet-prebuilt` runner mode: take `prebuilt.rs` out of `cfg(test)`. Allowlist two fixed selectors: the existing Kestrel selector and `generic::actual_generic_abi_only_process`. | APPROVED (D5(a); amends the plan's single `SELECTOR`). Implemented: `dotnet-prebuilt --input --report` with the closed `humanymous.dotnet-prebuilt-actor-input/2` input and two fixed selectors, `hosted` (Kestrel plus generic) and `package` (the package matrix). The parent stages the win-x64 app closure and the pinned runtime (336 files, manifest SHA-256 `eaf43db5…f6680`, muxer `1e881ec1…b822f`) and then publishes the actor carrier; the `dotnet-prebuilt-actor` child binds N3 and runs the harness. **Blocked before launch**: on windows-2025 `publish_actor_context` refuses after creation with "protected CI actor carrier required" (Validate runs 44456615333 and 44458337027, jobs 135334612350 and 135340157970). See 7. Only `hosted` is wired in delivery; `package` on Windows is not wired. |
| N5 | NoProfile `logonflags 0` with actual ProfileList and directory absence before and after. | APPROVED (D3). Implemented with the N1 variant: one `RegOpenKeyExW` ProfileList query plus a directory check. Ran in the N1 fixture (same run): ProfileList entry and profile directory absent before and after. |

N1 needs a real fixture. The fixture must show that the secondary-logon
service delivers the std handles to the actor child, that the child sees a
pipe, and that the parent's handle leaves no inheritable copy anywhere else.
If the fixture refutes this, N1 falls back to a `DuplicateHandle` design,
which needs a fresh review. This SoT does not pre-approve that fallback.
The fixture passed (run 44450072313), so the fallback is not used.

Windows SDK pin (2026-11-19). `component dotnet-host` on Windows acquires the
same SDK 10.0.403 / runtime 10.0.14 as Linux, from the win-x64 zip:
SHA-512 `7a114c318fa7f1d9822991b534b2814d0e78386ceda816b371f5141acefe741d87ccb606ede06cadac8f9ac5070e32b708a67bbd1619ff5e5a7858a27874ef48`,
300,745,255 bytes. A bounded zip reader in delivery (`dotnet_host/zip.rs`)
unpacks it: stored or deflate only, no zip64, multi-disk, encryption or
links, plain relative names, case-collision refusal, CRC-32 and size checks.
On windows-2025 the archive was acquired and verified, and the harness built.

### 4.2 Linux backend (delivery executor, same provider profile)

| ID | Change | Status |
|----|--------|--------|
| L1 | `delivery component dotnet-host` (following the `node-host` pattern) runs as the non-root hosted runner user and creates the harness with a piped creator channel. | Implemented (`scripts/ci/delivery/src/components/dotnet_host.rs`). |
| L2 | Containment is a transient systemd scope (cgroup v2), created through `sudo -n systemd-run --scope --uid=<runner uid>`. Termination goes through the scope. The terminal fact is the scope's `cgroup.events` showing `populated 0`. The existing `linux_tree_probe` prerequisites (systemd PID 1, unified cgroup, system bus) must be Observed first. | APPROVED (D3). Implemented and run on Linux; see 5. |
| L3 | The `parent_context` wire gets an OS-tagged actor identity: a uid on Linux, a SID on Windows. This is a `/2` schema owned by the platform codec owner; `/1` bytes stay frozen. | Implemented (`parent_context/host.rs`, `admission.rs`). |

L2 terminal fact, clarified. The kernel removes a scope's cgroup only once it
is empty, and systemd collects a transient scope once it is empty. So the
terminal fact is either `cgroup.events` reading `populated 0`, or the scope
unit no longer loaded together with its cgroup directory gone (`Collected`).
`populated 1`, or an unreadable state with the unit still loaded, is never
terminal and refuses a clean run.

The dotnet SoT already rejects process-group kill and post-reap PID kill as
proof. If L2 is not observed, Linux refuses with `Unsupported`. A process
group never stands in for the scope.

### 4.3 CI and accounting declarations (after source exists)

Declared in the change that adds the job (2026-11-18): only
`dotnet-hosted-linux` (hosted target, `os: [linux]`, job `dotnet`). Windows
hosted and both package rows stay undeclared, because nothing there has run.

Declared 2026-11-18 with the step `component dotnet-package` in the same
`dotnet` job: `dotnet-package-linux` (package target, `os: [linux]`). Windows
hosted and Windows package stay undeclared: they need N3 and N4, which are not
approved, and nothing has run on Windows.

2026-11-19: N3 and N4 are approved (D5(a)) and the Windows hosted step exists
(`component dotnet-host` on windows-2025 in the `dotnet` job), but the
hosted pilot has not passed, so `dotnet-hosted-windows` and
`dotnet-package-windows` stay undeclared, and the `job.runner` check that
would bind them to the windows-2025 runner is not added yet.

Also declared 2026-11-18: `dotnet-sdk-download` (package
`humanymous-delivery`, binary target, `os: [linux, macos, windows]`, job
`dotnet`). It names the ignored provider test
`components::dotnet_host::sdk::tests::actual_pinned_sdk_download_verifies`.
`component dotnet-host` runs that test alone with `--exact --ignored` before
its own SDK custody and refuses unless the exact single-test success footer is
observed. The row qualifies nothing on macOS or Windows.
These steps come after the source exists:
- `scripts/ci/native-pilots.json`:
  - `dotnet-hosted` for the hosted target, tests
    `[actual_kestrel_engine_audit_body_gc_and_shutdown,
    generic::actual_generic_abi_only_process]`;
  - `dotnet-package` for the package target, test
    `[sdk_package_and_real_host_matrix]`;
  - both with `os: [linux, windows]` and `executor: job`.
- The job named by those rows must be one that merge-ready requires. It uses
  no `actions/setup-dotnet` and no runner-installed SDK. The Rust provider
  (`scripts/ci/delivery/src/components/dotnet_host/sdk.rs`) downloads the
  SDK 10.0.403 linux-x64 tarball from Microsoft's official
  `builds.dotnet.microsoft.com` URL into a private owner-only temp directory.
  Before extraction it checks the exact size and the SHA-512 pinned in that
  file, copied from Microsoft's official release metadata
  (`release-metadata/10.0/releases.json`, whose URL sits next to the pin). A
  mismatch fails closed. Every SDK command runs with that private root as
  `DOTNET_ROOT`/`PATH` (set only for the child) and with `adapters/dotnet` as
  the working directory, so `global.json` (10.0.403, `rollForward: disable`)
  applies. `dotnet --version` must print exactly `10.0.403`; a list that only
  contains it is refused. The receipt records the tarball SHA-512, the
  resolved version, the muxer SHA-256 and `dotnet --info`. The private SDK is
  also the apphost runtime root, and it is removed after the launch phase.
  The first green hosted run (before this rule) resolved SDK 11.0.100 from the
  repository root; it is not evidence of the pin. Workflow edits are
  human-gated paths, so they go to owner review. Known gap: other jobs'
  `actions/setup-node` and `oven-sh/setup-bun` downloads are not yet
  hash-verified this way.
- `component dotnet-package` runs the same SDK custody itself (its own
  download, SHA-512 check and exact-version check; the two steps share no
  SDK). The controller's `PATH` names only that private SDK root. Its
  original envelope's harness lineage is the package controller binary that
  runs in the scope, and its native lineage is the release export it packs;
  each scoped result also records the SHA-256 of the binary it ran. Provider
  scratch lives under the canonical temp root (`/tmp` on hosted runners), not
  `RUNNER_TEMP`: the package socket
  `<private_work>/pkg/<mode>/audit/<profile>/decision.sock` adds 46 bytes, so
  `/home/runner/work/_temp` leaves no room under the 100-byte bound, while
  `/tmp/hm-dotnet-<12>/run-<12>` keeps 10 bytes of margin. The provider checks
  that real path before any build starts.
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

Update 2026-11-18. The fixture now mutates each binding field the
`request-rewrite` row names (route, method, raw target, host and a sanitized
header), one per request. `response-started` now also checks that a started
response keeps its single status line, carries no application header and is
never completed. Both changes compile against SDK 10.0.403; neither has run,
because `Host::start()` still refuses. Six cases remain unauthored, each for a
stated reason:
- `expired-result`. The authority sets `expires` to the request deadline
  (`authority/decision.rs`). An authenticated result can therefore never expire
  before the host's own deadline, so the fixture cannot tell authority expiry
  apart from `request-deadline`. A design change is needed first: an authority
  expiry shorter than the request deadline, or a test authority allowed to
  issue one.
- `drop-pending-decision`, `authority-restart`, `dispose-race`,
  `shutdown-active`. Each is a timing race between a held authority decision
  and host cancel, authority replacement, `HumanymousHost.Dispose` or
  application stop. Their pass conditions depend on Kestrel and native-binding
  ordering that only an actual run can show. Author them in the change that
  first runs the fixture, not blind.
- `wrong-peer`. The contract requires a real OS peer mismatch. The pilot
  configures its host once at startup, so the fixture needs a second host
  configuration, or a second authority endpoint with a different expected
  peer. That is a fixture design choice for the same first-run change.

Update 2026-11-18 (D3, D4). All six cases are now authored and have run on
Linux. The pilot gained a `replace-host` control (a new installation becomes
current and the old one is disposed), a `hold` handoff mode (600 ms) and a
`collectible-probe` control. The fixture gained a `/hold-await` authority gate
whose ceiling (4000 ms) outlives the 2000 ms host deadline.
- `expired-result`: a test-authority expiry of 300 ms against the 600 ms hold
  refuses with 503 and no contact. The control run with the production rule
  (expiry 0 = request deadline) passes the same hold with exactly one contact.
- `drop-pending-decision`: the host drops a decision the authority still holds
  at its deadline (503, no contact, zero native resources). After release, the
  next request adds exactly one contact.
- `wrong-peer`: the host is reconfigured to expect server uid+1 (Linux) or
  pipe server PID+1 (Windows). It refuses with 503, no contact and no new
  authority verdict, then recovers when restored.
- `dispose-race`: the installation is replaced and disposed while its request
  holds a decision. The released decision returns 503; the replacement serves
  exactly one new contact.
- `authority-restart`: epoch 1 is cancelled and its runtime shut down with a
  decision in flight (503). While nothing serves, the existing
  `authority-down` assertions run. Epoch 2 binds the same endpoint with a new
  credential and audit chain. The epoch 1 credential refuses with zero epoch 2
  verdicts; after `replace-host` with the epoch 2 credential, exactly one
  ALLOW and one contact follow.
- `shutdown-active`: stop arrives with one decision held and one request
  awaiting body input. `stopped` reports no new contact, neither request
  completes with 2xx, and the process exits 0. The collectible probe refuses
  a collectible load twice (JIT). Under NativeAOT no load is attempted, so the
  probe is listed in `not_applicable` with reason `no-dynamic-loading` and is
  not counted in `passed`. The observation states
  `native_runtime_unload_claimed: false`. Observed shutdown takes about
  1.96 s: pending work settles through the 2000 ms request deadline, not
  through immediate cancellation at host dispose.

Run evidence: three consecutive local runs on Linux x86_64 (2026-11-18),
venue `root-allocated-local`. The provider ran the harness as uid 1000 inside
a transient systemd scope (systemd 255 as PID 1 in a private namespace
sandbox, cgroup v2), with the creator channel a FIFO and the scope
`Collected`. Both hosted tests passed; 20 of 21 host cases are covered;
`package-install` stays not-run. This is not hosted CI qualification and
claims nothing for Windows.

Run evidence, package matrix (2026-11-18): four local runs on Linux x86_64,
the last on 55d76b1f, venue `root-allocated-local`, through
`dotnet-host-local launch-package` as uid 1000 in a transient scope (systemd
255 as PID 1 in a private namespace sandbox with a read-only host root, cgroup
v2), creator channel a FIFO, scope `Collected`. In those runs the SDK 10.0.403
tarball was checked against Microsoft's published SHA-512 by hand before
installation; the provider code did not check it then. Since the PR #120
review fix the provider itself performs that check, in CI and in local
`prepare` alike (4.3). Those four runs predate that fix, the canonical
temp-root scratch and the controller lineage fix. All four modes passed:
jit-source, jit-packed, aot-source and aot-packed, each with both profiles on
runtime 10.0.14. The projection reported `host_contract_complete: true` with
no required case unrun (`package-install` evidenced by the packed modes),
status `fixture-complete-cleanup-unverified`, `component_qualified: false`
and `cleanup: null`. The controller finished in 108-113 s of its 180 s budget.
This is not hosted CI qualification and claims nothing for Windows.

Historical note, before the controller wiring above:
**`Host::start()` alone cannot turn rows 72/73 green**. The package test
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

- N1, N2 and N5 passed the Windows fixture; N3's unit test passed on
  windows-2025. N4 is implemented but has not reached actor launch.
- **Carrier owner blocker (needs owner decision).** On windows-2025 the
  elevated runner user's default owner for new objects is that user, not
  Administrators (diagnostic `admin_default_owner: other`, run 44458337027).
  `publish_actor_context` requires an Administrators or SYSTEM owner, so it
  refuses after creating the carrier, and the partial carrier is removed.
  The publisher rule says never to repair or change an owner, so this change
  does not set the owner and does not relax the rule. A fix needs a decision:
  either a new reviewed native operation that creates the carrier with an
  Administrators owner (for example an explicit owner in the creation
  security descriptor, or the token default owner), or an amended publisher
  rule that accepts the publishing user as owner. Neither is approved.
- The 180 s original budget is unmeasured on Windows, because no run has
  reached actor launch.
- Parent death leaves account, profile and private residue as Unknown.
- The prebuilt plan already blocks a clean pass on profile residue (N5).
- Rows 72/73 ran locally on Linux in four modes, NativeAOT included; the
  hosted CI job has not run yet. The whole matrix shares one 180 s original
  budget, and the local margin is about 65 s. A slow hosted NuGet restore can
  exhaust it; the remedy would be a budget decision, never a renewed budget.
- Windows rows 24 and 73 need the carrier owner decision, then an actual
  hosted and package pass on windows-2025.
