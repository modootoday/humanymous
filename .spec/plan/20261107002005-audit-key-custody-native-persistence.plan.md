---
id: plan:root/20261107002005
title: Audit key custody native persistence
status: active
created: 2026-11-07T00:20:05Z
updated: 2026-11-07T07:40:50Z
rel:
  governs: [crates/audit/**]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody native persistence

## Goal

An actual active Codex goal was created at the user's explicit request:
sealed custody/vault/durable audit/independent witness/recovery/permissions/dual
control through real product integration, hosted pilots/e2e, strict accounting,
Specfold evidence and owned commits. Prior checkpoints are not completion of
this goal. The chat covers Humanymous audit, key custody and persistence.

Implement the accepted native sealed-custody boundary without changing scoring,
wire envelope, KDF or shared Cargo ownership. The original scope excluded the
issuer; the 2026-11-07 follow-up explicitly authorizes operational dual control,
independent durable witness, rotation, backup retirement and local sinks.

## Steps

Current accepted coding order under root's registered audit-custody lane071ccef:

1. Implement audit-owned neutral SealedLog to replace duplicated composition-root
   custody/writer lifetime logic. Runtime owns adoption and preserves public
   SealedAudit/ExternalWitness APIs; authority owns its local trait implementation.
2. Implement pure typed witnessed RecoveryPlan/coverage and explicit exclusive
   WAL missing-checkpoint repair. Product lab consumes it for head-ahead and
   durable-unwitnessed-tail outage cases, preserving keys/records and zero effects.
3. Complete prepared-ciphertext destination-bound rotation and retained-control
   anchor seam requested by runtime R10. Frozen Record fields are not repurposed
   to carry full LedgerAnchor. Contract/source acceptance precedes execution.
4. Request an explicitly assigned root local pipeline/isolated target for selected
   pure units/scoped checks (jobs1/incremental0 under the current canonical policy);
   real files/process/ACL/crash/witness/product cases run only in root's hosted
   batch. Record not_run until exact-source observation, including remote
   42348518353/63f69ae versus later sources. No duplicate dispatch or publication.

Native-security OS secret reader remains Windows owner responsibility; audit
reuses it when supplied and never adds parallel native secret-reading code.

Source implementation checkpoint: neutral SealedLog/PinnedWitness/zeroized Utf8
or native Bytes credential input, pure witnessed recovery plan and exclusive WAL
checkpoint repair are implemented. Root assigned pipelineA/shared target for
exactly4 pure recovery and2 pure preflight units, sequential jobs1/incremental0/
testthreads1; all6passed, scoped lib Clippy passed. Initial fixture compilation
failed E0596 (Tree cache requires mutable binding), executed0tests and is retained
separately from corrected attempt2. Actual files/process/IPC/crash tests executed0;
these remain hosted not_run. Start resources free34.92GiB/commit81.23%, within root
threshold; slot returned immediately after Clippy. No dependency/feature changes.
Current source-bound evidence: [[evidence:root/20261107073654]].

Next required work remains active: authored hosted tests for shared lifetime,
missing-witness-before-secret consumption, tail preservation and witness-ahead
repair; runtime/authority adoption and product-lab actual observation; prepared
rotation and external retained-control grant. Pure6 do not complete this goal.

Retained-control source checkpoint: independent verifier-only anchor store,
bounded inherited-pipe service/client and retained-only action capability are
implemented under the audit lane. Existing signature verification, scope policy,
private atomic storage and wire framing are reused. Local nonce consumption
precedes independent durable exact acknowledgement; no acknowledgement means
no effect grant. Seven pure tests and seven actual-file tests are authored; the
latter require hosted execution. No generic consumed-operation effect retry is
implemented. Runtime owns actual kill activation, audit commit and restart;
deployment must prove different process/owner/volume and external latest-pin
custody. New compilation/execution remains not_run pending root allocation.
Evidence: [[evidence:root/20261107075025]]. PreparedRotation now binds both source
and encrypted candidate, applies only with retained-only exact scope/action/target
grant and persists exact approved bytes through the existing custody transaction.
Its new pure target test and two hosted file tests remain not_run alongside the
retention source. Existing generation-only APIs are not reclassified as stronger
product authorization. Actual rotation crash/restart and runtime routes remain
required; no generic consumed-action retry is introduced.

Follow-up hosted source adds one ordinary actual-child retention service test,
making eight native cases authored after the seven pure units. Three bounded
child invocations exercise persistence, exact pinned restart and stale-pin
refusal; the shared client verifies actual captured protocol replies. No local
native execution, actor-isolation proof or additional ignored helper is claimed.
Source evidence: [[evidence:root/20261107080220]]. Root assigned A/shared for the
seven pure units, scoped lib Clippy, retainer-binary check and integration-target
--no-run compilation/link. All passed; before/after source/manifest/lock/toolchain
hashes match. Actual native execution0; hosted boundaries remain not_run. Slot
returned immediately. Existing fa327b7 facts remain historical, not rewritten.

1. Record contracts and critique before implementation.
2. Implement safe native permission validation and lifetime exclusive custody.
3. Implement bounded load/create, pending replacement/recovery and sealed vault
   snapshot persistence with honest OS durability reporting.
4. Run real-source native tests and subprocess contention/fault cases; request
   root Cargo integration, then record exact checks and remaining deployment gates.

## Record

New static security finding after8132117: recover_erasure_pending had no trusted
Scope input, and validate_update checked only subject commitment. Normal vault
mutation checked scope, but this lower-level pending recovery could accept a
same-subject permit from another installation if miswired. Source now requires
the configured scope before recovery I/O and reuses the permit predicate in
Vault::authorizes. One normal hosted regression uses valid signatures from both
scopes and verifies unchanged files on refusal plus correct final tombstone.
This is a code-inspection finding, not a claimed reproduced hosted exploit.
New compilation/execution remain not_run, and8132117 results remain historical.
Evidence: [[evidence:root/20261107084148]]. No push or new dispatch was performed.

### Exact Ubuntu hosted subset, not whole-product completion

Actual source8132117e7bf1445939d7dee9066fcf45ccf0915b contains audit20b57d2
unchanged. Run42355168729 native Ubuntujob128351926093 built all locked workspace
targets successfully, then executed control_operations16pass/fail0/ignored0/
filtered0/exit0/validatedtrue and durable_witness11pass/rawignored1/exit0/
validatedtrue. This supplies actual native execution for all eleven newly
authored cases in control_operations, including the latest snapshot and exact
prepared-recovery delta. Audit binary zero-test harnesses are not successes.

The actor stopped at host_process5discovered/2passed/3ignored/exit0 but
validatedfalse; overall15of88harnesses executed, coverage incomplete, errors1.
Full audit-library/custody/backup, macOS/Windows and product gates remain open.
The workflow skipped preservation, so required private originals were not
recovered. Exact completed-job REST log and derived structured summary remain
private; sanitized source/count/hash facts and limitations are recorded in
[[evidence:root/20261107083318]]. Earlier unrun checkpoints remain historical;
this subset does not retroactively qualify a whole candidate or later worktree.

Current recovery source closes the untyped pending-rotation selection gap with
PreparedRotationRecovery. Authenticate both envelopes, require exact identity
and vault preservation, bind exact source/pending ciphertexts, require explicit
retained rotation authority and recheck before existing native promotion. The
failed apply's consumed grant is not reissued; this is no generic runtime effect
retry. Two hosted-only file cases are authored, making eleven new native cases
after the initial owner slice. No actual native case executed locally. Prior
seven-pure/static evidence does not qualify this later delta. Root next-slot
request was updated before any allocation; no Cargo/process is owned.
Source evidence: [[evidence:root/20261107081215]].

### Current completion audit against inspected sources

This is an open-requirement audit, not a reduction of the active product goal.
The current checkout was inspected after7dba59d; old component results remain
source-bound to their historical manifests. No new hosted job was dispatched.

| Requirement | Inspected authoritative source/evidence | Current conclusion and next proof |
|---|---|---|
| Shared sealed custody/writer lifetime | audit/log/sealed.rs; runtime/startup.rs; authority/startup.rs and decision.rs | Shared owner and both consumer adoptions exist; full native lifetime/ACL/restart execution on current source remains unverified |
| Required external witness before secret/state access | SealedLog preflight and runtime startup wrappers; evidence73654 pure preflight | Source/pure evidence exists; actual ServiceHost/private secret-reader refusal needs hosted execution |
| Independent witness in real product host | ServiceHost bind_with_witness and lab/browser_product.rs injection | Consumer source exists; actual isolated process/owner/mount and product requests require hosted lab observation |
| Witness-ahead and complete ambiguous-tail recovery | shared planner/recover_pinned; runtime bind_recovered; authored native cases | Pure recovery checked on older source, native cases compiled before snapshot delta; current crash/outage/no-effect proof remains required |
| Signed dual approval and independent spent-anchor retention | control ledger/retention/service and retained-only capability; evidence80220 | Seven pure checks and static compilation passed before storage delta; actual native service, independent actor rights and runtime destructive handler remain unverified |
| Persistent settings and one-way kill state | management handler currently reports writable=false/destructive_controls=false; TrustedSnapshot source7dba59d | Native primitive implemented but not yet recompiled; actual typed WAL commit/CAS/restart/epoch/runtime mutation pipeline incomplete |
| Destination-bound passphrase rotation | custody/rotation.rs; two authored actual-file cases | Target-binding pure test passed on fa327b7; current native wrong-secret, substituted candidate, stale source and interrupted exact recovery need hosted proof; no automatic grant replay |
| Erasure and vault tombstone durability | custody/vault and subject-bound issuer; historical component evidence51648 | Current operational/admin wiring, authorized crash recovery, backup residual and deployment proof remain incomplete; physical/unknown-copy erasure is not established |
| Backup inventory/retirement/reconciliation | keys/backups.rs and corrected helper/parent contract | Historical native evidence exists; current full-source hosted strict accounting and product/operator integration remain required |
| Sink and export | verified bounded local sink.rs | Local component source exists; remote sink delivery/retry/receipt semantics remain unimplemented and are not silently counted complete |
| Bounded recovery resources and signing identity changes | whole-history Log/WAL/Merkle source and current API | Full streaming authenticated recovery and writer/witness signing-identity rotation remain unimplemented; passphrase rotation does not satisfy either |
| Final checks and delivery | current evidence manifests, runner's narrow rawignored4 contracts, root-owned hosted batch | Scoped pure/static results are insufficient for current cross-OS full suite, Docker e2e, exact source/images, private evidence recovery and product/deployment qualification |

The previous claim that no shared startup owner existed was contradicted by
current consumer source and corrected in the owning SoT. Management mutation
absence is directly visible in current observer-only code; it is not inferred
from missing green output. Root schedules current-source hosted checks; runtime
owns effect/CAS semantics, lab owns real requests/isolation, and audit continues
its owned native storage, cryptographic binding and recovery work.

Runtime accepted the audit-owned trusted native snapshot boundary for R07/R10.
Source exports TrustedSnapshot open/read/inspect/replace/exact promote_pending,
reusing the lifetime lock, native privacy checks, atomic transaction and explicit
durability. Shared PrivateState gained an instance byte limit; existing consumers
keep4MiB. Runtime owns full typed candidate/committed payload and authenticated
WAL commit/revision/epoch/apply semantics. One additional actual-file test is
authored; new source compilation/Clippy/native/product execution remain not_run.
The prior7pure/static check does not qualify this later storage delta.

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

## Strict helper-output correction

Windows runner rejected the later workspace probe despite child exit0 because
durable_witness inherited a second libtest summary. That failure and the
subsequent narrow-contract probe failure remain in the runner owner's private
evidence20261107051539. Authority completed and released its gate-edge checkpoint
e346834 before this source edit; audit claimed only these two test sources/docs.

Corrected both newly added helper parents with existing bounded test-support
capture (60seconds/64KiB), actual child completion markers, exit0/stderr0 and
the original signed-state/recovery checks. Windows witness12/backup8 and Linux11/7
passed; each outer stdout has one summary and excludes child transcripts/markers.
The exact helper/parent/ignored-count contract is canonical in the SoT and
[[evidence:root/20261107061214]], accepted by runner owner in16c9e89.
No runtime/authority/public production API or dependency delta. Next: runner
owner reruns whole workspace against correction commit; audit then releases
gate-edge to runtime. Remaining M3 requirements above are unchanged, not closed
by this test-only correction.

## Accepted validation placement and remaining execution

Root collected all13 owner views and finalized the user-directed execution policy.
The sole policy canon is [[sot:root/program-roadmap]] with implementation contract
[[sot:root/delivery]]; audit consumes those references rather than replicating
workflow lane/parallelism/storage logic. The earlier remote-parallelism opinion
was a proposal, now superseded by that accepted policy. This two-document update
uses root's explicit docs-only misc-lane exception; runtime's gate-edge/P0 source
ownership is unchanged.

Local candidates are resource-bounded deterministic pure policy/canonical/
verification/Merkle units. Actual audit ACL/files/locks/process/IPC/crash/witness/
backup/sink tests belong to hosted runtime qualification, including tests housed
under a library module. No local runtime/build/browser/Docker or broad private
spec scan was started for this update. Existing5528d74 targeted results and strict
helper/parent evidence remain historical source-bound facts, not qualification
of the next integrated candidate.

Still not_run for the next source-closed candidate: hosted full audit/native
cross-OS and downstream runtime/authority qualification, complete product Docker
and deployment/isolation requirements listed above. CI owner supplies hosted DAG;
root fixes candidate/dispatch and gathers sanitized source-bound results; runner/
lab owners retain exact helper and actual process evidence. Unknown/ignored,
missing or unsupported gates stay fail-closed. Required private originals not
retrieved through an approved private channel must be recorded as unavailable.
No paid resource creation, duplicated dispatch, gate removal or production API
change is authorized by this documentation-only placement update.
