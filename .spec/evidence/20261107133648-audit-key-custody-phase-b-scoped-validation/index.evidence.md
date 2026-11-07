---
id: evidence:root/20261107133648
title: Audit key custody phase B scoped validation
status: active
created: 2026-11-07T13:36:48Z
updated: 2026-11-07T13:40:50Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 005213ce3f4b088fd15e4cc5d59afb9f8d62e012610b7e867c18d6b23e6d148d, bytes: 2385, store: repo}
---

# Audit key custody phase B scoped validation

## What was measured

Root-allocated A, corrected immutable git archivefb06c4d161850a24d24a2ff735aff761e95eb090,
ordinary-user Windows/Rust1.99.1, jobs1/incremental0/thread1, locked+offline and
exclusive target/validation-audit. All-target check and strict Clippy passed.
Six single-summary harnesses passed33 unique Rust tests, failed/ignored/measured0:
scan15 (82 intentionally filtered), framing13 (84 filtered), verifier parity1,
record parity1, checkpoint parity2 and Merkle parity1 (all parity filters0).

All eight new composed two-pass tests ran: sparse prefix/frontier joins through
64 with complete unsigned record65, continued checks after earlier failure,
sequence/HMAC/base/witness priority and Scope, raw hash/signature preservation,
transcript/pin-sidecar mutation, heads beyond EOF, missing witness/new-empty,
and final cancellation/resource refusal. Go fixture rows verifier840/canonical92/
checkpoint180/Merkle66 are separate vector denominators, not extra Rust tests or
new unique rows across earlier batches. No full package suite was executed.

Full immutable snapshot2488 file hashes/inventory and reachable registry77
packages/3300 file hashes/inventory remained unchanged. Registry closure comes
from locked/offline Windows metadata, normal/build graph plus root audit dev
edges, conservatively including feature-unified inputs. All20 Cargo configuration
candidate paths from snapshot through drive ancestors and CARGO_HOME were absent
before/after; absence state is recorded, not20 existing hashed config files.
No config/credential contents or values are retained. Toolchain identity,
arguments/timings, manifests, inventories and original logs are attached to
private evidence:root/20261107132346. Large archives remain hash-bound local
inputs rather than attachments, following the existing4MiB attachment limit.

The first exact8dbb attempt compiled with3 private-interface warnings, then strict
Clippy failed101 before any tests. Both inspect and Diagnostic were narrowed to
scan-private in correctedfb06, without widening Source/Limits/Control, lint
suppression or crypto behavior change. The original archive/logs and correction
diff are preserved. The pass qualifiesfb06, never retroactively8dbb.
Session27344 terminal exit0, A returned, no process or further workload held.

## How to read it

Private two-pass component diagnostics only. Source remains a sealed in-memory
fixture; actual native producer is Unsupported. This proves neither native file
identity/ACL/crash behavior nor working-buffer128MiB/RSS, whole S1, operational
recovery/S2/S3, ServiceHost integration, hosted matrix or Docker product e2e.
No physical checkpoint/previous-link payload, mutable owner, repair or authority
is returned. Historical full140/Phase-A21 results have their own exact scopes;
do not sum overlapping cases with this33-test batch.
