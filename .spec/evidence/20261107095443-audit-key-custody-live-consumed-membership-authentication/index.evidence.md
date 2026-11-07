---
id: evidence:root/20261107095443
title: Audit key custody live consumed membership authentication
status: active
created: 2026-11-07T09:54:43Z
updated: 2026-11-07T09:54:43Z
rel:
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody live consumed membership authentication

## Historical scope

Original governing path: `crates/audit/src/control/ledger.rs` in the former
humanymous-audit package. Evidence/source-recording commit:
c09d6897b3ee3435792a74ffc9e2dae681a50ce7. The authored live-state correction,
unexecuted native cases and external-freshness exclusions remain unchanged below.
After Runtime relocation64f74c7a this retained evidence has no active path
governance; it does not qualify the moved Runtime ledger implementation.

## What was measured

Source review found verify_consumed checked PrivateState::read for I/O errors
but discarded its optional bytes and trusted cached spent membership. Actual
committed-file loss could therefore leave legacy resume_erasure eligible for
capability reconciliation in an already-open authority. Authored correction
requires present native bytes, existing full ledger authentication and exact
current authority anchor before querying membership from that authenticated body.

Extended existing native replay/rollback test with live-open rename-away loss,
corrupt bytes, authentic old-generation rollback and exact restoration. Each
bad state must refuse despite cached membership; restoration must authenticate.
No test count or ignored helper changed. Formatting/document checks only.

## How to read it

This is local actual-state authentication, not independent external freshness.
Retained actions still require their independently authenticated retainer path.
Legacy scoped subject reconciliation remains explicit; no generic effect replay
was added. No compile/native execution has qualified this new correction.
The root hybrid policy permits local validation only after assigned slot admission;
this lane owns no slot and has started no Cargo/runtime/Docker process.
