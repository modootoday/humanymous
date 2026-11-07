---
id: evidence:root/20261107093804
title: Audit key custody retained intent state source
status: active
created: 2026-11-07T09:38:04Z
updated: 2026-11-07T09:38:04Z
rel:
  governs: [crates/audit/src/control]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody retained intent state source

## What was measured

Authored RetainedControlState and ControlAuthority::retained_action_state.
The read-only factory authenticates actual native ledger bytes, rejects missing
or pending state and matches exact independent scope/public identity/current
anchor before returning a private-constructor action/target-bound view. It
includes a consumed proposal and distinct other consumed targets, preserving
past intent after expiry and preventing changed kill-target protocols from
looking like a healthy reset. Scope::validate reuses the existing identity rule.

One native test was authored for authenticated absence, unapplied retained kill,
later expiry, restart, other targets and actual ledger loss; the existing
retention-failure test also rejects a view while the external anchor is stale.
Lightweight formatting and document checks only; no compile or native execution.

## How to read it

This is neither an effect grant nor a portable inclusion proof or distributed
freshness lease. Runtime owns serialized issuance/startup admission and exact
scope/target checks. Covered kill restores disabled state; retained intent with
no covered audit commit leaves startup fail-closed/unavailable until explicit
reconciliation. No generic replay, automatic grant reissue or rearming exists.
All code is future source after813/c34/de338 and inherits none of their tests.
The benchmark quiet window prevents new validation; no shared graph edit,
runtime process, Docker run, push or workflow dispatch occurred.
