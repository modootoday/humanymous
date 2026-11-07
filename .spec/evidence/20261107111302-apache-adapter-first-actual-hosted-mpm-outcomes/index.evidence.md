---
id: evidence:root/20261107111302
title: Apache adapter first actual hosted MPM outcomes
status: active
created: 2026-11-07T11:13:02Z
updated: 2026-11-07T11:14:08Z
rel:
  governs: [crates/apache-adapter]
files:
  - {path: result.json, sha256: 3ae75eb2b0e117835b69c10a94e8318030f34e046d7dd4098846ec6c358150dd, bytes: 2300, store: repo}
---

# Apache adapter first actual hosted MPM outcomes

## What was measured

Actual standard GitHub run42366328484, component job128385068259, exact source
8042c6bc457297732332c199f01ca2f80ab3f48a. Safe units9and coordinator units10,
Linux pilot check and both preflight Clippy steps passed. The Dockerfile's actual
production/probe native module builds and warnings-denied Clippy completed, as
did its pilot executable build/Clippy. Build duration358.581seconds; actual
isolated Docker pilot duration4.306seconds. The image and independently copied
module/probe/httpd/ABI-receipt/lock hashes are attached in the safe result.

All45required scenario/MPM pairs executed:23passed,22failed,0not_run. The
strict validator accepted the report inventory/source/artifact bindings but
rejected its failed process/outcomes. The coordinator reported attach-command
failure and pilot cleanup false. Container/image cleanup was observed and the
separate always-ledger-cleanup step also passed. Full job logs were captured
privately; no private bundle or automatic artifact upload occurred.

## How to read it

This establishes the first actual native SDK build and actual MPM fixture
execution for these bytes, not a qualified Apache tuple. No case identity or
failure cause is reconstructed from aggregate totals. The original public
summary omitted the validated case array; those per-case private outputs were
not uploaded. A source-only follow-up projects the already validated case IDs,
results and whitelisted static assertion labels, with generic fallback for
unknown errors, and independently observed attach/container exits. It changes
neither the pilot's closed report schema nor strict45acceptance. One pure
privacy/failed-ID-binding test is added; it is not yet executed. Earlier10-test
receipts remain historical. Full product success remains false.
