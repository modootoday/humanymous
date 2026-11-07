---
id: evidence:root/20261107090447
title: Audit key custody Windows native subset
status: active
created: 2026-11-07T09:04:47Z
updated: 2026-11-07T09:05:41Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: ec4a831206f75786397e8fe79d47b7ed2ed5b6dd331839c4c5ae467ac0291f7e, bytes: 3662, store: repo}
---

# Audit key custody Windows native subset

## What was measured

Actual source8132117e7bf1445939d7dee9066fcf45ccf0915b, run42355168729,
windows-2025/x86_64 native job128351926153. Full locked workspace/all-target
build passed. control_operations discovered18/passed18/fail0/ignored0/filtered0/
exit0/validatedtrue; durable_witness discovered13/passed12/rawignored1/filtered0/
exit0/validatedtrue. Windows-specific cases account for the difference from
Ubuntu/macOS16/11. The raw helper and both zero-test audit binaries are excluded
from pass counts. Three OS observations are not added as distinct unique tests.

## How to read it

The workspace actor failed at host_process5discovered/2passed/3ignored/exit0/
validatedfalse after15of89harnesses. errors1, failed_tests0, coverage incomplete;
no full audit-library/custody/backup, complete cross-OS or product qualification.
The workflow skipped accounting preservation and private originals were not
recovered. Exact REST job log and separately derived summary are retained in
private evidence82246; attached facts contain their SHA256 hashes.

This candidate predates the pending-erasure scope correction a04603e1 and layout/
static-evidence checkpoint c34c3e91. Those changes are not qualified by these
native results. Root has since authorized controlled actual local timing tests;
its current benchmark quiet window is respected and no new validation started.
No push, rerun, dispatch, new local runtime or accounting relaxation occurred.
