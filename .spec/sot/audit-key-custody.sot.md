---
id: sot:root/audit-key-custody
title: Audit key custody
status: active
created: 2026-11-07T00:20:05Z
updated: 2026-11-07T06:15:14Z
verify: [cargo test -p humanymous-audit --locked]
rel:
  governs: [crates/audit/**]
  depends_on: [sot:root/rust-migration]
---

# Audit key custody

## Rule

### Authorized operational continuation, 2026-11-07

The user now authorizes the previously missing issuer and durable witness.
Two distinct pinned administrator identities AND public keys must strictly sign
one domain-separated proposal binding installation/node, action, target digest,
random operation ID and bounded issued/expiry times. Persist consumption before
issuing a subject-bound erasure capability; replay, clock rollback, scope/action
substitution and storage failure fail closed. No public boolean constructs a
permit. A permit can remove only its exact subject and persistence must reject
every additional unauthorized tombstone. Same-subject retries can complete after
a crash, but cannot authorize a second target or unrelated destructive action.

Independent witness storage uses a distinct seed/owner directory and authenticates
the writer/node. An externally retained signed minimum head and consistency proof
are mandatory on existing-state reopen; signed local files alone do not resist
whole-directory rollback. Never return a witness signature before atomic synced
persistence. The durable control ledger similarly requires its external signed
latest anchor on reopen, records monotonic observer time and never evicts spent
operation IDs to admit new destructive operations. Local-only endpoints and
independent process/account/HSM deployment remain consumer obligations.

Sealed backup files are private owner-controlled encrypted envelopes with exact
digest identities; retirement needs a separate two-admin RetireBackup action.
Unlink/fsync completion never claims physical overwrite of snapshots/SSD media.
Sinks must authenticate writer/witness and durably acknowledge exact commitments;
unsupported external sink types cannot be presented as delivered. Rotation uses
the existing envelope/KDF, preserves writer/vault/tombstones and requires the
approved exact committed ciphertext digest before consuming approval. Native
restart/process/fault pilots qualify component behavior, not whole-product e2e.

The guard, not the last duplicate OS file description, defines the writer lock's
lifetime. A temporary inherited/duplicated description must not retain custody
after its owner guard is dropped. Keep the lock inode and all owner/ACL checks;
do not unlink, retry away WouldBlock, or relax exclusivity. Verify acquisition
is denied while each live guard exists and that closing a stale duplicate cannot
unlock a successor. Use a deterministic duplicate-description regression before
attributing the intermittent remote failure to process-spawn inheritance.

Keep the frozen sealed AES256-GCM/scrypt envelope and existing vault API.
Custody owns a canonical, already provisioned private service directory. Every
component rejects symlinks/reparse points; the supervisor must keep ancestors
and the directory stable against hostile same-owner replacement. Unix requires
current effective UID ownership, 0700 directory and 0600 regular single-link
files. Windows requires configured owner SID and only simple owner allow ACEs;
directories must propagate owner-only file and directory inheritance. Unknown,
NULL or foreign grants fail closed. No automatic permission widening/repair.

Hold a nonblocking OS exclusive lock on a persistent lock inode for the custody
object lifetime. Cooperating processes cannot load stale state or generate two
identities. Check/open via no-follow native flags and verify handle metadata and
ACL. Locks are local-filesystem/cooperating-writer guarantees, not distributed
locking or protection against privileged/same-owner attackers.

Creation occurs only when both committed and pending files are absent. Pending
always means explicit recovery required. Wrong passwords, corruption, partial
files, permissions and I/O errors never call the creation callback. Writes use
exclusive pending creation, complete encrypted write, file sync, same-directory
rename and Unix directory sync. A failed commit poisons the object until reopen.
Errors after rename mean uncertain durability; never acknowledge mutation.
Windows reports file-synced namespace-unconfirmed rather than promising power
loss durable replacement: std exposes no equivalent verified directory flush.

Explicit pending promotion authenticates it and (when a committed file exists)
authenticates the old file and requires identical writer/HMAC/witness identity.
Promotion is an operator recovery action; no automatic timestamp/size guessing.
Recovery also preserves existing subject keys and tombstones; pending erasure
requires the exact consumed scope/subject permit via recover_erasure_pending.
General vault
persistence cannot introduce new tombstones, remove existing keys or replace key
bytes. A separate persist_erasure requires the same issuer-only production
ErasurePermit as shred; no boolean enables it at the public boundary. Legacy
unscoped vaults cannot apply production permits; new_scoped/load_scoped require
the independently provisioned installation/node. Scope and stretch mode are
external policy and do not change the historical encrypted snapshot envelope.
Runtime
must serialize snapshot persistence against responses to concurrent mutations.
Vault snapshots persist only inside the encrypted envelope. Rotation retains
identity and tombstones; old passwords cease opening the current envelope after
successful replacement, but old backups and storage snapshots remain residuals.
Only the latest committed snapshot is authoritative. External pinned witness,
control and backup anchors detect rollback only to their retained boundary;
they cannot defend if an attacker also rolls back independent anchor storage.
Local backup retirement implements unlink and directory sync, never physical
media overwrite or removal of unknown copies. Control last-time persists with
successful consumption and in-process observation poisons on rollback; rejecting
a request does not durably advance wall-clock highwater. Restarts therefore also
require an independent trustworthy clock, not a caller-supplied timestamp.

## Critique reflected

### Operational schema and consumer agreement, version 0.1

| Contract | Fields and lifecycle | State / reason |
|---|---|---|
| Scope | installation,node: exact nonempty UTF-8 <=512bytes, trim-stable, no control chars | Implemented; runtime/authority use pinned scope, never forwarded headers |
| Proposal | scope,action,target lowercase64hex,operation_id lowercase64hex/nonzero256bits,issued_at/expires_at trusted Unix seconds | Implemented/tested; expires_at is exclusive, TTL1..86400seconds bounded by provisioned max |
| Approval | administrator pinned bounded ID,signature canonical lowercase128hex Ed25519 | Implemented/tested; >=2 distinct IDs AND nonweak keys, every extra approval validated; unknown/duplicate rejected |
| Action | erase_subject,rotate_passphrase,retire_backup,kill_switch,permanent_ban | Implemented; signature code1..5 in this order; grants never authorize a different variant |
| Authorization | immutable verified proposal,digest,sorted administrators | Pure policy only; not executable before durable issuer consumption |
| LedgerAnchor | revision:u64,digest SHA256 of canonical Body JSON,signature over control-ledger/v1 domain+revision+digest | Implemented/tested; exact latest external pin, never local timestamp freshness |
| ControlReceipt | anchor,operation_id,durability | Consumption evidence only, not action completion; self-constructed receipts cannot reconcile effects |
| BackupAnchor/Entry | signed revision/digest; entries bind exact ciphertext digest/length and Active/Retiring(proposal)/Retired(operation_id) | Implemented/tested; no tombstone eviction, hash/permission/reappearing-copy refusal |
| WitnessReceipt | complete frozen Checkpoint,durability | Implemented/tested; no receipt until file persistence succeeds |
| SinkReceipt | complete final Checkpoint,exact export-byte digest,durability | Implemented/tested; final tree_size must cover every record; reuses existing verification |

Approval signing bytes are `humanymous/admin-approval/v1` plus NUL, then u64
big-endian length-prefixed installation/node UTF-8, action byte, decoded target32,
decoded operation32, issued_at/expires_at big-endian u64. Target commitments use
the distinct admin-target/v1 domain plus NUL and u64 length-prefixed target bytes.
Proposal digest and pinned policy fingerprint use distinct domains. Existing
sealed AES/scrypt, Record and Checkpoint canonical bytes are unchanged. New
internal schemas reject unknown fields; mixed-build wire/version negotiation
and stable third-party API compatibility are not implemented.

The private witness wire has newline UTF-8 JSON frames <=128KiB including newline.
First frame: WitnessServiceConfig{key_directory,state_directory,owner_sid,
passphrase private bytes,writer_public,node_id,bootstrap}. Bootstrap is tagged
mode=create_new or pinned{minimum,consistency_proof}; no loss-based default.
Commands are operation=state,counter_sign{checkpoint,consistency_proof},shutdown.
Replies are status=ready{witness_public,checkpoint,persistence_capability},
state{checkpoint},signed{checkpoint,persistence},refused{reason},stopped.
Refusal categories are invalid_command,rejected_checkpoint,unavailable.
Runtime and lab requested and accepted one audit-owned WitnessServiceClient
codec/CheckpointWitness implementation; consumer transport provenance/deadlines/
peer identity/child reap remain supervisor owned. This avoids separate runtime
and lab copies of framing/signature/proof logic. Client source/test completion
and source-bound results are tracked in the owning plan/evidence.

Runtime requested and received Log::flush (checkpoint then WAL flush, poison on
error), checkpoints and external co-signer injection. Runtime operational state
may replay authenticated audit events; a streaming visitor alone cannot remove
the existing whole-history/Merkle memory premise. Full bounded streaming recovery
is deferred explicitly. Runtime/authority additionally requested shared startup
Custody+Log lifetime ownership: proposed for a later audit turn by moving the
existing runtime SealedAudit implementation, not duplicating it. No current
audit-owned shared startup wrapper is claimed.

Policy errors are ControlError::{Configuration,Malformed,WrongScope,TimeWindow,
UnknownAdministrator,Quorum,DuplicateAdministrator,InvalidSignature}. Native
ledger/custody/backup/sink failures return bounded static io errors, retain pending
evidence and poison failed writers; no keys/credentials/payloads are echoed.
Recovery is explicit and authenticates the exact consumed operation or external
head. No admin network route or remote trusted identity is inferred by this crate.

Native writer ownership is implemented once by a crate-private ExclusiveFile in
storage and consumed by both custody and WAL. It owns an already validated File,
acquires the standard-library nonblocking exclusive lock and attempts explicit
unlock on Drop before the File closes. An OS unlock error is not hidden by retry,
unlink or a fabricated success receipt; File close remains the final fallback
and reacquisition may fail closed. The lock filename/inode must remain unchanged.
There is no public clone/unlock API. A cfg(test)-only duplicate-description hook
enables deterministic guard-lifetime regressions for both consumers. Remove the
now redundant fs4 dependency rather than keeping parallel native lock mechanisms.

The standard File contract confirms that duplicate/inherited descriptions can
extend a lock until explicit unlock or all descriptions close. This motivates
the ownership fix; it does not independently identify which concurrent spawn
occurred in the earlier CI process. [Rust File locking contract](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).

Blue: reuse seal/open, never fork crypto. Red: stale snapshots under separate
locks lose tombstones; lifetime lock and serialized replacement are required.
SRE: rename success followed by flush failure has unknown outcome; poison and
reopen, preserving pending when present. DPO: encrypted backup residuals preclude
claims of durable global erasure. Evaluator: native subprocess/restart/fault tests
do not replace Docker product e2e or independent witness/admin qualification.

## Why

## Verification

### Native helper accounting contract, version 0.1

Self-executed libtest children never inherit parent stdout/stderr. Parents use
the existing bounded test-support capture API (60seconds,64KiB), require OS
exit0, empty stderr and a static completion marker emitted/flushed only after
actual child assertions/state effects. Parent checks authenticated storage and
recovery separately; exit0 or a marker alone is not success. Child transcripts
remain captured inside the parent, so each outer harness emits exactly one
libtest summary. Unknown ignored tests or extra summaries still fail accounting.

| Native audit test target | Exact ignored entrypoint | Required real parent |
|---|---|---|
| durable_witness | child_witness_probe | witness_process_lifetime_and_interrupted_recovery |
| key_backups | child_retire_without_pin_acknowledgement | crash_reconcile_preserves_consumed_authorization |

Each target has exactly one raw ignored helper, excluded from pass counts only
when the exact parent actually passes, exit0/filtered0/fail0 hold and discovered
equals passed+1. Existing key_custody's two separately documented helpers retain
their existing narrow contract; total raw ignored remains4, never four successes.
The Windows runner owner accepted these exact names and conditions in16c9e89;
its pre-correction nested-summary failures remain evidence, not retroactive green.

Native tests exercise restart identity, vault/tombstone persistence, rotation,
parallel-process lock refusal, permission/alias rejection and interrupted pending
recovery. Actual workspace Cargo integration remains root-owned. Raw execution
logs remain private and only sanitized facts/hashes enter public Specfold records.
