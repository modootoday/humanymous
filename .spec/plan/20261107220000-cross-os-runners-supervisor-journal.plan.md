---
id: plan:root/20261107220000
title: Supervisor-owned private-root journal native admission and durability protocol
status: draft
created: 2026-11-07T22:00:00Z
updated: 2026-11-07T22:00:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/structural-development, plan:root/20261107194000]
---

# Decision proposal, no implementation or native I/O grant

The12 private symbolic contracts passed on immutable49d; they are ordering
evidence ONLY. They cannot create a production ledger, durable ACK, supervisor,
native identity, root or Ready. This proposal describes NEW reviewed production
work without writing Root-owned Runtime source, creating files/ACLs, invoking
providers, adding unsafe/dependencies or dispatching CI. Ignored-parent actual
venue routing remains separate in [[plan:root/20261107212000]]. Native8/private
parent foundation retains priority; no role-DATA parser changes its prerequisites.

# Break the bootstrap circle at O's independent state admission

Target root and journal state are DIFFERENT namespaces. The journal MUST NOT be
stored beneath the root it is required to preregister. Recommended first backend:
ONE pre-existing, independently provisioned O-owned state FILE in O's admitted
state parent; no automatic file/directory initialization in journal admission.
O is the sole journal writer/ACK issuer. Existing account labels, TEMP/UserProfile,
caller-configured paths, ACL snapshots, model values and ordinary SID ownership
alone cannot prove an admitted O or protected journal. No hidden fallback.

This is an explicit unresolved bootstrap dependency, not a claim the current
venue already has suitable O state. Root must choose an existing installer/native
supervisor state-provisioning boundary or separately review hosted fixture setup.
The first journal's genesis/provisioning is outside its own reservation ledger:
it needs an independent supervised allocation with an exact fixed planned state
locator and retained bootstrap obligation. It is not the target-root transaction
and cannot be justified by the eventual target root. If no independent trusted
state parent/file exists, return Unsupported BEFORE journal or target mutation.
Do not use create-in-TEMP then ACL repair as an unstated solution. Existing six
bridge wrappers cannot create a descriptor-at-create FILE or atomically return
a created DIRECTORY FileID; no seventh wrapper/new unsafe is implied.

Component-first profile may use a verified ordinary O process and same-account
target ownership under the explicit exclusion of hostile SAME-SID code and
privileged OS/admin mutation. This cannot be promoted to Authority's distinct-role
SID/producer contract. Different-SID caller S/O operation additionally needs a
reviewed authenticated association/handle observation seam; raw S reports are
DATA. Root must decide the actual O role manufacturer. A proposed sealed type
below is not evidence that its native factory exists; absent factory is Unsupported.

# Native file, parent and namespace admission

O's candidate file is opened internally, existing-only, read/write+READ_CONTROL,
share0 (exclusive read/write/delete sharing), OPEN_REPARSE_POINT, no truncate,
append fallback, create, owner/DACL repair or privilege enablement. Arbitrary
caller File handles cannot attest their original share flags. Retain the actual
opened File and inspect it with current platform windows_object::inspect_opened:
nonreparse disk FILE, actual nonzero volume64/file128, NTFS/persistent ACL/local
GUID plus existing safe fixed-drive check. Verify actual native O TokenUser,
elevation/impersonation profile and exact file owner/DACL; the initial journal
profile is O-only, with no unreviewed inherited grants/hard-link allowance.
Additional file link-count/provider facts missing from existing safe APIs refuse
or need separate review; path/FileID alone is not a link/provider attestation.

Journal parent and every selected ancestor require held replacement guards and
mutation exclusion from [[plan:root/20261107194000]]: proposed desired0x20001,
share3/noDELETE, flags0x02200000, same-object prefix reopens and fixed volume.
Parent ownership is not sufficient if ordinary outsiders can change its DACL or
retarget it in place. Root C:/ peer-add-subdirectory is not selected replacement;
no TrustedInstaller anchor whitelist is introduced. Actual share/provider/owner
admission and these precise ACL rules remain unqualified, not inferred from the
three historical Get-Acl descriptors. Share0/denied-delete native tests are required.

Initial narrow namespace profile: ONE fixed configured target parent per journal.
Its stable selector catalog ID, exact native parent ID, O identity and policy
revision are pinned in the genesis header and checked against current admission.
Changed parent identity/path/selector catalog refuses fresh admission, not a new
empty namespace. Physical key = actual volume/file parent ID + generated lowercase
leaf nonce; retain exact configured locator and stable selector association too.
Same physical key OR possibly overlapping selected locator conflicts. A new
attempt ID/revision/anchor claim cannot bypass an active/Unknown reservation.
No dynamic alternate parent/8.3/UNC/reparse/case alias admission. Unknown alias or
disjointness refuses; future multi-parent support requires a separate protocol.

# Proposed private production API and placement

After review, Root would own placement beneath
crates/sdk/humanymous-runtime/src/native_security/storage/root_journal/,
decomposed admission.rs, format.rs, journal.rs, recovery.rs and tests. No new
package, Cargo feature, public root re-export or use of the cfg(test) model as
production code. Existing Windows storage boundary gates this first backend.
O role/budget factories belong to Root's admitted supervisor seam, not this module.

```rust
// Proposed private interfaces, NOT implemented types or a callable grant.
SupervisorJournal::admit_existing(scope: AdmittedSupervisorO,
    selector: ConfiguredExistingJournal) -> Result<SupervisorJournal, AdmissionFailure>;
SupervisorJournal::begin_root(&mut self, anchor: UserRootParent,
    nonce: ValidatedNonce, original: OriginalBudgetOwner)
    -> Result<HeldJournalReservation<'_>, BeginFailure<'_>>;
HeldJournalReservation::create_fresh(self)
    -> Result<JournaledUserRoot<'_>, RetainedRootAttempt<'_>>;
JournaledUserRoot::cleanup_empty(self)
    -> Result<RecordedRootCleanup, RetainedRootAttempt<'_>>;
SupervisorJournal::recover_inert(scope: AdmittedSupervisorO,
    selector: ConfiguredExistingJournal) -> Result<RecoveryOnlyJournal, RecoveryFailure>;
```

All fields/construction remain private. No bool/callback/serde/raw-wire constructor
for admission, O, ACK, original budget, File observation or successful cleanup.
One concrete File/serial writer; no cloned writer handle, caller-provided storage
trait or callback. Reservation/root owns the SAME mutable journal borrow and actual
parent/child handles plus ORIGINAL budget; no independently renewed cleanup pair.
Borrowed failure poisons/retains the same owner; consuming failures retain actual
owners. Drop does not journal/sync/delete or manufacture cleanup/settlement.
AdmittedSupervisorO must already retain the original live work/cleanup clock
authority BEFORE the first admission open/read, not start a fresh timer at begin.
Root-attempt budget custody must come from that existing authority with no later
horizon; exact private clock-owner types require Root review. Recovery is itself
bounded by a separately admitted supervisor operation, never a renewal of the
old recovered attempt's pair. File open/read/write/sync can block in the kernel:
the O process requires the existing independent outer supervisor/Job and original
hard stop. Poll/timeout/termination request is not disk flush or cleanup proof.

Successful begin yields a non-Clone held reservation only AFTER native admission,
capacity/index checks, exact full tuple persistence and synchronization under an
accepted provider contract. It is a local operation capability, NOT NativeProof,
human evidence or Authority Ready. The first implementation is same-O execution:
O executes registered create itself and reads actual child FileID on the same
held File. Cross-process S must not substitute its own ACK or reported child ID;
its transport/producer association is a separate unresolved Root decision.

# One append-only log, two derived indexes, no-clobber protocol

Use a pre-provisioned fixed file and serial append offsets within it; no rename,
temporary replacement, rotation, directory creation or journal-compaction path.
Two in-memory indexes (attempt ID, selected namespace) are reconstructed from the
SAME committed log. They are not two independently renamed durable files.
No-clobber is enforced under the admitted exclusive owner before any mutation;
all indexes/capacity credits are reserved together, with no Unknown eviction.

Proposed bounded profile for review:1MiB file/256 fixed4KiB frames,16 attempts/
16 namespace slots/16 reserved tombstones, one outstanding append, bounded UTF16
parent<=256 and leaf/full-child locator<=320 code units. No secrets in records.
Each admitted attempt reserves its maximum required terminal/tombstone frame and
byte credits before begin; frame/attempt/pending/generation additions are checked.
Exact encoding sizes/maximum transition credits require a source-level arithmetic
review before implementation. Header/epoch writes consume capacity too. Full,
overflow, torn history or insufficient terminal credits refuses BEFORE native
root create, never evicts Unknown/tombstones or resets budgets to make room.

Each fixed frame binds format version, monotonic sequence, O journal generation,
previous-frame SHA256, kind, bounded exact payload length and a full-frame digest
covering zero padding. Existing Runtime sha2 suffices; no new dependency. Hashes
detect corruption/order, not authentication or malicious valid-prefix rollback.
Native O custody protects integrity under the stated threat ceiling; hostile
same-SID/admin/dishonest storage protection is not claimed. Native journal FILE
identity and configured header/journal instance are checked on every admission.

1. Validate actual O/file/anchor/selector, original budget, suffix and complete
   immutable tuple; resolve both indexes and reserve ALL terminal/capacity credit.
2. Write complete ReserveIntent frame containing attempt+namespace+owner+anchor+
   selector+nonce+ORIGINAL pair+generation. write_all then File::sync_all; failure
   retains reservation/unknown and poisons further admission, no ACK/create.
3. Write ReserveCommit referencing exact intent/sequence/digest; sync_all again.
   Publish the held local reservation only after both successful operations.
   The single log commit publishes BOTH logical indexes; no partial-index success.
4. Before the bridge call, append CreatePending+sync_all. Consume the one local
   create permit. Only then use existing descriptor-at-create bridge. Identical
   request replay reports retained state, never a second effect or renewed pair.
5. Immediately retain CreatedDirectory on return, then open/read native child
   FileID/ACL/provider under held parents. Failed open/interruption leaves
   CreatedUnopened/RetainedUnknown with NO blind path deletion. A planned nonce
   or later descriptor-looking file cannot recover original child identity.
6. Record exact native-held observations only when actually obtained. Cleanup
   requires same-owner exact-empty native observation and accepted stable-parent
   close/remove/absence protocol. Persist TerminalIntent+TerminalCommit and sync
   before releasing namespace. Tombstone and full original tuple remain retained.
   S-reported NotCreated/absence, timeout, epoch change or caller death cannot do it.

[Rust File::sync_all](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all)
requests synchronization; Drop ignores close errors. Windows
[FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)
and [file buffering](https://learn.microsoft.com/en-us/windows/win32/fileio/file-buffering)
have provider/cache semantics. Readback is not fsync/durability proof. Initial
qualification is explicit process-crash/reopen on an admitted fixed local NTFS
file, not universal power-failure or physical-media attestation. No direct volume
flush, unbuffered alignment mode or new native wrapper is introduced. If a required
directory-entry/power-loss guarantee cannot be established by existing safe APIs,
that profile is Unsupported; atomic in-memory indexes cannot fill that gap.

# Recovery, ACK loss and original clocks

Bound scan by admitted file/frame limits; require exact header/instance/native
file identity and a valid contiguous digest/sequence prefix. Full intent without
commit reserves the tuple as Unknown even if ACK was never delivered. Commit with
lost reply is retained, never recreated. Partial commit with intact intent keeps
its slot Unknown. Malformed/partial intent or ambiguous history whose namespace
cannot be determined quarantines ENTIRE journal: no guessed-empty indexes/new ACK.
Do not truncate/repair/reinitialize bad tails or replay a valid-prefix rollback as
fresh state. External rollback fencing remains a separate needed storage/anchor
decision if the stated native-custody/provider premise is insufficient.

O restart cannot reconstruct Rust Instant from a journal timestamp or SystemTime.
Persisted original pair is an UNVERIFIED claim after owner loss. Recommended first
slice returns RecoveryOnlyJournal: preserve old reservations/Unknown/tombstones,
report them, and perform NO new create/delete/old attempt resume. Original live
supervisor may retain its monotonic budget across target-child restart; that does
not imply O-process or machine restart clock authority. Resumption needs Root's
independently retained live clock owner/authenticated generation seam, otherwise
Unsupported. No new boot/uptime API, epoch reset, fresh duration or wall-clock lease.

If later allowing genuinely new DISJOINT work after cold recovery, it must receive
a NEW independently admitted budget while old pairs/Unknown remain immutable;
this is not included in first RecoveryOnly API. Generation increment/epoch fencing
must itself be persisted under the same single-writer/full-capacity rules before
any future grant. Unknown/tombstone retirement/compaction has no automatic TTL.

# Source/testing slices and exact unresolved Root decisions

Proposed source steps after review: concrete bounded frame codec and replay
scanner tests; concrete existing-File admission/serial writer; local registered
root wrapper using approved bridge; then independent actual Windows fixture.
Fault simulation belongs ONLY in private cfg(test) code with fixed closed fault
profiles, not caller callbacks/traits or a production durable-ACK factory.

Codec/recovery tests must cover truncation at every frame boundary/byte range,
bad padding/length/version/sequence/digest, duplicate intent/commit, intent-only,
lost ACK, partial-index observation, conflicting namespace/attempt/generation,
capacity/terminal-credit overflow, forged terminal facts, replay and cold clock
loss. These are data/I/O-contract tests, not actual OS durability proof.

Actual GHA venue proposal: Delivery-owned Windows native component job, exact
immutable source/tools/NTFS fixture, Root-admitted O state prepared SEPARATELY
from target parent, fixed Rust O/helper under existing independent Job supervisor,
one original budget and no local workstation/SCM/account grant. Test actual
exclusive-open refusal, O-owned SD/FileID/provider readback, flush/reopen, duplicate
registration, process death at acknowledged boundaries and cold recovery refusal.
Controlled fault injection is not a real disk-driver failure; process kill is not
power loss. CreatedUnopened negative EXPECTS retained unknown disk obligation;
VM teardown is not component cleanup proof. Raw SID/path/journal stays private,
public counts/hashes only. No existing workflow dispatch/component option is added.

Root decisions required before code/provider work:

1. Actual admitted O manufacturer and allowed same-account component profile,
   separately from formal Authority distinct-role/native producer contract.
2. Independent pre-existing journal parent/file provisioning and genesis durability
   allocation; no current suitable O state/file is proved by this proposal.
3. Accepted anchor/ACE/share/link-count/fixed-provider/namespace association facts
   available through existing safe APIs; unsupported facts must not be invented.
4. Process-crash-only durability scope versus stronger directory/power-failure/
   rollback guarantee; safe standard fsync cannot silently attest the latter.
5. Exact frame layout/caps/terminal credit math and epoch/recovery/quarantine policy.
6. Original clock owner's restart authority; first proposal is inert cold recovery.
7. Local O create/cleanup custody versus authenticated distinct-SID S transport,
   including actual child-handle observation and no caller-bool completion.
8. Root-owned Runtime source visibility/ownership grant, Delivery job/fixture
   routing, budgets and data retention; no native/journal I/O is authorized now.

Until these gates are accepted and actual venue-qualified, Native private parent,
.NET/Header host foundation and production journal admission remain OPEN. This
design does not borrow the pure model or role fixtures to manufacture completion.

## Root first-profile decisions and actual bootstrap follow-up

Root accepted decisions4 and6 ONLY at primitive scope: admitted local NTFS
process-crash/reopen durability and RecoveryOnly/inert cold O restart. Stronger
power-loss/directory/rollback requirements remain Unsupported. No reconstructed
Instant, resume/create/delete/new budget on cold recovery. This is not final
product acceptance and leaves actual installer/genesis/provider work required.

The pre-existing-file admission API above is a recovery/admission step, NOT the
eventual installation solution. [[plan:root/20261107223000]] proposes a SEPARATE
Platform descriptor-at-CREATE_NEW FILE and owned-Job/process amendment, after
pinned safe-upstream investigation. That route can provision O's first private
journal FILE under a native-admitted creation-safe parent, without first requiring
the target private directory. Fixed bootstrap slot/selector and retained unknown
obligation prevent random-new-journal bypass. Root+X must approve exact new native
scopes/features/lifetimes before code; original six wrappers remain current policy.
[[plan:root/20261107223500]] specifies exact bounded codec/replay DATA, not a native
ACK or budget factory. No actual native I/O or Root-owned source change here.
