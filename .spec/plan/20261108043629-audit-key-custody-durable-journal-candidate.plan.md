---
id: plan:root/20261108043629
title: Audit key custody durable journal candidate
status: draft
created: 2026-11-08T04:36:29Z
updated: 2026-11-08T04:36:29Z
rel:
  governs: [.spec.local/root/evidence/20261108043600-audit-key-custody-durable-journal/**]
  depends_on: [sot:root/audit-key-custody, plan:root/20261107151758]
---

# Audit key custody durable journal candidate

## Goal

Prepare an exact-preimage-bound Rust candidate for Audit durable journal/genesis
and RecoveryOnly ownership, as requested by Root on 2026-11-08. The actual branch
must remain codex/rust before every write. Root retains source application,
common Cargo/toolchain and all execution allocations. The isolated eight-package
source tools-integration-5f1010d4 is read-only.

Root subsequently confirmed the scope: Audit storage must not
duplicate Native's O supervisor namespace journal or manufacture original
installer, Authority role, native namespace or live monotonic clock authority.
Existing private Audit owner authentication remains read-only. No destruction,
erasure, ban, repair, Ready or production admission is granted by this plan.

## Steps

1. Keep the Root-confirmed existing Audit S2 WAL context and accepted contract.
2. Capture each target's isolated-source preimage SHA256/length (or required
   absence) and author the candidate only in this plan's unique evidence path.
3. Retain actual file ownership across uncertain writes. Require complete genesis
   and bounded contiguous history before exposing inspection. A restarted owner
   is RecoveryOnly; timestamps and valid checksums cannot restore live authority.
4. Author real file persistence, partial-write, failed synchronization and restart
   refusal tests. Distinguish authored tests from terminal executed outcomes.
5. Request a scoped Root execution allocation after the concrete candidate is
   reviewable. Keep original failures and source closure; product qualification
   remains separate from file-component tests.

## Record

2026-11-08T04:36:29Z: actual codex/rust verified. Existing audit-custody lane
renewed for candidate/spec evidence only. storage/mod.rs has identical SHA256
94ba932d45b00ea941ca2fa0d9c5e34cd2c0ce5fad4e536bafecccd21a17d905 in
the main tree and designated isolated baseline. No candidate Rust source or
compiler/runtime execution exists yet. Scope clarification was sent to Root and
Native before large edits.

2026-11-08T04:39:00Z: Native confirms there is no production supervisor journal
factory; its twelve symbolic tests are not persistence or genesis authority.
While the exact journal assignment is pending, authored four narrowly scoped
actual-file regression candidates against existing Audit PrivateState: persisted
roundtrip, partial initial state, partial successor, and complete unconfirmed
pending state across repeated reopen. These exercise the existing fail-closed
pending-state contract without adding a competing journal format or authority
manufacturer. Partial writes are materialized crash-state fixtures, not injected
kernel failures or an actual process-crash run. No sync-failure test or new
RecoveryOnly type is claimed yet.

The unique private evidence directory contains preimages.json and two mirrored
Rust files. private_state.rs changes only by adding a Linux/x86_64 test module;
its baseline SHA256 is e1872b29632d4800fc86733fcdaa003f8b92901c514ab987d373f183d48cf0ef
(5207 bytes). The new crash_tests.rs requires absence. Root must independently
check exact preimages before source application. Four tests authored, zero run;
no compiler, formatter or runtime allocation used. No tracked product source was
edited. All journal/genesis/recovery owner and full product gates remain open.

Root's subsequent explicit decision confirms existing Audit S2 WAL/hash-chain
persistence, retained existing-file owner, bounded recovery, integrity/replay
refusal and RecoveryOnly semantics. A new genesis wire is an explicit dependency
unless already accepted. Original-I and Native O namespace factories remain
separate. The designated isolated baseline is unchanged; sole integrator applies
source after exact-preimage review. The user's Cursor sole-integration decision
does not authorize this Audit chat to write source outside candidate evidence.

The candidate now also extends the existing actual retained NativeFiles/Custody
owner seam, preserving its private Linux/test-only status. The complete two-pass
scanner retains a fixed-size transcript (raw record/checkpoint digests, counts,
byte totals, segment/physical-sidecar presence and exact torn-tail observation).
Finalization compares this transcript in addition to Report/Work and consumes the
same original operation bounds. The result is explicitly RecoveryOnlyOwner;
bounded reinspect may use a fresh read operation but cannot restore an old live
clock, mutate/repair, append/checkpoint, replay effects or grant Ready. No new
genesis wire, namespace factory, dependency, manifest or public API is introduced.

Four new subcases extend the EXISTING controlled native fixture: complete history,
preserved torn tail, duplicate complete record and malformed complete frame.
Each reopens twice with existing identity and verifies byte/identity/membership
snapshots remain unchanged; successful read-only owners retain actual WAL and
custody locks. Replayed/malformed history refuses authentication. Torn-tail
acceptance is read-only and leaves exact partial bytes intact. These are four
subcases of one existing ignored fixture, not four new Rust tests. The positive
and three read-only negative actual-type probe copies name RecoveryOnlyOwner;
the historical immutable probe catalog and its old hashes remain untouched.

preimages.json schema2 now binds twelve candidate Rust files, requiring all
preimages/required absences to be checked before any application. Its SHA256 is
9870d14727988cf6327bfbddd72eeb29c247d082a69146116c6d381eb5986c75
(5466 bytes). Five standalone Rust tests and four native subcases/eight reopened
owners authored; ZERO compiled or executed. No formatter or native workload
started. Root received a bounded execution request. Sync failure injection,
actual process-crash recovery, original installer/genesis provisioning, production
factory, current-source native/type qualification and product gates remain open.

The fifth standalone test runs the actual bounded scanner on two equally sized
encodings with identical Report/Work: valid JSON space versus tab, then two
different equal-length raw torn tails. Both scans must authenticate their
complete history, preserve tail classification and retain different raw record
transcripts. This guards the reason for retaining exact transcript commitments;
Report/Work equality alone is not a byte commitment. Authored only, not executed.
