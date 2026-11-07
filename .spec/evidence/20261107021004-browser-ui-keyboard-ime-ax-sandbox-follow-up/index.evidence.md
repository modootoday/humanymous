---
id: evidence:root/20261107021004
title: Browser UI keyboard IME AX sandbox follow-up
status: active
created: 2026-11-07T02:10:04Z
updated: 2026-11-07T02:21:03Z
rel:
  depends_on: [sot:root/browser-ui]
files:
  - {path: results.json, sha256: a70f8a0b144cb25866513c733d7a7ab9b0157a3e2afd3e5432e907124880cf8d, bytes: 6559, store: repo}
---

# Browser UI keyboard IME AX sandbox follow-up

## What was measured
Windows x64, exact final source manifest and native artifact SHA256 in results.json.
Actual source-linked DLLs/bundle, Rust1.99.1, pinned CEF156 and Meari0.2.0.
Seven native units passed with zero ignored, separate explicit feature test passed
one case/four security overrides, Clippy/fmt/DLL build passed. Core nine existing
browser tests apply to unchanged core source. Root owns the shared membership/lock
and separate component CI integration; final commit reconciliation is separate.

Native Tab/physical Latin typing passed. With opt-in focused Korean context setup,
r -> ㄱ -> k -> 가 -> BackSpace -> ㄱ -> k -> 가 -> Return was observed on actual CEF
input via fresh native UIA readbacks and screenshots; no script/Unicode Value action
implemented that sequence. Language1042 and IME open/native-mode1 were observed.
IMM composition-length samples stayed0; this does not establish IMM preedit events.
F9 asked the native IME to convert/open candidates: conversion=true, open=false.
No visible candidate window/form: placement/selection remain unqualified. A working
native Hanja shortcut was requested from the user; no reply at evidence freeze.

Native Unicode Value actions/readbacks passed for both CEF and Meari inputs. Meari
tab geometry action failed with no cached bounds. Focus may expose the document
instead of field; values may lag a capture. Screen-reader speech, full traversal,
geometry, DPI and complete AX qualification remain open. Keyboard input is a
bounded successful path, not general input/browser qualification.

The fixture renderer directly reported access denied5 for a known generated
canary read and sibling create_new. Its PID joins to an owned restricted/Untrusted
renderer OS token. Only one of two observed renderers had the browser callback;
the spare is not file-tested. Utility policies, all-path file restrictions,
job/mitigations, OS egress, signing and escape resistance remain unqualified.
Normal and actual canceled/late-created child cleanup both exit0. BrowserReady
and external navigation remain false; no Docker product e2e result is claimed.

## How to read it

Public aggregate: results.json; raw evidence:root/20261107004558:
report-exact-final.json, pilot-exact-final.log, report-cancel-followup.json,
uia-exact-{consonant,syllable,backspace,commit,cef-value,host-value}.json,
report-candidate.json and uia-ime-candidate-{request,settled}.json, plus the tests,
Clippy, fmt/build and feature-negative logs. Earlier exploratory reports, failed
focus experiments and forced termination of only that experiment's own processes
are historical failures, not passes. The pointer-based focus change was removed.
The hard-deadline regression tests prove supervision survives a blocked close wake;
this is independent of claims about every GUI failure mode.

The user explicitly approved generated-profile deletion. Automatic approval review
again rejected the recursive deletion command containing path validation with blocked by policy.
Nothing was deleted; no alternative mechanism was attempted.31 generated private
profiles remain, all own native processes closed. Profiles and raw artifacts stay
private; only synthetic text/metadata aggregates are public. Legacy global private
spec errors remain separate from the browser-scoped checks.
