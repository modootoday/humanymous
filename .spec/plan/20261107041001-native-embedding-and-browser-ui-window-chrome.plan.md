---
id: plan:root/20261107041001
title: Native embedding SDK and borderless SVG browser shell
status: active
created: 2026-11-07T04:10:01Z
updated: 2026-11-07T04:21:39Z
rel:
  governs: [crates/browser/**]
  depends_on: [sot:root/native-embedding, sot:root/browser-ui, sot:root/chromium-lab-browser]
---

# Native embedding SDK and borderless SVG browser shell

## Goal

Canonicalize opt-in native SDK/DLL scope, then implement and measure the existing
browser's undecorated Windows host and SVG controls. SDK crates are a later slice.
Preserve Rust-only, sandbox-required, scoring freeze and AT compatibility.

## Steps

- [x] User authorized SDK scope and actual title-bar/SVG implementation.
- [x] Design panel/critique: process-owning browser DLL is not an SDK; lifecycle,
  ABI, privacy, observer/enforcer separation and AT requirements become SDK-01..08.
- [x] Add native embedding SoT and UI window/icon requirements before SDK code.
- [x] Implement borderless owned host, named local window controls and Rust SVG library.
- [x] Verify keyboard/AX command routing, unknown intent rejection and icon document rendering.
- [x] Run Rust tests/fmt/Clippy/build and bounded native checks for concrete defects and the user-added resize scope.
- [x] Record source/artifact identity, native observations and exact residuals.

## Record

Prior compact-shell work follows plan:root/20261107033358. This slice supersedes
its temporary Reload text with a font-independent SVG. No native Hanja replay is
required; previous IME evidence does not qualify this new window geometry.

## SDK follow-through gates

1. Pure Rust core/lifecycle and versioned ABI design with explicit budgets.
2. Add four planned crates through coordinated root workspace integration.
3. Rust consumer dynamically loads actual DLL and exercises init/callback/stop/
   unload, invalid buffers/versions, stale generations, concurrency and panic bounds.
4. Windows observation scoped to app-owned windows; physical/AT/remote/test controls.
5. Privacy/loss/performance, mandatory applicable sandbox and signing/distribution.
6. Separate authorization and canonical policy before enforcement; cross-OS separate.

All six SDK gates are not_run. Browser DLL/native fixture tests are not substitutes.


## User-directed resize delta

User observed missing resize after borderless conversion. Implement UI-16's
5-DIP local perimeter, eight directions and contextual cursors before completion.
No OS wrapper is restored. Core edge/delta/clamp tests plus native right/bottom/
corner drag and cursor readbacks are required. Initial borderless runs exposed
minimized-zero-size and closed-document polling errors; fixes must pass native
cleanup. Historical failed reports remain evidence, never promoted to passes.


## Latest evidence and handover

Evidence:root/20261107034226 records final core18/native11, explicit native
security-override1 test covering4 flags, warning-denied Clippy and targeted fmt.
Actual Windows96-DPI resize changed1116x759 ->1036x759 ->1036x699 ->996x669
->976x649, with6 CEF size readbacks. OS cursor identities included horizontal,
vertical, both diagonals, text and pointer. Custom close exit0 with CEF before-close
and host-closed receipts. Earlier minimized/closed polling and fast-queued press
sampling failures are retained separately. Five bounded window runs covered those
fixes and new resize scope; no Hanja replay, external browsing or scoring change.
GUI-tested source snapshot/artifact hashes are preserved. Final source additionally fixes maximized-edge arrow selection and DPI-scaled resize minima/capture cancellation on DPI change; these branches pass unit tests, Clippy and build but were not replayed in native GUI. Normal96-DPI gesture evidence refers to the preserved source snapshot and DLL.

Next: SDK ABI/lifecycle gates remain planned/not_run; internal URL capability
qualification, external navigation, Chrome rendering parity, complete sandbox,
keyboard resize/Snap, high-DPI/multi-monitor/screen-reader/cross-OS and new-geometry
IME qualification remain open. Read AGENTS.md, rust-migration, browser-ui,
chromium-lab-browser and native-embedding SoTs before continuing. Shared Cargo/
roadmap integration belongs to the root session. No detection scoring changed.


## Framework adapter and create-tool extension

User requested reuse in Electron and other framework environments. Native-embedding
version0.2 SDK-09..14 makes thin framework crates and a separate Rust create CLI
canonical design scope. The four foundation crates remain prerequisites; framework
packages/CLI have not been created. No runtime support or automation blocking is
added by this documentation delta, and browser evidence is not reused as proof.

- [x] Inspect upstream Electron sandbox/native modules and Node-API limits; distinguish
  actual host/engine facts from framework guesses and pin support per measured tuple.
- [x] Define Rust Electron main-only addon, Tauri/winit candidate adapters and generic
  C ABI consumer paths. Preserve sandbox, scoring freeze and AT boundaries.
- [x] Define dry-run integration manifest/Rust scaffolding and explicit external host
  glue, no authored foreign templates or silent install/overwrite/security changes.
- [ ] Implement and qualify foundation core/FFI/plugin lifecycle before adapters.
- [ ] Electron vertical slice: actual Rust .node artifact and conforming consumer,
  sandboxed renderer deny cases, typed IPC/sender isolation if exposed, app/window/
  Node environment teardown and dev/packaged target-tuple validation.
- [ ] Qualify Tauri/winit separately; .NET/Qt/Flutter remain candidates until actual
  foreign consumer/packaging/lifetime boundaries can be tested within Rust-only rules.
- [ ] Implement Rust create CLI with deterministic dry-run, path containment, no
  overwrite/install hooks, unqualified-target markers and versioned manifest checks.

Working names are not registry availability claims. Proposed invocation is
`create-humanymous-native --framework electron --dry-run`; no such executable was
built or run. Source/API research supports the design, not SDK compatibility.
Shared workspace registration and roadmap remain coordinated root work. Future
implementation must resolve external consumer glue without quietly adding authored
JS/TS/C++ or substituting the browser bootstrap DLL for a non-owning SDK.


Framework panel critique is reflected in SDK-10 (actual sender/session/generation
and revocation),12 (native handle/event-loop/Node cleanup and ASAR/packaging) and14
(atomic bounded writes). One read-only review covered blue/red/SRE/privacy/evaluator
views; no independent runtime tests were performed or implied. Documentation-only
extension: scoped spec check passes; framework crates/CLI remain unimplemented.
