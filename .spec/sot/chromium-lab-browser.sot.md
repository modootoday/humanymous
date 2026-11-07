---
id: sot:root/chromium-lab-browser
title: Rust Chromium laboratory browser and observation adapters
status: active
created: 2026-11-06T23:34:52Z
updated: 2026-11-07T05:36:25Z
verify: manual
rel:
  depends_on: [sot:root/adapter-ecosystem, sot:root/adapter-service-contract, sot:root/adapter-authority, sot:root/automation-evidence, sot:root/accessibility-agent-tools, sot:root/mobile-agent-tools, sot:root/crate-architecture, sot:root/cross-os-runners]
---

# Rust Chromium laboratory browser

Document version 0.4.0. The research/UI expansion below is authoritative. Initial research checked 2026-11-07 KST; three read-only agents
examined embedding, observation/adapters and security/distribution. The initial research performed no builds or browser executions; later bounded Windows native evidence is separate. This document adds no
scoring rule, runtime implementation, probing permission or production coverage.

## Brief and accepted constraints

Build a website anti-automation validation/debugging browser with an embedded
tab experience comparable to ChatGPT Desktop's built-in browser. Chromium is the
preferred publicly supported engine candidate. The user explicitly
requires all authored browser application code to be Rust: native UI, lifecycle,
adapter, evidence tools and tests. Do not introduce authored JS/TS Electron main,
preload, HTML web UI, C/C++ glue or a second scoring implementation. Chromium/CEF
itself is an external native dependency, not a Rust engine implementation. [C1]

OpenAI's official Browser documentation establishes separate browser profile,
embedded tabs/split view, Computer Use and developer-mode CDP inspection. It does
not establish which public embedding library implements its tab surface. Local
app packaging does not prove that the built-in browser uses CEF or a particular
Electron view API. These remain unknown, not implementation requirements. [O1]

The user withdrew Owl REA/reconstruction scope: no Owl clone or clean-room track
is part of this browser plan. Earlier limited static metadata remains private
historical evidence only. Do not continue native/source/runtime REA on that basis.
The user subsequently authorized targeted browser-embedding REA, delegated to
project chat 01a1b34e-f9e9-77d3-b883-0cc4d76763a4. That scoped investigation does
not reinstate whole-runtime reconstruction or establish the current tab API yet.

The browser is an engine-observation research instrument. Its analysis scope
includes user-directed websites that do not integrate humanymous. Local engine
observations, ordinary-page-visible signals and optional verified server joins
are separate evidence planes. Active automation/detection experiments remain
restricted to owned/authorized fixtures. It grants no trust to its own browser,
replaces no server enforcement and promises no T4 detection. Missing integration
means not_integrated, not a failed website defense or an invented verdict.

## Architecture decision and alternatives

| Approach | Meaning | Research disposition |
| --- | --- | --- |
| Rust native shell + CEF through cef-rs | Actual embedded Chromium browser with Rust-authored application | Preferred candidate, conditional on MSRV/sandbox/FFI/packaging qualification |
| Rust supervisor + external Chromium via CDP | Separate browser process with Rust diagnostics | Earlier contract pilot; not embedded-browser completion |
| Electron shell + Rust sidecar | Chromium plus Node/JS application shell | Excluded by explicit Rust-only authored-code requirement [E1] |
| Tauri 2 / Wry | OS webviews: WebView2, WKWebView, WebKitGTK | Not one Chromium engine across all desktop OSes [T1] |
| Tauri 3 alpha CEF runtime | New CEF runtime | Not baseline: current Windows runtime is unsandboxed and Required cannot start [T2] |
| Chromium fork | Full custom engine build and patch ownership | Deferred unless a required interface cannot be supplied through CEF |

cef-rs is maintained under tauri-apps, not an upstream Chromium-maintained Rust
API. Observed source 224a785c7b41758b51821cbb7395ea57ba493a63, dated 2026-11-01,
declares cef/cef-dll-sys 156.3.0+156.0.15, edition 2021, Apache-2.0 OR MIT. No
rust-version is declared; full binding closure compatibility remains unverified.
The user subsequently approved core/toolchain1.99.1. Lock exact features/dependency closure and
prove native target builds before adoption. README target support is not our
execution evidence. [C2]

Use one product bounded context, with native Rust UI and engine/lifecycle/observation/
evidence submodules. Final package names follow humanymous-* conventions only
after a concrete slice and dependency plan. Do not create one crate per protocol
or duplicate fixtures/security helpers. A native integration boundary must own
FFI thread/refcount/lifetime risks; preserve unsafe_code=forbid in shared core.
If chosen integration requires project unsafe code, resolve a narrowly reviewed
boundary policy before implementation rather than weakening workspace defaults.

The user selected modootoday/meari for UI. Meari is mandatory for the authored
Rust browser controls; egui/iced remain historical comparisons, not selected
implementations. Pin 1482b746ebc0d867f25d17d01c48c12df888ca6c, version0.2.0,
license MIT OR Apache-2.0. Its native renderer is not Chromium and does not replace
the target-content engine. Rust source need not include authored TSX/JS. [U1-U3]

Meari pins Rust1.99.1 and its mandatory oxc_parser0.154.0 declares Rust1.97.0.
The user explicitly approved core MSRV/toolchain1.99.1, removing that known floor
conflict with the former1.88 baseline. Keep desktop dependency/feature boundaries
separate and prove the full closure; no full Meari build was performed. [U2]

Meari's generic native child registrar and scoped parent leases are candidate
integration seams. Its video surface slot accepts real video nodes and requires
RenderOnly focus: it is not an interactive browser embed port. A Rust browser
surface/input/focus/IME/AX lifecycle contract is required. Do not disguise CEF as
a video node or carry a scoped raw parent handle beyond its allowed lifetime. [U3]

First qualify windowed CEF with a Meari Rust management window. Parent handles, focus,
DPI, resize and message-loop ownership require native tests. Off-screen rendering
adds paint/texture/input/IME/popup/accessibility responsibilities and must not be
assumed behaviorally neutral or screen-reader equivalent. [U1, C7]

```text
Trusted Rust native controls -> Rust browser supervisor -> CEF/Chromium processes
                                  |                         |
                                  | bounded observations    | approved research navigation
                                  v                         v
                           private lab evidence       Rust ingress / Gate
                                  |                         |
                                  +---- receipt join <--- shared authority/audit
                                                            |
                                                   independent origin counter
```

Chromium renderer, GPU and network subprocesses do not share the Rust authority's
trust. Target content receives no authority credential, signing key, arbitrary
filesystem/command bridge or management API. Native browser controls and target
pages have separate privilege and navigation boundaries. Permissions, downloads,
external protocols, popups and file selection default to explicit scoped policy.

## Sandbox, platforms and distribution gates

CEF Windows M138+ requires its supported toolchain/bootstrap integration; ordinary
Cargo executable linking is not proof of sandboxing. The supported bootstrap can
load a client DLL and convey sandbox state. macOS requires correct helper sandbox
initialization before framework load, nested packaging and signing. Linux needs
actual namespace/helper/seccomp compatibility. Never qualify an unsandboxed run
as normal browser coverage. A sandbox-policy setting alone is insufficient. [C3]

Tauri 3 alpha's current Windows limitation is specific to that runtime, not proof
that every CEF route is impossible. Its Linux Auto policy can also drop sandbox
under namespace/AppImage restrictions; fail-closed startup is required for this
product. No OS security feature is silently disabled to make a pilot pass. [T2]

cef-rs advertises Windows/Linux/macOS x64 and ARM64. Engine artifact availability,
native ABI, Rust host, actual sandbox and signed clean-machine packaging each
require independent evidence. Current Chrome for Testing lists linux-arm64 since
153.0.8001.0, linux64, mac-arm64, mac-x64, win32 and win64. It does not list
win-arm64; x64 engine emulation on Windows ARM is not native ARM qualification.
CEF and CfT artifact matrices are different. [C2, F1]

Engine update ownership includes Chromium/CEF security releases, native component
SBOM/notices, signed artifacts, integrity/provenance verification, safe atomic
upgrade/recovery and a version-policy gate for stale vulnerable builds. Historical
engine pins belong in bounded isolated regression fixtures. Cargo audit alone
does not inspect Chromium's native CVEs. Wrapper and CEF/Chromium/third-party
licenses must be accounted for separately; Google API services, codecs, Widevine
and Chrome branding are not automatically licensed or supplied by CEF. [C1, C4, C5]

## Additional adapter families

All IDs below are proposed contracts/modules, not delivered adapters.

| Family | Purpose | Authority boundary |
| --- | --- | --- |
| browser.cdp | External-engine diagnostics/control pilot | Browser observer only; versioned capability negotiation |
| browser.cef-devtools | CEF native in-process DevTools bridge | Same observation model, no public debugging port required |
| browser.bidi | Future cross-engine comparative transport | W3C Working Draft, implementer capabilities must be tested |
| browser.collector | Actual ordinary-page collector/RIT integration | No filling missing signals from privileged debug knowledge |
| browser.extension / native-host | Optional stock-browser companion | Deferred where authored extension JS is required by Rust-only rule |
| lab.evidence-join | Browser trace, server receipt and origin effect analysis | Correlation IDs never authorize effects |
| ingress.capture | Future genuine TLS/H2 connection observations | Server/capture provider owns provenance and stream binding |

CEF SendDevToolsMessage/ExecuteDevToolsMethod/AddDevToolsMessageObserver avoid a
public CDP endpoint. Browser-process UI-thread callbacks have transient payloads;
bounded owned copies/queues and explicit detach/subscription lifecycle are
required. External CDP prefers private inherited transport where supported;
otherwise a supervised restricted local endpoint. A local TCP port is not peer
authentication. Chrome 136+ protects its default profile from debugging switches;
use a fresh non-personal profile regardless of engine exceptions. [C6, D1]

CDP is high-privilege diagnostic control, not a stable wire protocol. Pin protocol
and engine; unknown command/event shapes and unavailable domains produce explicit
capability/loss states. WebDriver BiDi is currently the 2026-11-06 Working Draft,
not a Recommendation or certification of our adapter. [D2, D3]

## Observation contract proposal lab-observation/0.x

Before code, fix bounded schemas and version negotiation for:

- Run/observer identity and lifecycle epoch; engine/CEF/wrapper/protocol version
  and artifact hash; profile/config digest; OS/CPU/GPU and actual sandbox state.
- Target/session/frame/loader/request and redirect-hop IDs with scoped namespaces;
  monotonic clock domain plus wall-time mapping/uncertainty.
- Provenance runner_known, browser_observed, page_claimed, server_verified and
  wire_observed; cryptographic verification result separately from source label.
- Completeness complete, partial, unsupported, detached, overflow, truncated and
  unavailable; observer restart/loss counters and bounded queue/artifact budgets.
- Payload representation and byte fidelity; body digest only if computed over
  actually complete bytes. Redaction/truncation is explicit, never exact capture.
- Actual tool delivery, server verdict/receipt validation and independent origin
  count; action-to-request causal graph rather than assuming one action=request.
- Mode observe or intervention; cache/SW bypass, throttling, request replacement,
  injected instrumentation and network fault actions recorded as interventions.

CDP ExtraInfo may be absent or out of order. Header object abstraction and joined
duplicate values cannot establish wire ordering; header text may be absent under
H2/QUIC. Multipart post-data may omit file contents. Fetch stream takeover alters
response handling. Cache, workers, connection reuse/multiplexing, redirects,
retries and zero-request actions prevent naive ID joins. [D4]

NetLog is an optional pinned private diagnostic artifact, not the foundation of a
reliable network feature or production capture provider. Chromium's own design
warns against that use. Raw/sensitive capture modes can expose private bytes;
disable by default and use synthetic bounded traces only. [N1, N2]

Current humanymous.adapter/0.1 accepts the Node bounded complete application-HTTP
contract. It has capture=null and no browser collector, stream or admin endpoint.
The actual authority reports absent TLS/H2/collector/RIT planes and rejects routes
requiring them. Browser observations must not be forged into that envelope,
interpreted as node-raw exact bytes, or used to claim those routes are ready.
New server capture/session and browser collector slices require their own accepted
contract and tests while retaining the one shared enforcement authority.

## Requirements BRW-01 through BRW-10

- BRW-01 Rust-only: Authored browser code and tests are Rust. Native engine and
  upstream bootstrap dependencies are declared; Meari UI is required and Electron
  JS shell is excluded. Core and desktop use the user-approved1.99.1 baseline;
  every native dependency closure still needs actual qualification.
- BRW-02 No self-trust: Modified browser evidence is not human identity, server
  capture or a bypass credential. Keep actual authority and audit server-side.
- BRW-03 Safe process model: Prove sandbox and privilege isolation per tuple;
  reject unknown/disabled required sandbox. Keep trusted controls out of pages.
- BRW-04 Scoped operation: User-directed public passive research and explicitly
  scoped owned experiments have separate capabilities. Active automation uses
  owned fixture origin/package/egress allowlists, run budgets, fresh profiles
  and private authenticated control channels.
- BRW-05 Fidelity: Typed provenance, clock domains, byte representation and loss
  states survive every adapter. Never fabricate missing collector/capture/RIT.
- BRW-06 Non-interference: Baseline versus intervention runs are distinguished;
  no stealth/fingerprint camouflage or disabled site security as the default.
- BRW-07 Shared engine: No cloned scoring/rules/crypto; offline replay and live
  server decisions have explicit different labels and frozen scoring revisions.
- BRW-08 Accessibility: Keyboard/screen-reader/zoom/voice/zero-motor controls;
  no motor/speed/isTrusted prerequisite or accessibility-service ban.
- BRW-09 Privacy/update: Minimal bounded private artifacts, safe cleanup and
  signed versioned engine updates; native CVE and license checks beyond Cargo.
- BRW-10 Honest coverage: Tool/setup failure is not defense success; report all
  attempted/unsupported/error/loss/skip outcomes and retain the T4 ceiling.

## Milestones and qualification

| Milestone | Deliverable/gate | Current status |
| --- | --- | --- |
| B0 | Research, Rust-only constraints, trust/observation contracts | Research complete; detailed wire schema pending |
| B1 | Rust supervisor + pinned external-engine diagnostic pilot | Pinned actual embedded CEF bootstrap pilot exists; independent supervisor/observer wire completion pending |
| B2 | Meari Rust native UI + sandboxed CEF shell, tabs/navigation/permissions | Windows x64 bounded actual child/UI/newtab/internal-page pilot implemented; full sandbox/external navigation/history/multi-tab and other native tuples pending |
| B3 | Actual collector/session/ingress adapters and receipt join | Not implemented; existing missing-plane gates retained |
| B4 | Owned automation/AT controls, streaming/failure matrix | Not run |
| B5 | Signed clean-machine packages, update/CVE/SBOM, authored Pages docs | Not implemented/not run |

Unit tests must cover bounded schema parsing, provenance, loss, clock and lifecycle
handling. Native process pilots cover peer denial, renderer isolation, sandbox,
crash, cleanup and actual GUI accessibility. Docker-only backend/product e2e
remains the completion authority; native Windows/macOS browser pilots supplement
it rather than pretending a Linux container proves those native environments.

Compare stock current Chrome/Edge clean-profile human controls, CfT, CEF and
instrumentation on/off. CfT deliberately differs from ordinary Chrome and does
not auto-update; it is a separate experimental cohort, not a neutral human
baseline. Record flags, extensions, profile, GPU, fonts, cache/SW and proxy/CDN
termination. A local proxy changes what the origin observes. [F2, F3]

Cases include target/OOPIF/worker lifecycle; detach/queue loss; duplicate headers,
file/multipart/binary/compressed bodies; redirects/auth retries; streaming/SSE,
WebSocket and gRPC/H3 capabilities separately; uncertain delivery with once-only
origin effects and audit-before-effect; hostile target page denied local bridge,
filesystem and admin access; actual raw ingress capture; native screen-reader and
keyboard controls. Unsupported streams are explicit, not silently buffered into
a different semantic operation. All cases remain not_run.

## Critique and residuals

Deployment blockers include unsandboxed normal mode, private profile reuse,
unauthenticated control, page-to-admin privilege paths, browser-to-wire trust
promotion, engine update/provenance omissions, modified-browser-only performance
claims and motor-based exclusion. The custom engine/app being familiar or used by
ChatGPT does not qualify this new browser's security. No OpenAI code or runtime
redistribution permission is inferred. Standards draft-v0/public Pages must derive
authored proposal requirements and measured limits, not expose raw lab traces.

## Primary sources

- C1 https://github.com/chromiumembedded/cef/blob/46ff67dd15839c07f1c1c919170eb70dc4f353ba/README.md
- C2 https://github.com/tauri-apps/cef-rs/tree/224a785c7b41758b51821cbb7395ea57ba493a63
- C3 https://chromiumembedded.github.io/cef/sandbox_setup
- C4 https://github.com/chromiumembedded/cef/blob/46ff67dd15839c07f1c1c919170eb70dc4f353ba/docs/code_signing.md
- C5 https://www.chromium.org/developers/how-tos/api-keys/ ; https://www.chromium.org/audio-video/
- C8 https://chromium.googlesource.com/chromium/src/third_party/+/master/widevine/LICENSE ; https://github.com/chromium/chromium/blob/main/docs/security/updates.md
- C7 https://chromiumembedded.github.io/cef/general_usage
- C6 https://github.com/chromiumembedded/cef/blob/46ff67dd15839c07f1c1c919170eb70dc4f353ba/include/cef_browser.h ; https://github.com/chromiumembedded/cef/blob/46ff67dd15839c07f1c1c919170eb70dc4f353ba/include/cef_devtools_message_observer.h
- T1 https://v2.tauri.app/reference/webview-versions/
- T2 https://v3.tauri.app/develop/cef/
- E1 https://www.electronjs.org/docs/latest/tutorial/process-model ; https://www.electronjs.org/docs/latest/tutorial/security
- D1 https://developer.chrome.com/blog/remote-debugging-port
- D2 https://chromedevtools.github.io/devtools-protocol/
- D3 https://www.w3.org/TR/2026/WD-webdriver-bidi-20261106/
- D4 https://github.com/ChromeDevTools/devtools-protocol/blob/b0d96c6cdf39e5583e67a3762b2815a33154455b/json/browser_protocol.json
- N1 https://www.chromium.org/developers/design-documents/network-stack/netlog/
- N2 https://new.chromium.org/for-testers/providing-network-details/
- F1 https://github.com/GoogleChromeLabs/chrome-for-testing/blob/668bacf064563273e2bac4b2087bd7949884aef6/README.md
- F2 https://chromium.googlesource.com/chromium/src/+/HEAD/docs/chrome_for_testing/README.md
- F3 https://developer.chrome.com/docs/automation-and-testing/chrome-for-testing
- O1 https://learn.chatgpt.com/docs/browser
- U1 https://github.com/modootoday/meari/tree/1482b746ebc0d867f25d17d01c48c12df888ca6c
- U2 https://github.com/modootoday/meari/blob/1482b746ebc0d867f25d17d01c48c12df888ca6c/rust-toolchain.toml ; https://crates.io/api/v1/crates/oxc_parser/0.154.0
- U3 https://github.com/modootoday/meari/blob/1482b746ebc0d867f25d17d01c48c12df888ca6c/crates/meari-native/src/infrastructure/window/surface_slots/mod.rs ; https://github.com/modootoday/meari/blob/1482b746ebc0d867f25d17d01c48c12df888ca6c/crates/meari-native/src/infrastructure/window/parent/mod.rs

## Expanded research contract RSRCH-01 through RSRCH-16

This section supersedes the old owned-fixture-only research scope. It is accepted
design, not a claim that external navigation or observation adapters are delivered.
The current native pilot still denies external navigation. Rust-only, mandatory
sandbox, a11y, defense-only active testing and scoring freeze remain in force.

### Mission and operating modes

RSRCH-01: Study which browser/engine observations can distinguish manual use,
screenshot/pointer Computer Use, AX semantic actions, CDP/browser control and other
automation, and which of those observations an ordinary website could actually
obtain. A site's humanymous integration is not a prerequisite for local analysis.
Do not collapse automation, authorization, vendor identity and malicious intent.

RSRCH-02: Modes are explicit and immutable per run. external_passive_research is
user-directed ordinary public HTTPS browsing plus bounded local observation;
page navigation itself creates network effects. owned_validation additionally
permits pre-authorized automation, active probes and attack-catalog experiments
within declared fixture scope. CDP/AX clicks, script instrumentation, interception,
replay, fuzzing, fingerprint/challenge tests, private destination access and stress
traffic are not silently admitted by passive mode. No third-party active probing,
access-restriction bypass or adaptation to evade site defenses is authorized.
The public passive initial scope excludes personal accounts, credentials/forms,
imports from personal profiles and automatic submission. Controlled benchmark
cohorts use separate owned fixtures, not arbitrary production websites.

RSRCH-03: Each observation declares visibility_plane=engine_private |
page_observable | server_observed | wire_observed | unknown and independently
source_provenance=runner_known | browser_observed | page_claimed |
server_verified | wire_observed. Confidence is not trust authority. A local known
CDP command, observer attachment or synthetic-input request is ground truth for
that run, not proof of detection by the site. A page_observable claim requires a
measured ordinary-page exposure mechanism and its constraints, not a copied
privileged flag. Absence of observation is not proof of absence of automation.

RSRCH-04: Tool attribution requires actual control/delivery evidence. Record
requested, accepted, OS/protocol delivered, page effect, network effect and server
receipt separately. UIA Value changes, native OS injection and physical hardware
input are distinct cohorts even if they produce identical text or trusted events.
A control-channel label is not a website-visible fingerprint. T4 remains an
intentional ceiling; no speed/motor/isTrusted/AT-presence rule becomes enforcement.

### Research admission and trust boundaries

RSRCH-05: Derive per-tuple capabilities rather than fabricating one ready boolean:
engine_navigation_admitted, native_ui_capabilities, observer_capabilities,
server_planes and product_qualified each carry state/reason/evidence references.
Required sandbox disabled/unknown refuses navigation. Missing Gate/collector/RIT
or server integration does not itself disqualify passive rendering; it produces
not_integrated/unavailable server evidence. Legacy BrowserReady stays false for
the current prototype. UI capability failure cannot be hidden by screenshot-only
substitutes or permission weakening. Enabling external browsing is a later code
slice with its own admission tests; this design is not that admission receipt.

RSRCH-06: Target frames/workers/content remain hostile and never obtain native
control/admin commands, authority credentials, signing keys, filesystem access or
an unrestricted inspector bridge. No public unauthenticated debugging listener.
Pin protocol/capabilities; local TCP alone is not peer authentication. Validate
bounded data before native display, navigation or evidence joins. Reader text
cannot change observer policy, labels, experimental ground truth or credentials.

RSRCH-07: Initial destination policy admits operator-selected public HTTPS and
required public dependencies under a declared network policy. Block private,
loopback, link-local/metadata and management destinations across DNS changes,
redirects, frames, workers/SW, WebSocket/WebRTC and background paths. Browser
callbacks alone do not prove OS egress enforcement. Resolve CDN/subresource
admission explicitly: blocking required fonts/resources changes rendering and
must be classified as policy_restricted. Preserve TLS verification and normal
site CSP/security; do not disable sandbox/site isolation/security or camouflage
fingerprints to make a comparison pass. Proxy/DNS/DoH/cache/SW/cookies/background
traffic configuration is part of the run manifest.

### Observation model and experimental discipline

RSRCH-08: Proposed schema browser.research/0.2 includes run/experiment/cohort/scope,
operator authorization reference, engine/CEF/wrapper/protocol/artifact/config/
profile revisions, sandbox evidence and native tuple; observer epoch/sequence,
producer/visibility/provenance/fact_kind, scoped tab/frame/target/loader/request
IDs; monotonic clock/domain/wall mapping/uncertainty; action delivery stages;
representation/sensitivity/redaction/completeness; contamination references,
loss/budgets/retention and optional validated server receipt/origin effect join.
Every feature has a manifest mapping its collector hook, available CEF/protocol
API, actual exposure plane, supported versions, overhead and unsupported state.
An unavailable engine hook stays unsupported. CEF observer wrappers do not prove
Chromium-internal visibility that their public APIs do not expose. Any required
upstream hook/fork must be justified before implementation; authored C/C++ patches
remain excluded by Rust-only unless the user explicitly changes that constraint.
No duplicated scoring, cryptography, audit or detector engine is introduced.

RSRCH-09: States include not_requested, observed, inferred, hypothesis,
not_observed, unsupported, denied, unavailable, partial, truncated, overflow,
detached and error. Completeness applies only to a named domain/interval with
known subscription coverage. Keep attempted/failed/blocked/skipped/timed_out and
not_run cases in reports. Unknown protocol methods, queue gaps, observer restart
and uncertain effects cannot become zero-valued clean signals or defense passes.
No automatic retry of an action whose effect is uncertain.

RSRCH-10: Compare clean manual, legitimate AT, controlled native synthesis,
screenshot/pointer Computer Use, AX automation and CDP cohorts; record caller,
model/tool, host and action-delivery versions independently. Baselines include
observer off, minimum passive observer, specific enabled domains and explicit
intervention. Timestamp CDP attachment/subscriptions, AX activation, screenshots,
DOM execution, request replacement, throttling, cache/SW bypass and configuration
changes. An observer can produce its own tell; require paired/interleaved runs,
fresh-profile controls and blinded/held-out validation before causal claims.
Do not feed known cohort labels, commands or synthetic markers into a detector
and report that as independent discovery. A local observer and an evaluated
automation controller must have distinguishable identities and access policies.

RSRCH-11: Evaluate transferability in two stages: engine-private research may
suggest a hypothesis; ordinary-page/server exposure must then be established in
an owned test fixture. Report discrimination/false positives, uncertainty and
failed controls before proposing a SoT-backed signal. No proposal changes frozen
weights/thresholds/predicates/bands without separate explicit authorization.
Source/version registries and experiment manifests must permit negative-result
reproduction. Vendor/provider marketing is not ground truth for visiting traffic.

### Data, resources and lifecycle

RSRCH-12: Separate observed_in_memory, retained_locally and transmitted_externally.
Default research artifacts exclude page/AX text and values, screenshots, raw/full
URLs/query/fragment, headers/cookies/tokens, bodies, form/clipboard/keystroke data
and crash memory. Browser state needed to render is not automatically research
capture. Host/path/digests may identify activity and are not called anonymous.
Sensitive capture requires visible run-scoped opt-in, purpose, bounds/expiry and
revocation; redact before persistence/export and label reduced fidelity. Remote
Computer Use/model transmission needs explicit destination/data-class/retention
consent; browsing permission alone does not grant export permission.

RSRCH-13: Use dedicated fresh non-personal profiles. Persistent research profiles
or authenticated experiments require separately declared scope/lifecycle; no
personal Chrome profile/cookie/password/extension imports. Default-deny file and
external protocols, upload/download, capture, clipboard, location, notifications
and device permissions. Future grants bind origin/capability/epoch/expiry and
revocation. Maintain budgets for duration, tabs/actions/navigations/redirects,
requests/bytes, CPU/RAM, observation queues, payloads, AX tree size/depth, screenshot
cadence, artifacts and recovery attempts, with explicit exhaustion consequences.

RSRCH-14: Preserve Created -> Admitted -> Running -> Stopping -> Closed |
CleanupIncomplete lifecycle, per-process crash states, owned PID/start/image
validation and epoch revocation. Browser/renderer/GPU/network/observer failures
are distinct. Cleanup of ephemeral profile/cache, evidence retention and durable
audit/key erasure are different operations; durable erasure retains dual control.
A process-close receipt is not a disk-purge receipt. Existing retained pilot
profiles remain cleanup debt and do not define the production retention policy.

### Chrome rendering fidelity and native shell

RSRCH-15: Actual CEF/Chromium owns the target page surface. Meari renders browser
controls only; no page reconstruction, screenshot facade, video masquerade,
authored HTML app shell or target CSS patching. Normal admitted research browsing
must support ordinary target JS/CSS/images/fonts/iframes/workers and networking
under its declared policy. The existing script-disabled static fixture is only a
native input pilot, not evidence of normal website rendering or Chrome parity.

Compare exact full Chromium/CEF/build configuration, OS/arch, GPU/driver/backend,
DPR/page zoom/color/fonts/locale, resource versions, profile and flags. Same major
version alone is insufficient. Cohorts: matching Chrome for Testing where
available, separately recorded stock Chrome clean profile, upstream same-build CEF
reference, and this browser with observation off/on. CfT is a distinct cohort with
its own defaults, not an assumed neutral Chrome replacement. If exact matching
binaries are unavailable, declare mismatch and limit the conclusion.

| Area | Required owned-fixture qualification | Classification |
| --- | --- | --- |
| DOM/CSS | layout/flex/grid/transforms/stacking/scroll/forms/dynamic DOM/OOPIF | unexplained rendering discrepancy vs policy restriction |
| Fonts | pinned web/system fonts, shaping/fallback, Korean/CJK/emoji | environment mismatch vs renderer defect |
| GPU/canvas/WebGL | actual backend/acceleration, scenes/compositing/text | capability/driver difference vs defect |
| Media | declared codecs, playback/seek/fullscreen/captions/A-V sync | licensed/build limitation vs playback/compositor defect |
| Input/IME/AX | real mouse/keyboard/selection/scroll/native controls/IME/candidates/OS AT | native integration failure even if pixels match |
| Network/state | HTTPS/redirects/storage/cache/SW/WS/SSE and declared profile behavior | transport/configuration/policy difference |
| Host shell | content bounds, tabs/history/address/reload/stop/focus/close | shell completeness/geometry defect |
| Observation | off/on, dock positions and interventions | measurement overhead/contamination |

Capture window and content-only originals/diffs at equal content viewport/DPR,
not equal outer window size. Pair image checks with geometry/interaction checks;
derive tolerances from repeated same-browser noise. Record/limit mask areas and
never mask the tested feature. Performance uses paired/interleaved repeats,
cold/warm states, startup/navigation/frame pacing/input/resize/CPU/all-process
RAM, sample counts/median/p95/variation and all failures. Performance budgets must
be justified by a pilot, not invented as claimed Chrome equivalence. Chrome
sync/account/password/store/branded services and licensed codecs/CDM/DRM are
separate product/build/license rows, not automatically Blink rendering defects.

RSRCH-16: The selected UI is browser-first: compact top tabs/address/navigation,
real page filling the remaining client area, default-collapsed analysis dock
right or bottom. At default text scale, top chrome target <=88 DIP; at1280x800
client size collapsed content target >=89% height. Accessibility enlargement may
increase chrome height. Docking reallocates the CEF child; it cannot overlay,
reload, replace or change zoom/scroll/history/IME state of the page. Native child
bounds must match allocation within1 physical pixel at tested DPI. UI reports
engine-local/page-visible/server evidence separately and shows no integration
without implying page failure. Implemented controls operate or are explicitly
disabled; a visual tab/address does not imply working multi-tab navigation.

## Critique reflected and implementation frontier

P0 privileged observations mistaken for site detection -> RSRCH-03/04/08/11.
P0 external browsing silently grants active probing -> RSRCH-02/06/07.
P0 weak sandbox receipt or missing server plane conflated with readiness ->05/06.
P1 observer fingerprints/ground-truth leakage ->09/10/11; AT false positives ->04/10.
P1 private content/model export/profile residues ->12/13/14. P0 Chrome parity
claimed from engine brand or static fixture ->15; fixed560-DIP UI content offset
and diagnostic-dashboard chrome ->16. Panel used blue/red, privacy/SRE and evaluator
reviews; these are design findings, not five independent implementation results.

Current implementation frontier: pinned Windows bootstrap/child lifecycle and
bounded native input/IME/AX actions are partial evidence. External passive
navigation, complete observer feature registry, server joins, Chrome comparison
matrix, full native AX/DPI/cross-OS sandbox and product Docker e2e remain incomplete.
The native Hanja evidence is evidence:root/20261107033321; it predates the compact
UI and does not qualify new geometry. Follow plan:root/20261107033358 for this slice.


## Native embedding extension

RSRCH-17: The developer-integrated native SDK and Windows DLL extension is
canonical in sot:root/native-embedding. Share evidence/provenance contracts with
this browser, not its process-owning bootstrap entrypoint. Observation support,
website-visible automation detection and host enforcement remain independent
capabilities. Current implementation is browser-only; SDK runtime gates are not_run.


RSRCH-18: CEF internal chrome:// pages are a separate privileged navigation
capability, not public HTTPS targets or an application scheme to counterfeit.
Upstream supports internal WebUI (for example version and Chrome-runtime settings/
flags), but availability varies with exact build/style/configuration. Read-only
engine diagnostics require an exact-path allowlist, private-data display/export
rules and measured per-tuple support; settings/flags/debugging/crash URLs remain
separately denied unless explicitly admitted by policy. Never grant arbitrary
chrome:// access or weaken sandbox from scheme recognition. The historical fixture pilot
admitted none; current applied support is RSRCH-20 below. Primary references:
https://github.com/chromiumembedded/cef/issues/3892 and
https://github.com/chromiumembedded/cef/blob/master/docs/tutorial.md.


RSRCH-19: User-authorized newtab/internal diagnostics implementation follows
UI-17..22 in sot:root/browser-ui and plan:root/20261107050329. Distinguish upstream
registered URL, measured rendering support, product admission and state-changing
or destructive policy refusal. Pinned CEF156 sources show browser-initiated
quit/restart can act before OnBeforeBrowse; strict host preflight is mandatory.
chrome-urls exposes debug-page preference mutation and remains policy-denied.
Current matrix must be established by actual safe-page probes; no arbitrary
chrome:// compatibility or full sandbox qualification follows from engine branding.


RSRCH-20: Accepted/implemented/native-observed Windows x64 ALLOY matrix is
humanymous newtab plus chrome version/gpu/credits/license/sandbox, per UI-17..24
and evidence:root/20261107053625. Unknown and state-changing/destructive chrome
URLs stay denied. Bounded local diagnostics are available while external HTTP(S)
navigation and BrowserReady remain false. SDK/CLI/framework extension creates no
permission to browse or bypass a missing sandbox premise. These pages do not
expand website-visible detection signals or change scoring. Full network sandbox
is a known gap: WebUI reported Network Service Not Sandboxed; independent native
attestation remains outstanding. Actual browser lifecycle/OS-focus/resize/cleanup
receipts do not complete B0-B5. Observer wire/session joins, cross-OS/AT/IME/DPI,
Chrome comparison and signed distribution are tracked by the owning browser plan
and INTEGRATION rather than inferred from partial green.
