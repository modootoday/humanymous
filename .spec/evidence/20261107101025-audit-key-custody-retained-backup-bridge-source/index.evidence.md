---
id: evidence:root/20261107101025
title: Audit key custody retained backup bridge source
status: active
created: 2026-11-07T10:10:25Z
updated: 2026-11-07T10:10:25Z
rel:
  governs: [crates/audit/src/keys/backups.rs]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody retained backup bridge source

## What was measured

Root accepted design a2974a44 after independent native/security review and two
P1 clarifications became normative. Additive BackupInventory::retire_retained
and reconcile_retained reuse the existing Retiring/unlink/sync/Retired path.
They require complete retained acknowledgement scope/public-key/current-anchor
matching, signed anchor verification, actual authenticated inventory at current
anchor and actual control-ledger proposal membership. Normal application requires
Active with authentic file. Recovery requires exact prior Retiring proposal,
different fresh retained operation ID and trusted fresh time, preserving old
retirement ID in inventory while returning fresh reconciliation authorization.

Two native tests are authored: normal rejection for missing actual inventory/
ledger, cross-scope/cross-key grant and expired fresh capability, followed by
actual deletion and pinned restart; Windows sharing-denied unlink leaves actual
Retiring, wrong-prior recovery refuses, fresh approval reconciles after prior
expiry with exact old/fresh receipt identity. No ignored helper was added.
Formatting and scoped document checks only; compile/native tests are NOT_RUN.

Before allocation, the same tests were extended with actual live inventory
corruption and signed older-generation rollback while an inventory object is
open. Refusal must preserve both encrypted backup and fault bytes, then exact
restoration leaves Active. Recovery also requires rejection of reauthorizing
the spent prior operation before obtaining a different fresh grant. These are
authored native cases within the same two test functions, not additional passes.

The Windows case additionally covers both Retiring/file-present and an explicit
Retiring/file-already-unlinked fixture. The latter uses actual owned-file unlink,
reopens at the authenticated Retiring pin, refuses restore and requires fresh
exact reconciliation. It models post-unlink disk state; it does not claim an
actual process crash or power-loss observation. Test-function count is unchanged.

## How to read it

Independent implementation review after aa655e7b found no new P0/P1 and judged
the exact gates ready for validation. It confirmed actual inventory/pending/
anchor checks, complete retention identity, both ledger memberships and unchanged
lifecycle reuse. This is read-only source review, not compile or native evidence.

Earlier evidence95831's44 native passes do not qualify this later source.
Legacy retire/reconcile/resume APIs remain local-only compatibility; production
administration must dispatch retained-only methods. Callers must witness intent,
correlate both operation IDs/scope/digest/before-after inventory anchors and
independently retain the final inventory anchor before acknowledging completion.
Actual separate retainer account/storage, post-unlink/final-commit fault windows,
external-pin acknowledgement gaps, remote copies and physical erasure are not
proved or completed by this slice. No new manifest/scoring/remote transport,
generic replay or automatic recovery was added. No validation slot is held.
