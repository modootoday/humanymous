---
id: sot:root/native-embedding
title: Opt-in native SDK and DLL embedding
status: active
created: 2026-11-07T04:10:00Z
updated: 2026-11-07T04:17:05Z
verify: manual
rel:
  depends_on: [sot:root/rust-migration, sot:root/chromium-lab-browser, sot:root/browser-ui]
---

# Opt-in native SDK and DLL embedding

## Rule

Version 0.1.0. User-approved extension of the laboratory browser's observation
model into developer-integrated native desktop applications. This is an accepted
design, not implemented SDK support or evidence of automation blocking. Rust-only,
detection freeze, accessibility and mandatory sandbox boundaries remain binding.

SDK-01: The app developer explicitly initializes a Rust SDK or loads a Windows
cdylib through a versioned C ABI. Arbitrary process injection, global keyboard
hooks, DLL search-path modification, privilege escalation and third-party probing
are outside scope. Register only host-owned windows and approved capabilities.

SDK-02: Planned crates are humanymous-native-core (facts/capabilities/lifecycle),
humanymous-native-windows (owned-window OS adapter), humanymous-native-ffi (thin
versioned ABI) and humanymous-native-plugin (host registration/callback adapter).
Reuse existing scoring, audit and cryptography; do not duplicate policies or
engines. Actual crate creation/workspace registration requires its implementation
slice and checks. Naming, public rustdoc, module/file limits, reviewed unsafe and
testing follow sot:root/rust-migration and AGENTS.md.

SDK-03: Non-owning lifecycle is Uninitialized -> Initialized -> Running -> Stopping
-> Closed or CleanupIncomplete. Use generation-scoped opaque handles; define UI
thread, worker, reentrancy and callback ownership. Repeated init, stale handles,
shutdown during callback and deadline expiry return explicit errors. Stop revokes
authority before cleanup; cleanup completion is separately observed. Never exit
the host process or silently retry an uncertain side effect. No work in DllMain
beyond minimal loader-safe setup; start IO/workers only after explicit init.

SDK-04: ABI negotiates version, struct size and capabilities. Use fixed-width
integers, explicit lengths and bounded UTF-8 buffers, explicit allocation/free
pairs and errors with documented data lifetime. No Rust String/Vec/trait-object/
enum layout crosses the ABI. Catch panics at each exported boundary; prohibit
unwinding across foreign calls. Unload is forbidden while SDK workers or callbacks
remain live. A Rust-authored consumer must test the actual dynamic ABI.

SDK-05: First release is opt-in observation only. Reuse research provenance and
visibility planes; distinguish requested/delivered/observed/unknown. A DLL cannot
certify human identity or universal Computer Use/CDP detection. Enforcement needs
a separately canonical, explicitly authorized policy at the host's sensitive
operation boundary. Required evidence missing means that protected operation is
fail-closed, not whole-app termination or blanket input suppression. SDK addition
does not authorize scoring weights, thresholds, predicates or verdict changes.

SDK-06: AT/UIA presence, injected-input markers, input speed and motor richness
cannot individually gate Pass/CHALLENGE. Qualification includes legitimate AT,
remote desktop, physical input and authorized test automation controls. Preserve
the T4 ceiling and uncertainty; compromised in-process hosts remain a trust limit.

SDK-07: Loading an SDK is no sandbox attestation. Declare OS/architecture/runtime/
privilege capabilities and host/child trust boundaries. CEF adapters retain the
existing mandatory sandbox and unknown-deny requirements; do not reuse the browser
bootstrap's borrowed sandbox pointer as generic proof. The current experimental
RunBrowserDesktop owns CEF initialization/shutdown and calls process::exit on
failure: it MUST NOT be renamed/re-exported as an embeddable SDK entrypoint.

SDK-08: Default retained/exported data excludes raw keys, AX/window text,
screenshots, clipboard, credentials and personal process inventories. Observe
minimal app-owned metadata only. Sensitive capture/export requires per-run purpose,
scope, expiry, revocation and destination. Bound queues, loss counters, CPU/memory
budgets and shutdown time; expose unknown/lost data without inventing clean results.
Erasure/kill-switch/permanent-ban dual-control rules remain binding.

## Why

Blue/red review identifies host-process exit and in-process tampering as P0;
SDK-01/03/05/07 separate embedding from process-owning experiments. SRE requires
bounded stop/unload and callback lifetimes in03/04/08. Privacy requires minimal
app-owned capture in01/08. Evaluator requires actual DLL consumer and AT controls,
not promotion of browser fixture evidence, in04/05/06. These findings were checked
before SDK implementation; all SDK runtime and enforcement coverage is not_run.

## Examples

A host registers its own window, starts metadata observation, requests a bounded
snapshot and stops before unloading. No protected action is automatically blocked.
A stale handle returns an error; a missing required proof leaves an explicitly
configured protected action unavailable. Neither case terminates the host.

## Verification

Follow plan:root/20261107041001. ABI fuzz/negative inputs, panic containment,
concurrency/reentrancy, stop/unload, legitimate AT and input cohorts, sandbox,
signing/distribution, privacy/loss/performance and cross-OS remain unverified.
Current browser IME/native-child evidence does not satisfy these SDK gates.
