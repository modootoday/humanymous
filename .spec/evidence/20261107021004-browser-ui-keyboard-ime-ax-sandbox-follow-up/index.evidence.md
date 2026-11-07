---
id: evidence:root/20261107021004
title: Browser UI keyboard IME AX sandbox follow-up
status: active
created: 2026-11-07T02:10:04Z
updated: 2026-11-07T02:33:12Z
rel:
  depends_on: [sot:root/browser-ui]
files:
  - {path: hanja-keycode-research.json, sha256: 944db48628c9d34a6d38b303ca02695ec3d0d2f87969eaa1fb9c63bba7c2851d, bytes: 1502, store: repo}
  - {path: results.json, sha256: e7febe4753abb79815702a9ebec2d7f73fa36b7750105229c6c9550be8846d56, bytes: 7236, store: repo}
---

# Browser UI keyboard IME AX sandbox follow-up

## What was measured
Windows x64, exact final source manifest and native artifact SHA256 in results.json.
Actual source-linked DLLs/bundle, Rust1.99.1, pinned CEF156 and Meari0.2.0.
Seven native units passed with zero ignored, separate explicit feature test passed
one case/four security overrides, Clippy/fmt/DLL build passed. Core nine existing
browser tests apply to unchanged core source. Root owns the shared membership/lock
and separate component CI integration; root commit ac560ff integrates this source; new remote CI is pending.

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
Nothing was deleted; no alternative mechanism was attempted.32 generated private
profiles remain, all CEF test processes closed. Profiles and raw artifacts stay
private; only synthetic text/metadata aggregates are public. Legacy global private
spec errors remain separate from the browser-scoped checks.

OSK follow-up on unchanged a924603 source: the built-in Korean on-screen keyboard
exposed a Hanja key. Its click had no confirmed key-delivery/candidate receipt.
Candidate qualification remains open. The owned CEF pilot exited0. OSK UI-close
had no confirmed effect; stopping only its introduced PID/start-time matched
process returned access denied. No elevation/security bypass was attempted; the
introduced OSK window remains for manual close.32 generated profiles now remain.

Shared integration is committed/pushed in ac560ff with browser source a924603.
Root evidence .spec/evidence/20261106235519-program-status/cef-root-integration-results.json
records DLL build, native7/0ignored, Clippy and explicit1case/4flags pass. Its native
source SHA256 map was compared with this evidence and matches. Root artifact
hashes differ from the source-linked GUI bundle and have separate scope. The new
remote exact-integration CI is pending; neither that component gate nor native
Linux stubs establish full browser/IME/AX/sandbox or six-tuple runtime support.

## Hanja keycode research correction

The user confirmed a physical Hanja key. Official Windows VK_HANJA=0x19(25),
VK_KANJI=0x19 and VK_HANGUL=0x15(21). VK_RCONTROL=0xA3 is distinct; a layout
shortcut must not be assumed equivalent to the Hanja virtual key. The documented
computer-use API accepts keysym strings, not raw VK/scan-code arguments. Standard
Hangul_Hanja (X0xFF34) and Kanji (X0xFF21) were both tried on the owned CEF input
with native Korean 가 present; both returned unsupported key before delivery.
This is a tool mapping limitation, not an unknown/nonexistent Hanja key. No native
raw input helper, direct key-message injection or inferred hardware scan code was
used. A supported VK0x19 delivery path or an observed physical Hanja event is
needed to qualify actual candidate display/placement. Candidate remains open.
The unchanged-source pilot exited0;33 private generated profiles now remain.
See hanja-keycode-research.json in evidence:root/20261107021004 for primary links
and the exact unsuccessful aliases. No application source changed.
