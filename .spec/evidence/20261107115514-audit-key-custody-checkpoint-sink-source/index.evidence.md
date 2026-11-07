---
id: evidence:root/20261107115514
title: Audit key custody checkpoint sink source
status: active
created: 2026-11-07T11:55:14Z
updated: 2026-11-07T11:55:14Z
rel:
  governs: [crates/audit/src/checkpoint/signing]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody checkpoint sink source

## What was measured

Source-only shared checkpoint signing encoder, using root-owned signals sink
quote_json_go_to from56a2acdb. No duplicate escaping implementation or dependency
change. One deterministic encoder counts into a byte-limited sink, reserves the
exact message length fallibly, then replays through the same capped sink. It
checks replay length; signing_bytes remains the compatibility Vec wrapper with
usize::MAX limit. Later bounded inspection must supply its finite trusted limit.
Message field order, unsigned tree_size decimal and non-NFC strings remain the
frozen contract. Writer/witness signatures are excluded from signed bytes.

Checkpoint signature hex decoding now targets a fixed64-byte stack buffer,
preserving exact length/hex validation while avoiding attacker-sized decoded Vec.
Three authored pure tests cover all180 frozen checkpoints and exact/short budgets,
HTML/control/line-separator/decomposed Unicode/u64::MAX bytes, every partial sink
failure position, and synthetic counter exhaustion without output or wrap.
Existing checkpoint parity180 independently pins actual Go writer/witness
signatures and must run before qualification. Formatting/diff only; all new
compiler, Clippy, tests and runtime qualification NOT_RUN.

## How to read it

The compatibility wrapper still returns a complete message Vec; this source does
not wire the finite bounded verifier or production recovery. Count-and-replay
does not prove whole-startup128MiB/RSS. Source and root-owned escaping API both
need allocated execution. No S2/S3, native/hosted or product e2e claim.
