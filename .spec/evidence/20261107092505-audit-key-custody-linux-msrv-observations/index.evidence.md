---
id: evidence:root/20261107092505
title: Audit key custody Linux MSRV observations
status: active
created: 2026-11-07T09:25:05Z
updated: 2026-11-07T10:14:26Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 2cbe053de739a814c7712ca6406273fee479e67766f3017dc74a9d8005163b21, bytes: 4894, store: repo}
  - {path: accounting-correction.json, sha256: 46c16568f532cad484417da36f9d6019d473ac27eaf54350789a648b24af68e7, bytes: 738, store: repo}
---

# Audit key custody Linux MSRV observations

## What was measured

Accounting correction: historical raw output reports101 passes, but only100 are
accepted case passes. Exact git source8132117 confirms the ordinary library test
storage::wal::tests::process_lock_probe returns without assertions when its
parent environment is absent. Exclude that one outer no-op pass. Its real parent
already counts as one case; the captured child assertion adds no separate case.
Original facts.json and raw log are preserved unchanged. The attached
accounting-correction.json overrides their accepted-count interpretation.

Successful Linux MSRV job 128359709010 in run 42355168729 executed the
locked workspace all-target test command at exact source
8132117e7bf1445939d7dee9066fcf45ccf0915b. Its audit harnesses recorded
101 passes, zero failures, four raw ignored helpers and two zero-test binaries.
The sanitized facts.json preserves individual harness counts and footer times.

Library 54; checkpoint parity 2; control operations 16; durable witness 11;
key backups 7; key custody 8; Merkle, record and verification parity 1 each.
Ignored helpers are excluded, as is the nested legacy custody child pass.
Exact helper names and their passing parent cases were checked in the original
job log. The library includes filesystem behavior; 54 is not a pure-unit count.

## How to read it

These are source-bound component observations, not a successful native actor or
full product qualification. The aggregate run completed with failure: the strict
native actors stopped at host_process, quality failed, and product jobs failed.
The MSRV output does not replace native actor discovery/hash coverage evidence.
Footer seconds are harness wall times, not CPU/RSS measurements or job durations.

Later scope-recovery, bounded child capture, parser and retained-state changes
inherit none of these passes. No tests were rerun while deriving this evidence.
The private raw log SHA-256 and final version-2 derivation are in facts.json;
the earlier version-1 derivation with null aggregate fields remains private and
non-normative. Source metadata came from the completed job, not a watch summary.
