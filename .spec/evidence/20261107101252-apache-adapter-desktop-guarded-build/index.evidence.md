---
id: evidence:root/20261107101252
title: Apache adapter Desktop guarded build
status: active
created: 2026-11-07T10:12:52Z
updated: 2026-11-07T10:12:52Z
rel:
  governs: [crates/apache-adapter]
---

# Apache adapter Desktop guarded build

## What was measured

Root allocated C for an actual Windows Docker Desktop native build and pilot.
The immutable clean detached source was
e691b971c941725c1616cd9f668d020896e05cc1. The coordinator verified the explicit
Windows docker.exe, desktop-linux context, Linux server29.9.2 and exact server
identity before workload and cleanup. Admission sampled32.03GiBfree and
81.97percent committed. No WSL native Docker engine was used.

The first build exited1 after6.869seconds because the Windows child could not
resolve docker-credential-desktop. Its failure and private logs were retained.
A process-scoped parent Windows PATH prefix was verified by a WSL-launched
Windows where.exe resolving the canonical Docker companion executable. No
credential values, credential store or global PATH were read or changed.

The fresh-bundle retry completed actual upstream Apache/APR build and install,
safe crate tests and Clippy, then reached the selected httpd Rust dependency
compilation. At the root guard threshold88.2594percent committed, the owner
verified the build child's /proc command line against this run's unique label
and terminated only that Docker child. Coordinator exit1 reported
owned-command-signal at build after154.497seconds. It observed container/image
cleanup; a separate ledger cleanup exited0 and passed. A follow-up inspection
found no matching Windows Docker child or Linux coordinator process.

## How to read it

Neither attempt completed a native module/image or ran Apache scenarios.
There is no image or ABI qualification receipt. Observed case counts remain
null, not an invented zero-failure result. All45required scenarios are NOT_RUN.
fullProductSuccess remains false. Private raw logs, source manifests and owned
ledgers remain in private evidence; they are not public upload artifacts.

The source-only follow-up requires Windows-child where.exe resolution of the
canonical credential helper before a Docker workload. It awaits separate
Linux check/Clippy and actual execution. The next run requires root allocation
and memory admission; this guard-triggered failure does not grant a slot.
