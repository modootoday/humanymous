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

The authored backup recovery fixture named `crash_reconcile` is narrower than
arbitrary crash testing: its child finishes retirement, writes the control pin,
flushes a success marker and exits normally, while the parent deliberately keeps
the older backup-inventory pin. It exercises explicit reconciliation of that
pin mismatch. It does not inject process death between every write, simulate
power loss or demonstrate independently operated anchor custody. The Windows
pending-retirement fixture separately holds an inventory handle to force a
replacement failure before explicit recovery. These are test-source scenarios;
current source-bound execution results and broader interruption coverage must
be recorded separately.

For a vault pilot, provision the expected installation/node and pseudonym mode
independently before restore. Unscoped compatibility constructors do not grant
production erasure authority. Verify that the permit matches both subject and
scope, reverse links disappear, unrelated subjects remain intact and tombstones
survive restart. Persist the authorized snapshot before acknowledging completion;
an in-memory `shred` return is not durable erasure or historical-backup removal.

For interrupted erasure, `Custody::recover_erasure_pending` requires the trusted
installation/node scope as a separate argument and checks that the permit is
bound to it before recovery reads or promotion. Historical encrypted envelopes
do not supply that logical scope. Obtain it from authenticated service
configuration, never the request, the subject's spelling or the pending file.

Prepare two separately scoped, validly signed approvals for the same synthetic
subject. Verify that a foreign permit or a mismatched configured scope is
refused with both committed and pending ciphertexts unchanged. Then test the
exact-scope recovery, retained tombstone and an unaffected second subject.
The source includes this regression case; it is not evidence that the current
native recovery ran. Preserve both configuration and artifact identities in
the execution report rather than borrowing an older hosted result.

## Separately retained erasure grant source

Working custody source adds `erase_retained` and `recover_retained_erasure`.
The first checks that the in-memory vault snapshot matches the committed sealed
generation and that no pending transaction exists before consuming the scoped
subject grant. Its caller must witness audit intent first, serialize vault access
and stop serving on an error after in-memory shredding. A successful persistence
result covers that sealed generation, not backup retirement or unknown copies.

The retained recovery entry point requires a fresh independently retained grant
for the exact subject and scope; it does not reconstruct or replay the old
consumed capability. It authenticates the committed generation before running
the existing pending-erasure recovery, so a missing committed file cannot become
new provisioning. Record failures before and after grant execution separately:
the caller already holds a consumed grant, and a failed call does not restore
approval or prove that memory and disk are unchanged. These new entry points
still need source-bound native interruption and recovery execution.
