# UNAPPROVED B2 integration deltas

Authoring is authorized ONLY in this evidence bundle. Body, feature, live source,
compiler and native execution acceptance are separate Root+X gates. Root's
tools-integration-5f1010d4 isolated Platform is a READ-ONLY preimage. No manifest
or production/testing source has been changed by this candidate.

Proposed new default-off feature (feature-only.patch, NOT applied):

```toml
windows-supervisor = [
    "dep:windows-sys",
    "windows-sys/Win32_System_Threading",
    "windows-sys/Win32_System_JobObjects",
]
```

Root coordinates review/allocation; only the actual claimed integrator may apply
accepted common-source changes. Cursor is not inferred active from this document;
no opportunistic claim or parallel Codex/Cursor application.

Keep existing windows-sys0.61.2/base features and seven accepted Windows object
scopes unchanged. No WinSafe/new dependency/version/lock edge. Root coordinates
exact approval; only the active claimed integrator may apply the conditional
feature afterward. Module lib delta is
only cfg(all(windows,feature="windows-supervisor")) pub mod windows_supervisor;
no root unsafe allow or SDK facade proof reexport.

platform-module.patch adds six NEW module files and the exact pinned lib delta.
Root must compare EVERY input/candidate hash before applying; no overwrite of
newer isolated files. This patch excludes Cargo and any common-owner extraction.

# Separate common-owner extraction requirement

Current source locations read but not edited:

- Platform testing/process/capture_attempt.rs owns CaptureDeadlines/capture state.
- testing/process/owned_child.rs owns real std Child/status/stop and original pair.
- testing/process/capture_attempt/worker.rs owns actual JoinHandles/results.
- testing/tree_lifecycle.rs is SYMBOLIC only, not native completion authority.

Before production integration Root must select ONE normal private owner/clock
context (e.g. src/process_lifecycle/) and physically move/reuse applicable existing
safe implementations there, with compatibility paths only as qualified. Internal
concrete backend enum StdChild/WindowsTree, not public provider traits/callbacks.
Std Child and Windows actual native handles keep their own private spawn factories;
no PID/raw-handle/assigned bool adapter. Original pair/first-loss drain/effective
cleanup move with the single owner, never reconstruct/renew in the new backend.

This candidate's safe state.rs is the minimal B2 bookkeeping/testable arithmetic,
not a copied full production/testing capture controller. It must be reconciled
into that ONE owner before shipping both paths; it is not authorization to leave
two permanent independent clocks/controllers. Existing capture diagnostics,
partial-owner retention and actual worker join semantics stay unchanged. Root
owns exact visibility/move/preimage gate and original affected tests, not this
Native evidence author. No extraction patch or live change is supplied here.

# Separate B2-P and admitted invocation requirements

Selected source supports only fixed Rust test-helper CUI modes and NO inherited
stdio/handles. Actual test executable/source/namespace and independent original
outer controller are caller preconditions; held File/PE/local clock alone do not
provide them. No OriginalInvocationOwner, original-I, provider namespace, grant
one-use, W, PreparedObserverPreparation/Session or Ready factory exists here.

B2-P Delivery helper mode/output reservation/finite pipes/actual joins remain
separately proposed; not implemented or accepted by these six files. No generic
Command::output, new pipe/HANDLE_LIST/console/ResumeThread native scope is allowed.
Kill/Job0/process exit never become worker JOIN, durable journal cleanup or
successful capture. SCM/WMI/broker-spawned production service trees remain outside
the selected fixed-helper profile. B1 scope and provider/genesis manufacturers
do not gain any approval or proof from these B2 candidate bodies.
