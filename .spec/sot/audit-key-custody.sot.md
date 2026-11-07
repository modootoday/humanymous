---
id: sot:root/audit-key-custody
title: Audit key custody
status: active
created: 2026-11-07T00:20:05Z
updated: 2026-11-07T07:40:50Z
verify: [cargo test -p humanymous-audit --locked]
rel:
  governs: [crates/audit/**]
  depends_on: [sot:root/rust-migration]
---

# Audit key custody

## Rule

### Product-owned sealed startup and witnessed recovery delta

Audit owns the neutral SealedLog lifetime context: existing Custody plus Log,
preprovisioned private key/WAL paths and no runtime/authority DTO dependency.
Options bind owner,node,cadence,allow_new_history,external trusted checkpoint and
required external-witness policy. Missing required external witness is rejected
before filesystem opening or consuming the caller's zeroized unlock credential.
Startup never creates keys, never substitutes inline identity and never releases
the custody guard while the writer is usable. Runtime preserves its current
public wrapper API while adopting the shared owner; authority implements its
own local DurableAudit trait for this audit-owned type, not vice versa.

Witness recovery separates authenticated durable_sequence,witnessed_through,
authenticated_external_head,local_latest_checkpoint,ambiguous_tail_len,
fully_witnessed and explicit HMAC/writer/witness verification scope. An external
signed trusted head and distinct pinned writer/witness keys are mandatory.
Existing verify_history/canonical/crypto/Merkle algorithms remain the authority.
RequireCovered rejects complete unwitnessed tail; explicit PreserveAmbiguousTail
retains it without truncation, old-request retry, fresh keys or fabricated origin
effects. Root runtime supplies its fresh credential epoch and independent action
identity; a later distinct append may consistency-extend the retained history.

When the witness persisted a head but local checkpoint append failed, a pure
RecoveryPlan authenticates the candidate checkpoint against actual records and
the existing checkpoint chain. An explicit exclusive-WAL recovery method appends
only that missing checkpoint and syncs before reporting persistence. Old/forked
heads, missing required signatures/HMAC and unsupported gaps fail closed. Normal
sealed startup does not silently apply a required checkpoint repair. Complete
ambiguous records are never removed; existing torn-frame byte recovery accounting
remains separate. Namespace power-loss uncertainty is not hidden on Windows.

Runtime accepted neutral append/flush owner APIs and rejection-before-consumption.
Product-lab owner accepted the coverage fields and no-fallback/no-retry recovery
contract. These are accepted contracts awaiting source/hosted qualification;
no new local runtime qualification is implied by implementation.

Implemented source API0.2 (pure-unit checked, hosted boundary not_run):

| Type/API | Fields or accepted behavior |
|---|---|
| SealedLogOptions | key_directory,wal_directory,owner,node_id,checkpoint_every,allow_new_history,trusted_head,require_external_witness; trusted native options, not a wire DTO |
| UnlockCredential | Utf8(&mut Zeroizing<String>) or Bytes(&mut Zeroizing<Vec<u8>>); borrow before preflight, consume only at unseal; no new OS reader |
| PinnedWitness | new(Box<dyn CheckpointWitness>,expected_public,durability),durability(); no seed/transport extraction |
| SealedLog | open; explicit recover_pinned; append/flush/records/checkpoints/sequence; recovery_coverage/recovery_receipt; custody guard not extractable |
| SealedLogError | stage() and bounded io kind(); no sensitive raw source chain |
| TailRecovery | RequireCovered or explicit PreserveAmbiguousTail; never an old-effect retry authorization |
| WitnessRecoveryPlan | coverage(),history(),needs_checkpoint_append(); candidate verification does not itself persist anything |
| WitnessCoverage | durable_sequence,witnessed_through,authenticated_external_head,local_latest_checkpoint(before repair),ambiguous_tail_len,fully_witnessed,authentication(HMAC/writer/witness checked) |
| Wal::recover_witness_checkpoint | mandatory provisioned witness key/trusted head, existing verification; returns AppliedWitnessRecovery only after any missing checkpoint append+sync |
| AppliedWitnessRecovery | coverage,checkpoint_appended,persistence Option(native strength only when appended),recovered_torn_bytes; no effect acknowledgement |

Normal shared-owner startup rejects a complete unwitnessed tail. Explicit
recover_pinned validates the external client's exact head before local repair or
preservation, then reopens through ordinary Log authentication. Cold empty history
is explicit allow_new_history with existing keys; it has no claimed witnessed
coverage yet. HMAC32..256bytes matches the existing runtime service contract;
legacy sealed envelope import remains unchanged. Prototype errors are typed
WitnessRequired/Options/KeyDirectory/WalDirectory/Custody/Unlock/KeyMaterial/Wal/
Recovery/Log stages. Authority-local trait adoption and runtime wrapper migration
remain consumer work until the source checkpoint is accepted.

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
is deferred explicitly. Audit now owns the shared SealedLog startup/lifetime
wrapper described by API0.2 above. Current runtime/startup.rs and
authority/startup.rs consume it; authority/decision.rs implements its local
DurableAudit trait for SealedLog. These are source adoption facts, not hosted
qualification. The later TrustedSnapshot storage delta requires its own
source-bound validation and cannot inherit older startup test results.

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

### Independent control-anchor retention contract

Consumed, externally retained kill intent is distinct from a witnessed runtime
effect. A crash after grant issuance but before the kill audit commit must not
restore a healthy default. Startup authenticates its actual control ledger and
checks exact current independent retained scope/public identity/anchor before
obtaining a typed action/target-bound state view, including authenticated absence.
The view is read-only, not deserializable, not an execution grant and not a
distributed freshness lease. A consumed proposal remains an intent after expiry.
The view also exposes distinct other consumed targets for the queried action.
A kill consumer rejects those as an explicit migration issue even when its new
canonical target has no exact match; protocol changes must not rearm old intent.
Missing ledger/pending/native state, mismatched anchor or unavailable retainer
refuses a view; no raw bool/None default stands in for authenticated absence.

Runtime serializes startup admission with issuance. Covered kill audit restores
disabled state; retained kill intent without covered commit leaves service
fail-closed/unavailable pending explicit reconciliation. It never automatically
replays a generic operation or reissues a consumed grant. Established startup
uses existing keys/history and a fresh pinned broker/witness handshake; client
cached-head equality cannot defeat joint store/pin rollback or deliberate
reprovisioning. Logical scope changes require explicit migration, not rearming.

The retained-action boundary consumes the local signed approval ledger first,
then requests an exact previous-to-next anchor transition from an independently
owned retention service. Only its durable exact acknowledgement issues a
noncloneable retained-action capability. Failure leaves the nonce spent and
issues no capability; it never retries the protected effect. The existing local
ActionPermit remains a distinct lower-level capability.

Retention provisions one authority public key and scope together; that key must
not be reused across scopes because LedgerAnchor signs the ledger digest rather
than a separately recoverable scope field. Verify with the existing anchor
verifier, require exact previous anchor and revision+1, and persist before reply.
Same-revision rewrites, skipped revisions and stale parents fail closed. Restore
requires an independently retained exact latest anchor; a local signed snapshot
alone does not prove freshness. An acknowledgement does not prove the eventual
runtime effect, which still requires audit-before-apply and restart handling.

Native retention reuses PrivateState permissions, lifetime lock, bounded atomic
replacement and explicit durability. A transport implementation is a trusted
boundary responsible for authenticating its independent peer and bounded framing;
neither a trait implementation nor a local store establishes process, volume or
organizational independence. Windows namespace persistence uncertainty must be
reported and cannot satisfy a FileAndDirectory requirement. Hosted file, ACL,
crash, process and product qualification remain required.

### Scoped erasure recovery

Pending erasure recovery additionally requires the caller's trusted provisioned
installation/node Scope. Compare it with the immutable ErasurePermit scope
before reading or promoting pending state; checking only subject commitment is
insufficient because historical sealed snapshots do not encode logical scope.
The trusted service binds this argument to its configuration, never request JSON.
This is explicit recovery authority, not a persisted-payload authentication claim.

### Trusted native snapshot boundary

Runtime operational state uses an audit-owned TrustedSnapshot native primitive,
reusing PrivateState privacy checks, lifetime lock and pending replacement. The
trusted caller provisions absolute directory, owner, namespace and payload byte
limit (1..4MiB). Reads and writes enforce that instance limit, not just a global
allocation bound. read refuses pending; inspect exposes committed/pending bytes
for explicit recovery. replace writes/syncs pending then promotes with native
durability. promote_pending requires exact caller-authenticated committed and
pending byte expectations, compared under the lifetime lock. There is no discard,
reset, inferred freshness or implicit pending promotion.

This is trusted native atomicity, not a signed event, authorization, CAS revision
policy or audit commit. Runtime owns the full typed candidate/committed payload,
domain-separated event digest, authenticated WAL lookup, semantic revision checks,
audit-before-apply and session epoch invalidation. Native pending bytes are a
separate I/O interruption state from a typed operational candidate. Missing or
ambiguous authenticated commit state fails closed. Generic storage must not
invent recovery authorization or reuse frozen record reason/risk for payloads.

### Prepared rotation binding

Prepare rotation under the existing exclusive custody guard: authenticate the
current envelope, seal its unchanged complete material with the destination
credential, then retain only the encrypted candidate and source/candidate
ciphertext commitments. A versioned domain-separated descriptor binds both
digests; administrators sign its normal target commitment, never a plaintext
credential or an unsalted credential hash. The prepared object has no public
constructor, serialization, cloning or Debug output.

Apply requires the exact current source digest, exact prepared target and a
retained-only rotation grant bound to the provisioned scope and trusted time.
Persist the exact prepared encrypted bytes through the existing pending/rename/
sync transaction; do not regenerate ciphertext after approval. The source
identity, HMAC key, witness material and complete vault snapshot remain unchanged.
Unknown output requires explicit authenticated recovery, never automatic grant
reissue or effect replay. Existing generation-only rotate_approved remains a
distinct lower-level primitive and is not destination-bound product authority.

An interrupted prepared rotation is inspected after dropping/reopening custody.
Authenticate both current and pending envelopes and require exact unchanged
writer/HMAC/witness identity and vault bytes. An immutable recovery descriptor
binds the actual source and pending ciphertext digests. Recovery requires a new
explicitly approved and independently retained operation for that exact target;
it does not reissue the failed operation's grant. Immediately before promotion,
recheck both ciphertext digests under the lifetime lock. Changed pending bytes,
different vault state, stale source or wrong credentials fail without promotion.
Promote the exact existing pending bytes using the original sync/rename path,
preserving unknown-outcome poisoning. This is custody-specific recovery, not a
generic runtime effect replay mechanism.

## Verification

### Accepted execution-policy reference

Execution placement, scheduling, free-runner storage/privacy and readiness policy
are canonical only in [[sot:root/program-roadmap]] and [[sot:root/delivery]].
This section records audit's boundary, not a second scheduler or CI policy.

Selected deterministic in-memory administrator-policy, canonical-byte,
verification and Merkle units are eligible for resource-bounded local checks.
The Cargo `--lib` label alone is insufficient: existing library tests also open
WALs, acquire locks and exercise real files. Actual file ACL/mode/reparse/UID,
fsync/rename-sharing, IPC, child lifetime/crash/restart, witness-service/client,
backup retirement and sink-persistence tests require the hosted runtime tier.
They must not be relabeled as pure units to bypass the placement policy.

Audit API/control/WAL changes require downstream runtime/authority impact review;
while the accepted impact classifier is unimplemented, no required gate is
removed by path filtering. Hosted OS labels do not prove actor rights, HSM or
independent organizational witness custody, physical power-loss durability or
human/AT behavior. Each requires its appropriate direct evidence or an explicit
not_run limitation. Earlier source-bound green never qualifies a newer candidate.

Audit emits sanitized counts/status/source and byte hashes for the free branch
profile. Private keys, sealed custody material, private traces and raw evidence
are not public artifacts. If required private originals cannot be recovered
through an approved private channel, record that absence; do not claim recovery.
No new execution or workflow dispatch was performed for this policy-reference
update. Candidate hosted qualification and unresolved runtime/deployment gates
remain not_run pending root's source closure and execution assignment.

### Native helper accounting contract, version 0.2

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
| key_custody | child_lock_probe; child_interrupted_writer | competing_process_cannot_load_or_create; process_exit_releases_lock_and_rotation_pending_requires_recovery |

The witness and backup targets each have one raw ignored helper; key_custody
has exactly two, with both named parents required. A single outer summary,
exit0/filtered0/fail0/measured0 and discovered=passed+rawignored are mandatory.
Total raw ignored remains4, never four successes. key_custody's captured child
markers are exact standalone lines humanymous-key-lock-refused and
humanymous-key-pending-synced, written with leading/trailing newline and flushed
only after actual exclusion or pending-file sync. Parents require those markers
inside bounded capture, exit0, empty stderr and original native state/recovery
assertions. Marker leakage into outer stdout is rejected by the paired runner.
The old8132117 key_custody nested-summary exception is historical and is not
retrofitted to this new source. Runner owner accepted the paired single-summary
contract; both future source sides require fresh qualification.
The earlier witness/backup single-summary contract was accepted in16c9e89;
the key_custody update is a later paired source change. Historical pre-correction
nested-summary failures remain evidence, not retroactive green.

Retained erasure uses Custody::erase_retained with an owned RetainedActionPermit,
trusted configured scope/time and exact subject commitment. Before shredding,
the caller's vault snapshot must equal the authenticated committed sealed vault
and no pending transaction may exist. The existing scoped tombstone validation
and encrypted atomic replacement remain authoritative. The service must serialize
vault access and stop serving after any post-shred persistence error.
Custody::recover_retained_erasure requires a fresh retained subject approval,
authenticates an existing committed generation and uses the existing unchanged-
identity/exact-tombstone pending recovery. It never provisions on committed loss,
reconstructs an old grant or resets a consumed nonce. The caller owns witnessed
audit-before-apply; retention acknowledgement is not proof of effect completion.
This local-generation operation does not establish backup/unknown-copy erasure.

Native tests exercise restart identity, vault/tombstone persistence, rotation,
parallel-process lock refusal, permission/alias rejection and interrupted pending
recovery. Actual workspace Cargo integration remains root-owned. Raw execution
logs remain private and only sanitized facts/hashes enter public Specfold records.
