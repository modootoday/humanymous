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
4. Request an explicitly assigned root local pipeline/isolated target under the
   measured hybrid policy (jobs1/incremental0). Supported ordinary-user native
   files/process/IPC tests may run locally after allocation; protected account/
   service profiles and genuine hosted contracts retain their required venue.
   Docker e2e keeps its mandatory Desktop engine affinity. Record not_run until
   exact-source observation, including remote
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

Full current audit package Windows A complete: source174e5f21, all-targets locked
check/Clippy pass; all-targets tests110 accepted passes/5 rawignored excluded/
two zero-test bins/11 single outer summaries, zero fail/filtered/measured. Exact
helper identities and all real parents reconcile, no markers/nested summaries
leaked.112-file before/after closure zero drift. All handles terminal/A returned.
New retained backup/live corruption/rollback/old-expiry/fresh reconciliation and
post-unlink disk-state fixture actually pass. No real crash/power-loss claim.
Evidence:root/20261107102348, original raw102137. This closes current ordinary-user
Windows audit-package execution, not hosted/protected profiles/strict actor/
product e2e/remote sink or whole goal. Metadata parser attempt1 singular-test
error preserved separately from passing execution; final parse needs no rerun.

Full-audit source prereview found unignored library process_lock_probe returned
without parent env. Exact8132117 source confirms historical101 raw passes include
one no-op; accepted cases100, preserved rawfacts plus accounting correction92505.
Root authorized narrow helper lifetime/accounting fix: explicit ignored helper,
required real fixture env, bounded private capture/marker and original real-parent
WAL assertions. Future ignored5 excluded; runner owner pairing exact contract.
No new compile/native execution while admission memory is above85%; no slot.
Evidence:root/20261107101321. Retained backup source remains separately unqualified.

Retained backup bridge authored after accepted design/security critique:
retire_retained and reconcile_retained require full acknowledgement identity,
actual inventory/control authentication, exact state and fresh distinct recovery
approval. Two native tests cover identity/scope/loss/expiry and Windows actual
unlink-sharing fault plus fresh reconciliation after prior expiry. No ignored
helper added. Formatting/spec only; compile/native execution NOT_RUN, no slot.
Evidence:root/20261107101025. Prior44-pass snapshot does not qualify this source.

Root granted retained-backup source slice after the44-pass source checkpoint.
Design-sot v0.1 is normative in audit-key-custody.sot: additive retained-only
normal retirement and fresh-approval exact Retiring reconciliation; legacy local
entrypoints retain explicit compatibility. Independent native/security review
found no P0 and two P1 clarifications, reflected as full acknowledgement identity
and dual old/fresh-operation audit correlation plus exact inventory anchors.
Actual inventory bytes must also authenticate at current anchor before effects.
Fault windows and external-pin acknowledgement remain explicit unavailable/
pending states. Implementation and new native tests are next; no workload slot
or new-source qualification exists. Root rustfmt failure was separately corrected
in27675561 (two files, formatting only); prior44 proof remains exact-source history.

Latest source c09d6897 qualified on allocated local Windows A: all-targets locked
check and Clippy pass; reviewed10 pure pass; separately authorized complete
control_operations21/durable_witness13/key_custody10 native passes, rawignored3
excluded. Exact single outer summaries/zero failed/filtered/measured, both custody
parents and stalled-startup case pass. Recursive111 files zero drift before and
after, ordinary nonadmin token verified. All handles terminal/A returned. No
SCM/accounts/network/Docker. Evidence:root/20261107095831, private raw95528.
This supersedes source-only observations below only for those exact cases;
backup target/full library/other OS/protected profiles/product e2e remain open.

Live consumed-membership source correction: verify_consumed no longer discards
optional native read bytes or trusts cached membership after file loss. Actual
present bytes authenticate and match exact current anchor before membership;
existing native test adds live-open loss/corruption/rollback/restoration cases.
No compile/native execution yet. Evidence:root/20261107095443.

The measured root hybrid policy now ends the benchmark quiet window. This lane
has requested, but has not received, a new validation slot for latest retained
state/erasure/witness lifetime source. No idle slot is self-allocated. Historical
quiet-window/source-only records below describe their observation times, not a
continuing blanket ban on local native validation. Root's rust-migration SoT is
the execution-policy authority.

Witness fixture timeout source correction follows the docs owner's full review:
direct synchronous service pipe/read/wait paths now retain an absolute60-second
owned-child timer with kill/reap/drop cleanup. Expected rejection cannot accept
timeout. One actual stalled startup regression is authored; no ignored helper
added. No compile/native execution in quiet. Evidence:root/20261107095031.

Retained erasure bridge source: erase_retained requires exact committed vault
snapshot and no pending state; recover_retained_erasure authenticates existing
committed state and requires fresh retained scoped subject approval. Both reuse
existing tombstone/encrypted transaction rules. One native regression is authored
for scope rejection, nonce reuse, actual tombstone and wrong-subject/fresh exact
recovery with unchanged identity. No new Cargo/native execution in benchmark
quiet window. Evidence:root/20261107094725; caller audit-before-apply, product
integration and backup/unknown-copy removal remain open requirements.

R10 producer coordination prioritized a typed retained-intent query before the
planned retained erasure bridge. Source now reauthenticates actual control
ledger bytes and exact independent current anchor before providing a private
action/target-bound RetainedControlState, including authenticated absence and
other consumed targets. Runtime accepted fail-closed pending-intent semantics,
no automatic replay/reissue, serialized startup/issuance and no implicit target-
protocol migration reset. Scope::validate delegates the existing identity rule.
Native tests for intent/expiry/restart/ledger loss and stale external anchor are
authored; compile/execution remain not_run. Formatting/document checks only under
quiet window. Evidence: [[evidence:root/20261107093804]].

Docs review identified unbounded/inherited-stdio key_custody subprocess parents.
Its old nested output was covered by an explicit legacy runner exception, but
not a bounded child lifecycle. Both parents now reuse bounded capture and exact
leading-newline/flushed markers, exit0/stderr0 plus native lock/pending/identity
state assertions. The same two raw ignored helpers remain excluded, not passed.
Runner owner accepted a paired one-outer-summary contract, exact ignored set and
both passing parents, with nested/unknown/leaked-marker rejection. This future
source has no format/compiler/native result under the quiet window; old813
results are not inherited. Evidence: [[evidence:root/20261107091756]].

Root-assigned B validation for a04603e1 passed seven selected pure units,
all-target cargo check and integration --no-run compile/link. First all-target
Clippy rejected the retention test-module/item order; the failure and initial
closed107-file source set were preserved. Existing implementation/test blocks
were reordered without allow suppression or behavior changes, then all-target
Clippy and the final seven pure units passed. Only this declared reorder drifted
the recursive closure; manifests/lock/toolchain/deps matched. B returned promptly.
New foreign-scope native regression remains unexecuted, not an inherited813pass.
Evidence: [[evidence:root/20261107084937]]. No local actual-file/process execution.

After B return, precommit observation found root Cargo.lock had acquired three
Python-adapter dependency edges; code in the validated recursive audit closure
was unchanged. The passed results remain tied to original lock9b96ab7a... and
new worktree lock65b83031... revalidation is not_run. A separate attached delta
preserves both exact hashes. No audit manifest/lock edit, Cargo restart, stage
or foreign rollback occurred; root owns graph closure and next allocation.

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

### Exact hosted OS subsets, not whole-product completion

The same8132117 source completed Windows job128351926153: control_operations18
passed/ignored0 and durable_witness12passed/rawignored1, both validated and exit0.
It stopped at host_process unknownignored3 after15of89harnesses, not full suite
coverage. Separate source/count/hash facts and missing-private-original limits
are in [[evidence:root/20261107090447]]. The later scope correction remains
native-unexecuted despite its local pure/static pass.

The user now explicitly authorizes actual local Windows/WSL build/test/runtime
timing comparisons, superseding the earlier blanket hosted-only placement. A
root-controlled benchmark quiet window currently forbids new validation starts;
only coding/docs continue. Scheduling and the measured policy decision remain
canonical only in the root roadmap/delivery. Audit has no Cargo/application/
container process; its existing read-only hosted-run observer remains separate.

The same fixed8132117 candidate later completed macOS/aarch64 job128351926084.
Its exact actor summary also reports control_operations16pass and
durable_witness11pass/rawignored1, all corresponding rows validated; it stopped
at the same host_process unknownignored3 with15of88coverage. These are separate
OS observations, not54 unique tests. Source/count/raw-hash facts and missing
private-original limitations are in [[evidence:root/20261107084427]]. Neither
platform result qualifies the later a04603e1 scope correction. Windows was still
in progress at the earlier macOS observation; its later subset is recorded above.
No full cross-OS or product result is inferred.

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

### Current completion audit after Windows source qualification

Inspected current source after174e5f21 and complete Windows evidence102348. This supersedes the
historical matrix below only where actual source/evidence proves progress.

| Requirement | Current authoritative evidence | Remaining completion proof |
|---|---|---|
| Shared custody/writer lifetime and permissions | Current audit owner/restart/lock/Windows ACL/reparse/rename tests pass; runtime and authority adopt SealedLog | Full consumer lifecycle and deployment proof |
| Witness-before-secret preflight | Current pure2 pass before I/O/credential consumption | Actual ServiceHost/native secret-reader refusal |
| Independent witness | Current actual service child restart/rollback and absolute lifetime/stalled handshake pass | Product requests and independently provisioned account/storage/mount isolation |
| Witness recovery | Current pure4 and actual missing-head/ambiguous-tail native cases pass | Product crash/outage/no-origin-effect observations |
| Dual control and retained anchors | Current actual signed service/retained intent/expired intent/live ledger loss cases pass | Independent actor rights and destructive runtime admission/effect connection |
| Settings and permanent kill | TrustedSnapshot native case passes; ServiceHost bind_with_operator and settings source exist; KillStatus typed assessment exists | Exact consumer CAS/WAL/restart evidence; kill admission/mutation remains unwired and destructive_controls=false |
| Destination-bound rotation | Current candidate substitution/stale source/interrupted recovery cases pass | Operator/product workflow and remaining OS tuples |
| Erasure | Current retained erasure/fresh exact recovery/foreign-scope refusal and persistent tombstone cases pass | Product/admin connection, backup residual/deployment proof; physical/unknown-copy erasure remains unestablished |
| Backup retirement | Current full backup target10 cases pass, including additive retained-only bridge and fresh exact recovery; legacy local-only compatibility remains explicit | Production/operator dispatch, dual-operation audit correlation and independent final pin acknowledgement; actual crash/protected-profile/remaining OS proof |
| Sink | Current actual verified local sink restart/tamper case passes | Remote delivery/retry/receipt transport and production export connection |
| Recovery resource bounds and signing identity rotation | Current whole-history WAL/Tree source | Streaming authenticated recovery and writer/witness identity transition remain unimplemented; passphrase rotation is distinct |
| Final verification | Complete current Windows audit package110 accepted cases/rawignored5 excluded/two zero bins; all-target check/Clippy;112-file closure zero drift | Strict actor receipt, remaining OS/protected profiles, Docker product e2e and deployment qualification |

Current complete110 is ordinary-user Windows package evidence102348; the44-case
subset below is historical and not a second110/44 aggregate. A was returned.
Resource-only operator settings now have a separate source path; observer
settings remain nonwritable and destructive endpoints remain unavailable. The
older blanket observer-only conclusion below is historical. Root's measured
hybrid policy governs execution venue; current44 is ordinary-user Windows
component evidence, not hosted/full-workspace/product proof. Source is frozen
for the integration candidate. No new workload slot is held.

### Historical completion audit after snapshot introduction

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
