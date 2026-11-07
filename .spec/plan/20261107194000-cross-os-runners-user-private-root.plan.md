---
id: plan:root/20261107194000
title: Fresh ordinary-user private-root initialization at creation
status: draft
created: 2026-11-07T19:40:00Z
updated: 2026-11-07T21:00:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/rust-migration]
---

# Root-assigned private-root bootstrap proposal v0.1

Header's actual C:/Users/smile parent failed the existing protected/inheritable
owner-only policy. create_user_private_directory cannot bootstrap that policy:
it first requires an already-private parent. This draft breaks that circularity
for a NEW empty ordinary-user root only. No existing ACL/owner repair, TEMP or
UserProfile privacy inference, SCM/account operation, authority factory or new
FFI/unsafe is authorized. Implementation follows Root/X/ABI/Delivery review.
The planned actual-interactive fixture slice remains a separate pending gate.

## Existing source and approved boundary

native-security already depends on windows-object-bridge. Reuse its public safe
create_directory_with_descriptor(&Path,&SecurityDescriptor). It applies a borrowed
descriptor in existing CreateDirectoryW and returns a must_use CreatedDirectory
cleanup obligation BEFORE caller validation. It performs no open-existing fallback
or ACL repair and adds no wrapper/unsafe surface. Its bounded path encoder checks
UTF16 before the native call. The bridge returns ONLY a locator, not a created
File/identity or atomic handle-relative operation. Preserve this limitation.

Current structural integration exposes that SAME implementation as
humanymous_platform::windows_object behind Windows/windows-object. The old
package spelling above records the original source boundary, not a compatibility
alias request. create.rs bytes remain SHA256
c323434eacc915e561aed5d48307bf2e8a3c02adf66cb7ba9a39c9185b90bbec.

Reuse current_identity() for actual process TokenUser/elevation/enabled-admin
checks and impersonating-thread refusal; verify_windows_privacy on held new child
for ProtectedDirectory AND InheritableDirectory. inspect_opened on the SAME held
parent/child File supplies volume64+file128/local-NTFS/persistent-ACL observations.
Those are filesystem observations, not storage-provider trust or NativeProof.

Primary semantics: [CreateDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectoryw)
accepts a creation descriptor and refuses an existing target; [file access
rights](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights)
separate ownership, ACL modification and file operations;
[descriptor control](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-control)
defines inheritance protection. [RemoveDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-removedirectoryw)
requires emptiness and normally marks deletion until the last handle closes;
return success alone is not final absence. Do not remove a substituted junction.

## Proposed exact API (not implemented)

```rust
// Internal Windows storage API proposal; private fields/no Clone/raw handles.
inspect_user_root_parent(path: &Path) -> Result<UserRootParent, RootInspectError>;
UserRootParent::create_fresh(self, suffix_hex32: &str, original: RootDeadlines)
    -> Result<UserPrivateRoot, UserRootCreateFailure>;
UserPrivateRoot::path(&self) -> &Path; // locator only, no standalone permit
UserPrivateRoot::original_deadlines(&self) -> (Instant, Instant);
UserPrivateRoot::revalidate(&mut self) -> Result<(), RootRevalidationError>;
UserPrivateRoot::cleanup_empty(self)
    -> Result<UserRootCleanupObservation, RetainedUserRootFailure>;
// NotCreated returns the actual parent owner; Retained owns creation obligation,
// parent/ancestor guards, optional child File, actual observations and same clocks.
RetainedUserRootFailure::into_attempt(self) -> UserRootAttempt;
UserRootAttempt::cleanup_empty(self)
    -> Result<UserRootCleanupObservation, RetainedUserRootFailure>;
```

RootDeadlines contains one validated original run/cleanup pair before mutation;
no test-support dependency or independently replaceable cleanup horizon. Public
receipts have private actual fields; no bool/callback constructs a verified root.
No automatic deletion in Drop. Dropped/abandoned obligations remain Unknown.
Consumer owns its own fixed child-file set and must close/remove it explicitly;
this initializer does not recursively delete caller content or certify erasure.
Borrowed revalidation errors keep and poison the SAME borrowed root; they do not
move a resource out of &mut self. Consuming failures return UserRootAttempt,
which may be unverified and cannot masquerade as the verified success wrapper.

The suffix is exactly32 nonzero lowercase hexadecimal bytes-as-text. The sole
constructed component is humanymous-private-<suffix>, not arbitrary segments.
No env-selected default/reuse, relative/UNC/device/ADS/traversal/trailing-dot/space
or reserved-name alias. Caller nonce creation uses an already approved entropy
source; nonzero syntax is not entropy/freshness evidence or producer authority.

## Parent profile and identity prerequisites

Parent need NOT be owner-only/private/inheritable. It MUST be creation-safe under
the explicit trusted ordinary-user/OS premise, inspected without mutation:

1. Actual ordinary non-elevated, no enabled Administrators, no impersonation;
   SID is native queried, never caller text. Recheck before creation/publication.
2. Absolute canonical local fixed-NTFS path with persistent ACLs; every retained
   guard is non-reparse/disk/directory, actual identities nonzero, no remote or
   unsupported provider success. Bound component/depth/path/allocation budgets.
3. Immediate parent owner is actual SID, SYSTEM or BUILTIN Administrators only
   for the initial narrow TCB profile. Mutating allow rights may belong only to
   those principals. Read/traverse-only outsider grants may be accepted; they
   do not become child grants because the new descriptor is protected.
4. Refuse unknown/callback/object/conditional ACL forms, null/empty DACL, foreign
   mutation, WRITE_DAC/WRITE_OWNER/DELETE/FILE_DELETE_CHILD/create/write grants,
   generic write/all ambiguity or unavailable readback. No effective-access guess
   from account names/group strings. Deny/inherit-only/CreatorOwner handling needs
   exact pure policy rules before implementation, never silently ignored.
5. Hold parent and required ancestor guards for the entire create/use/cleanup
   attempt; reopen each selected locator and compare actual SAME-object identity
   before/after effects. No delete sharing on ancestor guards unless a reviewed
   profile proves the corresponding replacement exclusion. Upper-ancestor peer
   creation rights and selected-path replacement rights are different; the first
   conservative profile may refuse layouts it cannot prove.

P0 remains explicit: the initial trusted-owner/ACE/ancestor profile must be
accepted after actual read-only layout evidence. Do NOT claim every default
UserProfile or C: ancestor is eligible: TrustedInstaller/other owners, broad
create rights and inheritance can cause conservative refusal. A path label or
parent File alone is not complete ancestry/identity-root proof. Same-SID hostile
code and privileged OS/administrator replacement remain outside this narrow
ordinary-user component guarantee; no protection against them is claimed.

## Concrete preliminary layout and minimal Unsupported cases

Root supplied private private-root-layout-readonly.json, schema
humanymous.private-root-readonly-layout/1, observed 2026-11-07T19:33:19Z,
SHA256 6d8dbb8c6c44db39acc9982b8aa79ce63719ebb212071d99c38b514547e2ab35.
Get-Acl recorded binary descriptors and raw ACEs with mutations0. The following
table omits SID values and the local profile name. It is a candidate analysis,
NOT held bridge identity, actual ordinary-token association, fixed-provider
qualification or eligible-layout proof. No account-name lookup extends the TCB.
SY means SYSTEM, BA means BUILTIN Administrators; OI/CI/IO mean object/container
inheritance/inherit-only. All recorded ACE types are AccessAllowed.

| Level | Observed owner and exact ACE mask/flags classes | Candidate decision before any mutation |
| --- | --- | --- |
| Volume root C:/ | Owner is a service SID outside proposed actual-user/SY/BA set. Authenticated Users: 0x00000004, no flags; 0xE0010000, OI+CI+IO. SY/BA: 0x001F01FF, OI+CI. Users: 0x001200A9, OI+CI. Capability SID: 0x001000A1, no flags. | Unsupported under full-chain narrow profile: owner outside TCB, outsider add-subdirectory right, and IO/generic ACE. No inference that peer creation alone permits replacing the selected descendant. |
| C:/Users | Owner SY. Everyone: 0xA0000000, OI+CI+IO; 0x001200A9, no flags. SY/BA: 0x001F01FF, OI+CI. Users: 0x001200A9, no flags; 0xA0000000, OI+CI+IO. Capability SID: 0x00100021, no flags. | Unsupported under first-profile IO refusal even though current-object outsider masks are read/traverse classes. Protected child creation does not authorize silently ignoring ancestor IO rules. |
| Local user-profile directory | Owner BA. SY/BA and one local-account SID: 0x001F01FF, OI+CI. Capability SID: 0x00100020, no flags. DACL protected. | Immediate-parent ACL candidate only IF the local-account SID is independently equal to the actual ordinary TokenUser; otherwise foreign full-control refusal. Capability traverse-only grant needs an explicitly accepted read-only rule. Full-chain decision remains Unsupported because of the two ancestors above. |

For review, the smallest exact first-profile classifier could accept ONLY
AccessAllowed ACEs, flags none/OI/CI/OI+CI, with trusted actual-user/SY/BA masks
from an explicitly closed set (initial full-control candidate 0x001F01FF), and
outsider masks from the observed closed read-only set
{0x001200A9,0x001000A1,0x00100021,0x00100020}. This is a PROPOSAL, not accepted
effective-access policy: unfamiliar mask combinations refuse rather than being
normalized into an allow. Deny, IO, CreatorOwner, unknown ACE types/flags,
generic rights, null/empty DACL and missing identity all refuse. The proposed
set does not accept this observed full chain and must not be relaxed after a
pilot simply to obtain success. No current object needs ACL repair.

Minimal independently sufficient Unsupported cases are: unapproved ancestor
owner at C:/; IO ACE at C:/Users; local-account SID unequal to actual TokenUser
at the immediate parent; or failure to acquire/revalidate any required held
non-reparse identity with the accepted sharing profile. One counterexample is
enough; later checks cannot turn an earlier refusal into eligibility.

Decision still needed: retain a full chain to an admitted volume-root anchor
(which currently refuses), or separately review a stronger pre-admitted held
anchor with explicit replacement exclusion above it. Starting at the user-profile
path without that admission is not an alternative proof. Exact read-only open
access/share flags, guard acquisition order, identity reopen comparisons, and
supervisor custody of the planned locator BEFORE mutation remain unresolved.
The supervisor must distinguish planned locator from created-object identity;
neither a preannounced name nor CreatedDirectory proves the latter.

Cleanup must freeze the sequence: revalidate while child and parent guards are
held; require known-empty root and known child identity; close the child only
under the accepted replacement-exclusion premise; remove the exact locator;
observe final absence while retaining parent guards. Close/remove/absence error
or expired original clock retains the same obligation, known IDs and original
pair as Unknown. A missing child handle/ID remains CreatedUnopened and cannot
enter that path-removal sequence. No existing bridge operation closes the atomic
create/open ID gap or provides handle-bound deletion. These are acceptance gates,
not grants to mutate or begin the separately allocated hosted pilot; actual0.

## Fresh create, readback and retained failure states

Prevalidate exact suffix, parent profile, bounded descriptor and original clocks
BEFORE create. Construct owner actualSID plus protected DACL granting only that
SID inheritable full control (OICI); no broad/world/default parent inheritance.
Validate the descriptor using existing safe wrappers BEFORE passing it borrowed
to the existing bridge. The directory is empty; no secret/payload is written.

State transitions are explicit:

| State | Required custody / allowed result |
| --- | --- |
| ParentInspected | Held native parent/ancestor identities, exact SID/profile, original clocks; no mutation. |
| CreateCallPending | Independent supervision; a kernel call has no universal return bound. |
| NotCreated | No created obligation returned; fail without touching any existing target. Bridge OsFailure erases the specific code: never infer ownership or recover by opening existing. |
| CreatedUnopened | Retain CreatedDirectory immediately plus all parents/clocks. Child identity unknown; no consumer success and NO blind path deletion. |
| ChildHeldUnverified | Actual child File retained, no reparse, parent/path identities rechecked; ACL/owner/fixed-NTFS readback still pending. |
| ChildVerified | Actual owner-only protected+inheritable readback and original time satisfied; metadata locator usable only while custody/premises persist. |
| CleanupPending / Unknown | Same owner/obligation returned on every query/open/ACL/time/remove/close error. Missing child identity never becomes a cleanup success. |
| CleanupObserved | Exact tracked empty root only; actual held handles closed and bounded post-removal absence observed, with scope/premises recorded. Not secure erasure or provider attestation. |

No fs::create_dir-then-repair. Existing directory/file/reparse/collision always
refuses; do not delete or modify it. Successful bridge return followed by failed
open cannot atomically recover the original ID using this bridge. Without the
creation-safe parent premise that branch stays Unknown/retained. If full hostile
parent resistance or atomic handle-relative create/delete is required, request a
SEPARATE reviewed native boundary; do not expand current six wrappers implicitly.

Empty cleanup revalidates parent/root held identities and excludes reparse/path
substitution before touching a locator. Exact handle sharing/close order must be
frozen before implementation: deny-delete handles may block our own removal;
delete-sharing can permit pending-delete until final close. A close-to-path-delete
gap is permissible ONLY under the accepted stable-parent/same-SID/OS premise,
never portrayed as race-free identity-bound deletion. No broad recursive remove,
delete-on-Drop, ACL repair or retry with renewed deadlines. If conditions cannot
be met using existing safe operations, return retained Unknown rather than delete.

## Pure negatives and actual hosted pilot proposal

L1 authored-next (Rust only, no mutation yet): suffix/path bounds, reserved/alias
forms, reversed/expired clocks, descriptor exact owner/protection/OICI and noextra
ACE, all foreign mutation/generic/unknown ACL cases, unsupported provider/reparse,
wrong parent/child IDs, created-unopened ownership, retained partial failure and
deadline conservation, existing-target refusal, nonempty/replaced-child cleanup
refusal. Model facts alone are never an actual directory-create/readback proof.

L2 proposed first venue: explicitly allocated disposable native Windows runner,
actual ordinary actor under existing independent Job supervisor, source/tool/
bridge/descriptor pins and one original run/cleanup pair. NO local mutation or
SCM/account operation is granted by this proposal. Any ephemeral actor/fixture
provisioning is a separate hosted-only Root allocation using existing mechanisms.

Smallest positive uses a freshly supervised creation-safe parent that includes
SYSTEM/BA access and therefore deliberately fails existing owner-only privacy,
but satisfies the new creation-safe policy. Actual ordinary actor creates ONLY
one fresh root via approved at-create descriptor, holds/readbacks parent/root
IDs/owner/DACL/provider facts, creates one nonsecret CreateNew file, verifies its
held-file privacy before write, writes/reads exact bounded bytes, explicitly
closes/removes that known file and empty root, observes absence and outer Job0.
No existing UserProfile/ACL is modified to make the test pass.

Initial actual negatives: already-existing root unchanged (identity/ACL/content
before/after), wrong/unsafe parent refused before create, controlled post-create
readback failure retains the exact obligation rather than deleting unknown path,
nonempty root cleanup refusal followed by explicit same-owner recovery. Injected
failure is controlled setup evidence, not all OS failure modes. Unsupported
ancestor/provider layouts stay Unsupported. All actual counts currently0.

Root will coordinate X/ABI/Delivery before implementation. Header receiver needs
this private-storage component, not an Authority credential/NativeProof grant.
Runtime/SCM producer association, admitted host, interactive workers, GPU and
full product gates remain separate. No new graph/unsafe or consumer activation.

## Selected-parent anchor amendment v0.2 (review only)

Root requested a narrower replacement argument after the five opaque-ID fixture
tests. This amendment proposes replacing v0.1's blanket full-chain owner/IO/peer
creation refusal with ROLE-SPECIFIC checks. It is NOT accepted implementation
policy, a whole-venue eligibility result, or a mutation/actor allocation. The
original table/refusals remain the historical conservative proposal; they are
not silently treated as tests of this amendment. Existing captured3SD input is
unchanged SHA6d8dbb8c6c44db39acc9982b8aa79ce63719ebb212071d99c38b514547e2ab35.

### Separate privacy, selected namespace and unrelated peers

Privacy comes from actual TokenUser ownership plus the protected owner-only OICI
descriptor applied at NEW directory creation, followed by same-held-child ACL
readback before any payload. The anchor is permitted to have SY/BA and readonly
outsiders: it need not satisfy child privacy. This is bootstrap, not repair.

Namespace stability is a separate conjunction: exact configured selected parent,
its held native identity and volume, every selected ancestor guarded against
ordinary delete/rename, and no ordinary outsider able to change ACL/owner or
retarget an ancestor in place. Peer creation alone is not selected-object
replacement. A creator can cause a fresh-name collision (refused, no modification)
without acquiring access to an already-created protected child. The initial
amendment allows the OBSERVED add-subdirectory bit only at ancestor-role nodes;
it need not admit outsider creation rights on the immediate anchor.

Microsoft distinguishes [FILE_ADD_SUBDIRECTORY=0x4 from
FILE_DELETE_CHILD=0x40](https://learn.microsoft.com/en-us/windows/win32/fileio/file-access-rights-constants).
[CreateFile sharing](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
excludes subsequent delete/rename access while the relevant sharing contract is
held; [ACE_HEADER](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-ace_header)
defines IO as not effective on its attached object. These documented distinctions
motivate the design; they are not an actual share-mode/provider pilot.

### Exact proposed role and ACE table

Only parsed ordinary AccessAllowed ACEs with complete SID/mask/flag readback;
unknown/object/callback/conditional/deny/CreatorOwner/null/empty/invalid encodings
refuse in the initial classifier. Deny is conservatively refused, not treated as
an allow. No effective-access/group-name oracle is introduced. Every effective
grant to a SID outside actual TokenUser/SY/BA is evaluated as outsider, including
groups that happen to contain the current actor. Unknown flag bits refuse.

| Object role | Owner admission | Effective ordinary allow masks | Inheritance handling |
| --- | --- | --- | --- |
| Immediate parent anchor | Actual TokenUser, SY or BA only | Trusted principals exact0x001F01FF; outsider closed readonly set0x001200A9/0x001000A1/0x00100021/0x00100020. No outsider0x40,DELETE,WRITE_DAC,WRITE_OWNER,writeEA/writeAttributes or creation bits. | none/OI/CI/OI+CI, optionally recorded INHERITED flag; first anchor profile still refuses IO/generic ambiguity. |
| Selected ancestor guard, including volume-root directory | NOT admitted as creation anchor; no new owner whitelist entry | Same readonly set and exact0x00000004 peer-add-subdirectory; trusted actual TokenUser/SY/BA exact full access. No outsider current-object deletion, ACL/owner change, writeEA/writeAttributes or generic mask. | Recognize exact observed OI+CI+IO masks0xE0010000 and0xA0000000 ONLY as non-effective on that ancestor; record rather than normalize into rights. Inspect each selected child independently; new child descriptor is protected. |
| Newly created private root | Actual TokenUser only | Sole actual-SID full control, exact protected+OICI policy and existing privacy verifier | Reject every outsider/IO-only/no-propagation/missing required inheritance form; no inherited parent grant. |

Flags are evaluated before current-object mask classification: those two CLOSED
ancestor IO forms are not grants on the attached ancestor. Effective generic
rights are never admitted under the IO exception. No blanket IO ignore, generic
mapping guess or broad read/write mask widening. Metadata list/byte/path/depth
bounds remain required. These exact sets need X/ABI/Root policy review.

Ancestor ownership is NOT thereby declared harmless: an owner can change its
DACL, which a deny-delete guard does not prevent. The candidate's ceiling excludes
same-SID hostile code and privileged OS/administrator mutation. An ordinary
outsider-owned ancestor or an unclassified owner with mutation capability remains
Unsupported until a separate read-only admission proves the relevant exclusion.
The captured root service-owner SID is NOT added to anchor TCB, and its account
label alone supplies no admission. No denial of unrelated C:/ peer creation or
TrustedInstaller whitelist is needed to state this conditional argument; current
input still lacks the native/principal admission needed to instantiate it.

### Apply to the actual three-descriptor layout

| Observed level | Amendment candidate | Still missing / concrete refusal |
| --- | --- | --- |
| C:/ | Ancestor-role readonly masks plus exact peer0x4 and recognized non-effective IO need not fail selected-replacement policy. Root service owner is never an anchor candidate. | Actual held nonreparse/fixed NTFS identity/guard and owner-mutation threat exclusion unqualified. Ordinary outsider DELETE_CHILD/WRITE_DAC or unclassified mutable owner refuses, independently of peer0x4. |
| C:/Users | SY-owned ancestor with effective readonly outsiders; recognized IO does not apply to current object. | Held guard/reopen and selected-child identity/ACL not established by Get-Acl. Effective outsider0x40 or selected-parent substitution refuses. |
| Local user-profile directory | BA-owned immediate anchor candidate, full local-account grant only IF it equals actual native TokenUser; capability traverse0x00100020 fits readonly set. | TokenUser/elevation/admin/impersonation equality, held identity/provider, no-reparse chain and accepted selector custody remain unqualified; different account full-control grant refuses. |

This table qualifies no venue. It identifies checks that may succeed in a later
explicit read-only native allocation; it does not reinterpret preliminary
PowerShell descriptor snapshots as native association/provider/ordinary-actor
evidence. Unsupported remains the actual current outcome pending those checks.

### Guard acquisition, configured selector and API custody

Propose inspect_user_root_parent(selector: &ConfiguredParentSelector) rather than
accepting env/TEMP/UserProfile defaults. The selector binds one exact canonical
bounded path and configured identity expectation, but is unverified DATA until
opened. Only same-held File observation can construct UserRootParent. Store the
actual SID/volume/file IDs, exact selected components, policy revision, held chain
and original clock pair privately; no external ID/path claim constructs custody.

Acquire guards volume-root to anchor with safe OpenOptionsExt, proposed desired
READ_CONTROL|FILE_LIST_DIRECTORY=0x00020001, share READ|WRITE=0x3 (NO DELETE),
BACKUP_SEMANTICS|OPEN_REPARSE_POINT=0x02200000, open-existing only. Refuse denied
opens or prior incompatible delete handles, no retry with broader sharing or
privilege. Existing inspect_parent uses READ_CONTROL-only0x20000: do NOT declare
it a qualified deny-delete guard just because share_delete is omitted. Explicit
directory read access and actual share-conflict tests are prerequisites. The
documented [share-check contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-iocheckshareaccess)
depends on granted access and filesystem state; no unconditional kernel bound.

For each prefix: inspect SAME held File as nonreparse directory, local fixed NTFS
and persistent ACL; inspect bounded SD; retain it before opening the next prefix;
reopen the locator using the same flags and compare volume64/file128 plus volume
GUID to held identity. Revalidate all selected prefix associations before create,
after child open and before cleanup. A share guard constrains delete/rename, not
in-place ACL/reparse changes, privilege bypass or drive-letter remapping; require
the explicit mutation/namespace ceiling and refuse if it cannot be admitted.
OPEN_REPARSE_POINT protects only the opened final component; inspecting and
holding EVERY selected prefix is mandatory. Filesystem/owner changes discovered
after an effect yield retained Unknown, never retroactive privacy success.

Before CreateDirectoryW, the independently supervised attempt registers the exact
nonce, full child locator, configured selector revision, actual held-anchor ID,
original deadlines and state Planned. Registration must be durable/acknowledged
BEFORE native invocation, without treating a locator or acknowledgement as a
creation/identity/cleanup proof. If registration is unavailable, refuse before
mutation. Root must select the existing supervising owner and journal mechanism;
a new authority callback/bool or a helper process is not authorized here.

Creation success first retains CreatedDirectory, then opens/guards the child and
reads actual FileID/owner/DACL/provider on that SAME handle. The planned nonce is
not a FileID. CreateCallPending/interruption/CreatedUnopened with unknown ID stays
RetainedUnknown; no path-based recovery deletion, even if a later open looks like
the expected descriptor. A controller crash leaves a planned/unknown journal,
not transferred handle custody or automatic cleanup. Stable-anchor premise and
post-open ID reduce ordinary namespace ambiguity; they do not make create+open
atomic, attest the native producer, or prove a human/root ownership authority.

Existing exact-empty cleanup sequence remains: verify child identity and guards,
no remaining consumer files, close child only under admitted replacement premise,
path-remove, observe absence while parents remain held. Every uncertainty retains
obligation/known IDs/original pair. No new unsafe, handle-bound delete claim,
descriptor repair, recursive cleanup or clock renewal.

### Narrow tests to author after policy acceptance (none executed)

Pure matrix: ancestor peer0x4 accepted but effective0x40/DELETE/WRITE_DAC/OWNER/
writeAttributes denied; same bits on anchor refused; exact IO forms non-effective
only on ancestor, effective generic and unknown flags denied; ordinary mutable
ancestor owner unsupported; no root-owner-to-anchor TCB promotion; wrong actual
TokenUser/profile grant denied; selected-prefix ID/volume/reparse mismatch denied;
guard access0x20000-only not a qualified guard; missing journal acknowledgement
refuses before call; creation interruption/unopened retains Unknown with no
delete; post-open failures and cleanup clocks preserve SAME owner and IDs.

Separate later controlled native tests must demonstrate actual ancestor/anchor
rename/delete opens fail while guard held, prior delete-open acquisition refuses,
and unrelated peer creation does not authorize selected replacement. No such
test is granted now. Existing three SDs are fixed offline fixtures for the policy
matrix, not permission to mutate C:/, its profile, ACLs or any peer directory.
Only Root-coordinated review/read-only admission and an explicit hosted fixture
allocation can advance this proposal toward the .NET/Header foundation.

## No-clobber attempt registration amendment (X P1, design only)

Concrete failure to prevent: legitimate attempt A registers anchor/suffix, creates
the directory, then crashes before learning its FileID. Attempt B reuses the same
selected namespace and overwrites the registration/original clocks. B's later
NotCreated could then erase A's real CreatedUnopened/Unknown obligation. Neither
fresh syntax nor an in-memory nonce map prevents this across supervisor restart.

The SOLE authoritative journal owner is independently supervised O, or an
equivalently verified protected broker admitted by the same reviewed protocol.
Caller bools, callbacks, serialized ACK claims and in-memory model transitions
cannot construct an authoritative registration, durable ACK or root capability.
O must retain an independently protected journal/anchor association; caller-held
locator or a record beneath an unqualified path is not sufficient. Exact storage
atomicity, persistence/flush/crash recovery, namespace canonicalization and broker
authentication remain review prerequisites, NOT implemented guarantees.

Proposed registration identity is an immutable tuple: globally distinct attempt
ID; actual owner identity; native anchor volume/file identity; configured selector
and policy revision; exact canonical selected child namespace and nonce; original
run/cleanup pair; journal generation. Attempt identity and namespace reservation
are separate indexes. Reserve BEFORE native create, with atomic NO-CLOBBER in
both indexes and an ACK bound to the entire tuple. For overlapping path spellings
or distinct anchor claims that may select the SAME namespace, uncertainty refuses
reservation; string inequality is not disjointness proof. A caller-chosen new
attempt ID or selector revision cannot bypass an existing namespace reservation.

| Existing authoritative state | New registration / completion rule |
| --- | --- |
| Reserved / acknowledged / CreateCallPending | Different attempt touching same selected namespace refuses; no replacement of record, owner or clocks. |
| CreatedUnopened / Unknown / recovery pending | Reservation survives crash/restart/handoff. New attempt touching that namespace refuses, including identical nonce replay; no automatic expiry/garbage-collection clears it. |
| Same attempt retransmits identical tuple | O may replay the SAME persisted ACK only after proving exact generation/tuple and retained record; it never resets clocks or repeats a native call. No new attempt is manufactured. |
| Same attempt ID with any changed tuple field | Refuse as conflict, preserving original record and budget. |
| B reports NotCreated | It applies ONLY to B's verified current record if B was actually admitted. It cannot close A's record or release A's namespace. Collision refusal is not proof about A's create result. |
| Terminal cleanup known for exact attempt | Release namespace only through O's separately reviewed identity-bound terminal protocol; retain immutable attempt/tombstone for stale ACK/completion replay rejection. Timeout, caller death, journal absence or path absence alone is not this proof. |

For fresh B to receive NotCreated, its admission must have succeeded against a
disjoint namespace and its own create outcome must be associated with B. The
particular A/B collision above refuses B BEFORE create. No local clock renewal,
replacement parent, new generation or reopened path turns A Unknown into Known.
If O restarts without sufficient durable history, all possibly affected slots
remain unqualified/Unknown and no fresh registration is ACKed under a guessed
empty ledger. Do not claim exactly-once effects from journal transitions alone.

Crash handoff transfers only custody that is actually retained and admitted.
A durable planned locator does not recover an unknown created FileID or transfer
the dead process's File handles. O owns the unresolved obligation and original
pair until the reviewed recovery protocol settles it; O may report retained
Unknown without gaining a delete permit. A new O/broker epoch needs authenticated
history/generation fencing; it cannot silently resurrect expired work horizons.

### Proposed pure model slice for Root review before code

Author a private, platform-independent transition model with explicit unverified
facts and caller-supplied symbolic attempt/namespace IDs. The model may return
ModelReserved/ModelConflict/ModelUnknown transitions ONLY. No file/socket/process
I/O, journal durability assertion, native File constructor, authority callback,
DurableAck type, native root/Ready factory, or production admission integration.
Explicitly model conservation of the original tuple and distinct namespace index;
do not export an empty/new model as a supervisor restart success path.

Proposed negatives and expected assertions:

1. A reserve/create-unopened/crash -> B same anchor+suffix: B rejected; A's exact
   owner/unknown/clock tuple and reserved namespace unchanged.
2. B new attempt ID, nonce replay, alternate spelling/selector revision or claimed
   anchor alias cannot overwrite a namespace known equal or not proven disjoint.
3. Duplicate A identical tuple can only observe same model record; altered owner,
   nonce, anchor, attempt generation or original pair refuses with zero mutation.
4. Stale ACK/completion from old attempt/epoch cannot release current reservation;
   B NotCreated never clears A Unknown, whether delivered before or after restart.
5. Original run expiry, cleanup expiry, clock rollback/rebase/new process start,
   crash handoff and missing journal facts do not reset either original deadline
   or expire Unknown into a free slot. Distinct namespace B retains its own pair.
6. Lost ACK after reservation remains reserved; uncertain append/flush/recovery
   is modeled Unknown, never durable success. Partial insertion in either index
   cannot become admitted state or allow a second namespace owner.
7. Only an exact-attempt terminal observation can propose model release; path
   absence/caller boolean/timeout are insufficient. A retained tombstone denies
   replay even after model namespace release. This models ordering, not native
   cleanup, secure erasure, durable persistence or supervisor authentication.

Implementation, precise pure API/module placement and actual test allocation
remain pending Root review. No new code/native I/O/journal/directory/ACL/unsafe or
callback has been introduced by this amendment. Native eight-family/private
parent foundation remains the priority; Docker public-role DATA parsing does not
provide any of these local persistence/ownership prerequisites.
