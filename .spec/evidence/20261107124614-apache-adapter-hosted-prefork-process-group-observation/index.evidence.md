---
id: evidence:root/20261107124614
title: Apache adapter hosted prefork process group observation
status: active
created: 2026-11-07T12:46:14Z
updated: 2026-11-07T12:46:35Z
rel:
  governs: [crates/apache-adapter]
files:
  - {path: result.json, sha256: f9df0456157d87c4f78f1fbe62b87a16d0f0135d46f8a12862fd14c0a5832b18, bytes: 9252, store: repo}
---

# Apache adapter hosted prefork process group observation

## What was measured

Genuine hosted run42373232097/job128405466493 on exactb5c15f0df324373adcbe9a139bfc73cdb2efcd3b
passed safe12/native13 tests, checks, both preflight Clippy gates and actual SDK
builds. The real component observed35passed/10failed/0not_run: event15/15,
worker15/15,prefork5/15. All nested/cache prefork assertions passed, but later
primary requests were refused, inflight was absent and cleanup failed. Attach/
container exits1/1; pilot cleanup false. Outer and separate ledger cleanup passed.

The isolated fresh prefork diagnostic observed its CLI helper terminate by
SIGTERM15 with no exit code, zero stdout/stderr bytes, no allowlisted AH code,
no timeout/limit/forced cleanup, correct PID-file match and real nested proof.
Master was alive after CLI reaping; subsequent pidfd stop observed masterexit0
and reaping. Diagnostic cleanup passed. This does not qualify general CLI use.

## How to read it

The pinned prefork shutdown source sends SIGTERM to getpgrp(), not only the
master's PID. Current std::Command children inherit the pilot process group;
actual runtime PGIDs were not captured, so historical numeric group identities
cannot be inferred. Source and the observed helper signal motivate isolating
each foreground master/helper group, with actual kernel ownership checks in
the next candidate. Earlier35/10and33/12runs remain failures. Root review and a
new exact-source actual retry are required; no scoring, bound or case changes.
[Pinned shutdown](https://github.com/apache/httpd/blob/e8ada83f004f63bca1e0cc75e374bc25b06cdc7b/server/mpm/prefork/prefork.c#L1001-L1016).
