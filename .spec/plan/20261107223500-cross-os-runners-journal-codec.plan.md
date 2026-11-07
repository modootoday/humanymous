---
id: plan:root/20261107223500
title: Bounded journal codec and replay DATA schema proposal
status: draft
created: 2026-11-07T22:35:00Z
updated: 2026-11-07T22:35:00Z
rel:
  depends_on: [sot:root/cross-os-runners, plan:root/20261107220000, plan:root/20261107223000]
---

# DATA only, no implementation

Exact proposed codec for Root+X review. No Rust source, actual bytes/file/provider,
serde, new dependency/unsafe, IO/callback or wrapper is authored. Parser outputs
UNVERIFIED claims: neither valid hash/sequence nor a commit frame constructs O,
native File/Job identity, live budget, durable ACK, root, NativeProof or Ready.
The12 pure-model contracts do not implement this format or its durability.

Root-approved profile: process-crash/reopen on admitted local NTFS only; cold O
recovery is INERT, no Instant reconstruction/resume/create/delete/new budget.
Power-loss/directory/rollback assurance remains Unsupported. Installer/genesis/
actual native provider remain required, with separate Platform amendment review.

# Fixed framing and exact integer representation

File capacity1,048,576 bytes =256 slots of4096 bytes, fixed at provisioning.
No growing/rotating/repairing/truncating file during this first profile. Maximum
16 attempt IDs,16 active namespaces,16 reserved/retained tombstones, one append
in progress. All integer fields little-endian; sequence starts1 and uses checked
increment, epoch and journal ID nonzero. Frame digest is existing Runtime SHA256
over bytes0..4064; digest occupies last32 bytes and is not its own input.

| Byte offsets (exclusive end) | Length | Field / exact constraint |
| --- | --- | --- |
|0..8|8|Magic ASCII HMYRJL01 |
|8..10|2|u16 format version1 |
|10..12|2|u16 kind1..9, closed below |
|12..14|2|u16 header length96 |
|14..16|2|u16 flags0 |
|16..24|8|u64 sequence, first1/contiguous/no wrap |
|24..32|8|u64 journal epoch, equals genesis epoch throughout first profile |
|32..36|4|u32 payload length<=3968, exact per-kind parse consumption |
|36..40|4|reserved zero |
|40..72|32|Previous FULL frame digest; zero only at sequence1 |
|72..88|16|Nonzero journal-instance claim, equals genesis instance |
|88..96|8|reserved zero |
|96..4064|3968|Payload then ALL zero padding |
|4064..4096|32|SHA256 claim of preceding4064 bytes |

An all-zero4096-byte slot is UnusedData only, never a valid genesis. First unused
slot begins a suffix that must remain entirely zero; a later nonzero slot is a
hole/corruption and quarantines the whole replay view. Nonzero partial frames or
file length other than exact1MiB refuse. No automatic EOF/truncation repair.
Hash equality detects data corruption/order under the provider premise, not
authentication or replay/rollback resistance against a writer able to replace
the whole prefix. Native custody/immutable bootstrap selector is independent.

# Bounded field primitives

- NativeObjectClaim: volume u64 LE + opaque file128 bytes (24 total), nonzero
  components where required. It is NOT OpenedObjectObservation or an owned File.
- AttemptClaim/journal instance: fixed16 bytes, not all zero; uniqueness is per
  journal instance and retained across terminal tombstones. Syntax is not entropy.
- PrincipalClaim: u16 byte length then binary SID<=68 bytes; revision1, subauthority
  count<=15, exact length8+4*count. It is not current TokenUser/owner-role evidence.
- PathClaim: u16 code-unit count then UTF16LE, no NUL/invalid surrogate. Parent<=256,
  complete child<=320 units; leaf uses fixed lowercase humanymous-private- plus
  exact32 lowercase nonzero hex nonce. Reject UNC/device/ADS/traversal/ambiguous
  trailing dot/space and unrecognized path forms. Grammar is not canonical native
  association; only admitted held handles can supply that association.
- SelectorClaim: stable catalog ID32 + policy digest32. Revision cannot replace
  the stable namespace association or evade an old reservation.
- OriginalPairClaim: clock-domain ID16 + start/run/cleanup u64 values; require
  start<run<cleanup and exact retained equality. This is DATA after decoding,
  never a restored Rust Instant, duration grant or native clock manufacturer.
- Generation/sequence/accounting values are checked, bounded DATA. Unknown enum
  values, extra bytes, duplicated fields or integer overflow refuse, no defaults.

Closed schemas use fixed-order fields and explicit bounded lengths, not generic
TLV or JSON maps. No unknown-field/extension bag or serializer-derived authority.
No secrets, tokens, key bytes, process command lines or raw diagnostics in records.

ProfileCaps is EXACT26 bytes, in this order: file_bytes u32=1048576;
frame_bytes u16=4096; frame_count u16=256; attempt_limit u16=16;
namespace_limit u16=16; tombstone_limit u16=16; pending_limit u16=1;
parent_units u16=256; child_units u16=320; payload_bytes u16=3968;
attempt_frame_credit u16=8; reserved u16=0. Different caps require another version,
not a caller option. ReserveCharge is EXACT14 bytes: frame_credit u16=8,
retained_bytes u32=32768, attempt_slots/namespace_slots/tombstone_slots/pending_ops
four u16 values each1. OriginalPairClaim is EXACT40 bytes; its digest hashes those
40 canonical bytes. NativeObjectClaim is exactly24, SelectorClaim exactly64.

# Kinds and payloads (no optional inference)

|Kind|Name|Exact payload contract |
| --- | --- | --- |
|1|GenesisIntent|Journal24; O-state-parent24; target-parent24; PrincipalClaim; SelectorClaim64; journal-file PathClaim<=320 units; target-parent PathClaim<=256 units; bootstrap grant ID16; ProfileCaps26, exactly in that order. |
|2|GenesisCommit|Intent sequence u64 + intent digest32; must bind immediate GenesisIntent and same epoch/instance. |
|3|ReserveIntent|Attempt16; generation u64; PrincipalClaim; target-parent24; SelectorClaim64; parent/child PathClaims; nonce16; OriginalPairClaim40; namespace-key digest32; ReserveCharge14, exactly in that order. |
|4|ReserveCommit|Intent sequence u64 + digest32; one-to-one preceding valid uncommitted ReserveIntent in this serial profile. |
|5|CreatePending|Attempt16 + exact ReserveCommit sequence u64/digest32; consumes the single local create transition in DATA ordering. |
|6|CreatedHeld|Attempt16 + child NativeObjectClaim24 + descriptor digest32; claims recorded native observation, never recreates held custody on replay. |
|7|Unknown|Attempt16 + closed u16 reason: interruption/unopened/IO-uncertain/observation-failed; no payload error text. Idempotent state, no repeated append loop. |
|8|TerminalIntent|Attempt16 + u16 outcome + exact original parent24 + original-pair digest32. Outcome NoCreateBeforeCall: child24 allzero, allowed ONLY before any CreatePending; outcome KnownEmptyRemoval: nonzero child24 matches prior CreatedHeld and cleanup-observation digest32. Other field combinations refuse. |
|9|TerminalCommit|Intent sequence u64 + digest32; binds sole pending TerminalIntent and original attempt/epoch, publishes namespace release and retained tombstone DATA together. |

Closed reason/outcome u16 enums use1-based values in their listed order. Attempt
generation equals genesis/header epoch in this first profile; changing it cannot
reuse an old Attempt16. Namespace-key digest is SHA256 of target-parent24 then
nonce16 then stable selector catalog ID32; derived leaf must exactly equal the
lowercase hex encoding of that nonce after the fixed prefix. Parent/selector/
PrincipalClaim must exactly match genesis association, without native authority
being inferred from equality. All payloads consume the table's ordered fields.
Offset/size/charge assertions are required in source before a writer. No variable
zero-as-missing branch for Native IDs.
Maximum GenesisIntent payload is1404 bytes and maximum ReserveIntent1440 bytes
under those bounds; exact fixed payload sizes are GenesisCommit/ReserveCommit/
TerminalCommit40, CreatePending56, CreatedHeld72, Unknown18 and TerminalIntent130.
NoCreate's canonical child24/cleanup-digest32 are both zero; KnownEmptyRemoval
requires nonzero matching child and nonzero observation digest. Clock-domain ID
is nonzero DATA; these checks never turn it into a live clock authority.
TerminalIntent always includes the32-byte cleanup-observation digest: zero ONLY
for NoCreateBeforeCall. Other valid fields are fully determined by kind/outcome.

This proposal is stricter than a symbolic NotCreated transition: once actual
CreatePending is persisted, generic directory BridgeError/timeout/caller report
cannot release a reservation as NoCreate. It stays Unknown unless actual same-
held child ownership and exact-empty cleanup are discharged. The parser merely
checks consistency of CLAIMS; O still must independently obtain native facts.

# Replay DATA state and capacity conservation

Accept exactly one genesis intent+commit at slots0/1 before any reservation.
All records bind its epoch/instance/selector/parent. Changed anchor or catalog
cannot start a second journal in the same file. No epoch-rotation record is
supported by this first inert-cold profile; unsupported migration refuses.

Reserve publishes two logical indexes from ONE log commit: immutable attempt
tuple and selected namespace. Commit reference must match exact intent bytes,
not only a caller nonce. Duplicate attempt/namespace/generation or conflicting
alias/selector claims quarantine; a hot identical retransmission reads the
existing record instead of writing a second commit. Fixed one-parent scope means
different actual parent/selector does NOT imply disjointness. Key digest is a
data association checksum, not proof of canonical physical namespace.

Reserve enough terminal capacity BEFORE local grant: proposed credit8 frames per
new attempt. At most7 first-profile events per attempt (reserve intent+commit,
create pending, created held, unknown, terminal intent+commit); credit includes
one extra failure allowance, never an arbitrary retry budget. Genesis consumes2
frames,16 attempts reserve128 more, leaving bounded unassigned capacity. Check
all outstanding credits plus used frames/bytes/slots with checked arithmetic;
terminal writes consume reserved credit rather than letting other attempts take
it. Unknown and tombstone never evicted; credits release only under exact terminal
protocol, and attempt/tombstone records remain permanently bounded in this file.
Precise byte-charge identities must be verified by tests; no threshold guessing.

Intact uncommitted ReserveIntent conservatively reserves its tuple as Unknown.
Intact intent with missing/torn commit keeps that slot even if no ACK was sent.
Unknown/torn intent whose namespace cannot be identified quarantines WHOLE ledger.
Never skip a corrupt frame and infer empty space. TerminalIntent without commit
does NOT release namespace; reserve+commit with lost reply never repeats create.
Checksum-valid altered ordering or duplicate commit also refuses.

Cold replay outputs only DataConsistent/RetainedUnknownClaims/QuarantinedData and
bounded counts, claim views and reasons. Even a complete terminal DATA pair does
not manufacture native cleanup proof for another owner. No JournalReady,
DurableAck/NativeRoot/AdmittedSupervisor/Budget conversion, File constructor,
callback or production journal writer from parser output. Native admission and
Root-approved process-crash writer semantics remain separate; cold O operation
is observation-only under its own independently admitted supervision bound.

## Closed predecessor/successor grammar (X P2 refinement)

This table is exhaustive for serialized frames. Every unlisted predecessor/kind
pair is InvalidTransition -> QuarantinedData with prior claims/reservations kept,
never a guessed empty ledger. A valid DATA successor still authorizes no effect.
Global transaction state is separate from each attempt's state.

| Global predecessor | Only accepted next frame | Result |
| --- | --- | --- |
|NoGenesis, slot0|GenesisIntent, sequence1|GenesisPending; no attempt records |
|GenesisPending|IMMEDIATE GenesisCommit, sequence2 with exact intent reference|NoPendingTransaction |
|NoPendingTransaction|ReserveIntent satisfying all key/capacity rules|ReservePending(A) |
|ReservePending(A)|IMMEDIATE matching ReserveCommit(A)|NoPendingTransaction; A=Reserved |
|NoPendingTransaction|One allowed per-attempt frame below|Apply its attempt transition; TerminalIntent sets TerminalPending(A) |
|TerminalPending(A)|IMMEDIATE matching TerminalCommit(A)|NoPendingTransaction; A=Terminal/tombstone |

No second intent, Unknown, other attempt event, unrelated commit, epoch frame or
blank-gap-then-record can interleave with a pending intent. Valid end/zero suffix
after ReservePending conservatively retains A Unknown; after TerminalPending it
retains A's namespace/credits (no release); incomplete genesis quarantines all.
A subsequent frame rather than the required adjacent commit is corruption, not
an interrupted valid history to skip over.

| Attempt predecessor while global NoPendingTransaction | Only accepted frame(s) | Result / required association |
| --- | --- | --- |
|Absent attempt|ReserveIntent (global rule)|No published attempt until matching commit; outstanding intent already reserves uncertainty |
|Reserved|CreatePending OR Unknown OR TerminalIntent(NoCreateBeforeCall)|PendingCreate, UnknownWithoutChild, or terminal pending; exact immutable reservation reference |
|PendingCreate|CreatedHeld OR Unknown|HeldChild once with native-object CLAIM, or UnknownWithoutChild; never NoCreate |
|HeldChild|Unknown OR TerminalIntent(KnownEmptyRemoval)|UnknownWithChild preserving SAME child claim, or terminal pending with exact child/pair/parent association |
|UnknownWithoutChild|NONE|Reservation retained; no delayed CreatedHeld, CreatePending, NoCreate or caller cleanup recovery |
|UnknownWithChild|TerminalIntent(KnownEmptyRemoval) ONLY|Exact previously retained child/pair/parent; DATA consistency only, no restored File custody |
|Terminal|NONE|Attempt and tombstone immutable; no Unknown/create/replay/release again |

Unknown is idempotent at the local observation API: identical repeated observation
returns the existing Unknown status with NO appended frame/credit/clock change.
A second physical Unknown frame is a noncanonical duplicate and quarantines
replay; no repeated-write loop consumes the spare frame credit. The listed
Unknown reason IO-uncertain refers to TARGET-root IO, not a failed journal write/
sync. Journal IO uncertainty poisons the writer and emits no speculative next
frame; recovery derives uncertainty from the persisted prefix.

A different attempt may reserve a proven-disjoint namespace only with no pending
GLOBAL intent and sufficient retained credit. It cannot reuse an Unknown slot.
UnknownWithoutChild's no-repair rule is deliberately conservative for this first
profile; any future same-held native recovery needs separate reviewed semantics,
not a DATA parser shortcut or a reconstructed native observation.

Table-driven invalid-order fixtures must include: CreatedHeld before CreatePending;
duplicate CreatedHeld; CreatePending after Unknown, TerminalIntent or TerminalCommit;
Unknown after terminal; duplicate physical Unknown; second ReserveIntent during
ReservePending; different-attempt intent during TerminalPending; missing/nonadjacent
or wrong-reference commits; second genesis; Terminal(NoCreate) after PendingCreate;
KnownEmptyRemoval without prior HeldChild; delayed CreatedHeld after Unknown;
and reused attempt after terminal. Each fixture asserts quarantine plus no loss
of prior reservation/tombstone/pair claims, no synthetic ACK/Root/Ready. This is
part of the proposed12-test inventory (table loops in tests8..10), not new native
test authority or an implemented codec. All remain NOT_RUN.

# Proposed API, source slice and negative inventory

Root-owned proposed location native_security/storage/root_journal/format.rs and
replay.rs under consolidated Runtime; no source writes until Root+X review.
UnverifiedFrameClaims::decode(&[u8;4096]) and
ReplayDataView::scan_bounded(&[u8]) return only the DATA types above. Constructors
cannot supply a native factory; runtime consumers may validate claims only after
independent native O/file/clock admission. Do not import/copy the test-only model
as the implementation. Initial tests use byte arrays/slices, no File/provider IO.

Proposed twelve bounded codec/replay tests (not implemented/NOT_RUN):

1. header_offsets_endianness_and_each_kind_roundtrip_preserve_claims
2. exact_frame_file_and_payload_bounds_refuse_short_long_and_trailing_data
3. unknown_version_kind_flags_reserved_and_nonzero_padding_refuse
4. malformed_sid_utf16_path_nonce_and_native_id_claims_refuse
5. digest_previous_sequence_instance_and_epoch_corruption_quarantine
6. intent_only_and_torn_commit_retain_namespace_without_grant
7. torn_unknown_intent_and_nonzero_after_unused_suffix_quarantine_all
8. duplicate_attempt_namespace_commit_and_changed_tuple_refuse
9. created_pending_cannot_become_no_create_or_caller_cleanup
10. terminal_intent_requires_exact_commit_and_preserves_tombstone
11. every_capacity_credit_and_checked_overflow_refuses_without_eviction
12. cold_replay_never_restores_budget_or_exports_native_capability

Fixed representative records and bounded mutation loops, no huge allocations.
Compile-fail external/native type construction checks need Root's final visibility
surface, not a token-string assertion as capability proof. Actual file partial
write/sync/reopen/O-death and genesis native tests require later independent
Windows GHA allocation after the Platform amendment; no CI dispatch now.
