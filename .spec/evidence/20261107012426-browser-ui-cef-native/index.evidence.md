---
id: evidence:root/20261107012426
title: Windows sandbox-required CEF child implementation and native results
status: active
created: 2026-11-07T01:24:26Z
updated: 2026-11-07T01:44:22Z
rel:
  depends_on: [sot:root/browser-ui]
files:
  - {path: results.json, sha256: 500164b12a70deaa1ea776bfabf247d0ddd0ea4280e313ab84ff1499949f3b11, bytes: 5968, store: repo}
---

# Windows sandbox-required CEF child implementation and native results

## What was measured

2026-11-07 KST, actual Windows x64 native prototype, Rust1.99.1. Two Rust DLLs
link against pinned CEF156 and Meari0.2.0. The upstream bootstrap executes real
CEF subprocess handling, then the browser-only DLL creates the actual Meari
parent and CEF windowed child. No external-window docking/video masquerade or
authored JS/TS/HTML shell is used. Static target data is generated from Rust IR.

| Scope | Observed result |
| --- | --- |
| Integrated core tests | 9 passed after shared style reuse |
| Native library tests | 4 passed; default listing separately ignores one provisioned-bundle test |
| Explicit native negative test | 1 passed, exercising 4 rejected security switches before UI/profile creation |
| Final native normal pilot | Created child, actual parent/geometry/focus-within-child observed, on_before_close and Meari cleanup completed; exit0 |
| Final cancellation pilot | Genuine creation after cancellation observed, immediately closed, actual child/parent cleanup completed; exit0; not counted as attached/ready |
| Final native UIA | Actual CEF document and named input, Unicode Value action readback; normal close completed |
| Renderer properties | Exact owned executable/descendant/role checks, actual restricted tokens and Untrusted RID0 for observed renderers |
| Build/lints | Actual DLL link, Clippy -D warnings, formatting passed through source-linked private workspace |

The native packages are not yet members of the shared root workspace. The private
workspace references the actual authored lib.rs files and real integrated core;
no algorithms are copied into the harness. Root owns membership/lock integration.
Source SHA256, exact binary hashes and sanitized results are in
[results.json](results.json). Raw logs, private dependency lock, binaries/build
cache and synthetic-only UIA outputs are registered in
private evidence:root/20261107004558.

## How to read it

BrowserReady=false and arbitrary target navigation remains disabled. Renderer
restricted-token/integrity checks are limited sandbox evidence. They do not
qualify every utility process, filesystem denial, complete sandbox policy or
escape resistance. Upstream bootstrap/libcef were Authenticode NotSigned. No
signed clean-machine package, native CVE/SBOM qualification, six-native-tuple
coverage or Docker product e2e is claimed.

CEF SystemInfo.getProcessInfo returned unsupported_or_failed; the distinct bounded
native OS role query is recorded separately. Actual AX value editing is not an
IME composition/candidate-window, keyboard-input or screen-reader test. Earlier
keyboard injection had no confirmed value change. Cached UIA geometry was
unavailable for final interactive resize, and system-menu automation did not
establish a resize pass. The final report verifies initial child bounds; an
earlier dirty prototype report9 separately observed two resize readbacks.

Exploratory pump hangs/close timeouts, native/tool setup errors and uncompiled
callback-test attempts remain private failures, not successful cases. Only final
source-hashed native reports support the final positive claims. Earlier dirty
experiments lack complete source snapshots and are not fully reproducible gates.
No server ALLOW, human identity or anti-automation performance is measured.

All pilot windows/processes closed. Bulk deletion of retained self-generated
synthetic profile directories was rejected by automatic approval review, whose
only stated reason was `blocked by policy`. No alternative deletion was attempted.
The private runtime cache remains, and must not be published or inspected for
credentials. Disk-profile deletion is distinct from successful native child close.
