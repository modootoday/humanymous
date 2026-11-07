---
id: evidence:root/20261107034226
title: Browser UI research design and compact Chromium shell
status: active
created: 2026-11-07T03:42:26Z
updated: 2026-11-07T04:17:05Z
rel:
  depends_on: [sot:root/browser-ui, sot:root/chromium-lab-browser, sot:root/native-embedding]
files:
  - {path: results.json, sha256: 6d949e55a21ddb07c14455f2c57bc3167e5fc2d3e0e0f35149c8df044d542f51, bytes: 19681, store: repo}
  - {path: native-preview.png, sha256: 16af87c7c10d8947ff02b77d09a79ca2fc99b29e44dddc02d2884c36ee2de15d, bytes: 47789, store: repo}
---

# Browser UI research design and compact Chromium shell

## What was measured

Actual Windows96-DPI CEF child with Meari controls: undecorated host, SVG icons,
right/bottom panels and native resize/cursors. East/south/SE/SW gestures changed
1116x759 to976x649 through recorded intermediate sizes;6 CEF size readbacks.
OS cursor identities independently included ew/ns/nwse/nesw resize, text and
pointer. Custom close exit0 with CEF before-close and host-closed receipts.
Predecessor runs measured maximize/minimize/restore and exposed two fixed errors.
Core18/native11 and native security-override1 (4flags) pass; no ignored tests.
Final warning-denied Clippy and targeted fmt pass.

## How to read it

results.json binds final sources, GUI-tested source snapshot/artifacts and private
raw reports. Final source differs by Clippy item/guard restructuring plus post-review maximized
arrow and DPI minimum/capture fixes. These pass tests/lints/build; their native GUI
branches are unverified. The preserved GUI snapshot/DLL identify actual gestures.
Initial failed reports are preserved separately. Parsed SVG/AX/model tests do not
qualify native screen readers, all8 native directions or high-DPI behavior.
The public preview contains only generated fixture data; raw private evidence is
not published. Retained synthetic profiles remain private; deletion was not retried.

## Limits

The real CEF engine surface is established; Chrome rendering parity is not.
Full sandbox, external/internal-URL navigation admission, observers, SDK DLL ABI,
new-geometry IME, keyboard resize/Snap, high-DPI/monitor/cross-OS and full AX/product
e2e remain unqualified. No score changes, Hanja replay or sandbox weakening.
SDK scope is accepted design only. Follow plan:root/20261107041001 and
plan:root/20261107033358; project conventions remain sot:root/rust-migration.


## Spec and preview disclosure

Owned public documents and the private native manifest each check with0 errors/
warnings. The whole-tree snapshot did not pass:3048 errors in other shared
private-evidence attachments and1 U2 warning. U2 scans text but skips binary PNG;
no automatic secret scan of pixels is claimed. The published native-preview.png
was manually visually reviewed: generated owned CEF fixture and local UI only,
no personal page/profile or unrelated desktop content. Its original bytes are
hashed above. No exemption or scanning-policy weakening was added.


Canonical-byte correction: attached results.json is explicitly LF under
.gitattributes; its declared hash/length cover clean-checkout Git bytes. Existing
validation-worktree source hashes remain intact, with canonical Git source hashes
recorded separately. This fixes text-byte portability only, not runtime evidence.
