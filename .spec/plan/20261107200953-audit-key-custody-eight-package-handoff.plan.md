---
id: plan:root/20261107200953
title: Audit key custody eight package handoff
status: draft
created: 2026-11-07T20:09:53Z
updated: 2026-11-07T20:09:53Z
---

# Audit key custody eight package handoff

## Goal

Hand off current Audit source for Root's accepted eight-public-package baseline
v0, [[sot:root/structural-development]]. Audit maps to humanymous-runtime under
crates/sdk with an audit context; Root chooses the exact module/path transaction.
This packet authorizes no independent move, manifest change, branch/worktree,
push, synchronization, publication or main merge. Detection remains frozen.
The full custody/witness/recovery/authority/product goal remains unfinished.

## Steps

1. Root names and freezes an exact slice before any source move. Recheck the
   attached path hashes and require no intervening owned changes. Audit has not
   transferred or released its source lane merely by preparing this packet.
2. Move/import-rewrite through Root's atomic workspace/lock transaction. Follow
   engine/core first, runtime/platform afterward; no compatibility helper crate
   or independent package topology decision by Audit.
3. Narrow every formerly crate-wide Audit constructor/helper to private or
   audit-context scope BEFORE the runtime/authority sibling modules share a crate.
   Curate public APIs separately; public package status is not public custody.
4. Coordinate Delivery's historical projection and binary-role contracts before
   removing old paths; then qualify a new source-bound post-move candidate.

### Current exact payload and leases

[[evidence:root/20261107201200]] attaches paths.json:109 tracked crates/audit
files plus the existing owning SoT/four plans,114 inputs total, with raw byte
length, SHA256 and canonical Git blob equality. Manifest SHA256:
7cf4bf356bd686bf9e95be02f65c3820d631c1646a9657097943a52e9af9350e.
Snapshot repository HEAD is a65dac2feebdd0666c78707ccb99d0e17b868e3a; latest
Audit source commit is99317a5065e6cb2bd1d456678dc0c23124fb73ed. These are source
identities, not qualification of the current whole repository.

Audit code and prior canon are clean. Only this packet/evidence is newly authored
in this transaction. No owned compiler/process/session handles, target leases,
container, path move or Git transaction remains live. Current Windows process
query for validation-audit/audit-read-only/audit-owner matched zero; this does not
assert foreign work is idle. Prior Windows target and Docker prerequisite window
were explicitly returned. No new Docker, WSL, compiler or structural worktree was
launched. All private raw evidence/source archives stay private and out of Cargo
artifacts; historical public evidence remains byte-preserved.

### API, visibility, unsafe and dependency hazards

- Audit currently exports checkpoint/control/keys/lifecycle/log/merkle/record/
  sink/storage/verification modules. Do not blanket re-export this entire reference
  API through the supported SDK. KeyMaterial raw seed/HMAC getters and mutable,
  history-materializing Log are not a substitute for the sealed production owner.
- storage::ExclusiveFile, storage::PrivateState and its open/commit/promote methods,
  keys::custody::permissions, merkle::frontier and bounded record/framing helpers
  have crate-wide reach today. A merge broadens that reach to Runtime/Authority
  unless Root narrows it to the audit context. Review every pub(crate), not only
  these examples. Preserve private fields and sealed native Source implementation.
- Custody/KeyMaterial/NativeFiles stay non-extractable through sealed ownership;
  VerifiedScan consumes the same actual owner and borrows ORIGINAL Control/Limits.
  No sibling module may create a proof from Diagnostic/Report, cloned data, public
  testing features or a caller success flag. ReadOnlyOwner has no Ready/write
  interface; open_native still returns Unsupported. Ancestor cfg(test) and the
  Linux/x86_64 leaf are deliberate: do not promote them via a public testing flag.
- Keep approved unsafe Windows object/handle conversion in platform, not runtime
  or core. No new unsafe is supplied by this handoff. Audit currently consumes
  safe native-security on Windows; that move must not reverse the dependency to
  core or expose raw File/FD identity getters as construction authority.
- Preserve audit-before-effect DurableAudit semantics in Runtime and Authority:
  Ok requires required durability/independent coverage; uncertain write poisons
  and refuses effects. Keep zeroized credential ownership, exact pin binding,
  dual-control spending, vault/backup/erasure and independent witness boundaries.
- Current normal dependency humanymous-signals supplies wire quoting, nullable
  decoding and Signal types. Root's core-first migration must update the actual
  Audit import sites and the one owning implementation, not add a temporary
  signals helper package. Runtime depends on core; core must not acquire Audit's
  native/crypto/I/O dependencies. Preserve Unix Rustix fs/process and OS-specific
  permissions dependency selection and native/WASM separation.
- Current dev dependency humanymous-test-support moves to platform's explicit
  testing capability. That feature is consumer-selectable and grants no native
  proof, trusted lease or readiness. Actual worker cleanup/owned-process semantics
  must survive changes of imports and module boundaries.

### Binary, fixture and Delivery integration hazards

audit-witness and audit-control-retainer are two actual standalone bin targets.
Package consolidation must not collapse their independent process/key/retainer
roles into an in-process closure. Root must name their target/install placement
and feature/allowlist policy. Integration tests bind CARGO_BIN_EXE_audit-witness
and CARGO_BIN_EXE_audit-control-retainer; renaming or moving targets changes those
contracts and requires actual child-process pilots, not just library compilation.
Keep fixture/pilot helper roles distinct from installed product commands.

Parity tests and unit modules include frozen JSON via relative paths; transport
support also uses explicit path attributes. Move fixtures with their owning
context or deliberately update the new package's local includes; do not introduce
cross-published-package path includes or copies of the implementation.

Delivery's new audit_source checker currently binds old Audit file paths and
include_bytes paths. Its historical check uses exactf9b0cda3 + only the declared
99317 test seam:193 baseline inputs,192 unchanged/one wiring modification and
three additions. Raw99317 also contains two later foreign test-support baseline
changes, so it is not that projection. Do not transplant post-move rewritten
Audit source into the old projection or silently rewrite the old baseline,
selectors or hashes. Before Audit paths disappear, Root/Delivery must either bind
historical data/seams to their immutable original Git objects or explicitly
replace the unexecuted component with a NEW post-move source/catalog/baseline.
The new CLI/tooling package cannot keep path includes into another published
package as a permanent compatibility mechanism.

### Required verification after the actual move

Use Rust1.99.1/edition2024, locked scoped tests, all-target check, fmt and strict
Clippy for the new runtime audit context and affected consumers. Discover actual
post-move test names with --list before exact selection; renamed paths/module
prefixes must never yield zero selected tests and a false green. Root owns broad
workspace/facade/collector and feature-role compatibility gates.

Re-run frozen canonical/Unicode/record/checkpoint/Merkle/parity tests and actual
custody/WAL/permissions/backup/erasure/retainer/witness subprocess tests, preserving
ignored helpers and zero-test binaries as separate denominators. Use actual
Linux/Windows/macOS where supported; mocks or Unsupported are not native success.

The current private recovery source still needs an actual precondition gate,
13 exact semantic cases and12 compiler attempts (positive before/after plus ten
reason/span-specific negative probes). The prerequisite is ONE separate fixture
gate, not a fourteenth semantic pass. Four capacity/RSS/FD cases remain unrun.
Root must bind a NEW candidate if paths/visibility/graph change. Full production
S2 recovery/write/repair/Ready, bounded consumer folds, remote sink/independent
witness composition and ServiceHost/product gates remain open. Product e2e is
Docker-only and must use actual source/image/topology; no old proof transfers.

### Files and historical evidence to retain

Read AGENTS.md, .agents/rules/10-rust-conventions.md, structural-development and
rust-migration SoTs, audit-key-custody SoT, the four existing Audit plans and
.agents/lessons/HARD-WON.md. Canonical provider is this Codex Audit chat; no other
provider's private memory is required. Current snapshots/data live in Specfold.

-99317a50: actual closed prerequisite/catalog source ONLY, independently reviewed
  with no confirmed P0/P1; no compile/native success claimed.
-f9b0cda3: actual private owner/empty-history/auth-refusal source ONLY.
-377e3f21 / evidence162841: exact older8cbc Windows six-case/check/Clippy proof.
-e506ab04 / evidence165921: Docker toolchain prerequisite unavailable, zero tests,
  owned container removed and explicit CI handoff; not filesystem qualification.
-9bb51b44 / evidence144415: older3d325 Linux native six-case proof, exact scope.
-a5cd6a85 / evidence155745: older8cbc GNU five pure-transition proof only.

Earlier canonical/parity/component evidence remains scoped to its own immutable
source. Raw archives, original failures and private evidence references must not
be rewritten to the new package names or packaged as public runtime artifacts.

## Record

Handoff ready for Root's exact slice freeze; source ownership has not yet moved.
No unrelated code edit or local workload was performed to prepare this packet.

### CORE-SLICE-01 authorized consumer delta

After the pre-core packet commit368a7960, Root explicitly assigned only
crates/audit/src/** consumer imports for CORE-SLICE-01. The Audit lane is narrowed
to source plus own canon/evidence; all manifests/lock and structural moves stay
Root-owned. Four use paths change humanymous_signals to humanymous_core::signals:
checkpoint/mod.rs, checkpoint/signing.rs, checkpoint/signing/tests.rs and
record/mod.rs. There were no matches for the other six assigned core contexts.

Attached core-slice-01.json supplies exact before/after hashes for these four
paths. Each has ONE replacement, and reversing that replacement reconstructs the
pre-core SHA256 exactly. No body, visibility, scoring, manifest, fixture, path,
binary, reference or history change is included. These four after-hashes supersede
only their entries in the original114-input pre-core snapshot; that historical
manifest is deliberately not rewritten. No alias or remaining old Rust core
crate reference was found under Audit src after the edit.

Direct pinned Windows Rust1.99.1 rustfmt --check and diff checks passed. No
compiler, Clippy or tests ran: Root must declare the workspace coherent first.
No target/process lease, ref operation or structural move is held. Existing
immutable hosted/source-specific work is untouched; this source-only import
delta does not inherit any historical qualification.

### RUNTIME-SLICE-02 Audit owner disposition (read-only)

Reviewed Audit src checkpoint f322091cc257bdff344d9a6343c76ddb8885b73f with clean
owned source. The accepted runtime children are gateway/authority/audit/capture/
network/native_security; gateway retains the former runtime application child.
No source/import/manifest/visibility/path change or compiler was performed for
this disposition. Platform lower-cluster qualification must precede this move.
The current graph/manifests remain Root-owned and are not qualified by this review.

| Exact surface | Required post-merge disposition | Legitimate caller / concrete reason |
| --- | --- | --- |
| keys/vault.rs ErasurePermit::issued | pub(in crate::audit), not crate-wide | control/ledger.rs erase/resume_erasure and keys/custody/erasure.rs retained execution/recovery both mint only after their existing checked durable flow; no gateway/authority direct construction exception |
| ErasurePermit::bound_to and Vault::authorizes | pub(in crate::audit::keys) or equivalent narrower keys visibility | vault.rs and sibling keys/custody.rs consume these; private-to-vault would break real custody checks, runtime-wide visibility is unnecessary |
| control/policy.rs Authorization.proposal/digest/administrators | Private fields in policy; retain public read-only accessors | Only Administrators::authorize constructs them in policy.rs. ledger.rs consumes digest()/administrators() getters; no direct-field consumer exception was found |
| Custody dir/owner/_lock/poisoned; ready/read/exists/commit internals | Keep existing private fields/methods | Public acquire and authenticated load/recovery entrypoints retain their validation and lifetime lock; never replace these with a sibling builder/Default/from-data constructor |
| keys/custody::permissions and its directory/create/existing/file/directory_file helpers | pub(in crate::audit) maximum | Used by multiple Audit subcontexts; gateway/authority must not obtain a raw custody file construction path |
| storage::PrivateState and open/open_bounded/read/pending/committed/commit/promote_pending | pub(in crate::audit) maximum | Shared Audit ledger/snapshot/backups persistence, not a new arbitrary sibling state writer |
| storage::ExclusiveFile, acquire and test-only duplicate_description | pub(in crate::audit) maximum; preserve cfg gates | Native/custody/witness storage shares it; testing capability in platform must not become permission to construct an Audit ownership proof |
| RetainedControlState::checked | Preserve control-only parent scope, equivalent pub(in crate::audit::control) | control/ledger.rs constructs after authenticated state/exact pin checks. A returned state is observation, not an effect grant or healthy lease |
| ActionPermit and RetainedActionPermit fields | Keep private to control/operations.rs | Only checked local consumption and exact independent retention construct these; do not add constructors, Clone, Deserialize or public test builders |
| Checkpoint::verify_signature_bounded, record/framing/merkle internal helpers | Audit context or narrower | They are shared Audit algorithms, not sibling proof constructors; preserve one implementation and fixed signing/wire bytes |
| NativeSource/NativeFiles/OpeningOwner/VerifiedScan/ReadOnlyOwner and Source seal | Keep current private nesting plus ancestor cfg(test)/Linux-x86_64 gates | No sibling access exception. Original bounds, actual same-owned files/keys and non-Ready read-only result remain mandatory; public testing features do not grant capabilities |

There is NO required exception that lets gateway, authority, capture, network or
native_security construct ErasurePermit or mutate Authorization approval fields.
Leaving issued or those fields pub(crate) after the merge would create new sibling
construction reach that did not exist across the old packages. Narrow before the
first moved compile. The signature-stage Authorization is not a durable effect
grant; do not overstate field privacy as proof of native effect enforcement.

The public checked storage::permissions WindowsPrivacy/verify_windows_privacy
re-export has a real existing Authority transport caller. Map its implementation
to crate::native_security::storage or an explicitly curated safe public path;
do not solve the import by widening custody::permissions. Keep the approved
Windows object bridge unsafe implementation in platform. No additional helper
package or forwarding-only module is required by Audit.

Old Audit-local crate::control/checkpoint/keys/storage/log/record/verification/
merkle references must become crate::audit::<context> references, preserving
deeper module parents. Old external humanymous_native_security becomes the
accepted native_security context. Already migrated humanymous_core::signals
remains external core. Qualify Authorization names by context: Audit control's
signature result, Authority startup's SignatureCheckedAuthorization and gateway
session Authorization are distinct types, not candidates for root-level aliases.

### Runtime move blockers and required test identities

1. Resolve the two independent service targets audit-witness and
   audit-control-retainer BEFORE moving src/bin: runtime package consolidation
   must preserve separate processes, credential stores, exact external anchors
   and lock lifetimes. Root must state target/install/required-feature placement;
   no in-process replacement or automatic default-install promotion. Update
   CARGO_BIN_EXE_audit-witness and CARGO_BIN_EXE_audit-control-retainer consumers
   atomically with the actual target map.
2. Frozen JSON fixtures records/checkpoints/verification/keys/merkle and UI probe
   files move only with their owning tests or get new package-local includes.
   Preserve bytes. Historical f9 baseline193/99317 seam/probe selectors stay
   immutable; Delivery's old-path source-check/include_bytes consumer needs its
   explicit historical-object strategy or a NEW post-move baseline before those
   paths disappear. A compiler success under an unrelated new tree is not old
   f9 qualification.
3. Add actual merged-context compile checks before declaring the new privacy
   boundary preserved: a gateway/authority sibling cannot call issued, construct
   Authorization fields, construct RetainedControlState/permits, or build native
   proof/Ready from data. Use actual moved types and positive legal Audit issuer
   controls. Expected privacy diagnostics are the invariant for THESE access
   checks; they are not substitutes for the separate lifetime/move/signature
   probes, where privacy-only failures remain invalid evidence.

Concrete existing identities to preserve/discover under the moved context:

- Library control::policy::tests: two_independent_signatures_bind_every_operation_field;
  missing_duplicate_unknown_and_invalid_approvals_fail_closed;
  exact_time_bounds_and_canonical_ids_are_required;
  aliases_and_weak_keys_cannot_form_quorum.
- control_operations integration target:
  exact_subject_scope_erasure_is_durable_and_cannot_authorize_another_subject;
  pending_erasure_rejects_same_subject_permit_from_foreign_scope;
  retained_erasure_rejects_scope_and_recovers_only_with_fresh_exact_approval;
  durable_consumption_rejects_replay_and_whole_file_rollback;
  external_retention_failure_spends_nonce_without_issuing_effect_grant;
  retention_service_real_child_persists_restarts_and_rejects_stale_pin;
  sealed_owner_retains_both_locks_and_repairs_only_missing_external_checkpoint.
- durable_witness integration target:
  typed_client_rejects_forged_or_mismatched_receipts_and_poisoned_cache;
  restart_pins_latest_head_and_refuses_disk_rollback_or_missing_state;
  witness_process_lifetime_and_interrupted_recovery.
- key_custody integration target: competing_process_cannot_load_or_create;
  process_exit_releases_lock_and_rotation_pending_requires_recovery.
- key_backups integration target:
  retained_retirement_refuses_missing_inventory_foreign_authority_and_expired_grant;
  crash_reconcile_preserves_consumed_authorization.

These are required identities, not newly executed tests. Discover actual moved
names/targets before selection and require nonzero exact discovery. Native child
cases need real hosted OS/process tests; helper ignores/zero-test binaries and
the separate prerequisite fixture gate do not count as semantic passes. Retain
the original13-case/12-compiler-attempt obligations and every broader gate above.

### PLATFORM-SLICE-02 authorized consumer delta

Root authorized only ordinary Rust source/test references for this slice. The
Audit work lane was extended by its same owner to src and test Rust files,
excluding fixture JSON, manifests and structural moves. Eight files contain ten
references, all changed from humanymous_test_support::<suffix> to
humanymous_platform::testing::<suffix>. No Windows object bridge reference was
found, so that mapping required no artificial edit.

Exact paths and before/after hashes are attached as platform-slice-02.json in
[[evidence:root/20261107201200]]. Reversing the one qualified-prefix replacement
per occurrence reproduces every original file SHA256. This covers storage/wal.rs
unit code and integration tests control_operations, durable_witness, key_backups,
key_custody, record_parity, verification_parity and support/child_lifetime.
No behavior/authority/unsafe/visibility edit, feature/dependency-category change
by Audit, old-package alias, forwarding shim, manifest/lock edit or path/ref move
was introduced. Root retains all graph and frozen platform-source ownership.

The entire tracked fixture directory remains unchanged, including the immutable
f9 baseline193 and source-commitment/catalog JSON. Existing evidence and previous
hash snapshots were not rewritten; the eight new after-hashes are an additive
consumer delta, not a claim that old f9 executed against the current tree.
Diff checks and residual old-reference search passed. No compiler, unit test,
Clippy, fmt or CI ran; even scoped Cargo fmt waits for Root to declare manifests
coherent. No process/target lease is held. Revalidate actual helper APIs, target
roles and dev-only testing features once Root releases the coherent candidate.

### Final Audit contribution to six-context runtime freeze

Root fixture canon662cf7a2 is accepted without source changes. The attached
runtime-slice-02-fixtures.json binds the current Audit source checkpoint8d362f49,
seven data files with exact bytes/SHA256/Git blob/content commit, eight integration
target proposals and two independent bin-role aliases. Audit is ready to supply
this disposition to the six-context freeze; it does not approve other owners'
source or bypass Root's selected Runtime test/403-diagnostic preconditions.

All seven Audit-owned files move, without data replication, to package-local
tests/fixtures/audit/{checkpoints,keys,merkle,records,verification,
native-owner-core-f9b0cda3,native-owner-validation}.json. Their bytes remain exact,
including historical f9 source path/hash strings. No Audit data replica exception
is requested. Root's ONLY data replicas remain core human-chrome-allow.json and
collector pilot/fixture.json, with producer authority, exact original commit/path/
Git blob/copied-byte hash/license, equality check and coordinated parity updates.
No source copy, public fixture API, helper package or cross-package include.

The eight explicit integration target proposals are audit-checkpoint-parity,
audit-control-operations, audit-durable-witness, audit-key-backups,
audit-key-custody, audit-merkle-parity, audit-record-parity and
audit-verification-parity, rooted in tests/audit/<existing filename>. Root freezes
these exact aliases before Cargo edits. Move support/child_lifetime.rs with that
context; preserve or rewrite its path attribute deliberately. Relative JSON
includes and the CARGO_MANIFEST_DIR/tests/fixtures/records.json anchor must resolve
to tests/fixtures/audit inside the extracted runtime package. Package inventory
and clean extracted selected-test compilation are separate mandatory checks.

Propose preserving audit-witness and audit-control-retainer names and their
CARGO_BIN_EXE strings, with explicit paths to src/audit/bin/<name>.rs if Root
keeps these targets in runtime. Each bin is a separate Rust crate and needs the
existing checked service entrypoint available under the curated public Audit
library context; do not widen private permit/custody constructors to make it
compile. Root still must assign target/install/required-feature/native-role
policy. A shared package ID never grants either role the other's credentials,
retainer/witness authority or sibling native rights. Unsupported/no-op target
profiles are not successful native pilots.

The RUNTIME-SLICE-02 visibility table above is unchanged: ErasurePermit::issued
Audit-only, bound_to/Vault::authorizes keys-only, Authorization fields private,
raw custody/state/lock helpers Audit-only, checked retained-state construction
control-only and actual owner/proof nesting private. No gateway/authority issuer
exception is needed. Actual sibling forgery-negative checks remain required.

Remaining source edits are ONLY future Root-frozen work: owning-context crate
paths/internal native_security imports, visibility narrowing, fixture/path
attributes, explicit test/bin mappings and Delivery historical-object/new-baseline
consumer handling. No such edits, Cargo changes, moves, compilers or CI were
performed for this final disposition. Current source/tests are clean at8d362f49;
no owned live handles, target leases or path-moving operations. Root alone decides
freeze after its selected test and403 diagnostic settle, then rechecks exact hashes.

### Root relocation source freeze and writer release

At Root's explicit Runtime6-to1 preparation request, Audit stops all source/test
writes at8d362f497c43cef5c5b9e5694011cd5b0300cba9. Attached
runtime-slice-02-source-freeze.json records106 actual src/test files, full
inventory equality to tracked paths and per-file byte/SHA256/canonical Git blob
matches. No source dirt or owned run/target/path-move lease exists. Common
Cargo.toml remains Root-owned and is excluded from this source freeze.

The earlier109-file package inventory also includes KEYS-INTEGRATION.md and
LICENSE-Go.txt. Their hashes are included as ancillary preservation obligations:
Root must carry operational notes and upstream attribution with the Audit
context/package, not silently omit them during directory consolidation.

After this packet commit Audit releases src/test coverage and retains only its
own Specfold coordination paths. Root alone may perform the relocation after its
remaining writer/test/403 gates settle. The exact constructor/field disposition,
standalone role constraints, immutable fixture/projection boundaries and new
source-bound validation obligations above remain part of the atomic move.
Audit requires a new explicit path assignment before writing relocated source.
