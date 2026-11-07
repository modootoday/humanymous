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

## How to read it

Earlier evidence95831's44 native passes do not qualify this later source.
Legacy retire/reconcile/resume APIs remain local-only compatibility; production
administration must dispatch retained-only methods. Callers must witness intent,
correlate both operation IDs/scope/digest/before-after inventory anchors and
independently retain the final inventory anchor before acknowledging completion.
Actual separate retainer account/storage, post-unlink/final-commit fault windows,
external-pin acknowledgement gaps, remote copies and physical erasure are not
proved or completed by this slice. No new manifest/scoring/remote transport,
generic replay or automatic recovery was added. No validation slot is held.
