---
layout: doc
title: Prepare an erasure pilot
permalink: /runbooks/erasure-crypto-shred.html
reviewed: 2026-11-07
---
# Prepare an erasure pilot

**Diátaxis quadrant:** Runbook. **Audience:** privacy operators and audit reviewers.

The Rust erasure path is not operationally qualified. Use synthetic subjects and
an isolated data set when preparing a test; do not execute against visitor data.

Inventory subject-linked ciphertext, keys, replicas, exports and retention rules.
Define authorized independent approvals and the expected audit-continuity result.
Test cancellation, restart and storage failure around the irreversible boundary.
Verify that the intended subject's data is inaccessible while unrelated subjects
and valid audit proofs remain intact.

Key deletion alone does not establish removal from every backup or export.
Document residual copies and purpose-specific handling. See [privacy review](../start-here/compliance-dpo.md).

## Prepare backup-retirement cases

The current `BackupInventory` API manages exact encrypted local envelope files
and a signed inventory with an independently retained latest anchor. It is a
component API, not a qualified end-to-end erasure workflow. Use synthetic files
and record expected transitions before testing:

| Case | Evidence to inspect |
| --- | --- |
| Create or restore | Exact encrypted digest/length, valid passphrase, active inventory state and private file ownership; wrong credentials must not create a backup |
| Interrupted creation | Explicit orphan listing and authenticated adoption; never silently count an untracked file as inventoried or retired |
| Authorized retirement | Exact scope and backup-digest target, independently approved RetireBackup action consumed before removal, then Active/Retiring/Retired transitions |
| Interrupted retirement | Original consumed proposal, current authenticated control state, external inventory pin and exact-operation reconciliation; no blind retry or unsigned reset |
| Reappearing copy | A retired file that reappears is a failure, not a new active backup; unrelated entries and tombstones remain unchanged |
| Capacity or storage failure | Explicit refusal/poison/recovery outcome, retained files and pending state, reported platform durability and independently retained new anchor |

Inventory signing authority alone is not retirement approval. Restore accepts
only a currently active, hash-verified envelope; retiring/retired entries are
not restore sources. Retired tombstones occupy the bounded inventory rather
than being evicted to admit new entries. A failed creation/retirement call may
have changed a file or persisted a transition, so preserve evidence before
selecting an explicit recovery path.

A retirement receipt concerns the named local file and its reported sync
strength. It does not establish physical overwriting, deletion from snapshots,
remote replicas, exported copies or unknown archives. Orphan discovery is also
limited to recognized local content-addressed files. Record residual copies and
independent retention/custody decisions explicitly; do not mark global erasure
complete from one local unlink or a valid inventory signature.
