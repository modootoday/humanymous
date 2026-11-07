---
id: evidence:root/20261107053625
title: Real newtab and measured CEF internal URLs
status: active
created: 2026-11-07T05:36:25Z
updated: 2026-11-07T05:40:34Z
rel:
  depends_on: [sot:root/browser-ui, sot:root/chromium-lab-browser, plan:root/20261107050329]
files:
  - {path: results.json, sha256: c33a6fb9af2f1aca871663930e1f506b1cbf46d62c5f91eb2c07c5e31896a691, bytes: 20625, store: repo}
---

# Real newtab and measured CEF internal URLs

Actual Windows x64 CEF156.0.15 / Chromium156.0.8078.80 ALLOY child,
mandatory sandbox retained. Script-free `humanymous://newtab/` provides a real
GET form and five diagnostic links. Native address Enter, home, reload, resized
CEF viewport and coordinated close were observed. Parent OS keyboard focus was
fixed after a real failure; visual native focus alone had left keys in CEF.

## Measured support and admission

| Route | Exact-build result | Product policy |
|---|---|---|
| humanymous://newtab/ | Actual document, form/link and responsive layout | Admitted |
| chrome://version/ | Content identity and native AX version fields | Admitted; logo/assets parity incomplete |
| chrome://gpu/ | Content identity and native AX populated feature status | Admitted; GPU policy qualification incomplete |
| chrome://credits/ | 5,225,290-byte document and native readable credits | Admitted after bounded8MiB retry |
| chrome://license/ | Content identity, native keyboard Enter and AX license | Admitted |
| chrome://sandbox/ | Content identity, real form submission and native status table | Admitted; no full sandbox attestation |
| chrome-urls/settings/flags/inspect/newtab/unknown chrome hosts | Policy-denied, not executed | Denied |
| crash/hang/quit/restart | Preflight-denied; never executed | Denied |

This is a curated tuple-specific matrix, not enumeration of every upstream URL.
CEF/Chromium source registration is not the same as measured rendering support.
`chrome://chrome-urls` can change a debug-page preference; quit/restart can act
before CEF navigation throttle callbacks. Host exact preflight precedes LoadURL.
Builtin WebUI JS is upstream engine content; no application JS is authored or
injected. Script-free newtab has its own restrictive CSP. File export, popups,
permissions and external resources/navigation are denied.

Core24/native13 tests pass, one native feature test covers4 weakening flags with
zero created profiles, warning-denied Clippy and targeted fmt pass. The final
12-second native probe records7 identified documents and close exit0. First
credits budget exhaustion, focus failure, encoding fault and build/test retries
are preserved. No Hanja replay, third-party probing, profile deletion or product
e2e was performed. Raw paths/device details/diagnostic content remain private or
transient; no screenshot was exported because a foreign system overlay occluded
the owned window. results.json contains structural receipts and source/artifact
hashes only; LF attachment bytes are checked against canonical Git bytes.

## Limits and source binding

The interactive focus/resize run uses its own source snapshot and DLL hash. The
final source adds explicit download refusal; final native document/cleanup probe
passes, while interactive export denial remains unexercised. Working-source and
canonical LF hashes are distinct for inherited CRLF files. Raw results remain in
private evidence20261107050329; no profile/cache/build tree is published.

Full sandbox=false. The sandbox WebUI displayed Network Service Not Sandboxed and
RendererAppContainer disabled; this is engine-reported evidence, not independent
OS qualification. Transient GPU exit78, native shell AX click bounds, OS screen
readers, new-layout IME, high DPI/multi-monitor, full Chrome assets/parity,
external navigation, multi-tab/history, observers and signed distribution remain
open. BrowserReady remains false.

Primary source review: [pinned CEF WebUI tests](https://github.com/chromiumembedded/cef/blob/cf671c072925b27823a61181152f4f288e8e4306/tests/ceftests/webui_unittest.cc),
[Chromium156 chrome-urls handler](https://raw.githubusercontent.com/chromium/chromium/refs/tags/156.0.8078.80/chrome/browser/ui/webui/chrome_urls/chrome_urls_handler.cc),
[Chromium156 about handler](https://raw.githubusercontent.com/chromium/chromium/refs/tags/156.0.8078.80/chrome/browser/browser_about_handler.cc).

Public scoped Specfold check (`--no-local`) passes0errors/0warnings. A check
including private discovery was stopped after more than2minutes without a
result; it is not a pass. All41 bounded private raw attachments independently
match their SHA-256 and byte-size manifest.
