---
id: sot:root/accessibility-agent-tools
title: Vercel accessibility desktop automation research
status: active
created: 2026-11-06T23:24:35Z
updated: 2026-11-06T23:24:35Z
verify: manual
rel:
  depends_on: [sot:root/automation-evidence, sot:root/adapter-ecosystem, sot:root/cross-os-runners]
---

# Accessibility-driven agent tools

Research version 0.1.0, checked 2026-11-07 KST. Three read-only agents examined
official Vercel material, release metadata and source. No tool installation,
execution, OS permission change or third-party probing occurred. This is neither
measured Humanymous detection nor authorization to change frozen scoring.

## Identity, dates and maturity

The user's tentative desktop-use name resolves to vercel-labs/agent-desktop.
Vercel's browser-versus-desktop article is dated 2026-11-05. GitHub Latest at
observation is v1.2.0, published 2026-11-05T02:43:54Z, pinned release commit
2b0a7334fc111730454b726e6800aa41cdafc093. Earlier v1.0.0 was published on
2026-10-28; do not call the November article its proven first-ever launch.
Observed main b43ac4f6e892ef608f8851428613c4009f0e84c7 is a separate snapshot.
Release metadata/ref and artifact digests are attached to research evidence. [V1-V3]

License metadata is Apache-2.0. The tool is a CLI/C library, with stdio MCP
support in the reviewed release. It does not contain an LLM or autonomous agent;
the caller owns the observe/act/verify loop. The tool release version, JSON
envelope version 2.6 and any caller model version are different identifiers.
Published native artifacts cover x64/ARM64 macOS, Windows and Linux. npm launcher
requirements and Rust source-build requirements are also distinct: a Rust 1.89
tool build does not silently raise Humanymous's Rust 1.88 MSRV. [V2-V4]

Vercel's current Labs listing places agent-desktop among active experiments,
while a repository badge uses product wording. Neither SemVer 1.x nor a badge
establishes production GA, SLA or universal application support. [V5]

## Mechanism and observation boundary

The tool reads an application's accessibility tree into a structured snapshot,
assigns snapshot-qualified element references and invokes supported actions.
It does not infer arbitrary controls from pixels. Screenshots are a separate
capability, not a generic vision fallback for missing accessibility trees. [V4]

| Path | Reviewed behavior | Humanymous consequence |
| --- | --- | --- |
| macOS | AX API; permission and app/session requirements | OS access is not a server-verified human identity |
| Windows | UI Automation in the signed-in interactive desktop | UIA action, injected input and remote HTTP are different mechanisms |
| Linux | AT-SPI in a graphical session; X11/Wayland capabilities differ | Container/SSH shell alone does not establish GUI availability |
| Default ref action | Semantic accessibility action without implicit global physical input | Sparse pointer/motor events do not imply malicious automation |
| Headed mode | Permits supported physical input; does not force every action to use it | Observe actual delivered mechanism per action, not only CLI flags |
| Chromium/Electron bridge | Fresh launch can expose loopback CDP for agent-browser | Native AX and browser CDP phases need separate provenance |

The reviewed physical paths include platform input mechanisms such as CGEvent,
SendInput and Linux X11/Wayland integrations. Availability is command/surface/
OS-specific. Headless semantic mode is not a guarantee of operation without an
interactive desktop, graphical session or OS permissions. [V6-V9]

Official documentation recommends agent-browser for webpage content. Browser
menus/file dialogs and a Chromium/Electron CDP bridge are different contexts;
agent-desktop itself is not the browser CDP controller. Do not generalize native
tree access into universal HTML form or custom-rendered UI support. [V1, V4]

Snapshot refs, ambiguity/staleness refusal and partial-tree completeness are
local locator safety mechanisms. They are not Humanymous nonces or authorization
receipts. Successful tool delivery can remain unverified at the application
level. Re-read fixture state and reconcile actual server/origin effects before
claiming a completed action; never retry a possibly delivered side effect
blindly. [V10]

Explicit input commands can use physical paths even without headed mode (for example some macOS press operations). Observe actual delivery per command. Windows SendInput returns the inserted event count; return value/GetLastError do not identify UIPI as the failure cause. [V6, W4]

## What websites can and cannot infer

No general website API identifying this tool's AX/UIA/AT-SPI usage or OS permission
was verified. No unique UA/IP/TLS/navigator/DOM-event fingerprint was established.
Observed browser events are not equivalent to authenticated tool/operator
identity. Event.isTrusted distinguishes event dispatch conditions, not natural
persons; the WebDriver working draft also specifies trusted automated input.
Actual AX/UIA event behavior must be measured per OS/browser/action tuple. [W1, W2]

Therefore both universal undetectability and universal accessibility-fingerprint
detection claims exceed this research. A coherent real browser at human pace
remains within Humanymous's intentional T4 ceiling. Model provider, orchestrator,
tool mechanism, desktop host, egress, user authorization and harmful behavior
must remain separate dimensions. Datacenter blocking does not identify local
desktop execution, and an accessibility API is not malicious intent.

## Project requirements AX-01 through AX-07

- AX-01 Attribution: Do not use accessibility permission/API use, motor richness
  or speed as independent human/malicious verdict or Pass prerequisites.
- AX-02 Provenance: Keep lab tool traces, OS observations, browser/adapter claims
  and Rust server observations distinct. Lab-known mechanism is not production
  actor attribution and cannot grant capture capability.
- AX-03 Binding: Protect origin effects with server-owned identity, authenticated
  scope, tenant, method/target/entity binding, policy revision, replay/freshness
  and durable audit. Keep authorized machine/API flows valid without fake browser
  events. Use existing shared authority, not a new vendor-specific detector.
- AX-04 Accessibility: Pair keyboard, screen-reader, switch/zero-motor and relevant
  human controls. Do not add mandatory gestures, movement or speed requirements.
- AX-05 Loss: Permission refusal, missing tree, unsupported surface, stale ref,
  timeout or agent failure is not detector success. Retain all attempt classes.
- AX-06 Privacy: Full trees, titles, clipboard, screenshots, notifications and raw
  values are excluded from default production telemetry. Synthetic lab fixtures
  and private minimal evidence only; redaction alone does not make images public.
- AX-07 Claims: Report only actually executed tuples. Preserve T4 residuals and
  detection freeze; CHALLENGE is not human ALLOW or a completed agent block.

These extend the existing automation-evidence causal model; they do not introduce
new weights, hard rules, gesture challenges or a production AX telemetry feed.

## Owned-lab qualification plan

| Experimental group | Prerequisites and paired evidence |
| --- | --- |
| Native semantic AX/UIA/AT-SPI | Synthetic app, pinned tool/hash/OS, explicit permission; human keyboard/AT controls |
| Native browser chrome/dialog | Supported exposed control; reconcile owned webpage/server result |
| Agent-browser CDP | Fresh controlled profile, loopback endpoint, same browser/version control |
| Headed physical input | Explicit lab permission, interactive session and observed mechanism |
| Authorized direct HTTP | Owned API, correct scope plus unauthorized/replay controls |
| Hybrid UI/browser/API | Bind each transition to one session/request and one origin effect |
| Missing permission/tree/stale ref | Separate unavailable/unsupported/tool-error denominator, no block credit |

Case inventory: ax-tool-version-and-runtime, ax-semantic-no-motor-attribution,
ax-headed-mechanism-observed, ax-cdp-path-distinct,
ax-permission-unavailable-not-detection, ax-stale-ref-no-double-effect,
human-keyboard-control, human-screenreader-control, human-zero-motor-control,
coherent-browser-ceiling, hybrid-request-binding, hybrid-replay-denied,
api-authorized-machine-control, telemetry-no-raw-ax-content and
origin-once-after-audit.

Record planned and observed mechanism, actual tool/release/artifact hash,
OS/browser/session, supported surface, permission/capability, tool delivery
disposition, actual application/origin effect, Humanymous verdict and
error/skip/unavailable/unsupported counts. Keep the action-to-request causal
graph and observer provenance; trace IDs never authorize another effect.

macOS AX needs the launching app's Accessibility permission and screenshots need
separate Screen Recording permission. Windows requires an interactive signed-in
desktop, not Session 0/SYSTEM/service/locked/secure desktop or Server Core. A
background standard-user IPC runner is not automatically a GUI automation
runner. Linux needs a graphical session/accessibility bus and actual supported
window-manager/input path. Native Windows/macOS supplements are not substituted
by a Linux GUI container. Preserve Docker-only product e2e authority and classify
native GUI results separately. No host desktop or bus is mounted into attack
containers as a shortcut. [V7-V9]

A future tool runner must pin artifact checksums, scope owned fixture apps and
avoid user's personal desktop content. UI labels/tree data are untrusted input
to the calling model, not instructions or authority. CDP remains loopback-only;
never expose desktop automation/MCP/control endpoints publicly. Permission and
cleanup failures remain visible, not silently bypassed.

## Status and standards implications

All tool-execution and blocking-performance cases are not_run. Research did not
install the tool, grant TCC/UIA access, run MCP, capture a desktop or alter
Humanymous scoring. Product browser/Gate qualification is still incomplete.

Public draft-v0 requirements should generalize mechanism-versus-identity,
accessibility non-discrimination, server-owned effect binding, data minimization
and observation limits. Vendor-specific evasion/blocking recipes or disability
inferences are not standards evidence. Any public docs update remains authored
and must preserve proposal/non-adoption status. [W3]

## Official sources

- V1 https://vercel.com/i/browser-vs-desktop-automation
- V2 https://github.com/vercel-labs/agent-desktop/releases/tag/v1.2.0
- V3 https://github.com/vercel-labs/agent-desktop/tree/2b0a7334fc111730454b726e6800aa41cdafc093
- V4 https://agent-browser.dev/desktop
- V5 https://vercel.com/labs
- V6 https://github.com/vercel-labs/agent-desktop/blob/2b0a7334fc111730454b726e6800aa41cdafc093/CONCEPTS.md
- V7 https://agent-browser.dev/desktop/platforms/macos
- V8 https://agent-browser.dev/desktop/platforms/windows
- V9 https://agent-browser.dev/desktop/platforms/linux
- V10 https://github.com/vercel-labs/agent-desktop/blob/2b0a7334fc111730454b726e6800aa41cdafc093/docs/faq.md ; https://github.com/vercel-labs/agent-desktop/blob/2b0a7334fc111730454b726e6800aa41cdafc093/apps/docs/content/docs/snapshots-and-refs.mdx
- W1 https://dom.spec.whatwg.org/#dom-event-istrusted
- W2 https://www.w3.org/TR/2026/WD-webdriver2-20260702/
- W3 https://www.w3.org/TR/privacy-principles/#data-minimization

- W4 https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput
- V11 https://github.com/vercel-labs/agent-desktop/blob/2b0a7334fc111730454b726e6800aa41cdafc093/docs/json-output.md

