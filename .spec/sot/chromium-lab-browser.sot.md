---
id: sot:root/chromium-lab-browser
title: Rust Chromium laboratory browser and observation adapters
status: active
created: 2026-11-06T23:34:52Z
updated: 2026-11-06T23:34:52Z
verify: manual
rel:
  depends_on: [sot:root/adapter-ecosystem, sot:root/adapter-service-contract, sot:root/adapter-authority, sot:root/automation-evidence, sot:root/accessibility-agent-tools, sot:root/mobile-agent-tools, sot:root/crate-architecture, sot:root/cross-os-runners]
---

# Rust Chromium laboratory browser

Document version 0.1.0, research checked 2026-11-07 KST. Three read-only agents
examined embedding, observation/adapters and security/distribution. All builds,
browser executions and qualification cases are not_run. This document adds no
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

The browser measures owned/authorized fixtures, explains observations and joins
them to actual server decisions. It does not grant trust to its own browser,
replace server enforcement or promise T4 detection. Keep live server verdicts
separate from optional offline replay through the same Rust libraries.

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
rust-version is declared; Rust 1.88 compatibility remains unverified. The core
MSRV is not raised by this research. Lock exact features/dependency closure and
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
Current Meari cannot be represented as compatible with core MSRV1.88. Keep a
separate desktop toolchain/build boundary until a reviewed dependency decision;
do not silently raise core MSRV. No full Meari closure build was performed. [U2]

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
                                  | bounded observations    | owned target requests
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
  JS shell is excluded. Desktop dependency/toolchain scope does not change core MSRV.
- BRW-02 No self-trust: Modified browser evidence is not human identity, server
  capture or a bypass credential. Keep actual authority and audit server-side.
- BRW-03 Safe process model: Prove sandbox and privilege isolation per tuple;
  reject unknown/disabled required sandbox. Keep trusted controls out of pages.
- BRW-04 Scoped automation: Owned fixture origin/package and egress allowlists,
  run budgets, fresh profiles and private authenticated control channels.
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
| B1 | Rust supervisor + pinned external-engine diagnostic pilot | Not implemented; not embedded completion |
| B2 | Meari Rust native UI + sandboxed CEF shell, tabs/navigation/permissions | Not implemented; interactive surface/toolchain and six native tuple qualification pending |
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
