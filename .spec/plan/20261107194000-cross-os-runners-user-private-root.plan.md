---
id: plan:root/20261107194000
title: Fresh ordinary-user private-root initialization at creation
status: draft
created: 2026-11-07T19:40:00Z
updated: 2026-11-07T19:40:00Z
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
