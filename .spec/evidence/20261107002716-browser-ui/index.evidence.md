---
id: evidence:root/20261107002716
title: Meari native UI and CEF qualification
status: active
created: 2026-11-07T00:27:16Z
updated: 2026-11-07T00:33:40Z
rel:
  depends_on: [sot:root/browser-ui]
files:
  - {path: results.json, sha256: c8ba542c8c3369d1289bf15a1923e6712f6777cb01461a093bb0d4a7d048d61d, bytes: 2794, store: repo}
---

# Meari native UI and CEF qualification

## What was measured

2026-11-07 KST, Windows x64, Rust1.99.1. Actual root workspace package:
9 tests passed, 0 failed/ignored; Clippy with warnings denied and package fmt
passed. Tests cover exact-origin/deny policy, bounded geometry and generation
cancellation/close, real Meari keyboard panels, AccessKit address edits and
injected IME preedit/commit. They do not qualify OS IME candidate windows.

The integrated native Meari window was visually observed. UIA exposed named
headings, status, address and panels after explicit aria labels. Address focus
and synthetic typing, coordinate panel switch, keyboard Tab+Right switch and
Alt+F4 close were observed. Asynchronous close was rechecked with zero remaining
pilot windows. The pinned Noto cache was absent; host font fallback was reported.
An initial indexed UIA click lacked cached bounds, retained as tool/AT limitation.

Pinned upstream cef-rs sandbox-feature source/native build check passed after
correcting existing CMake/Ninja PATH and Windows long object paths using a
temporary private-directory SUBST alias. It was removed afterward. Earlier two
failures are retained privately. CEF/Chromium native file hashes identify the
downloaded files; they are not independent signing or sandbox attestations.

Sanitized metadata, exact authored Rust source SHA256 and native artifact hashes
are in [results.json](results.json). Raw source/build caches and UIA outputs are
registered under private evidence:root/20261107003533. A separate curated private
workspace-test log is registered under evidence:root/20261107002919.

## How to read it

Management UI plus lifecycle guard is implemented; actual CEF adapter is not.
Target-tab count remains0; address edits do not send requests. CEF attach,
runtime sandbox, OS AT/native IME/DPI, all six native tuples and Docker product
e2e are not_run/not_implemented as applicable. No page frame/mock browser,
authority credential, copied scorer or personal browser profile is present.
No human ALLOW or anti-automation performance result follows from these tests.
