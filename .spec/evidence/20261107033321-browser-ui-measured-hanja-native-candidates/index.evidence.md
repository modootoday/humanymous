---
id: evidence:root/20261107033321
title: Browser UI measured Hanja native candidates
status: active
created: 2026-11-07T03:33:21Z
updated: 2026-11-07T03:33:58Z
rel:
  depends_on: [sot:root/browser-ui]
files:
  - {path: results.json, sha256: 9e3856e4c26983a5a44c11c17f301a750ca363ed92bd6f3cdf063c29b543f5e8, bytes: 4692, store: repo}
---

# Browser UI measured Hanja native candidates

## What was measured

## How to read it

## Measured scope and limits

User-provided VK19/scanF1/extended key was replayed through bounded Rust SendInput
only in the owned foreground CEF child. Real native candidates appeared, 家 and
second candidate價 were selected, below/above-field placement and Escape dismissal
were visually observed. Native HWND candidate geometry was unavailable. Selection
was observed before a final report-label-only amendment; final artifact/source
hashes identify the later above-placement/escape run, not an invented exact
selection rerun. Computer Use stopped on user physical Escape; no subsequent
Hanja inputs were performed. No changes to scoring, sandbox requirement or
target-navigation admission. BrowserReady remains false.

Raw reports/logs, uia-measured-* captures and measured-candidate-above.png /
measured-candidate-escape.png stay private in evidence:root/20261107004558.
This evidence predates the browser-style UI redesign and does not qualify its
new geometry, cross-monitor DPI, screen-reader candidates or full sandbox.
