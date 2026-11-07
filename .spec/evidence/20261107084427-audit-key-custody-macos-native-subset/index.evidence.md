---
id: evidence:root/20261107084427
title: Audit key custody macOS native subset
status: active
created: 2026-11-07T08:44:27Z
updated: 2026-11-07T08:45:14Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: e1928937a0977612bc86e023e46ed2a389319f46944540fecc6d54ea0d8a2324, bytes: 3633, store: repo}
---

# Audit key custody macOS native subset

## What was measured

Actual source8132117e7bf1445939d7dee9066fcf45ccf0915b, run42355168729,
macos-15/aarch64 native job128351926084. Full locked workspace/all-target build
passed. Actual control_operations discovered16/passed16/fail0/ignored0/filtered0/
exit0/validatedtrue; durable_witness discovered12/passed11/rawignored1/filtered0/
exit0/validatedtrue. The existing ignored-helper contract is unchanged and neither
zero-test audit binary counts as a success. These are the same target tests on
another OS/architecture, not additional unique tests.

## How to read it

The workspace actor failed after15of88harnesses at host_process5discovered/
2passed/3ignored/exit0/validatedfalse. Overall errors1, failed_tests0, incomplete
coverage; later audit-library/custody/backup and whole-product gates remain open.
Required private originals were not recovered after workflow preservation was
skipped. Private evidence82246 retains the exact REST job log and separately
derived structured summary; attached facts include their SHA256 byte hashes.

These source-bound observations do not qualify the later a04603e1 pending erasure
scope correction or any current runtime worktree. Windows was still building
when this summary was retrieved. No rerun, local native execution, publication,
new dispatch or accounting relaxation was performed by this chat.
