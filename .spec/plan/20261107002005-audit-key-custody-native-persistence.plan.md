---
id: plan:root/20261107002005
title: Audit key custody native persistence
status: active
created: 2026-11-07T00:20:05Z
updated: 2026-11-07T03:39:00Z
rel:
  governs: [crates/audit/**]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody native persistence

## Goal

Implement the accepted native sealed-custody boundary without changing scoring,
wire envelope, KDF or shared Cargo ownership. The original scope excluded the
issuer; the 2026-11-07 follow-up explicitly authorizes operational dual control,
independent durable witness, rotation, backup retirement and local sinks.

## Steps

1. Record contracts and critique before implementation.
2. Implement safe native permission validation and lifetime exclusive custody.
3. Implement bounded load/create, pending replacement/recovery and sealed vault
   snapshot persistence with honest OS durability reporting.
4. Run real-source native tests and subprocess contention/fault cases; request
   root Cargo integration, then record exact checks and remaining deployment gates.

## Record

2026-11-07: source complete and root-owned Cargo dependencies integrated. Actual
workspace locked audit suite passed on Windows49/Linux47; custody cases10/8,
with two separately invoked subprocess helper entrypoints excluded from totals.
Clippy all-targets with denied warnings passed on both systems. Native tests
cover private ACL/modes, NTFS junction/Unix aliases, restart/snapshot/rotation,
child-process exclusion and interrupted promotion. Windows actual rename-sharing
failure verifies poisoning and authenticated recovery. No power-loss hardware
test or whole-product e2e is claimed. API/dependency details and independent
witness/admin/runtime gates are recorded in crates/audit/KEYS-INTEGRATION.md.
Sanitized source/execution hashes: [[evidence:root/20261107002839]].
Owned documents/bundles pass Specfold. Full check has out-of-lane private
analysis-layout and root evidence-attachment findings; no unrelated paths were
modified to suppress them. Public files are LF with matching Git-filtered bytes.

## Subsequent guard-lifetime repair

Commit d179562 replaces close-only custody/WAL lock ownership with a shared
explicit-release guard. Historical receipts above qualify only their original
source. The reproduced duplicate-description failure, same-inode/successor
regressions, Linux/Windows checks and executed other-UID helper proof are in
evidence:root/20261106235519/custody-lifetime-results.json and
plan:root/20261107031001. Exact patched-commit native CI remains pending.

## Operational continuation and serial handoff

- Implemented two distinct pinned Ed25519 administrator approvals, signed
  action/scope/target/nonce/time binding, durable spent ledger, exact external
  latest-anchor restart, typed noncloneable runtime grant and subject/scope
  erasure permits. Ordinary replay fails; explicit narrow crash reconciliation
  never changes action/target or remints destroyed subject keys.
- Implemented separate sealed-custody audit-witness child process and private
  inherited-pipe protocol, independently durable signed head, external minimum
  and consistency-proof restore. Log has a public external co-signer trait seam,
  receipt signature/binding/durability verification and graceful flush.
- Implemented approved ciphertext-generation-bound passphrase rotation, sealed
  backup inventory/local retirement/orphan reconciliation and fully verified
  bounded local durable sink. Root ExclusiveFile/WAL repair is unchanged.
- Windows all-target audit suite78/Linux75 passed; raw four ignored subprocess
  helper entrypoints and zero-test binary harness are excluded from counts.
  Native children execute real witness service/lock/crash cases. New source
  Clippy passed before concurrent shared-lock delta; final rerun is recorded in
  [[evidence:root/20261107051648]], along with remaining qualification gates.
- Root requests a safe temporary gate-edge handoff to authority for shared
  client/transport, then runtime integration. This checkpoint is not whole M3
  completion: separate OS-account/HSM/anchor storage qualification, runtime
  admin transport/wiring, streaming recovery, remote sinks, physical/unknown-copy
  erasure, power-loss behavior and whole-product Docker e2e remain explicit.

Consumer agreement updates: runtime and Compose jointly requested one audit-owned
WitnessServiceClient codec; implemented connect/from_ready with pinned readiness,
exact receipt/signature/proof checks and error poisoning, preserving consumer
deadline/peer/reap ownership. Current witness12Windows/11Linux native cases pass.
Authority/runtime requested shared sealed startup owner; accepted as follow-up
proposal, not implemented in this checkpoint. Move runtime SealedAudit source
instead of adding a second implementation. Proposed stronger approved-rotation
destination binding must commit the prepared encrypted candidate; current approved
generation rotation intentionally does not sign the caller-selected new secret.
Next audit turn after authority/runtime integration must track these remaining
requirements and deployed anchor/account isolation, not close all M3 as complete.

Final operational checkpoint: exact unchanged working manifest, Windows81 and
Linux77 full actual audit all-target tests passed (final client and exact-permit
tombstone-recovery case included); four raw ignored process helpers and zero
binary harness excluded. Full outputs and failed shared-lock Linux attempt are
preserved separately; root resolved shared lock without audit dependency changes.
Current precise API, commands, files-first and residual handoff are in
crates/audit/KEYS-INTEGRATION.md. Next owner: authority common IPC/client, then
runtime; audit retains the explicitly listed follow-up requirements.
