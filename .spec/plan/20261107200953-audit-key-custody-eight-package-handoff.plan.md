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
