---
layout: doc
title: Security evidence and audit status
permalink: /reference/security-audit.html
reviewed: 2026-11-07
---
# Security evidence and audit status

**Diátaxis quadrant:** Reference. **Audience:** security reviewers and adopters.

No completed independent audit or production certification of the Rust replacement
is claimed. Unit checks, dependency advisory scans and deployment pilots answer
different questions and must be reported separately.

## Implemented audit boundaries

| Rust surface | What it establishes and what remains separate |
| --- | --- |
| `record` and `checkpoint` | Canonical event commitments, record HMAC and signed tree heads; producers must pseudonymize data before constructing records |
| `verification::verify` | Checks supplied records and checkpoints, with optional HMAC; inspect the actual verification scope before claiming an external anchor |
| `verification::verify_with_witness` | Checks witness signatures on supplied checkpoints; an empty checkpoint list does not establish a witnessed history |
| `storage::Wal` and `log::Log` | Durable local append and authenticated replay; rollback resistance needs a trusted head retained outside the local WAL |
| External witness client/process | A separately supervised co-signer protocol; a trusted broker still owns process/peer authentication, pipe provenance, deadlines and reaping |
| `sink::DurableSink` | Authenticated bounded local export and a receipt; no remote delivery, WORM storage or automatic personal-data sanitization is implied |
| `keys::backups::BackupInventory` | Signed externally pinned inventory of exact encrypted local files; retirement requires consumed approval and does not erase unknown/remote/snapshot copies |

The compatibility verifier can return `OK: true` with class `hmac-unchecked`
when no HMAC key was supplied. Its native `Report.scope` records checked records,
HMAC coverage, checkpoints and witnessed checkpoints, but is omitted from the
compatibility JSON serialization. Preserve that scope separately in a review
report. A successful result without supplied checkpoints is not proof of an
independently retained anchor.

Audit records and checkpoints have distinct frozen byte rules. Selected record
strings use the record canonicalizer's normalization; checkpoint signing bytes
use a fixed-order JSON representation without NFC normalization. Neither rule
should be applied to request-body bytes or the developing standards profile by
analogy. See [continuity verification](../how-to/verify-audit-log.md).

Filesystem durability is also explicit: successful Unix file-and-directory sync
differs from Windows file sync with rename-namespace power-loss durability
unconfirmed. A configured strict witness-durability requirement must not be
silently downgraded. Component persistence is not independent process isolation
or complete deployment qualification.

A reviewable report names the exact source and artifact, covered interfaces,
threat assumptions, tools/database dates, findings and unresolved conditions.
Unexecuted checks remain pending. A clean advisory scan only means that its
database found no matching known advisory in the scanned dependency set.

Required boundaries include missing evidence, origin bypass, local management,
replay, audit storage failure and destructive-action approval. Report sensitive
findings through [security disclosure](security-disclosure.md).

## Witness process entrypoint

The `audit-witness` binary delegates to the shared private stdin/stdout witness
service on native Unix/Windows builds. It opens no public network listener and
reports service failure with a nonzero exit. Its nonnative stub has no witness
service capability. Binary existence or a zero-test harness does not prove the
service ran, durable signatures were produced or witness custody was independent.
The supervisor still owns pipe provenance, native deadlines, process identity,
key/head isolation and reaping. See [administrative role boundaries](rbac-separation-of-duties.md)
for approval consumption versus actual operational effects.

## Durable witness and private client contract

`DurableWitness` holds a private, lifetime-locked latest-head store and refuses
using the writer's key as the witness key. It validates append-only extension,
commits the signed head and only then returns its persistence receipt. Failure
after witness advancement poisons the wrapper until authenticated recovery.
Existing-state reopen requires an independently retained minimum head and
consistency proof. Missing or pending state does not authorize automatic identity
replacement or silently selecting first-provisioning mode.

The private service exchanges newline-terminated JSON frames bounded to 128 KiB,
including the newline. The first frame contains supervisor-provisioned private
configuration; subsequent State/CounterSign/Shutdown requests carry no seed or
unlock password. Key and latest-head directories must be distinct. The service
loads a preprovisioned witness identity and does not create a replacement key on
failure. Private pipes, authenticated process provenance and native I/O deadlines
are supervisor responsibilities; possession of a public checkpoint is not peer
authentication.

`WitnessServiceClient::connect` performs the private configuration handshake.
`from_ready` accepts brokered handles and independently provisioned public keys,
node and bootstrap without exposing unlock bytes to the runtime. It verifies
exact returned commitments, writer/witness signatures and continuity before
updating its head cache. Refusal, malformed replies or I/O failure poison the
client; reconnect against an independently pinned latest head. `last_size` and
`validate_local_head` inspect that verified cache, not a fresh remote query.

A Ready persistence capability does not mean an absent checkpoint was stored.
Apply the required durability policy to each actual signed receipt; Windows
file sync still does not establish rename-namespace power-loss durability.
A Stopped frame is also distinct from observing process exit, reaping and lock
release. Separate process execution, OS-account/key/head isolation, inaccessible
mounts and independent retained anchors need deployment evidence; shared-host
parent or privileged compromise is not excluded by this protocol.

## Control-anchor retention boundary

`NativeAnchorRetainer` stores the original authority-signed control anchor under
a lifetime lock. Provisioning is explicit at revision zero; reopening requires
the exact independently supplied latest pin. A retained successor must verify
under the provisioned key and advance exactly one revision. Pending recovery
requires the exact committed and pending anchors, scope and authority identity;
opening a directory does not authorize discarding pending state or resetting it.

`audit-control-retainer` delegates to a private-pipe service with State, Retain
and Shutdown commands. Its configuration contains the verification key, scope
and external pin, with no authority signing seed. The verified client rejects
unexpected or stale acknowledgements and poisons the connection after unknown
outcomes. The supervisor owns live peer authentication, I/O deadlines, process
exit/reaping and independent account/storage provisioning. A signed anchor or a
separate process name does not prove those boundaries. Retention is also distinct
from protected-action completion and from the audit-log witness protocol.

The reported persistence strength remains explicit: Windows namespace uncertainty
cannot satisfy `RetentionRequirement::FileAndDirectory`. A receipt authenticates
the exact acknowledgement; it does not independently observe the peer's sync.

## Local export and shared persistence scope

`DurableSink` requires independently provisioned node, HMAC and writer identity,
plus a witness key when that scope requires it. It validates the complete
supplied history and final checkpoint coverage before replacing the export.
Existing-state reopen requires the exact externally retained latest checkpoint;
pending export recovery authenticates both the committed pin and staged history.
Its receipt binds the checkpoint, exact stored JSON digest and actual native
persistence strength. It is not a remote-delivery or object-store acknowledgement.

The shared private-state helper bounds each persisted JSON file to 4 MiB and
uses a lifetime writer lock, private file checks, staged sync/rename and explicit
pending-state recovery. The sink serializes and validates a complete buffered
export; the file bound is not a constant-memory streaming WAL/Tree recovery
contract. The helper itself does not verify signatures or select fresh trusted
anchors: each owning control/witness/sink layer authenticates its state.

A lock acquisition conflict is a real ownership failure, not permission to
unlink the lock inode or bypass exclusive access. The guard attempts an
explicit unlock on drop instead of relying only on closing a file while duplicate
descriptions remain. Failed unlock falls back to closing the owned file; any
remaining kernel lock must still be respected. Platform and supervisor assumptions
require actual regression evidence. An I/O
failure can leave staged or committed bytes and is not proof that nothing changed.
Inputs must already be minimized/pseudonymized; the sink does not redact raw
personal data, provide WORM media or remove previously exported copies.

`TrustedSnapshot` is a separate private native atomicity primitive. Its caller
selects a 1-byte to 4-MiB limit and a trusted namespace; opening creates no
payload or initial operational state. `inspect` returns committed and pending
bytes without authenticating either. `promote_pending` compares exact bytes
already authenticated by the caller, while the caller still owns scope, semantic
revision/CAS, signed WAL correlation and audit-before-apply. A native pending
replacement is not the runtime's semantic candidate state. Failed writes poison
the owner until reopen; there is no automatic reset, discard or effect retry.

## Prepared passphrase rotation

`Custody::prepare_rotation` authenticates the current complete snapshot and
encrypts its replacement before approval. The proposal target commits to both
the source ciphertext digest and exact encrypted candidate; it is not a password
hash. `apply_prepared_rotation` consumes an independently retained grant and
checks that source and pending-state conditions still match before writing
those exact bytes. It does not re-encrypt after approval or replace identities.

Interrupted rotation has a separate inspection/recovery path. Inspection checks
both credentials, unchanged complete key identity and identical vault state.
Recovery requires a fresh explicit retained grant for that exact ciphertext
pair and rechecks the on-disk digests before promotion. An inspection descriptor
is not an execution grant, and a previously consumed grant cannot be reissued.
These library interfaces do not establish an installed rotation command or
cross-platform deployment qualification.

## Event metadata provenance

Record stamping fills a missing event ID from local time and OS entropy, a
missing timestamp from the local clock and a missing event version from the
compatibility default. Existing supplied values are retained. This operation
does not authenticate a remote claimed event time, validate producer identity
or prove ingestion completeness. Preserve the actual producer and observation
boundary before interpreting a sealed event as evidence.

## Sealed writer lifetime and explicit recovery

`SealedLog` owns existing key custody and the WAL/witness writer for one service
lifetime. It does not expose mutable custody or a mutable inner log. Required
external-witness refusal precedes state access and credential consumption;
startup never falls back to generating replacement keys. `allow_new_history`
permits history initialization, not key provisioning. Unlock bytes come from
the trusted composition root, not a public request field.

Normal startup refuses an unwitnessed complete tail. `recover_pinned` is a
separate explicit operation requiring a pinned witness and independently retained
head, with a declared tail policy. Its receipt distinguishes checkpoint repair
from verification only, reports native persistence strength and prior torn bytes,
and exposes witnessed coverage versus ambiguous records. It does not authorize
retrying origin effects. The record-reading API returns a complete buffered
history; this is not constant-memory streaming recovery. Startup errors expose
a bounded stage and I/O category without retaining raw paths or credentials.

## Append, checkpoint and effect ordering

`Log::emit_and_act` calls the application callback only after append and any due
checkpoint succeed. This orders operations; it is not an atomic transaction
with the callback's external state. A record can already be durable when a later
checkpoint fails, so the returned error does not authorize a blind logical retry.
Append/checkpoint/flush failures poison the writer until verified recovery.

A required witness is checked on each emitted checkpoint. The first witnessed
record receives an immediate checkpoint; later emission follows the configured
cadence or explicit checkpoint/flush. Successful append does not imply a new
independent co-signature for every individual record. External receipts must
match the complete expected commitment, signature and required durability, with
no fallback to writer-only success. Graceful-stop handling needs an actual final
checkpoint and WAL flush, plus the consuming service's shutdown evidence.
