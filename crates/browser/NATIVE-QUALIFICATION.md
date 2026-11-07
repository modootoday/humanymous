# Native browser qualification frontier

This execution note derives from sot:root/browser-ui,
sot:root/chromium-lab-browser, sot:root/native-embedding and
plan:root/20261107071648. It adds no support claim or execution allocation.
Root owns the final source, graph and execution allocation; this browser lane
owns the Meari/CEF integration. CI owns the existing provisioning runner.

## Current evidence boundary

Windows x64 sourcee77 passed native compilation and four synthetic protocol tests.
Its bootstrap stage failed with an unrecovered child/case result; desktop2 was
not reached and final source closure was unknown. Source925 adds shared child
ownership and closed observations. Its warm-only compile was interrupted when
cold dependencies appeared; marker3/check/Clippy/runtime remain not_run.
The first execution and subsequent evidence retain separate exact source scopes.

Older Windows GUI, IME and UIA pilot observations are historical component
evidence, not qualification of the current source/layout. No other OS/architecture
tuple is qualified by Windows compilation, a stub, cfg exclusion or a planning
manifest. Full BrowserReady and external target browsing remain disabled.

## Required observations and owning seams

| Boundary | Evidence needed on the actual exact-source tuple | Owner and present frontier |
|---|---|---|
| CEF lifecycle | Actual parent/child identity, seven ordered diagnostic routes, current ticket/status/content receipts, real before-close and parent-close, cancellation and failure receipts; separately prove descendant termination | Browser integration + CI process gate. Protocol4 is synthetic; current desktop2 not_run. ManagedChild owns only the direct child. |
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
