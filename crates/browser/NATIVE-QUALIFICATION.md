# Native browser qualification frontier

This execution note derives from sot:root/browser-ui,
sot:root/chromium-lab-browser, sot:root/native-embedding and
plan:root/20261107071648. It adds no support claim or execution allocation.
Root owns the final source, graph and execution allocation; this browser lane
owns the Meari/CEF integration. CI owns the existing provisioning runner.

## Current evidence boundary

UI-38 is a newer unexecuted source candidate: model-only retained tab projection,
responsive CSS for viewport-only geometry, checked owner/definition cache and
opt-in bounded numeric handler timings. New document/action/source-staleness and
timing-ring tests have not run. The ce24 normal build and human manual receipts
remain exact historical baseline; human150-200ms-plus Meari lag is still open.
Root owns next frozen unit/check/native/manual allocations and Meari release pin.

UI candidate046b5f1b passed43 native-ui document tests, including crowded-tab
geometry and existing action checks. Desktop test-profile compilation required
cef-dll-sys rebuilding and the allocated guard stopped it before test execution;
check/Clippy were unreached. No production DLL or second GUI was built/launched.
Changed-file formatter passed. Source87/fixed SDK3 hashes were unchanged; this
does not establish cache write exclusion or full tool/SDK provenance. Continuation
regression and source reproducer remain unexecuted native tests. Root alone can
allocate the next bounded native test-profile build; no implicit widening.

The first root-allocated fef GUI run is now retained in controls-gui-first.json:
two actual CEF IDs, CtrlT/CtrlTab and Version/newtab accessible content observed;
Back to newtab failed generation3 with load_error-3 and no commit/content. Final
visual observation was a blank surface with a load-error notice. Do not promote
this run to a passing controls gate. Both before_close/host_closed were reported,
but wait status, CtrlW timing and descendant-tree exit remain unknown.237 runtime
input hashes were unchanged. IME/AT/full sandbox/Chrome parity remain open.

Source-only follow-up preserves an authorized main-frame CURRENT_TAB ticket at
OnOpenURLFromTab, adding bounded open_continued receipts. The callback was
previously always cancelled, but first-run callback causation is unobserved.
Fresh tests/check/Clippy and a separately allocated GUI run are still required.

Exact fef69a92 subsequently passed local Windows bootstrap+desktop all-target
check, production DLL build and desktop native-pilot-tests all-target strict
Clippy. The87-file source/manifest and three fixed SDK input closures were
unchanged; actual artifact hashes are in controls-native-preflight.json. No
runtime/process/GUI test executed in that batch. Full keyboard/IME/AT/multi-tab
cleanup and sandbox policy remain open despite compilation and feature lint.

Newer UI-32/33 history/tab/shortcut source ce05c45e passed30 default pure tests,
default check/strict Clippy and formatter with explicit173008f0 graph overlay.
Subsequent d75c351c passed42 native-ui document tests, check/strict Clippy/fmt with
87 declared files unchanged. Required native document AX actions and retained
input/focus passed after three preserved scene failures. CEF compilation was
unexecuted at that scene checkpoint; fef above closes compilation, not GUI.
Actual d2148b0a build in run42388354925 subsequently failed101 with E0599 at
browser_controls.rs:76 before any protocol/bootstrap/desktop tests. Static pinned
winit review identified the obsolete super_key() call and its meta_key() API.
The correction was then unverified; fef above closes compilation/lint only.
Private compiler text was not
recovered; diagnostic source review and compiler-message custody stay separate.
The old single-child ec4 results below do not qualify the new lifecycle source.

Windows x64 sourceec4 passed native compilation, desktop synthetic protocol4,
bootstrap synthetic marker3, actual bootstrap1 and actual desktop2, after runner14/
Clippy success. Four refusal children exited78 with observed reaping and unchanged
profiles. Final declared source_changed=false; original private report bodies
were not retrieved. That run did not execute native-feature strict Clippy;
manual GUI/IME/AT remain open, while fef covers the later compile/lint gate.
Earlier0a1/e77 failures and925 local cold-dependency deferral retain their separate
source scopes and unknown causes; the new success does not rewrite those records.

Older Windows GUI, IME and UIA pilot observations are historical component
evidence, not qualification of the current source/layout. No other OS/architecture
tuple is qualified by Windows compilation, a stub, cfg exclusion or a planning
manifest. Full BrowserReady and external target browsing remain disabled.

## Required observations and owning seams

| Boundary | Evidence needed on the actual exact-source tuple | Owner and present frontier |
|---|---|---|
| CEF lifecycle | Actual parent/child identity, seven ordered diagnostic routes, current ticket/status/content receipts, real before-close and parent-close, cancellation and failure receipts; separately prove descendant termination | Browser integration + CI process gate. Ec4 desktop2 passed bounded producer/consumer checks; protocol4 is synthetic. Failure streams and descendants remain open. ManagedChild owns only the direct child. |
| Native keyboard and focus | Physical key edits in both CEF fixture and Meari address, Tab/Shift+Tab traversal and focus readbacks across both surfaces; IME-confirm Enter must not navigate | Browser integration with suitable Windows input environment. Programmatic Unicode/value actions are not physical key evidence. |
| Korean IME | Actual incremental composition/update/commit/Backspace/Escape, physical or measured VK19/scanF1/extended Hanja delivery, visible candidate identity, caret-relative placement, selection and resulting committed text | Browser integration + installed IME and interactive operator/environment. SendInput insertion or IMM request success is not candidate proof; known IME class geometry alone is not ownership proof. |
| Accessibility | Fresh roles/names/states/values, focus and usable bounds for Meari controls and CEF content; keyboard-only actions; real OS screen-reader output on synthetic content | Browser lane owns app integration; root owns any upstream Meari pin change. CEF accessibility enablement/UIA exposure alone is not full AX/AT qualification. Missing bounds/stale values remain failures or unknown. |
| Window/DPI | All eight resize directions/cursors, maximize/restore/minimize, keyboard resize/Snap, DPI transitions and multi-monitor candidate/AX geometry with cancellation of capture | Browser integration. Historical96-DPI gestures do not qualify high-DPI or current-source behavior. |
| Full sandbox | Renderer/GPU/network/utility identity and policy, token/mitigation/job boundaries, source-bound file-denial correlation and positive controls, cleanup and distribution policy | Native-security/root + browser/CI observations. Renderer Untrusted/restricted token plus one canary does not attest all processes or paths. Prior Network Service Not Sandboxed observation remains unresolved. |
| Chrome rendering fidelity | Pinned real Chrome reference, matching content/assets/fonts/scale/GPU conditions and bounded visual/behavior comparison | Browser/root reference-baseline allocation. Actual CEF rendering or matching Chromium version alone is not Chrome parity. |

The SDK/native-ABI/framework owners qualify their separate embedding contracts.
Loading a DLL, native-cli planning output or this process-owning browser entry is
not an SDK lifecycle/ABI/sandbox qualification. Product-lab browser receipts also
do not substitute for Meari native GUI/IME/OS screen-reader observations.

## Suitable environment and receipt prerequisites

Before any actual GUI run, root must allocate the exact Windows tuple and private
fresh bundle, state the interactive desktop and installed IME/AT capabilities,
and bind current source, graph, configuration and binary hashes. Unsupported
desktop/language/AT availability is explicit, never an ignored successful gate.
Do not change sandbox or substitute another engine to make the run pass.

Keep input-fixture mode, normal owner-newtab mode and internal-probe mode distinct
in receipts. Record viewport/caret bounds, DPI and monitor context. Native input,
accessibility value actions, candidate-window observations and committed text are
different evidence streams. A successful API request is not its visible effect.
Reuse approved owners instead of adding global hooks, foreign input queues or
unscoped desktop/process inspection.

Use synthetic owned content and private raw retention. Publish only bounded
counts/statuses/source identities and permitted diagnostics. Raw keys, AX text,
candidate text, speech, device diagnostics, profiles and screenshots stay private
unless separately reviewed and authorized. Public binary previews are not image
secret scans. Missing raw recovery or post-execution closure stays unknown.

The next implementation decision follows actual safe bootstrap observations and
desktop receipts. Do not guess the bootstrap failure cause, erase old failures,
weaken exit78/profile predicates or replace absent observations with zero.

## Original fef GUI component recipe (first run failed; rerun unallocated)

This is a qualification-pilot entry, not a production native-SDK capability
factory. The normal, non-cfg(test) Rust DLLs export RunConsoleMain and
RunBrowserDesktop. The pinned SDK bootstrapc.exe, renamed
humanymous_cef_bootstrap.exe, supplies the borrowed sandbox pointer. It calls the
bootstrap DLL; execute_process dispatches subprocess roles before the browser
loads the fixed sibling desktop DLL. The desktop creates a fresh synthetic
profile, requires no_sandbox=0, registers local pages and runs the Meari/CEF host.
No test-only credential factory, fabricated NativeFull capability or production
credential/provider admission is used. Those SDK/service contracts are not wired
to this standalone pilot and are not qualified by its GUI. BrowserReady=false.
The concrete host composition is NativeWindowPreparation::new followed by
NativeHostRuntime::new(document, ChildWindowHost::new(Host, children)) and attach.
Host rejects every target callback admission; the registered CefChild leases own
cleanup. This normal Cargo artifact is not an SDK production-admission factory.

Before launch, verify the fef DLLs against controls-native-preflight.json:
bootstrap287744B/SHA25660e80c8724be9165f8045ec1d6d3556ef887bf38c9cb6dce6ed16f30685f4980;
desktop58548736B/SHA25656df65a0b1e3545389b2d45eb70a9bea245108b2ef10b424a50474bb501a9341.
Use bootstrapc.exe, not the distinct GUI bootstrap.exe: the Rust export is
RunConsoleMain. Reuse the pinned cef-app.exe.manifest with asInvoker; do not
author a new manifest or request elevation. The SDK loader and manifest hashes
are privately retained in gui-entry-inputs.json. The earlier compilation closure
covered bootstrap.exe, not bootstrapc.exe, and does not substitute for this check.

Create a nonexistent case directory under the private browser evidence bundle.
Copy the two exact DLLs, renamed console loader, unchanged upstream manifest and
the same whitelisted runtime/locales as scripts/ci/cef-pilot/src/provisioning/
bundles.rs. No links, previous reports, profiles, cookies or canaries are reused.
Validate every copied file hash and recheck input hashes immediately before use.
Prepared bytes or flag rejection alone never count as rendering observations.

Required Windows preconditions are an unlocked interactive desktop, the current
asInvoker account able to read the owned bundle and create its synthetic profile,
the matching pinned CEF runtime/sandbox bootstrap and usable native window surface.
This offline component does not request a password, DPAPI secret, IPC credential,
authority token or SDK production-provider credential. Their absence is not
waived for production: that integration remains unqualified. Stop on a locked
desktop, elevation/security prompt, mismatched input or unsupported provider;
never bypass it. Korean IME and screen-reader observations additionally require
the actual installed/available IME and approved OS AT/operator. Missing providers
remain not_run rather than inferred from document tests or API acceptance.

After root runtime allocation, launch exactly the owned executable with
--native-pilot-seconds=180, no fixture/probe/IME-replay flag for the first case.
The only routes are humanymous://newtab/ and permitted chrome:// diagnostics.
Record actual window visibility and a real rendered owner page, independently of
process title. Then observe Version -> Back -> Forward, Reload, Home, another
tab with a different diagnostic, keyboard cycling and retained per-tab address/
history, close of an inactive tab, and final-tab or parent close. Ctrl+T/W/Tab,
Ctrl+Shift+Tab/L, Ctrl+R/F5 and Alt+Left/Right need actual native delivery and
visible resulting state; pure mappings are insufficient. Record input provenance
(operator, native automation, indexed AX action) without human-attestation claims.
Capture per-tab CEF IDs, committed fixed routes and lossless native receipts.

Attempt Stop only while a pending load is actually observed. An idle stop or a
page that finished first does not qualify cancellation. IME preedit/candidate
placement/selection and actual OS screen-reader output are distinct follow-ups,
with caret/DPI/monitor context and private raw retention; screenshots cannot
replace missing AX. A separate fresh case requires a separate runtime allocation
if the first180s window cannot cover an observation. No timer/provider/claim is
silently broadened to obtain a passing result.

The existing Rust host requests coordinated close at180s and fail-stops78 at195s.
The observing owner tracks the exact launched PID, creation identity and bundle
path through a bounded outer210s deadline. Normal success requires exit0,
host_closed, before_close for every requested/created tab and no incomplete or
lost receipts. If the internal or outer deadline fires, stop app input and retain
incomplete/unknown cleanup; use only the existing owned process/child cleanup
authority. Never kill processes by a shared CEF name or terminate foreign tasks.
Window disappearance, command acceptance and direct-child exit do not prove
descendant termination or full sandbox policy. Raw keys/AX/candidate/speech/
screenshots/profiles remain private; publish only reviewed bounded summaries.
