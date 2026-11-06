---
id: sot:root/mobile-agent-tools
title: Mobile device automation mechanisms and evidence
status: active
created: 2026-11-06T23:31:13Z
updated: 2026-11-06T23:31:13Z
verify: manual
rel:
  depends_on: [sot:root/automation-evidence, sot:root/accessibility-agent-tools, sot:root/adapter-ecosystem, sot:root/cross-os-runners]
---

# Mobile automation research

Document version 0.1.0; researched 2026-11-07 KST. Three read-only researchers
cover Android, iOS and evidence/attestation. This is researched capability and
qualification design, not executed product coverage. No device connection,
installation, signing, permission change, paid device allocation or probing was
performed. Existing scoring freeze, accessibility requirements and T4 ceiling
remain in force. No mobile SDK or detector is shipped by this document.

## Scope and vocabulary

Separate controller (human/script/LLM), driver, action mechanism, target surface,
device execution environment, egress topology and server evidence. An AI model
can orchestrate a conventional driver; an MCP interface or script alone is not
an autonomous model. Real devices, Android emulators and Apple simulators are
different qualification targets. Remote control from a datacenter does not prove
that the phone's traffic exits through a datacenter IP.

| Family | Verified role | Important boundary |
| --- | --- | --- |
| Android UI/Application Exerciser Monkey | Seeded pseudo-random UI/system stress on device or emulator | Repeatable event generation does not prove a semantic task or identical server outcome [A1] |
| monkeyrunner | Separate host-side scripting API | Officially unmaintained; do not confuse with Monkey or select as new baseline [A2] |
| adb input / device input injection | Coordinates, keys and input through a provisioned debug connection | Driver/control authorization is separate from authorization of a protected server effect [A3] |
| UI Automator | Instrumented cross-app/system UI interaction | Version-specific device and instrumentation requirements; not identical to a user-installed AccessibilityService [A4] |
| Espresso | In-app UI testing and synchronization | Requires app/test integration; not general arbitrary-app desktop-style control [A5] |
| Android AccessibilityService | Exposed nodes, semantic actions and supported gesture dispatch | Legitimate TalkBack/switch/voice use shares this surface; technical capability differs from Play distribution permission [A6, A7] |
| Appium UiAutomator2 | Android native/hybrid/web driver | Native and WebView/browser contexts have distinct mechanisms and prerequisites [A8] |
| scrcpy | Android mirroring and control | Control/input modes differ; video plus remote input is not model reasoning [A9] |
| XCTest / XCUIAutomation | Apple UI querying, interaction and assertions | Test tooling, not an LLM or universal privileged device-control API [I1] |
| Appium XCUITest / WDA | Apple native/hybrid/web automation | Signing, trust, developer settings, host and inspectability requirements apply [I2-I4] |
| Maestro | Declarative UI flows using OS accessibility/input interfaces | Android and documented iOS Simulator support must not imply every real-iOS/cloud/WebView target [M1] |
| Android Studio Agent Mode | AI development loop can inspect a connected app and use adb input | Confirms AI orchestration of conventional mobile controls; not a new universal remote website fingerprint [G1] |
| Mobile Next mobile-mcp | Agent-facing accessibility snapshots and screenshot/coordinate interaction | README claims real/emulated/simulated targets; each host/device capability still requires qualification [M2] |

Mobile MCP source observation is pinned at commit
57efc27d96971f96caba69b62665fa057ebc65ac (2026-11-02). Its README and roadmap are
different evidence: a feature described as planned is not shipped merely because
another tool supports it. Driver backend and transport evolve; do not assume an
old WDA architecture or expose its device-control server publicly. [M2, M3]

## Platform and test infrastructure limits

Android real-device debug paths need explicit authorized development access;
accessibility services need their actual declared capabilities and enabled state.
Browser/WebView control may additionally require debugging/inspectability and
compatible browser drivers. A coordinate action, accessible-node action and DOM
operation are separate cases even when all produce the same HTTP request.

Observed UiAutomator2 latest is v9.0.1; its requirements distinguish driver 6+
(Android API26+) from earlier driver ranges. UI Automator's modern 2.4 API guide
is still under development and uses an alpha example: this is not stable-release
qualification. scrcpy observed latest is v5.0.1; SDK/UHID/AOA paths differ, and
its OTG mode does not require USB debugging. Do not generalize the usual adb
setup to every scrcpy mode. Android dispatchGesture needs its declared capability;
its completion callback does not prove a durable app/server result. [A4, A6, A8, A9]

Apple Xcode builds and simulator execution require macOS. However, current
Appium XCUITest documentation explicitly supports limited Windows/Linux control
of real iOS/tvOS 18+ devices using RemoteXPC and preinstalled or externally
managed WDA. Automatic selection and normal xcodebuild startup are unavailable
there; WDA still requires an Xcode build/provisioning path. Do not retain the old
blanket claim that every iOS driver session must run on macOS. [I3]

Native XCUITest and a WebKit debug context are not equivalent. Supported hybrid
operations require a supported inspectable web surface; reading a native tree
does not guarantee DOM access or actionability. Record Safari, embedded WebView,
system authentication surface and native app separately. [I4]

CI may orchestrate an emulator, Mac simulator or a device farm, but a job label
does not establish actual hardware/session access. Real-device farms demonstrate
that physical phones can be automated. Provider re-signing, app entitlements,
network/USB restrictions and cleanup can change the measured environment. A
simulator cannot qualify production hardware-backed attestation. [F1]

Device Farm has no carrier connection/SMS coverage; do not use it to qualify
cellular egress. Its remote-access re-signing can confound app integrity tests.
The 2026-11-06 AWS Maestro example is specifically real Android, not evidence
of general real-iOS Maestro support. [F2-F4]

Docker Compose remains the product backend/topology authority. External mobile
drivers can target the owned lab through an explicitly isolated test ingress;
record native-device supplemental evidence separately. No host ADB daemon,
personal-device USB, control socket or private desktop bus is mounted into an
untrusted attack container. Android acceleration and iOS simulators are not
assumed portable inside arbitrary Linux containers.

## Observation and attestation boundaries

Android ActivityManager.isUserAMonkey is an app-local narrow state query, not a
generic detector for every automation framework. isRunningInUserTestHarness is
also not a universal instrumentation-test indicator. Neither is a website API
or a server-authenticated assertion merely because a client forwards a boolean.
No vendor-specific remote fingerprint was established. [A10]

| Evidence | What it can support | What it cannot establish |
| --- | --- | --- |
| Browser events / touch / isTrusted | Observed browser dispatch and interaction data | Human identity, absence of accessibility automation or a named mobile driver |
| TLS/H2/IP/device hints | Connection/client observations under actual termination topology | Owner intent, carrier identity or unautomated physical device |
| Play Integrity | Verified native-app/device/request-scoped integrity verdict | A natural person, all control tools, or a generic attestation API for arbitrary websites [P1, P2] |
| App Attest | App-instance key attestation and request assertions verified by own server | Human control or a successful UI workflow [P3] |
| DeviceCheck | Service-specific per-device state facilities | General human proof or unrestricted permanent device identification [P4] |
| WebAuthn | RP-bound authentication and specified presence/verification flags | Continuous human operation of the entire subsequent session [P5] |

Play app-access-risk controlling/capturing/overlay categories concern possible
interference. Verified accessibility services known to Google Play are exempt;
empty/unevaluated results are not clean results. Genuine devices can generate
valid integrity evidence while being used for automation. Integrity requirements
are scoped native-channel policy, not a browser-channel prerequisite. [P1, P2]

Server verification must check the expected app/environment, challenge or
request-content binding, freshness and replay/sequence rules as required by each
provider. Google standard requestHash and classic nonce differ; App Attest key
attestation differs from per-request assertion. Preserve unavailable, unsupported,
quota, outage, validation failure and cryptographic invalid states separately.
Provider success is not an ALLOW decision and tokens do not replace user scope,
tenant/method/target/body/session binding or durable audit. [P1-P3, P7, P8]

For App Attest, verify app/environment and key association, attestation trust,
one-time challenge and per-request assertion signature/counter according to
Apple's server protocol. Separate development from production. For WebAuthn,
virtual-authenticator lab success does not qualify hardware attestation. [P5, P6]

Google Play's accessibility policy distinguishes static human-defined automation
from prohibited autonomous initiate/plan/execute use, with a narrow qualifying
disability-assistance exception. This is a distribution-policy boundary, not
proof of a technical impossibility or grounds to flag every accessibility app.
Do not claim isAccessibilityTool by self-declaration makes a tool verified. [A7]

## Canonical requirements MOB-01 through MOB-09

- MOB-01 Mechanism: Record random stress, scripted native input, accessibility
  semantic, browser/WebView protocol, screenshot/coordinate, direct HTTP and
  hybrid phases separately. Controllers and mechanisms have separate IDs.
- MOB-02 Provenance: Lab-known driver/device facts never become production
  actor identity. Client claims cannot grant server capture capability.
- MOB-03 Accessibility: Pair human TalkBack/VoiceOver, switch/voice, external
  keyboard and reduced/zero-motor controls. No mandatory motor richness, speed,
  touch gesture or blanket accessibility-service prohibition.
- MOB-04 Binding: Keep one shared Rust authority for auth scope, exact effect
  binding, nonce/sequence, idempotency and audit-before-origin. No parallel
  mobile-only scoring implementation or trust-all-mobile header.
- MOB-05 Attestation: Verify provider-specific evidence server-side, with bounded
  inputs, explicit environment/key trust and request context. A valid result is
  scoped evidence, never human proof. Native integrations remain proposed.
- MOB-06 Loss: Unsupported OS, no device permission, unavailable tree, setup/signing
  failure, timeout, crash, ANR and quota/outage retain separate denominators.
  Tool failure and absence of a server request earn no detector-block credit.
- MOB-07 Privacy: Default telemetry excludes full UI trees, screens, clipboard,
  notifications, contacts, account/advertising IDs and device serials. Use
  run-scoped fixture IDs; keep required minimal private traces bounded/retained.
- MOB-08 Isolation: Use owned fixtures, synthetic accounts/data, explicit device
  selection, package and egress allowlists, bounded event/run budgets, credential
  expiry, exclusive device lease and verified cleanup. No public ADB/WDA/MCP.
- MOB-09 Claims: Report actual OS/tool/app/browser/driver/transport tuples and
  effects. Physical hardware, successful attestation and CHALLENGE each have
  different meanings; none establishes universal prevention or human ALLOW.

## Owned-lab qualification matrix

| Group | Required observation/control |
| --- | --- |
| Seeded Monkey stress | Seed, package scope, event budget, initial fixture state, crash/ANR, actual request/effect counts |
| Deterministic Android flow | UI Automator/Appium/Espresso separately; actual delivered action and state reconciliation |
| Accessibility semantic / gestures | Explicit service capabilities; paired human AT controls; no motor-based pass condition |
| Remote mirrored input | Input mode/transport and app outcome, not only a visible video feed |
| iOS native | Simulator versus real hardware; host, signing/WDA and privilege tuple |
| Mobile browser / hybrid | Chrome/Safari/WebView and native↔DOM transition; same-request provenance |
| LLM/MCP orchestration | Pin model/controller and driver separately; treat UI content as untrusted prompt data |
| Authorized machine API | Valid scope and device-independent non-browser control; no fabricated pointer telemetry |
| Integrity positive/negative | Correct/wrong app, request mismatch, stale/replay, wrong environment, absent/error and real automated-device controls |
| Timeout/retry and reconnect | Independent once-only origin counter, fresh binding, audit order; no blind duplicate action |
| Human/mobile accessibility | TalkBack/VoiceOver/switch/voice/keyboard, zoom/rotation and reduced-motion relevant controls |

Case IDs: mob-monkey-seeded-stress, mob-scripted-native-flow,
mob-accessibility-semantic, mob-remote-input-mode, mob-ios-real-vs-simulator,
mob-hybrid-context-binding, mob-ai-controller-provenance,
mob-authorized-api-control, mob-attestation-not-human,
mob-attestation-request-mismatch, mob-attestation-replay,
mob-attestation-unavailable, mob-human-at-control,
mob-timeout-origin-once and mob-private-artifact-cleanup.

Every case is currently not_run. Record attempted/eligible/executed outcomes,
actual mechanism, setup errors, unsupported tuples, request counts, durable audit
receipts, independent origin effects and human-control outcomes. Event counts,
flow counts and server-effect counts are different denominators. A seeded stress
pass does not qualify purposeful AI task completion. Renewed UI observation after
uncertain delivery must precede any retry of an effect.

## Critique reflected and residuals

Physical phones are automatable: require automated-real-device positive controls
rather than equating hardware trust with a human. Accessibility parity overrides
tempting motor/tool-presence rules. Native app-local flags are not browser proof.
Latest non-macOS driver support is narrower than full simulator/build support.
App re-signing and WebView debug settings must be explicit experimental factors.
Token relay/replay/request substitution need context validation, not a cached
device-trust ALLOW. These are design requirements; no measured detection benefit
or implemented mobile attestation adapter is asserted.

Public standards draft-v0 may derive mechanism/provenance, accessible controls,
request binding and minimization requirements. Public Pages content must remain
authored, proposal-labelled and free of private device traces; this internal SoT
is not itself a published standard. T4 coherent browser residual remains.

Independent read-only critique found no P0/P1 blocker for research-only canon.
Its P2 source-specificity feedback is reflected in provider server-verification,
Device Farm re-signing and fixed WebAuthn Recommendation references below.

## Primary sources

- A1 https://developer.android.com/studio/test/other-testing-tools/monkey
- A2 https://developer.android.com/studio/test/monkeyrunner
- A3 https://developer.android.com/tools/adb
- A4 https://developer.android.com/training/testing/other-components/ui-automator
- A5 https://developer.android.com/training/testing/espresso
- A6 https://developer.android.com/reference/android/accessibilityservice/AccessibilityService
- A7 https://support.google.com/googleplay/android-developer/answer/10964491
- A8 https://github.com/appium/appium-uiautomator2-driver
- A9 https://github.com/Genymobile/scrcpy
- A10 https://developer.android.com/reference/android/app/ActivityManager#isUserAMonkey()
- I1 https://developer.apple.com/documentation/xcuiautomation
- I2 https://appium.github.io/appium-xcuitest-driver/latest/getting-started/device-setup/
- I3 https://appium.github.io/appium-xcuitest-driver/latest/guides/non-macos-hosts/
- I4 https://appium.github.io/appium-xcuitest-driver/latest/guides/hybrid/
- M1 https://docs.maestro.dev/get-started/how-maestro-works
- M4 https://docs.maestro.dev/get-started/supported-platform/ios
- M2 https://github.com/mobile-next/mobile-mcp/blob/57efc27d96971f96caba69b62665fa057ebc65ac/README.md
- M3 https://github.com/mobile-next/mobile-mcp/blob/57efc27d96971f96caba69b62665fa057ebc65ac/ROADMAP.md
- G1 https://developer.android.com/studio/gemini/agent-mode
- F1 https://docs.aws.amazon.com/devicefarm/latest/developerguide/welcome.html
- F2 https://docs.aws.amazon.com/devicefarm/latest/developerguide/sessions.html
- F3 https://docs.aws.amazon.com/devicefarm/latest/developerguide/devices.html
- F4 https://aws.amazon.com/blogs/devops/run-maestro-tests-on-real-android-devices-using-aws-device-farm/
- P1 https://developer.android.com/google/play/integrity/overview
- P2 https://developer.android.com/google/play/integrity/setup
- P3 https://developer.apple.com/documentation/devicecheck/establishing-your-app-s-integrity
- P4 https://developer.apple.com/documentation/devicecheck
- P5 https://www.w3.org/TR/2026/REC-webauthn-3-20260825/
- P6 https://developer.apple.com/documentation/devicecheck/validating-apps-that-connect-to-your-server
- P7 https://developer.android.com/google/play/integrity/verdicts
- P8 https://developer.android.com/google/play/integrity/standard
