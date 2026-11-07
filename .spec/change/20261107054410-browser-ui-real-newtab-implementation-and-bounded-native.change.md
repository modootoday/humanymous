---
id: change:root/20261107054410
title: Browser UI real newtab implementation and bounded native evidence
status: active
created: 2026-11-07T05:44:10Z
updated: 2026-11-07T05:44:10Z
rel:
  depends_on: [plan:root/20261107002014, plan:root/20261107033358, plan:root/20261107041001, sot:root/rust-migration, plan:root/20261107050329]
---

# Browser UI real newtab implementation and bounded native evidence


## Accepted and implemented delta

Commit586ff95 implements UI-17..24 under browser-ui0.5 and Chromium laboratory0.4.
The root Rust migration contract is unchanged: Rust1.99.1/edition2024 authored
code/tests only, upstream pinned CEF/Meari/html5ever, mandatory sandbox and no
scoring change. Root alone resolved the shared lock delta; the browser owner
modified only its optional package dependencies. Native SDK/ABI/framework/CLI
scope remains separately planned, not implemented by this change.

Actual owner newtab, real form/links, exact preflight for version/gpu/credits/
license/sandbox, builtin asset scoping and explicit download refusal replace the
historical fixture-only/no-navigation state. Unknown/state-changing/destructive
URLs and external browsing remain denied. One actual tab; no fake history.
Parent OS keyboard focus is explicitly transferred/read back on native UI press.

## Verification and contracts with other owners

Evidence:root/20261107053625 binds native source/artifacts and retained failures.
Core24/native13 tests plus feature1/4negative flags, Clippy and targeted fmt pass.
Final12-second native matrix/cleanup exits0,7document identities observed.
Interactive form/links/address Enter/reload/home/96-DPI resize/close use their
separately identified GUI source; final download refusal is not interactively
exercised. Full sandbox/BrowserReady=false: WebUI Network Service Not Sandboxed,
transient GPU exit78, shell AX click bounds, native AT/new-layout IME/high-DPI,
Chrome parity, external navigation/observers and distribution remain open.

The browser owner informed root of the exact optional dependency delta, accepted
lock ownership, measured state and B0-B5 frontier through INTEGRATION and the
canonical newtab plan. No other owner's source/spec was edited. Four D1 warnings
on implementation commit586ff95 requested linked change notes for the governing
historical plans and migration SoT; this note links all four without mutating
root-owned migration canon. Public scoped Specfold check has0errors/0warnings;
private discovery check was stopped without a result and is not a pass.

## Canonical bytes

All30 browser Rust/manifests in the final source receipt match their canonical LF
Git blobs at586ff95 (zero mismatches). Public results.json Git blob is20625bytes,
SHA-256 c33a6fb9af2f1aca871663930e1f506b1cbf46d62c5f91eb2c07c5e31896a691,
identical to registered attachment bytes. Inherited CRLF working bytes are
separately retained in the validated-source receipt. No screenshot/device data,
profile deletion or third-party probing is part of this checkpoint.
