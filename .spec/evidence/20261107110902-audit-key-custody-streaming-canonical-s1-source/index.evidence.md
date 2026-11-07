---
id: evidence:root/20261107110902
title: Audit key custody streaming canonical S1 source
status: active
created: 2026-11-07T11:09:02Z
updated: 2026-11-07T11:09:02Z
rel:
  governs: [crates/audit/src/record]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody streaming canonical S1 source

## What was measured

Root accepted restricted S1 after independent review of43cad32e; count/replay
framing, unsigned proof preservation and separate compatibility Scope conditions
were incorporated in plan103624 and owning SoT before code. Authored the first
PART of S1: shared canonical field emitter and Go stream-safe NFC iterator.
Compatibility canonical_bytes collects the emitter; record_hash uses an8KiB
buffered SHA256 sink without a complete canonical Vec. Normalized strings and
joined rules/signals are emitted without full intermediate payload buffers.

The same deterministic emitter counts UTF-8 bytes before its u32 prefix and
replays through a length/total-budget checked sink. Array reference limits are
checked before collection; overflow, changed replay and I/O errors refuse.
Raw rule/verdict bytes, original-ID Go tie permutation, original-empty omission,
confidence formatting, signed-i64 seq cast and original field order are retained
by source intent. Existing public APIs retain compatibility mode, not a newly
claimed deployment resource profile.

Five pure tests are authored: all92 immutable Go canonical vectors at exact/
short budgets, item gate before output, count/replay disagreement before excess
payload, length overflow without massive allocation, and partial-write/I/O failure.
Existing13k+ Unicode15 corpus and record-hash fixtures remain unchanged. Format,
scoped Specfold and diff checks only; no compile or test execution yet.

## How to read it

Later exact sourcef88e76f4 passed allocated all-target check/Clippy and9
record-focused Rust tests covering92 unique canonical fixtures/13464 Unicode
texts with114-file zero drift. Evidence:root/20261107111706. This supersedes the
authored-only NOT_RUN statement above for that precise scope; no complete S1 or
allocation guarantee follows. Full audit regression is separately tracked.

This is not all of S1: frame preflight, incremental compatibility verifier,
frontier/ranges, native inspection and checkpoint message encoder remain pending.
S2/S3 and production recovery/authority mutation are not authorized. No128MiB
allocation or RSS guarantee is measured; normalization internals, sort references
and buffered hashing remain in the future capacity ledger. Prior110 Windows
passes are exact old-source history and do not qualify this delta. No root
manifest/dependency/scoring change or new workload occurred.
