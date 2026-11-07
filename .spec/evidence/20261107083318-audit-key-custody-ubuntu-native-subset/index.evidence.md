---
id: evidence:root/20261107083318
title: Audit key custody Ubuntu native subset
status: active
created: 2026-11-07T08:33:18Z
updated: 2026-11-07T08:33:45Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: afefff1e3d07d9c389af8bb60c95af986bd4a3b4a6cb30c8b9d57b786eec3e3b, bytes: 3721, store: repo}
---

# Audit key custody Ubuntu native subset

## What was measured

Actual candidate8132117e7bf1445939d7dee9066fcf45ccf0915b, GitHub run42355168729,
Ubuntu native job128351926093. Full locked workspace/all-target build passed.
The actor executed15of88harnesses before strict accounting stopped at
host_process(discovered5/passed2/rawignored3/exit0/validatedfalse).

The actual public actor summary records control_operations discovered16/passed16,
failed0/ignored0/filtered0/exit0/validatedtrue. All eleven new ordinary actual
file/process cases in that target were therefore executed, including sealed
lifetime/recovery, retained anchors, native snapshot and prepared rotation
recovery. durable_witness records discovered12/passed11/rawignored1/filtered0/
exit0/validatedtrue under the existing exact helper/parent accounting contract.
Both audit binaries have discovered0/passed0 and never count as test successes.

## How to read it

This proves only those exact candidate/OS/target observations. The workspace job
failed with errors1 and coverage_complete=false; it is not a full audit suite,
cross-OS, authority/runtime integration or product Docker qualification. Later
audit-library, custody and backup harnesses were not reached. Current worktree
changes outside this fixed SHA do not inherit these results.

The workflow skipped its accounting preservation step. Original private harness
transcripts were not recovered. The actual completed-job REST log and a derived
summary with only timestamp/ANSI prefixes removed are retained privately in
evidence:root/20261107082246; attached sanitized facts include both byte hashes.
Do not claim the derived summary reconstructs missing originals. The earlier
startup_failure42354873001 remains separate with0jobs/0tests. No new dispatch,
rerun, push, local native test or gate weakening was performed by this chat.
