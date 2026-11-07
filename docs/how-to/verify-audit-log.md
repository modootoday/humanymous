---
layout: doc
title: Plan audit continuity verification
permalink: /how-to/verify-audit-log.html
reviewed: 2026-11-07
---
# Plan audit continuity verification

**Diátaxis quadrant:** How-to. **Audience:** audit reviewers.

Rust audit CLI usage is not release-qualified. Prepare synthetic records with
known canonical bytes, roots and inclusion/consistency expectations. Retain
valid historical vectors as migration comparison evidence.

1. Preserve the original artifact and its byte digest. Use synthetic fixtures or
   a separately controlled copy for recovery experiments; do not edit evidence
   merely to make it parse.
2. Provision the expected writer key, required witness key, owning node and
   external trusted head independently of the history being checked. Reading a
   key or head from that same untrusted history does not create an external anchor.
3. Choose the required checks explicitly. `verification::verify` checks supplied
   records/checkpoints; `verify_with_witness` additionally checks supplied witness
   signatures. Require nonzero checkpoint/witness coverage when the claim needs
   those anchors. An empty HMAC key leaves that layer unchecked.
4. Retain the native verification scope along with the compatibility JSON result.
   `Report.scope` is not serialized into that JSON, and `OK: true` can coexist
   with `hmac-unchecked`. Record exactly which layers passed or were omitted.
5. Exercise failure cases below and retain every failure, recovery change and
   unexecuted case. Name the source, platform, filesystem policy and fixture bytes.

`storage::recover_segment(..., terminal: true, ...)` is a repair operation: it
can truncate an incomplete terminal frame and sync the modified file. It is not
a read-only verification function and does not acquire exclusive directory
ownership itself. Parsing recovered frames alone does not authenticate their
chain. Keep repair, authenticated replay and independent-head verification as
separate steps and report the number of truncated bytes.

For authenticated replay, `verify_history` requires the configured node, a
nonempty HMAC key, trusted writer identity and any required witness identity.
An external trusted head must occur exactly in the verified checkpoint history.
Empty storage is accepted only under explicit first-install permission and
without conflicting checkpoints or an external pin. Do not infer first install
from missing files. Verification protects rollback only through the trusted head;
it cannot prove that an unanchored tail was never deleted.

`plan_witness_recovery` is a read-only planner, requiring a distinct witness
identity and independently retained latest head. Its candidate history may include
a missing local checkpoint, so inspect `needs_checkpoint_append` before claiming
that disk already contains it. `RequireCovered` rejects a complete unwitnessed
tail; `PreserveAmbiguousTail` keeps those authenticated records without claiming
their effects are known or acknowledged. Neither option removes complete records.

`Wal::recover_witness_checkpoint` is the separate mutating operation under the
WAL's lifetime writer lock. It can append the exact authenticated external head;
it does not fabricate or re-sign a checkpoint. Retain the performed-append flag,
actual persistence strength, ambiguous-tail count and prior torn-byte count.
Verification-only recovery has no new persistence receipt. These outcomes do
not authorize replaying application effects or resetting an external anchor.

For a recovery comparison, retain separate results for an already covered
history, a missing local copy of the external head, a complete ambiguous tail,
a stale or conflicting pin, bad HMAC and the wrong witness. The source includes
pure planning fixtures for these distinctions. Reading or passing those fixtures
does not execute filesystem repair, interrupted sync, process restart or an
independent witness deployment; measure those boundaries separately.

When a native test starts a helper process, preserve its bounded output and
observed completion separately from the parent harness. Inherited helper output
can contain another test summary; a successful child exit alone does not prove
which assertions ran. Require the declared helper contract, timeout, exit and
reaping evidence before accepting the parent's coverage. Ignored helper entries
remain visible in raw counts rather than becoming additional passing tests.

A terminal JSON value without its newline is uncommitted even if its JSON is
valid. Recovery may discard that final incomplete frame; malformed
newline-terminated frames still fail. The configured frame bound includes the
newline and also applies to unterminated data, so enabling tail repair does not
permit an oversized or corrupt frame. Preserve the original bytes before repair.

For reproducible frozen component comparisons, from the repository root run:

```console
cargo test --locked -p humanymous-runtime --no-default-features --features audit --test audit-verification-parity --test audit-checkpoint-parity --test audit-merkle-parity --test audit-record-parity
```

These compare the actual Rust components with retained migration vectors. They
do not run a complete service, witness-process isolation or storage-outage pilot.

Test tampering, malformed proof lengths, truncated storage, restart/replay and
unavailable durable storage. Verify the claimed commitment scope; a valid proof
does not establish that an event was benign or that no event was omitted before
ingestion. Record tool/artifact identity and residual limits.
See [security evidence](../reference/security-audit.md).

## Select the exact proof boundary

The log's inclusion leaves are ASCII record-hash text bytes, not decoded digest
bytes or record JSON. Preserve that byte layer when checking paths. A bare
`InclusionProof::verify` validates a supplied path against a supplied root; it
does not authenticate that root or inspect the record body/HMAC.

`checkpointed_inclusion` exports a proof only for an exact checkpoint in the
log's persisted authenticated chain. `CheckpointInclusion::verify` checks the
writer signature, optional required witness signature, size and Merkle path.
Provision trusted keys independently; selecting no witness checks only the
writer. Recompute and verify the record/chain layers separately when claiming
that a particular record body is committed. An unsigned current tip or a path
at a not-yet-checkpointed prefix is not an independently retained signed anchor.
