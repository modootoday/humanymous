---
id: evidence:root/20261107102326
title: Hosted Pages validation and artifact transfer failure
status: active
created: 2026-11-07T10:23:26Z
files:
  - {path: result.json, sha256: e5057c340c99df638922490d5782c8dc2b2ecaba37e59449c95d8c2b11af9790, bytes: 1279, store: repo}
updated: 2026-11-07T10:24:54Z
---

# Source-bound Pages evidence

GitHub run 42363224485 completed successfully on source 8624b57c6ff4c9f8eb3763814659e17520e858b8. The preflight log proves 60 tests passed with zero failed, ignored or filtered, followed by strict Clippy and workflow policy success. Both in-job base URL validations passed; Pages independently observed 88 rendered pages per base URL.

The downloaded generic apex and subpath artifacts both lack .well-known/security.txt. A successful in-job build therefore does not qualify the transferred artifacts or deployment. Automatic branch artifact uploads also diverged from the free branch policy and are being removed. Future public-site packaging must preserve required files and pass round-trip validation. No browser, assistive-technology or deployment qualification is claimed.

Root independently retrieved the pinned upload-pages-artifact action source and
confirmed its tar exclusion of dot directories. Disabling generic branch uploads
alone therefore does not repair the main Pages packaging path. A public-site
allowlist and actual extracted-archive validation must precede deployment.

