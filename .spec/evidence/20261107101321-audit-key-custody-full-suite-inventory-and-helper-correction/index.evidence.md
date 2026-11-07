---
id: evidence:root/20261107101321
title: Audit key custody full suite inventory and helper correction
status: active
created: 2026-11-07T10:13:21Z
updated: 2026-11-07T10:13:21Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody full suite inventory and helper correction

## What was measured

Static source inventory before full audit allocation: one library harness,
two zero-test binary harnesses and eight integration targets (checkpoint parity,
control operations, durable witness, backups, custody, Merkle/record/verification
parity). Native Windows review found only owned temp filesystem/ACL/reparse/
sharing-lock operations and owned audit child binaries/self-executed helpers;
no network listener/client, SCM or account mutation code was found in this crate.
This is source prereview, not executable test discovery or an actual run.

Review found library process_lock_probe returned on absent fixture env and was
counted as a no-op pass. Exact historical8132117 source has the same flaw; its
101 raw MSRV passes mean100 accepted case passes after excluding this helper.
Raw logs/facts remain immutable, with an attached correction in evidence92505.

Root authorized narrow source correction: explicit ignored helper, fail without
actual parent env, parent --ignored plus existing bounded private capture60s/
64KiB, exit0/empty stderr and standalone flushed humanymous-wal-lock-refused
marker only after actual WouldBlock assertion. Parent keeps original native WAL
rotation/checkpoint/restart assertions. Capture does not leak nested summaries.

## How to read it

Future audit ignored inventory totals5, not5 successes: library1, witness1,
backups1, custody2. Library helper's required real parent is the exact full name
storage::wal::tests::exclusive_writer_rotation_checkpoint_and_restart. Runner
owner is pairing the exact exception and marker-leak rejection; until qualified,
no strict actor or full suite success is claimed. New source is format-only
checked/NOT_RUN. Memory admission blocked new validation starts, no slot owned.
Prior reviewed pure10 and native44 exclude this helper and remain source-bound.
