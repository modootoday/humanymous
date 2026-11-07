---
id: evidence:root/20261107114649
title: Audit key custody frame cursor source
status: active
created: 2026-11-07T11:46:49Z
updated: 2026-11-07T11:46:49Z
rel:
  governs: [crates/audit/src/storage/framing]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody frame cursor source

## What was measured

Source-only restricted S1 frame cursor, after root accepted next private framing/
preflight/signing source work. No Cargo workload; Pages holds the heavy Desktop
Ruby build. Formatting and diff checks only. Compilation/tests NOT_RUN.

One reusable frame Vec and borrowed next-frame slice replace duplicated framing
logic. Compatibility decode still collects values through the same cursor.
Bounds include LF and apply before extension/consumption of an offending chunk.
Growth requests are capped at max_frame; allocator overhead/capacity is not a
measured bound. Checked committed offset advances only for newline frames. CRLF,
empty-line skipping, and allowed non-newline tail behavior remain unchanged.
Any frame/read/allocation/offset failure poisons the cursor; retry requires a new
scan. No repair, signing, trusted state or authority is emitted.

Four authored pure tests cover byte-fragmented CRLF/blanks/torn offsets,
oversized-frame refusal before offending input consumption, valid prefix followed
by injected I/O failure, and synthetic offset overflow/no-resume. Existing three
framing tests retain JSON corruption and exact frame-boundary coverage.

## How to read it

This is a framing primitive, not JSON allocation preflight or authenticated EOF.
The compatibility wrapper still keeps all decoded values. Native read-only
inspection, incremental cryptographic verifier, complete ScanLimits/cancellation
and production S2/S3 remain incomplete. No128MiB/RSS or e2e qualification.
Prior Merkle proof114310 and full115 f88 evidence do not qualify this new source.
