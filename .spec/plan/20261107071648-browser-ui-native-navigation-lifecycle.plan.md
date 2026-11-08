---
id: plan:root/20261107071648
title: Native browser navigation lifecycle and strict probe closure
status: active
created: 2026-11-07T07:16:48Z
updated: 2026-11-07T07:16:48Z
rel:
  governs: [crates/browser/**]
  depends_on: [sot:root/browser-ui, plan:root/20261107050329]
---

# Native navigation lifecycle and strict probe closure

## PLATFORM-SLICE-02 delegated Browser consumer imports

Root authorizes only two current-path Rust consumer substitutions:
bootstrap/src/negative_pilot.rs uses humanymous_platform::testing::process::ManagedChild;
desktop/src/pilot_process.rs uses humanymous_platform::testing::process::capture_with_stderr.
Suffixes and behavior are unchanged. There are no matching Rust references to
humanymous_windows_object_bridge or legacy package references in native-cli.
platform-slice-02-consumers.json binds the two before Git blobs/reconstructed
working SHA256 and after LF SHA256 against7e6e9638. Remaining old Rust references
in the scoped paths are zero. No helper alias, manifest/path move, authority or
unsafe change was authored. Root alone owns the28 frozen moves, dependency
categories/features, lock and coherent graph. No compiler, unit/Clippy, global
formatter, CI or actual CEF/process execution occurred. Scoped Cargo formatting
and source-bound checks wait for root's coherent allocation; previous results
do not qualify the new import/dependency boundary.

## Accepted public-family baseline v0: Browser-last handoff

The human confirmed the eight-public-package baseline in structural-development.
Root alone sequences moves/graph/ref synchronization: engine/core, runtime/platform,
tools, role-qualified adapters, then Browser. This handoff authorizes no local
path/package/import rename, new crate, shim, manifest edit, ref switch or publication.

structural-v0-handoff.json binds61 tracked Browser/native-cli paths to observed
HEAD59f6aa2d8b11c5d9049efb05eb3eaf51e970b7cc with raw SHA256 and canonical Git
blob identities. All61 match that base; owned source paths have zero dirty files.
No Browser compiler/process/target lease or path-moving operation is active.
The observed global process/marker snapshot is not a permanent exclusion lock or
descendant-cleanup proof. Bootstrap Cargo.toml is included read-only but remains
root graph ownership; common manifests/toolchain and test-support stay outside
this Browser writer's scope. Root must issue exact source/destination/hash slice
instructions before any topology implementation.

Required root decisions before Browser path freeze:

- Preserve separately linked minimal bootstrap and desktop DLL artifacts under
  the final public-family design. A package consolidation must not load Meari/UI
  dependencies in CEF subprocess bootstrap, introduce a permanent private helper
  package, or invent an unreviewed compatibility shim. Current RunConsoleMain and
  RunBrowserDesktop exports, fixed sibling DLL basename and borrowed sandbox
  pointer are runtime ABI, not merely Rust import paths.
- Keep pure Browser/native-cli forbid-unsafe separate from audited native FFI /
  Win32 scopes currently under deny-unsafe plus local allowances. Consolidating
  below a forbid root cannot preserve those allowances automatically. Platform
  placement must preserve UI-thread parent leases, child close order, no target
  authority and sandbox-required initialization; do not widen pub re-exports.
- Coordinate native-cli scaffold public symbols/manifest generator identity and
  humanymous-native binary naming with the tools owner. CARGO_BIN_EXE tests,
  version metadata and explicit unimplemented adapter claims are observable.
  Generated manifests never constitute a protection factory or Ready capability.
- Remap relative shared/scheme.rs and desktop/pilot-contract.json includes,
  native-cli test_support path, Cargo path dependencies, provisioning DLL filenames
  and CI selectors atomically across the owning slices. Do not rewrite immutable
  old source/evidence paths or infer Windows coverage from cfg-empty other targets.

After root's map, require locked graph/visibility/check/Clippy tests; default and
native-ui document geometry/focus/AX/source-staleness oracles; scaffold storage/
CLI tests; Windows bootstrap/desktop normal artifact import/export and dispatch
verification; and fresh source/artifact-bound CEF sandbox/native lifecycle pilots.
Product e2e remains Docker-only and cannot replace actual OS/IME/AT/GPU evidence.
fef and ce24 results remain bound to their old trees/artifacts. UI38 f9 is still a
source-only candidate; UI39 aa22 is research only. Same-route callback attribution,
full sandbox, Meari latency/GPU, IME/AT and Chrome parity remain open. Publication
and main integration are not authorized by the baseline approval.

## UI-39 project GPU research contribution, no execution change

Root assigns Browser's Meari-window/CEF-compositing boundary within project-wide
GPU research. GPU-BOUNDARY.md records current CPU/windowed baselines, proposed
selection modes, OS/architecture/vendor/runtime eligibility, hardware versus
software/virtual classification, transfer/queue costs, parity, lifecycle budgets
and evidence gates. Primary tagged wgpu30.0.1/official versioned API sources are
research only. Exact Meari release/API and all new GPU hardware pilots remain
unqualified. No blanket driver/vendor support follows from an upstream API table.

Next implementation decision must compare optimized CPU/UI38 controls with true
GPU-window presentation, define explicit fallback/required refusal and preserve
AX/IME/child/DPI ownership. Root owns pins/shared graph/cross-cutting policy and
Performance owns the comparison matrix. Existing CPU experiments finish without
retuning. No GPU load, new dependency/SDK/driver/shader, unsafe exception, backend
or model switch is authorized. Review missing tagged candidate API, bounded GPU
completion/resource drain and actual OS display evidence before any support claim.

## UI-38 pinned source candidate, not executed

Root authorizes a bounded Browser-only source slice while Meari prepares its own
release. Keep the1482 pin and shared graph unchanged. Retained Projection owns
checked host/source identity and acknowledges only successful model commits;
viewport changes use existing host environment/CSS layout, not source replacement.
Intrinsic tab min/max/shrink and count-specific media thresholds preserve narrow
title/close behavior. Existing width-only rebuild is removed only together with
that layout path, not as an isolated guard deletion.

New native document oracle source holds a prepared replacement across repeated
width/DPI host commits, expects no model-source commit, preserves address node,
dirty value/focus and new-tab geometry, rejects stale host updates, and checks
CSS-hidden AX close rejection plus real tab/title/membership invalidation. The
prepared replacement must become stale after a true model change. Unsupported
CSS/AX or source-generation behavior blocks acceptance and requires upstream API
work; authored assertions are not execution evidence.
Independent source review found inline display:flex would override responsive
inactive-close hiding. Remove visible inline overrides and keep normal visibility
in CSS. Preserve a focused inactive close through narrowing, then hide it when
focus moves; add a separate focus/AX oracle. This was a source finding/correction,
not an executed failure. New instrumentation excludes view construction and CEF
sync, has no extent/event-receive correlation, and last submission may be older;
it cannot report p95 receive-to-submit or complete causal stage attribution.

Opt-in --native-shell-timings records only numeric monotonic start/duration,
source-commit counter, finite stage enum and last submitted scene generation.
Meari resize/redraw handler duration is combined dispatch work, not a separate
raster/present measurement or OS display acknowledgement; those remain null.
Ring capacity512 and encoded512KiB cap retain explicit dropped/saturated state.
Default-disabled instrumentation emits no timing records. Deadline owner/tests
move unchanged into host_deadline.rs to keep the host composition below600 lines.
No new Cargo/native/GUI execution, dependency fetch, pin or graph change is granted.

## Priority shell responsiveness follow-up (UI-37; design only)

The ce24 normal build passed and human manual pilot exited0 after15.53s, with
one tab/before_close/host_closed and25 resize readbacks at96DPI. The human reports
CEF content is fine but Meari trails rapid resize by150-200ms or more. Record this
as estimated severe lag, not an instrumentation result. Current window backend
is VelloCPU/softbuffer; the artifact is dev/unoptimized. No new GUI is allocated.

Pipeline: Browser window_event pre-syncs child bounds, then Meari driver receives
surface/DPI events; its inner event processing and refresh_host commit viewport
environment. RedrawRequested enters checked_render, resolves assets/layout,
configures target size, prepares checked vector scene and submits to the CPU
window renderer. It rasterizes RGBA, converts pixels and calls softbuffer present.
Browser's periodic step separately sees width changes and reprojects all64 tab
slots through full retained source replacement. These are separate candidate
costs. No source reading establishes their timing or a displayed-frame timestamp.

First reviewable optimization proposal: keep retained replacement for membership,
selection, title and action metadata; let viewport-only geometry flow through
pinned responsive CSS/layout if native geometry/AX/action oracles pass. Candidate
CSS uses shrink/min/max widths and model-count classes for width-dependent close/
title visibility; no source rewrite for each pixel width. Verify1/2/8 tabs,
threshold crossings, active-close visibility, hidden-close rejection, tab clicks,
focus/dirty address and multiple DPI. A failing pinned capability requires an
upstream explicit layout-only transaction, not a metadata bypass. No code change
is accepted from this proposal until the oracle and ownership seam are reviewed.

Timing proposal: optional Rust-only bounded512-record ring, monotonic generation
and durations, dropped counts, no raw user content. Separate Browser projection/
commit from Meari resolve/scene/raster/swizzle/present-call. Compare optimized CPU
control and latest-size coalescing before asserting GPU necessity. OS display
acknowledgement remains unknown unless independently measured. Performance owns
its comparison/budget canon; Browser owns UI/child/AX integration and root owns
Meari dependency/pin changes. GPU candidate must be a true window renderer with
bounded completion/resource lifetime, not offscreen readback or indefinite wait.

## Resize lag and thick white perimeter (UI-36)

Keep this source delta separate from UI-35. Reduce shared resize edge5to3DIP,
paint exposed perimeter in frame background, remove placeholder padding/text,
and use zero perimeter when maximized. Keep normal resize hit/corner ownership,
minima and DPI scaling. Synchronize CEF child bounds on native surface/DPI events
before parent rendering, without waiting for the10ms service throttle; retain
bounded native readbacks and existing periodic recovery. Add pure multi-DPI
normal/max/dock allocation coverage. Actual thin-edge usability and compositor
latency/visual improvement remain unallocated. Source fixes do not inherit043
document results or the historical96DPI GUI qualification.

Exact046b5f1b native-ui document tests passed43/43, with zero failed/ignored/
filtered cases. Desktop library tests stopped on the native-profile cef-dll-sys
rebuild guard before execution; check/Clippy were unreached. Formatter passed and
declared87 source/3SDK hashes were unchanged; cache write exclusion is unproven.
Retain initial MSVC quoting failure and premature timezone-based watchdog stop;
corrected orchestration kept the original600s window and stopped at101.8s.
See controls-046-validation.json. No second GUI or production build was allocated.

## User-requested tab control layout (UI-35)

Keep this source delta separate from the navigation continuation correction.
Place the independent named close action within each tab's trailing area with
a28-DIP hit target; reserve title space and avoid nested buttons. Remove the
growing list spacer that pushes new-tab to the window's far side: keep compact
new-tab immediately after a shrinking/overflowing tab list, before flexible caption
space and fixed window controls. Retain IDs, selection, native actions and dirty
address/focus across projection. Review risks: overlap must not steal tab clicks,
overflow must not hide new-tab or window controls, and focus must remain visible.
Document/geometry/action regressions qualify structure only; fresh native visual,
DPI and OS AT observation remain unallocated, separate from compiled source.
The user then required Chrome's many-tab handling. Shrink widths before overflow,
preserve the active close, hide narrow inactive close/title and keep its document
icon. Reproject on viewport width changes as well as tab membership/selection.
Test1/2/8-tab geometry and narrow inactive actions. Live8/lifetime64 admission
limits remain separate from visual behavior. Primary Chromium main sources were
reviewed; exact pinned156 metrics/feature defaults and visual parity remain open.

## First actual controls GUI failure and bounded continuation correction

The single fef GUI run created two distinct CEF children and observed CtrlT/
CtrlTab, but Back accepted newtab generation3 and terminated load_error-3 without
commit/content. Preserve its blank final screenshot, callback-trace absence and
all raw data privately. Both before_close/host_closed do not prove exit0 or full
descendant cleanup; CtrlW versus timed shutdown remains unknown. No second GUI
run is allocated. controls-gui-first.json records237 unchanged runtime inputs.

Correct the source-level OnOpenURLFromTab gap only for an existing live pending
exact route in main frame/CURRENT_TAB, preserving its ticket and resource guards.
Record bounded open_continued; regress foreign frame/disposition, supersession,
Stop, completion, revocation and closure. This is a candidate correction, not
proven first-run causation. Run locked offline scoped library/protocol tests,
check/strict Clippy/fmt on frozen source before considering new artifacts/GUI.
Same-route callback attribution and full sandbox/IME/AT/Chrome gates remain open.

## Actual bounded Windows CEF gate passed

Latest fef69a92 local native preflight passed bootstrap+desktop all-target check,
production build and desktop native-pilot-tests all-target strict Clippy. Actual
durations are3m09,4m04 and3.41s; the supervised batch finished about443s within600.
No timeout branch was exercised. The87-file source/manifest and three fixed SDK
input closures were unchanged. Both built DLL hashes and the earlier actual
d214 failure are retained; this new success does not diagnose unrecovered raw
compiler text or promote old runtime counts. Actual GUI/process/keyboard/IME/AT,
full sandbox and descendant termination remain separate gates. No GUI launched.

New actual run42388354925/job128450452895 at d2148b0a failed native build101 after
runner14/helper-Clippy success. Safe diagnostics identify E0599 at native
browser_controls.rs:76; rendered compiler text and the private build-log body
were not recovered. Protocol/bootstrap/desktop tests were unreached and source
postcheck is unknown. Static inspection of pinned winit0.31.0-beta.3 identifies
ModifiersState::meta_key(), with no super_key() method at that call site. The
one-line adapter correction is source-only until the next actual native build;
scene42 at d75 and old ec4 process counts do not discharge this failed gate.

Latest native-ui document gate passed42/42 at d75c351c, zero ignored/filtered;
all-target check, strict Clippy and formatter passed. The87-file declared closure
is unchanged. controls-scene-validation.json preserves22ac39/2 (unsupported AX
actions),74f40/1 (explicit disabled migration contract),27441/1 (negative fixture
used String rather than Bool disabled) and the corrected success. No assertion
was removed or warning suppressed. CEF compilation, actual history/tab/keyboard/
IME/AT and full sandbox remain separate, unexecuted gates for this new source.

UI-32/33 pure validation passed on browser source ce05c45e with the literal
173008f0 graph overlay (Cargo.lock and CLI Cargo.toml):30 passed, zero failed,
ignored or filtered; default all-target check, strict Clippy and frozen-source
formatter passed. The86-file declared browser/manifest closure had zero change
records. The first live-worktree attempt exited101 before compilation because
the approved CLI dependency edge required lock reconciliation; it remains in
controls-default-validation.json, evidence:root/20261107073932. No lock flag was
weakened. Native-ui scene, CEF build/Clippy and actual history/tab/keyboard/IME/AT
gates remain unexecuted for this newer source. Subsequent UI-only tab overflow and
native modifier-reset changes do not inherit native qualification from ec4.

Current user-requested UI-32 expansion: implement actual CEF Back/Forward entry
traversal, loading/availability projection and Stop, retaining local-only guards,
sandbox and current IME confirmation policy. Review findings incorporated before
code: do not simulate history with load_url; reject stale or malformed entry
snapshots; issue a fresh typed ticket before traversal; cancel pending authority
on Stop; project disabled state for keyboard/AT as well as pointer use. Future
multi-tab and external browsing require their own lifecycle/security design.
Source changes do not inherit the older ec4 native pass. Required gates are pure
entry/document tests, scoped native compilation/Clippy and actual fresh-bundle
history/Stop observations. All are pending for this expansion until recorded.

The user selected tab add/close/switch and basic shortcuts next (UI-33). Reuse
Meari's multi-child lifecycle rather than replacing content in one browser. Review
requirements: monotonic tab IDs, capacity retained through actual cleanup, focus
and IME reset on switch, inactive-command rejection, late-create close, final-tab
window close and all-child cleanup receipts. Pure model/shortcuts and scene tests
must precede actual multi-child pilot; initial single-child tests remain distinct.

First actual native-ui scene batch at22ac1b36 executed41 tests:39 passed,2 failed,
zero ignored/filtered. Both newly required accessibility Click actions were
unsupported after direct source/DOM mutation; no check/Clippy stage ran. The
86-file declared snapshot remained unchanged. Correction uses the pinned public
retained-source prepare/commit API to synchronize action metadata/layout/AX,
keeps required action assertions and adds dirty input/node/focus preservation.
Remove the unused disabled-button helper rather than suppressing its warning.
This is a scene boundary finding, not a native CEF or full AT result.

Dedicated run42377685195/job128418538461 completed successfully at exact
ec4f2da09a78ffa3ae7661661fc1d54b921f534b. Actual results are runner14/Clippy,
native build0, desktop synthetic protocol4/13filtered, bootstrap synthetic
marker3/3filtered, actual bootstrap1/5filtered and actual desktop2. All selected
tests report zero failure/ignore. The four original refusal children each report
spawn/wait/reaped true, exit78, deadline false and no errors. Profile inventories
before/after succeeded and afterchanged=false. Normal completion did not call
explicit cleanup; timeout/error cleanup remains a separate test frontier.

The safe projection reports stage complete and source_changed=false within its
declared source scope. Artifacts0/upload skipped. Safe stdout is retained with
cef-run-42377685195.json and cef-run-42377685195-safe.json in
evidence:root/20261107073932. Raw private report/stage-log bodies were not retrieved;
their emitted hashes are commitments, not independent recovery. Do not infer
exact unprojected per-route/PID/canary counts from them.

This closes the bounded hosted component gate only. Actual keyboard/IME/Hanja
candidates, AX/screen-reader behavior, DPI, Chrome parity, full sandbox policy,
descendant termination and other tuples remain open. Native-feature strict Clippy
was not run by this workflow. Earlier failures and925 warm deferral remain intact;
their unknown causes are not retroactively diagnosed by this success.

## Allocated warm bootstrap compile: interrupted and deferred

Root allocated C for immutable9251376f99dba1fbe36af4bb2908ed5e39dd6047 warm-only
bootstrap test compilation and synthetic marker3. A clean detached snapshot,
locked/offline scoped graph and cached CEF SDK/wrapper were inspected. The compile
showed cold registry dependency compilation despite those cached CEF fingerprints.
The owning exec session7488 was interrupted on first log observation; it terminated
with exec1, not a known Cargo diagnostic exit. No download/cef-dll-sys build lines
were observed. Some dependency artifacts were produced before interruption; no
retry occurred and no leftover cargo/rustc/cmake/ninja/cl processes were observed.

Marker3, bootstrap runtime, check and Clippy are not_run. Source2459, registry80
packages/3709files, Git dependency161 and cached SDK941 formed a7270-file inspection
scope with zero file/config/flag/tool drift. Eighteen Cargo ancestor/home config
candidates and alternate CARGO_HOME resolution were recorded, plus actual cargo/
rustc executable hashes. Compiled target cache entirety and MSVC linker identity
remain unverified. No Python validation helper was copied or executed. C returned;
bootstrap-warm-925.json in evidence:root/20261107073932 preserves private receipt
hashes and the exact limited outcome. Another allocation or hosted candidate is
required; this is not compile success, source-error diagnosis or native readiness.

## Bootstrap child ownership and accepted closed observations

Root accepted a separate test-only cleanup correction: the existing four-case
bootstrap rejection test delegates to negative_pilot.rs and uses the shared
humanymous_test_support::process::ManagedChild. Its ten-second deadline, four
fixed flags, per-child exit78 and before/after synthetic-profile equality remain.
No custom supervisor, unsafe code or production bootstrap path is introduced.
This fixes an identified kill-without-wait risk; it is not the established cause
of either failed hosted run. Source formatting passes; compilation/runtime is
not_run. Root integrated the existing workspace Windows dev-dependencies
humanymous-test-support and serde_json plus two lock dependency entries in
072c8e5a6bc4cfbfdb21f6d3ec5942c84d3fe8f8. No package version changed.

Root accepted a bootstrap_rejection
block in the existing owned pilot-contract.json, not a parallel verifier. It
names the original exact test, schema1, expected exit78, deadline10000ms and four
fixed IDs: no_sandbox, no_sandbox_zero, disable_web_security, disable_gpu_sandbox.
The prefix is HUMANYMOUS_BOOTSTRAP_OBSERVATION followed by one bounded
JSON record. Case facts are spawn_ok, wait_completed, deadline_expired, exit_code
or null, cleanup_attempted, reaped only when wait/stop actually observed status,
and fixed spawn/wait/cleanup error-kind labels. Missing facts remain null, not
success. Separate before/after profile inventory records carry only observation
success, actual changed boolean when available, and fixed error kind; no names.

The source producer emits facts before unchanged assertions. It includes no PID, raw messages, flags,
paths or child output. CI plans --show-output to preserve exact harness result
lines. Parent-owned child streams stay private and no child output is forwarded;
unread pipe backpressure may cause a deadline failure, never passing output proof.
The marker producer and three pure privacy/null-observation tests are authored,
not compiled or executed. Their exact selector/names are in the same contract;
CI owns consumer synchronization. Deadline is null until wait is attempted;
cleanup_attempted describes explicit stop after wait error, not unobserved Drop
effects. Missing/duplicate/ignored receipts cannot become inferred executed counts.
Root's earlier77ca workspace build does not qualify this new source or graph edge.
Descendant cleanup remains unqualified.

## Diagnostic CEF execution: native build and protocol passed, bootstrap failed

Run42372115353/job128402202007 executed exact source
e77b9792b5c31db25e008c0b4a6f42ae7ac969da. Runner11 and Clippy passed; native
build exited0 with bounded diagnostics untruncated. The existing native-feature
test binary executed all four declared synthetic protocol tests:4 passed,
0 failed/ignored,13 filtered. This validates the report regressions in that
source, not CEF initialization or actual desktop behavior.

The next bootstrap-rejection stage failed. Its child exit, executed-case receipt
and private log were not projected, so the case's execution and cause remain
unknown. Desktop2 was not reached. source_changed=null because post-execution
closure was not reached; do not claim an unchanged runtime source. Artifacts0,
upload skipped. The safe report/source/log hashes are retained in
cef-run-42372115353.json in evidence:root/20261107073932; private report and native
log bodies were not recovered. CI owns safe typed bootstrap-stage diagnostics;
root owns any subsequent execution. The older0a1 failure cause remains unknown.

## First actual CEF execution: failed before protocol tests

Dedicated run42370958371/job128398793332 executed exact source
0a1bb36ef53cf67a7f8739a2ad470398747235e1. The53 candidate inputs matched the
frozen checkpoint. Six standalone runner tests and runner Clippy passed. Actual
native-build stopped with child exit101 and wrapper exit1; no native-feature
test binary, protocol4, bootstrap1 or desktop2 qualification followed. Upload was
skipped and artifacts API returned0. Native-build.log and report body were not
recovered, so the root cause remains unknown. See cef-run-42370958371.json in
evidence:root/20261107073932. Preserve this failure; do not rerun the opaque
candidate unchanged or infer a compiler/SDK/network source fix from exit101.
CI owns bounded safe build diagnostics before a root-owned subsequent execution.

Read-only review found no new CEF calls in the production813-to0a1 delta; new
protocol modules are cfg(test) and were not the initial native-build execution.
This does not establish API correctness. Creation/cancellation can race callbacks
and the strict empty-inventory check must expose unexpected observations. Owned
OS token snapshots can fail or miss transient processes; absent joins stay unknown.

## Frozen first native candidate and required execution

The next candidate combines browser cancellation verifier35cca50f, exact protocol
inventory3d52eec0, shared captured-PID ownership and the existing CI-owned
scripts/ci/cef-pilot runner. Its standalone six runner tests/check/Clippy passed
separately; no Windows desktop test binary or CEF process result follows from that.
next-cef-candidate.json in evidence:root/20261107073932 binds the committed source
closure and prerequisites. Root owns the final pushed SHA and sole dispatch.

Run the existing .github/workflows/cef-native-component.yml on standard
windows-2025 x64, from a clean checkout matching GITHUB_SHA. ACT is rejected;
do not spoof hosted environment variables for a local run. Root must allocate
any local heavy build/SDK/GUI separately. The runner's actual entry command is
`cargo run --locked --manifest-path scripts/ci/cef-pilot/Cargo.toml -- target/cef-native-evidence`.

The ordered gate groups are:

1. Resolve the unchanged locked graph, exact CEF/Meari pins, Visual Studio tools,
   SDK runtime/manifest and required locales. Preserve source/runner failures.
2. Compile bootstrap and desktop, then compile the existing Windows desktop test
   binary with native-pilot-tests and --lib --no-run. A compile failure means
   later test groups are not_run, not passing or ignored.
3. Execute the contract's pilot_assertions::tests:: selector: exactly four named
   synthetic protocol tests, zero failed/ignored, actual filtered count retained.
   This exercises35cca cancellation negatives without starting a CEF process.
4. Execute exactly one bootstrap security-override case covering four refused
   flags before UI/profile creation. Preserve its separate denominator.
5. Provision two distinct fresh private bundles. Execute exactly two desktop
   cases: positive exit0 with seven ordered route observations and real cleanup;
   cancellation exit78 with actual empty renderer inventory, initial-transaction
   receipts and real cleanup. Do not manufacture zero from missing observations.
6. Compare source closure after execution and bind actual binary hashes. Publish
   only sanitized report.json; private runtime profiles/source/device data and
   captured stderr stay outside the public artifact allowlist.

The seven executed tests are protocol4 + bootstrap1 + actual desktop2, not seven
native UI tests. The positive desktop case separately checks seven navigation
steps: newtab, the five admitted diagnostics, then newtab again. Failed or missing
canary/token joins, source loss, unexpected cancellation callbacks or unsupported
GUI availability fail this bounded qualification rather than weakening sandbox.
Creation/cancellation scheduling and cleanup remain actual runtime questions.

After this bounded gate, separate real keyboard/IME composition and Hanja
candidate display/placement/selection, native AX focus/geometry and screen-reader,
DPI/monitor, process-tree termination and full sandbox-policy gates remain open.
External browsing and BrowserReady stay disabled until their controlling gates.
No SDK, Meari substitution, extra authored language or scoring change is introduced.

Implement actual Rust Meari/CEF host lifecycle, permissions, input and accessibility
with supported-tuple evidence. REA stays disabled; no new browser research,
unallocated SDK/GUI/heavy compile or duplicate dispatch. Current source authoring
is permitted; root assigns actual execution and the final source/dispatch. Shared
Cargo/lock/toolchain/push belongs to root. Current execution authority is recorded
below; historical hosted-only notes do not override later user authorization.
Central execution policy remains sot:root/program-roadmap and sot:root/delivery.

- [x] Claim browser-only source/spec paths after product-lab owner released misc;
  root-approved transition to the formal browser-ui lane completed.
- [x] Inspect existing seams; accept UI-25..27 and record same-URL attribution limit.
- [x] Implement one pure ticket/terminal state owner and ordered seven-step probe.
- [x] Wire native task rejection, unavailable frame, revoke/closed callbacks and
  strict report/exit closure; collect failure outcomes and receipt-loss count.
- [x] Bound owner/query/resource parsing before allocation; reset canceled IME.
- [x] Run only root-assigned resource-bounded pure unit/fmt/scoped Clippy checks.
- [x] Execute source-bound bounded Windows compile/protocol/bootstrap/desktop gate
  at exactec4, preserving separate counts and safe-source closure.
- [ ] Complete GUI/IME/AT/GPU/DPI, full policy/failure-stream and supported-tuple
  qualification beyond that bounded component gate.
- [x] Register exact public/private evidence and coordinated owned-path checkpoint
  for implemented source/pure units; hosted qualification remains open.

Review: P0 cleanup-only exit0 on failed/missing internal probe is rejected by
UI-26; P1 post-task failure left requested state and missing closed guards are
covered by UI-25; URL/parser and canceled composition bounds by UI-27. Residual
same-URL engine callback attribution is explicit and grants no authority. Do not
claim native compilation or a completed browser from pure unit success.

Dependency owners: root assigns local validation slots and remote source
closure/dispatch; CI/native-runner owners provide suitable hosted execution and
raw recovery policy. Product-lab ownership is deployments/test-lab only. This
browser writer edits no shared manifests, CI workflow or other owner's canon.

Implementation checkpoint is source-only until assigned pure-unit and hosted gates
execute. Report revision2 adds requested/finished/observed/passed and receipt loss;
older schema/missing fields remain unknown. Native task/source ticket protection
does not resolve same-URL engine callback attribution. No local native build or
CEF/runtime execution has occurred in this slice.


Hosted runner seam (not_run): positive --native-internal-probe must supply report
navigation_schema2, requested/finished/passed true, observed7, dropped0 and real
before_close/host_closed. Also combine --native-cancel-before-created with the
probe flag: expected nonzero with explicit incomplete probe, while cleanup must
still be measured separately. Do not add a synthetic expected-failure flag that
returns the desired code without exercising actual owned lifecycle. These cases
need CI/native-runner owner execution after root source closure; no duplicate
dispatch and no CEF workflow edits by the browser owner.

Source-only evidence:root/20261107073932 records actual formatter/spec/whitespace
checks and zero executed unit/native tests. Linked change:root/20261107073933
records all governing historical plan deltas without changing root-owned canon.
Goal and verification queue remain active; no completion claim.

Root-assigned A/shared target completed default pure23/23 and Clippy with jobs1/
incremental0. First missing-field-rustdoc Clippy failure and corrected rerun are
preserved separately in evidence:root/20261107073932. Native adapter compile and
all actual hosted native/GUI/IME/AT/GPU/DPI gates remain not_run; goal active.


Root expanded the exclusive writer scope to crates/native-cli (scaffold consumer),
not operational crates/cli. Source helper cfg now matches its Windows-only library
consumer; integration process helpers are unchanged. Existing native-embedding
SoT/plan07042614 still own its API contract. New hosted Linux Clippy is not_run;
no Cargo during root's graph window. Track that source fix separately from the
23 passing browser pure units, which did not compile/test native-cli.


UI-28 follow-up: scheme registration currently logs failure but does not prevent
owner factory startup. Record actual per-process acceptance, reject factory
startup without browser acceptance, and skip renderer canary work without its own
acceptance. Missing reports remain unknown; no false file-denial/all-process claim.
This is a native-only source delta after c4b58be, not covered by its pure23 result.
Actual compile/probe and missing-registration failure test are hosted not_run.

UI-29/30: implement independent receipt verification and contemporaneous bounded
owned-token correlation for renderer canary claims. File-probe schema2 is separate
from navigation schema2. Missing/failed token observations disqualify canary joins.
Protocol fixtures do not count as process tests. Shared supervisor dependency and
actual hosted positive/cancellation driver remain pending root graph coordination;
new native compilation and protocol execution are not_run.

Root subsequently approved the narrow desktop native-pilot-tests feature. The
two provisioned process cases are wired behind that explicit feature, with no
ignored cases and no successful environment-missing skips. The manifest is frozen
for root lock resolution. Future hosted execution uses separate fresh bundle
variables HUMANYMOUS_CEF_POSITIVE_BUNDLE and HUMANYMOUS_CEF_CANCELLED_BUNDLE:
`cargo test --locked -p humanymous-cef-desktop --features native-pilot-tests pilot_process:: -- --test-threads=1 --nocapture`.
Require exactly2 passed/0 ignored; default tests do not discharge this gate.
No Cargo or native execution occurred during this graph window. Source/artifact
provisioning and raw receipt retention remain the owning hosted runner's duties.

The user subsequently authorized actual local Windows/WSL validation for measured
comparison with GitHub, superseding the blanket hosted-only restriction. A root
benchmark quiet window is currently active: no new Cargo/native/runtime/model/
Docker commands. Browser owns no active execution handle or allocated pipeline.
Root will measure an isolated committed checkout before revising execution policy;
this authorization is not a benchmark result or tuple qualification. Historical
not_run records are preserved. Common manifests/locks/toolchain remain root-owned.


## Browser three-to-one actual transformation candidate

Root requested actual source and manifest candidates under evidence-only ownership.
three-to-one-integration.md and its source-map/source/caller patch attachments in
evidence:root/20261107073932 bind49 Browser preimages,47 destination entries and
seven Rust caller candidates. Both patches passed applicability checks against
preserved preimages; no product source, manifest, lock, path or execution changed.
The single-package composition preserves separate mutually exclusive bootstrap
and desktop role builds, ABI/bundle basenames, sandbox, pure unsafe prohibition,
pinned upstream inputs and normal SDK separation. Root owns final graph/Tools
rebase, contract replica provenance, Delivery triggers and coherent validation.
No historical pilot qualifies this composition; unfinished CEF/UI/pilot and
Core/Gate demo contracts remain open. Full application order and limitations are
in the attached integration contract. No compiler/native/browser run occurred.


## Explicit d340 standard-context correction

Root found native Rust implementations outside package src in the initial
d340 candidate. src-native-delta.md and its full map, four applicable patches
and static receipt supersede only proposed layout/composition/include paths.
All49 original Browser records/47destinations/7callers retain their preimages;
42 Rust files now reside under src, with one shared scheme source and10checked
literal include targets. Original d340 fullcandidate/patch/map/pins are immutable.
Root may choose a fresh corrected-map application or exact-d340 delta route,
never both. Runtime/ABI/unsafe/test identities, contract counts, feature roles,
sandbox and model API are unchanged. No source/compiler/native/tool execution
grant follows. Root owns final graph/Tools assembly and all unexecuted gates.


## CA-BROWSER-ROLE-CALLER-01 actual caller correction

Root found main/provisioning transformations absent from earlier applicable
caller patches because mixed path separators skipped conditional edits. Prior
caller integration claims were overstated; immutable originals retain that failure.
root-cef-callers-correction.md/json and root-cef-callers-corrected.patch now bind
seven exact current Root isolated preimages to actual complete Rust postimages.
Sequential explicit role builds, immediate create-new verified DLL preservation,
fresh bundle assembly/hash joins, six explicit role tests, merged source roots
and version2 separate receipts are present in inspected function bodies.
Package-local CLI contract include/provenance stays unchanged; Root review and
replica refresh are mandatory before new selectors can pass. Three new Rust
regressions are authored/unexecuted. Applicability/static-source checks pass;
no formatter/compiler/test/SDK/native/browser execution or live source change.
This supersedes only prior caller candidates. Root adopts against exact hashes,
then resolves schema consumers and runs coherent role/native qualification.


## CA-BROWSER-ROLE-CALLER-01 Cargo artifact join delta

Root adopted d08 caller code but found guessed target/debug DLL preservation
was not joined to an actual selected-role compiler artifact. New fresh frozen
seven-file Root preimages and four changed postimages are attached in
cargo-artifact-role-join.md/json/patch. Metadata opaque package/manifest/library
identity, exact features/profile, one actual output, observed successful command
and final build-finished now precede immediate actual-path preservation. Cached,
duplicate/conflicting/stale/missing/foreign/escaping outputs refuse the join.
Private receipt retains source/metadata/ledger/role/path hashes and identity;
public projection and Root explicit same-byte test cleanup remain unchanged.
Four pure regressions authored/unrun. Patch applicability and static source
checks passed; no formatter/compiler/metadata/test/SDK/PE/native/hosted execution.
Root finalepoch/contract refresh and real role/pilot gates remain required.


## UI-40 Meari0.3.0 release review and upgrade candidate

Official v0.3.0 resolves to ef3d2d0a8acd2bc3c01ce40258d70a33c7e819a4.
The annotated tag and unyanked registry packages are verified; no GitHub Release
post/assets were observed. Current1482pin and shared/isolated manifests/lock are
unchanged. meari-030-review.md/json records20officialblob-matched source files,
compatibility/toolchain/distribution findings and read-only merged consumers.
Root candidate order: exactcoherentMearipin withCPUfeatures; lock/source/metadata
epoch rejoin and pure/document/rolegraph/strict gates; realSDKroles/artifactjoins;
CPUrelease/dev fullBrowser resize/input/IME/AX/DPI/sandbox; separatetrustedGPU
feature/selection, structuredhardwareclass/sceneparity/memory/lifecycle/package
and actualdisplay-validation gates. Unknown VirtualGpu/Other must not become
required-hardware success; upstream GPU choice alone is insufficient.
Adversarial verdict needs-work beforeGPUadoption; CPUupgrade also unqualified.
Upstream Linux reports do not count as our passes. No local Cargo resolution,
compiler/test/Clippy/SDK/GPU/process/browser execution or publication occurred.
