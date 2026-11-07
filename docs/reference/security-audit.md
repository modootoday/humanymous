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
