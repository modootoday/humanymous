---
id: plan:root/20261107121729
title: Draft retained Windows opened-file identity for audit passes
status: draft
created: 2026-11-07T12:17:29Z
updated: 2026-11-07T12:17:29Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/native-security, sot:root/audit-key-custody]
---

# Additive file identity edge/API proposal v0.1

Root requested exact review only; no graph/lock/code or native execution changes.
Audit needs same opened-file identity+length before/after each read pass, and
freshly reopened path identity, under its retained lifetime WAL lock. Existing
native-security has no such shared API. Size/mtime/path alone is not identity.

Proposed single additive native-security manifest edge:
[target.'cfg(windows)'.dependencies] same-file = "=1.0.6".
Root lock already pins checksum
93fc1dc3aaa9bfed95e02e6eadabb4baf7e3078b0bd1b4d7b6b0b68378900502,
winapi-util0.1.11 and windows-sys0.61.2. No separate winapi-util direct edge, no
root Cargo edit, nightly windows_by_handle or authored unsafe is requested.
Root must confirm actual feature/transitive unification in its graph window.

The LOCAL PINNED primary source same-file1.0.6 src/win.rs explicitly uses cached
64bit volume/index keys. It warns ReFS128 identifiers can collide and both
handles must stay open. Equality does NOT check size despite an older comment
mentioning size mitigation. Reject ReFS, remote/network, unknown, unsupported
providers; this cannot be universal Windows identity/attestation. Existing
native-security volume::require_fixed only checks drive type, NOT filesystem
or actual opened-object provider. It is insufficient by itself.

Proposed API (opaque provider construction is STILL AN OPEN gate):

```rust
struct VerifiedOpenedLocalNtfsFile; // actual file/provider coupled; no public permit
struct RetainedFileIdentity; // private same_file::Handle, no Debug/Serialize
struct FilePassObservation; // private length plus retained object association

RetainedFileIdentity::from_verified_opened(
    opened: &VerifiedOpenedLocalNtfsFile,
) -> Result<Self, FileIdentityError>;
RetainedFileIdentity::observe(&self) -> Result<FilePassObservation, FileIdentityError>;
RetainedFileIdentity::same_opened(&self, other: &Self) -> bool;
```

from_verified_opened clones ONLY the actual file coupled inside the verified
owner, then safe same_file::Handle::from_file; the
original audit reader and retained identity clone remain open across ALL passes.
Fresh path reopen uses the SAME verified provider/ancestor/open policy, creates
another retained handle and compares while BOTH handles are open. Failure does
not close the original owner. Fresh length comes from retained as_file.metadata
at each observation, not cached equality key. Never expose as_file_mut, an inner
mutable File getter, raw cached key, numeric identity constructor or detached ID:
replacing the inner File could make a cached identity refer to another object.
No new authored unsafe std::File-to-winsafe HFILE adapter.

Supported-provider recipe must be explicit and actual: fixed LOCAL NTFS, verified
opened-file-to-provider association, regular file, protected nonreparse ancestors,
held lifetime WAL lock, actual same native object across pass/path reopens. Safe
winsafe0.0.29 GetVolumeInformation/GetVolumePathName exist for PATH observations,
but those alone do not authenticate the provider of an arbitrary std::File.
Actual handle/provider binding remains P0; do not mint a capability from a caller
string, guessed drive letter, environment label or unverified struct field. If
existing safe APIs cannot prove the selected opened provider, return Unsupported
and separately review a safe128bit file-ID/provider alternative. No unsafe waiver.

Errors are static/redacted: UnsupportedProvider, CloneFailed, QueryFailed,
NotRegularFile, ChangedLength, ChangedOpenedObject, ReopenedPathChanged. Unknown
query/provider/identity is failure, never true equality. Length/identity stability
does not detect every same-length in-place mutation or prove audit authenticity;
Audit still owns lock, canonical scan, integrity and trusted provider assumptions.

Pure graph/API review precedes implementation; allocated Windows actual tests
must cover retained original plus reopen, different files/hardlink semantics,
replacement attempt, length change, query failure, file close/reuse and explicit
unsupported-provider outcomes. Do not simulate ReFS denial as an actual ReFS
provider probe. Audit consumer contract and Root graph/API approval are required.

## Audit critique disposition v0.2

Audit independently requires per-handle provider association: a standalone NTFS
permit must not admit a different std::File/ReFS/remote object. The proposed
signature now accepts ONLY an opaque file+provider coupled verified-opened owner;
its actual safe constructor is still P0 and unimplemented. No generic reusable
provider token, caller-supplied assertion or weakened Unsupported fallback.

RetainedFileIdentity exposes NO inner &File getter, mutable getter, raw handle
or into-file operation. Read/Seek on &File and try_clone can share/change offsets;
observe must remain internal metadata-only. Actual ownership tests must prove
acquisition/observe/Eq/drop do not change the audit reader position and observations
cannot be transplanted to another owner/provider. Legitimate hardlink identity
is not alias/privacy approval; Audit independently enforces that policy. No
actual ReFS/provider result or graph/code acceptance follows from this critique.
