---
id: evidence:root/20261107113711
title: Apache adapter hosted per MPM failure diagnosis
status: active
created: 2026-11-07T11:37:11Z
updated: 2026-11-07T11:38:46Z
rel:
  governs: [crates/apache-adapter]
files:
  - {path: result.json, sha256: 0d3ffbb6455f6d79d0bed57844451a1364c8c483ab00239e6d8004f9a6890613, bytes: 10234, store: repo}
---

# Apache adapter hosted per MPM failure diagnosis

## What was measured

Actual GitHub run42367750854, component job128389302171, exact source
d65da705398b9b10a558d9e3e934467c2fedf43e. Safe9and coordinator11units passed,
each with zero failure/ignored/filtered. Linux pilot check and both preflight
Clippy steps passed; actual production/probe SDK builds and Clippy completed.
All45required pairs ran:23passed/22failed/0not_run. Attach and actual container
exits were independently1/1. Pilot cleanup remained false; outer container/image
cleanup and separate ledger cleanup passed. Attached safe projection binds
source/manifest/image/five independent artifact hashes and every case result.

Event passed14of15, with subrequest-rejection failing its nested assertion.
Worker passed8of15, failing the seven allow/probe/deny/challenge/reload cases.
Prefork passed only cleanup; cache denial failed at httpd-stop and several later
cases had generic operation-failed labels. The latter generic labels do not
establish a specific OS/I/O cause or authorize changing resource bounds.

## How to read it

Pinned upstream query implementations and the wrapper identify a concrete
unsupported async-query initialization bug for worker/prefork. The separate
event probe incorrectly ran a rejected lookup-only subrequest. Candidate fixes
retain fail-closed identity/capability checks and the exact existing nested
status/marker/origin assertion; owning SoT records primary source links.
Three new safe capability boundary tests and a bounded IO-kind extension of
the existing privacy test are authored but unrun. New expected pure denominators
are12safe and11coordinator. No later pass is inherited here. All22actual failures
and false pilot cleanup remain failures; fullProductSuccess remains false.
