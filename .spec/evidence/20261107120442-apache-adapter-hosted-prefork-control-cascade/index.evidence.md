---
id: evidence:root/20261107120442
title: Apache adapter hosted prefork control cascade
status: active
created: 2026-11-07T12:04:42Z
updated: 2026-11-07T12:07:22Z
rel:
  governs: [crates/apache-adapter]
files:
  - {path: result.json, sha256: 9a04877f2d9e3f18735128f2ad8d90454e0a5cb5b2e412a23eeed8827b959095, bytes: 9665, store: repo}
---

# Apache adapter hosted prefork control cascade

## What was measured

Actual GitHub run42369797560, component job128395369698, exact source
2d66c0df7f5ea4e925a30d2f0b39c05bebd696ad. Safe12/coordinator11units passed
with zero failures, ignored or filtered tests. Linux check, preflight Clippy and
actual production/probe SDK builds/Clippy completed. All45pairs ran:33passed,
12failed,0not_run. Event and worker each passed15/15. Prefork passed binary
replay, spoofed provenance and cleanup, totaling3/15.

The first prefork failure reached the real subrequest assertion, then failed
the separate httpd-stop command. Redirect and cache cases likewise reached
stop and failed there. Later primary-server requests observed connection
refused; reload could not observe an inflight authority request. The primary
connection failures can be a cascade from earlier process-control faults and
are retained as actual failures, not additional independently proven causes.

Attach/container exits were1/1. Pilot cleanup remained false; outer container/
image cleanup and separate ledger cleanup passed. The attached safe projection
includes all45results and actual image/source/manifest/five artifact identities.
Full job logs were captured privately. No private bundle was uploaded.

## How to read it

The MPM query and rejected-lookup changes improved actual coverage from23to33
passes and fixed both complete event/worker groups. They do not qualify the
whole Apache component. Historical CLI stdout/stderr were redirected to null
and cannot be reconstructed; no Apache error code or exact CLI cause is inferred.
Root accepted investigating an instance-pinned pidfd control fixture with the
same upstream TERM/graceful signals and actual exit/generation/cleanup gates,
without any PID limit, deadline, expected-case or scoring relaxation. New code
and original-CLI diagnostic capture require separate review/actual proof.
