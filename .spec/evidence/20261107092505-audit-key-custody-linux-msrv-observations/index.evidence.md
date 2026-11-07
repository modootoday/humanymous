---
id: evidence:root/20261107092505
title: Audit key custody Linux MSRV observations
status: active
created: 2026-11-07T09:25:05Z
updated: 2026-11-07T09:25:05Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody Linux MSRV observations

## What was measured

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
