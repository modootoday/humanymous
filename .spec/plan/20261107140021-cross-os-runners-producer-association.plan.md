---
id: plan:root/20261107140021
title: Windows parent-issued opened-object association and isolated query exception
status: draft
created: 2026-11-07T14:00:21Z
updated: 2026-11-07T14:00:21Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/native-security, sot:root/rust-migration]
---

# Decision proposal v0.1

Root requested a positive producer-association implementation path. This is a
reviewable proposal, not an unsafe exception, graph adoption, native execution,
or qualification. Existing Unsupported remains mandatory before HostIO. No
scoring, L1–L7 policy, C# workload source, account or SCM change is proposed here.

## Trust boundary and separate decisions

The trusted controller is the actual elevated Windows parent; SYSTEM and enabled
Administrators on this ephemeral hosted machine are inside its trust boundary.
A malicious administrator, kernel/provider or injected trusted controller defeats
this premise. Ordinary actor processes, environment labels, paths, hashes and
decoded DTOs are outside it. No local primitive proves GitHub hosted origin.
The parent must independently perform the accepted hosted/source/artifact
preflight before publishing an association. source_sha remains invocation source;
managed/native/harness compiled lineages remain distinct.

Request three separate Root decisions: (A) this protected parent-issued association
protocol and its trust premise; (B) an isolated authored-unsafe Windows query crate
exception, including exact lint/SoT scope; (C) subsequent bounded real hosted
account/Job/filesystem tests. Acceptance of A does not authorize B or C. No change
to workspace unsafe_code=forbid is requested. A new package with an explicit local
unsafe lint policy requires Root approval; it must not inherit then attempt to
override forbid. No fork, generated shim, C#/C glue or hidden foreign unsafe.

## Same-handle query boundary

Proposed unpublished Windows-only crate humanymous-windows-object-query, Rust
1.99.1/edition 2024. Proposed sole direct dependency windows-sys = "=0.61.2",
features Win32_Foundation and Win32_Storage_FileSystem; already locked is not
approval of a new direct edge or feature unification. Root owns graph changes.
No same-file 64-bit identity is used on this path. Existing safe wrappers continue
to own tokens, ACLs and process handles; this crate owns no process or Job.

```rust
pub struct OpenedObjectObservation { /* private identity, provider and attributes */ }
pub enum QueryError { UnsupportedProvider, InvalidObject, QueryFailed }
pub fn inspect_opened(file: &std::fs::File)
    -> Result<OpenedObjectObservation, QueryError>;
// Metadata comparison only; neither authority nor an independent constructor.
pub fn same_object(a: &OpenedObjectObservation, b: &OpenedObjectObservation) -> bool;
```

Observation has no Serialize/Deserialize, public numeric constructor, File getter,
raw handle getter, Debug or authority conversion. native-security wraps it with
the SAME owned File and retains that owner; no detached observation admits a
different File. The query function borrows AsHandle for the complete synchronous
call, never constructs an owning handle, closes one, changes offsets or issues IO.
An immutable File borrow keeps the kernel handle live; no user-provided raw handle
is admitted. API use is safe only under the existing trusted OS/provider premise.

Exact allowed FFI calls:

1. GetFileType: require FILE_TYPE_DISK, otherwise InvalidObject. It grants no
   local-provider assertion by itself.
2. GetFileInformationByHandleEx(FileIdInfo), correctly sized/aligned FILE_ID_INFO:
   retain full VolumeSerialNumber64 + FileId128, never truncate to index64.
3. GetFileInformationByHandleEx(FileAttributeTagInfo): reject every reparse object;
   directory versus regular object follows existing role checks.
4. GetVolumeInformationByHandleW on the SAME borrowed file: bounded zero-initialized
   UTF16 filesystem buffer, require terminated exact NTFS and persistent ACL flag.
5. GetFinalPathNameByHandleW with VOLUME_NAME_GUID|FILE_NAME_NORMALIZED, bounded
   resize with cap 32768 WCHAR and no unbounded retry: require local volume-GUID
   form, reject UNC/device/unknown forms. Couple this with fixed-drive observation
   of that volume and held protected ancestor handles. Path observation alone is
   insufficient. Reject unsupported/missing provider facts, including a network
   filesystem reporting NTFS. This is a local Windows NTFS profile, not universal
   storage-provider attestation or proof against a malicious filesystem driver.

All outputs are initialized POD buffers, exact size checked with checked u32
conversion. On failure no output is interpreted; capture OS error immediately
and expose a redacted typed error. No transmute, pointer arithmetic, ownership
conversion, callback, asynchronous buffer or manual extern layout. Use pinned
windows-sys generated types/functions. Invalid handle tests use a private cfg(test)
raw-call seam inside the exception crate, never public invalid File construction.
Zero/invalid IDs and unrecognized query results fail closed pending documented
provider support. Neither ERROR_INVALID_PARAMETER nor FileRemoteProtocolInfo
failure is interpreted as proof of locality.

## Protected association record, not a hash upgrade

Keep the frozen nine-field parent-context binding unchanged. Propose a separate
fixed association.json <=16KiB under the SAME protected fresh hmctx-nonce root.
This is a NEW Rust metadata file, unrelated to the approved three C# files.
It requires explicit protocol approval and a strict shared codec before use.
It contains schema humanymous.parent-native-association/1, nonce, actual actor
SID, parent PID+creation time, child PID+creation time, root/context/binding full
volume64+file128 identities, context/binding/provenance commitments and the exact
original four-field budget. No secret, bearer token or renewable deadline.

Only a private constructor borrowing PublishedActorContext and the existing
actual retained child/Job owner may emit it. That constructor queries actual
objects; it cannot accept a caller-made Verified, PID, Job ID or identity DTO.
The parent creates the harness suspended, stores actual process/thread handles
immediately, configures/assigns its actual Job, observes actual TokenUser=reserved
actor and exact executable/preflight pins, then publishes association create-new.
On partial failure it retains both actual carrier and process owners for cleanup.
No resume occurs until publication and independent readback succeed.

Association is write-once, flushed, reopened read-only with write/delete sharing
denied, exact Admins/SYSTEM owner and protected three-ACE role DACL. The parent
retains association/root/context/binding and protected ancestors until child Job
and all readers settle. Remove current write-close/reopen ambiguity by comparing
writer-observed FileID128 to reopened pin while lifetime custody of protected
ancestors remains; if a simultaneous pin can be safely opened with compatible
sharing, prefer retaining it before writer close. Never assume existing share=0
allows this. ACL/readback failure returns retained owner, no repair/fallback.

Receiver opens fixed association through the same native role/provider/ancestor
validator and retains every pin. It checks current actual PID+creation time and
TokenUser, actual opened root/context/binding IDs against the protected record,
shared binding/codec exact fields and original budget. It opens and RETAINS the
recorded parent process, checks creation time, live state, primary token elevation
and enabled Administrator/SYSTEM role, and pinned executable identity. PID alone,
parent-PID ancestry and a path-only image query are insufficient. Re-read the
association from its retained handle before upgrading; reject any changed bytes.
Provider/ACL/identity checks apply before trusting the record's expected values.

Why this is independent: an ordinary actor cannot create an Admins/SYSTEM-owned
record or write the protected root/record under the assumed OS access checks.
Its expected file identities come from a parent-only constructor observing real
handles, not its own JSON/hash. Another ordinary process cannot reuse the record
because actual child PID+creation time and SID differ; the parent retains the
original process handle to prevent PID ABA. The parent's actual Job observation
is a parent assertion authenticated by protected custody, NOT a receiver-native
Job query. If that delegated assertion is unacceptable, A must be rejected and
an authenticated live IPC/duplicated limited-rights Job query protocol specified
separately. FileID does not authenticate producer PID or GitHub by itself.

This proposal needs safe process creation-time/query and pinned executable
association APIs reviewed before implementation. The file-query exception above
does NOT authorize additional process/token/Job FFI. Any missing safe process
primitive requires a separately enumerated exception amendment, never silent
extension. Source/executable pins must bind the actual launched image; reopening
an image path after process launch is not sufficient. Until both are resolved,
require_exact_producer_identity stays Unsupported.

## Implementation and actual hosted gates

1. Root accepts/rejects A and B; promote accepted normative slices to owning SoTs.
   Freeze exact API, process/image primitive review and safe sharing sequence.
2. Implement query crate and pure error/codec tests; allocated offline all-target
   check/strict Clippy, dependency closure and unsafe-site audit. No actual account.
3. Add opaque verified carrier+association owner, borrowing existing actor/process
   owners; integrate consumer before UnverifiedHostLaunch upgrade/Command. Keep
   original parent monotonic watchdog, one budget, no per-profile renewal.
4. With C explicitly allocated on genuine hosted Windows, observe positive
   elevated parent -> fresh reserved ordinary child -> actual local NTFS same
   opened IDs -> exact binding -> trusted harness guard -> approved prebuilt
   apphost -> bounded pipes/natural exit/Job-empty/reader joins. Apphost receives
   fixed environment only, neither parent locators nor GitHub/token variables.
5. Run separately labelled actual negatives: ordinary actor forged/copied record,
   different SID/nonce, same SID different process, stale process creation time,
   different context object with identical bytes, root substitution, hardlink
   alias policy, rename/delete/replacement attempt under retained pins, changed
   length/bytes, expired original budget, terminated parent and partial publication.
   Query/provider/invalid-handle fault seams are component tests, not actual ReFS
   or remote-volume tests. Actual ReFS/SMB rejection needs those real providers
   or remains explicitly NOT_RUN. No third-party endpoints are involved.
6. Every negative must prove no Command/Ready, preserve the actual owner and fit
   original cleanup deadline. Report exact tests/ignored/filtered counts and
   source/lock/tool/config/registry hashes before+after. Explicitly settle Job,
   readers, handles, three metadata files, empty owned root, profile residual,
   account and scratch; no recursive deletion or Drop=cleanup claim.

## Adversarial disposition

P0: identity is not origin. Separate protected parent assertion and actual child
binding above are required. P0: source/path labels cannot prove launched image;
safe pinned process-image binding remains an explicit prerequisite. P0: no
unsafe exception is authorized by this draft. P1: root/component replacement and
writer-reopen gaps require lifetime ancestor custody and observed object equality.
P1: stale parent/process claims require creation-time/live checks and held handles.
SRE requires bounded query buffers, retained failures and original cleanup budget;
DPO requires static error classes and no raw context/SID/path logging. Evaluator
verdict: needs-work until A/B decisions and remaining process primitives are
resolved; no positive native or hosted result is claimed.

## Primary evidence reviewed 2026-11-07

- [GetFileInformationByHandleEx](https://learn.microsoft.com/en-us/windows/desktop/api/winbase/nf-winbase-getfileinformationbyhandleex)
  defines the handle query and information classes.
- [FILE_ID_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info)
  specifies volume serial plus full128 file identity for comparing open handles.
- [GetVolumeInformationByHandleW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getvolumeinformationbyhandlew)
  queries the filesystem/volume associated with the supplied handle; its persistent
  ACL flag is capability information, not itself a complete provider proof.
- [GetFinalPathNameByHandleW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew)
  defines volume-GUID form, bounded output and failure behavior; this proposal
  combines it with held-object/provider checks rather than trusting a path.
- [GetProcessTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes)
  defines creation time queried from an actual process handle with query rights.

The protocol, trust premise and safety restrictions are proposed design inferences,
not guarantees made by these API documents. No new dependency or code is present.

## Prerequisite survey and custody sequence v0.2

Root accepted A's trust-boundary/protocol direction as DESIGN ONLY. B (unsafe
boundary/graph) and C (actual account/Job tests) remain unaccepted. This section
refines rather than executes that direction.

### Exact existing safe process APIs

Pinned winsafe0.0.29 kernel/handles/hprocess.rs SHA256
2c7928ad1832a7e94a48c2a5117bf874a9c340167f1ffd1bda1e837e44060f5d:

- HPROCESS::OpenProcess(co::PROCESS, false, actual_pid) returns an owning
  CloseHandleGuard<HPROCESS>. Request QUERY_LIMITED_INFORMATION|SYNCHRONIZE only;
  if rights are denied, fail closed. Never enable SeDebugPrivilege.
- HPROCESS::GetProcessTimes() returns creation/exit/kernel/user FILETIME. Compare
  the creation value exactly as 64 bits, not converted wall-clock milliseconds.
  Exit time is undefined for a running process and is not a liveness check.
- HPROCESS::GetProcessId() queries the actual opened process. For self use
  HPROCESS::GetCurrentProcess(); no unsafe raw-handle adaptation is needed.
- HPROCESS::WaitForSingleObject(Some(0)) gives a zero-wait live/exited observation;
  fail on errors/unexpected wait outcomes. Do not interpret exit code259 alone
  as proof of liveness, since an exited process may return that numeric code.
- HPROCESS::QueryFullProcessImageName(co::PROCESS_NAME) returns ONLY an image
  path string, not FileID, section handle, loaded-byte hash or launch attestation.
  This pinned wrapper allocates MAX_PATH+1 once, with no retry. Long names fail;
  reject rather than inventing truncation/normalization equivalence.

Pinned advapi/handles/hprocess.rs SHA256
f13b2c1f2a39b624b8e97582c0f069a7b9d9a5a093c89515198f58e3fe5509fc:
HPROCESS::OpenProcessToken(co::TOKEN::QUERY) returns an owning token guard.
Reuse safe GetTokenInformation User/Elevation/Groups and enabled-not-deny-only
Administrator checks already implemented for the current process. Do not reuse
current_identity() to claim facts about a remote process token. Unsupported
impersonation, inaccessible remote tokens or unknown groups fail closed.

Existing actor.rs raw Handle cannot be safely retyped as winsafe HPROCESS via
from_ptr. Instead extend its private actual-process owner with the kernel-returned
PROCESS_INFORMATION.dwProcessId captured alongside its existing held child HANDLE.
Safe OpenProcess on that internally captured PID is then checked and held; while
the original child remains suspended/alive there is no intervening exit/reuse.
No caller PID constructor or caller DTO is admitted. An observation after the
child has exited is not the suspended-launch proof. Any future cross-API handle
duplication would be a separately reviewed boundary, not an assumed safe cast.

No authored process/token FFI is needed for these observations. The exact safe
API survey found no supported retained image-file/section handle identity getter
in winsafe; GetMappedFileName/GetModuleFileNameEx also return names. Do not claim
that psapi features or NtQueryInformationProcess path results solve object binding.
No undocumented native syscall or process-memory inspection is proposed.

### Actual child image binding under the accepted parent trust premise

Propose PRE-LAUNCH namespace custody instead of a nonexistent image-handle getter.
The trusted parent opens and validates the executable File before CreateProcess,
hashes that actual read handle against the independent expected artifact, and
retains it with FILE_SHARE_READ only. Hold every relevant protected nonreparse
ancestor directory with delete sharing denied; require no actor write/add/delete
rights, local supported provider, fixed absolute application path without ADS,
relative components, search, symlink, mount/reparse crossings or shell expansion.
The parent passes that exact path as NONNULL lpApplicationName to the EXISTING
quarantined CreateProcessWithLogonW call. It retains returned actual child/thread
handles immediately while suspended, obtains safe PID/time/token/name observations,
and reopens the same fixed executable path under custody to compare full identity
and digest. Image path query is only a consistency check after pre-launch custody.

This binds the parent-selected file to the launch by OS path resolution plus
unchanged protected namespace, not by directly querying the process section's
FileID. Existing writable/delete handles or writable mappings must be excluded
by successful restrictive opens and protected provisioning; do not adopt an
arbitrary previously actor-writable tree merely by changing its ACL. IFEO/debugger,
loader policy redirection and trusted-admin modification are inside the accepted
trusted-machine/controller premise, not detected by a path comparison. Actual
pilot must confirm the supported hosted policy. If Root requires section-object
identity independent of those assumptions, this path remains a P0 gap: no
supported safe section-to-file API was found, and no extra FFI is requested to
pretend otherwise. Explicitly accept/reject this custody inference before code.

For the ALREADY running parent, a freshly reopened executable path cannot prove
its original loaded object. Trust its actual elevated primary token under A;
do not add a retrospective source-attestation claim. To require parent launch
image binding too, its trusted supervisor must apply the SAME pre-launch custody
sequence and retain an actual parent owner, or qualify a distinct launch chain.
No root-of-trust regression from this impossibility is hidden in the metadata.

### Concrete writer -> reader transition, without an object gap

Current share=0 writer followed by drop/reopen does not implement this sequence.
Proposed per-file publication after a protected root is established:

1. Create-new writer W requesting read/write and sharing READ only (no DELETE).
   Install/inspect exact role ACL, bounded write, flush, query full opened ID.
2. While W is OPEN, open transitional read-only pin T sharing READ|WRITE, no
   DELETE. This permits W's existing write access. W's READ sharing permits T's
   read access. Validate T's ACL/provider/type/ID against W and read/hash bytes.
3. Close W only after T is retained. T's rights are read-only; its WRITE sharing
   is not write authority. The exact ACL/root custody denies ordinary actor write.
4. While T is OPEN, open final read-only pin R sharing READ only; T has no write
   access, so this is compatible. Compare R/T full identities and byte commitment,
   then close T. Retain R through child/reader settlement. Never close both pins.
5. Any failed step returns actual retained W/T/R and created-path flags; no new
   timeout or implicit cleanup. Concurrent trusted-admin mutation is excluded by
   A, not allegedly prevented during T's sharing window.

The ROOT needs an analogous custody transition without dropping all directory
pins: maintain a read-only directory pin with sharing compatible with the initial
ACL-setting handle, then close the ACL-setting handle and establish final deny-
write/delete pin before publishing files. Every ancestor stays held throughout.

NEW prerequisite found: current fs::create_dir then set_new_acl has a pre-ACL
publication window. Safe winsafe::CreateDirectory accepts SECURITY_ATTRIBUTES,
but the existing windows-permissions SecurityDescriptor is an opaque self-relative
allocation; there is no verified safe conversion into winsafe's distinct typed
SECURITY_DESCRIPTOR. Do not cast/reinterpret it outside an approved boundary.
Do not use empty/default DACL or a guessed nonce as a substitute for access control.

Explicit additional B2 proposal, NOT within the original query allowlist: a
separate create_carrier_directory(path:&Path, descriptor:&SecurityDescriptor)
wrapper calls ONLY CreateDirectoryW with a stack windows-sys SECURITY_ATTRIBUTES
pointing to the validated borrowed descriptor, bInheritHandle=false. It borrows
the LocalBox-backed allocation for the entire synchronous call, owns UTF16+NUL
path storage, rejects embedded NUL and existing destination, performs no owner
repair and returns a created-path obligation before further fallible inspection.
This needs explicit windows-sys Win32_Security feature plus direct
windows-permissions="=0.2.4" in the proposed bridge, and a scope/name adjustment
from query-only crate. No manual descriptor layout, from_ptr ownership or allocator
conversion. The descriptor must include the protected final DACL AT CREATION;
native-security still independently validates native default owner and exact ACL.
Files inherit only Admins/SYSTEM from the protected root at creation, with the
actor's noninheriting root RX entry; actor RO is added before any locator release.
Whether atomic-directory creation can reuse a different existing SAFE adapter is
still reviewable, but the current default-create/repair sequence cannot qualify.

### Additional primary sources and claim boundaries

[QueryFullProcessImageNameW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamew)
documents a name and process query rights, not file-object attestation.
[CreateProcessWithLogonW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createprocesswithlogonw)
defines the explicit application-name parameter.
[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
defines bidirectional sharing compatibility and the handle lifetime of sharing
restrictions. Those semantics motivate W/T/R; actual compatibility, ACL-at-create,
failed-transition owner retention, parent token access and executable custody
remain allocated component/pilot tests, all NOT_RUN in this source survey.

## Exact B/B2 exception surface for Root decision v0.3

Root accepted W/T/R as design under the trusted-admin premise; actual sharing and
retained-failure tests remain gates. fs::create_dir followed by ACL repair is
explicitly excluded. A's direction remains design-only; B/B2/C remain unaccepted.

Proposed paths/manifests, all absent until approved:

- crates/windows-object-bridge/Cargo.toml and src/lib.rs: unpublished cfg(windows)
  bridge; src/query.rs holds only the five listed file-query calls, src/create.rs
  holds only CreateDirectoryW. No process/token/Job calls are added.
- That manifest: workspace version/edition/rust-version/license/repository; direct
  windows-sys="=0.61.2" features Win32_Foundation, Win32_Storage_FileSystem,
  Win32_Security; windows-permissions="=0.2.4" only for typed borrowed descriptor.
  Root verifies feature closure and Cargo.lock. No winsafe/crypto/async edge.
- Root Cargo.toml adds member/exact workspace path dependency if accepted;
  crates/native-security/Cargo.toml adds only cfg(windows) bridge path edge.
  No runner/.NET graph edge; they consume native-security safe owners.
- Owning SoTs rust-migration/native-security must enumerate the authored-unsafe
  exception and rationale. No workspace forbid change and no generated stub.
  New bridge manifest DOES NOT inherit workspace lints; use [lints.rust]
  unsafe_code="deny" and all otherwise applicable workspace lint values copied
  explicitly. Only the six audited private wrappers have allow(unsafe_code).
  All safe public policy/composition code stays denied; outside this new crate
  authored unsafe stays forbidden except the already accepted actor boundary.

Minimal safe bridge signatures (proposal only):

```rust
pub struct OpenedObjectObservation { /* private full identity/provider facts */ }
pub struct UnverifiedIdentityClaim { /* private [u8;24], NOT authority */ }
pub enum BridgeError { InvalidInput, InvalidObject, UnsupportedProvider, OsFailure }
pub fn inspect_opened(file: &File) -> Result<OpenedObjectObservation, BridgeError>;
impl OpenedObjectObservation {
    pub fn same_object(&self, other: &Self) -> bool;
    pub fn unverified_claim(&self) -> UnverifiedIdentityClaim;
    pub fn matches_claim(&self, claim: &UnverifiedIdentityClaim) -> bool;
}
impl UnverifiedIdentityClaim {
    pub fn from_wire(bytes: [u8;24]) -> Self;
    pub fn to_wire(&self) -> [u8;24];
}
pub struct CreatedDirectory { /* private path cleanup obligation, no authority */ }
pub fn create_directory_with_descriptor(
    path: &Path, descriptor: &windows_permissions::SecurityDescriptor,
) -> Result<CreatedDirectory, BridgeError>;
```

24byte unverified identity wire is volume64 little-endian followed by opaque
FileId128 bytes. This explicitly refines v0.1's no-numeric-getter requirement:
wire EXPECTATIONS must be representable, but are a separate freely forgeable
claim type, never OpenedObjectObservation or verified carrier. Same-object uses
both native observations and retained owners in native-security; matches_claim
alone is no authorization. No serde/Debug on native observations/owners, no File
or raw handle getter. Native-security owns role/ACL/provider/profile policy and
strict association codec; a bridge-produced observation alone is insufficient.

Per-call safety obligations, with no unsafe code implemented:

| Private FFI call | Exact obligation |
| --- | --- |
| GetFileType | Borrowed AsHandle remains valid for synchronous call; no ownership conversion; accept only DISK and do not infer locality. |
| GetFileInformationByHandleEx FileIdInfo | Generated FILE_ID_INFO initialized zero, exact size u32, aligned mutable pointer valid for full call; read only after BOOL success; keep full128. |
| GetFileInformationByHandleEx FileAttributeTagInfo | Same borrow and generated correctly sized/aligned FILE_ATTRIBUTE_TAG_INFO; reject reparse; no unchecked output use on false. |
| GetVolumeInformationByHandleW | Same file handle, initialized bounded UTF16 filesystem buffer with length in WCHARs, valid optional scalar outputs/null pointers; interpret only success and explicit terminator. |
| GetFinalPathNameByHandleW | Same handle, initialized bounded owned UTF16 buffer; returned length checked before any slice/string; cap32768, finite retry; zero is failure, required size is not valid output. |
| CreateDirectoryW | Borrowed valid descriptor allocation and owned NUL-terminated UTF16 path live for full synchronous call; generated SECURITY_ATTRIBUTES correctly initialized, noninherit; API only reads descriptor; false yields no created-directory token. |

Descriptor pointer acquisition is a RAW POINTER BORROW, not typed reinterpretation:
`std::ptr::from_ref(descriptor).cast::<std::ffi::c_void>().cast_mut()` supplies the
native opaque descriptor pointer from a valid windows-permissions allocation.
That safe raw pointer expression does not dereference/rebuild a winsafe or
windows-sys SECURITY_DESCRIPTOR, does not create a mutable Rust reference, and
does not transfer/free ownership. windows-permissions documents SecurityDescriptor
as pointer-only opaque type backed by native conversion/LocalBox; the actual
allocation, not zero-size Rust marker storage, supplies Windows-required alignment.
The wrapper accepts ONLY a borrowed descriptor produced by that safe provider.
No zeroed fabricated descriptor, raw-pointer argument or transmute is admitted.
CreateDirectoryW reads the borrowed security descriptor despite the ABI's mutable
void pointer field; no Rust aliasing violation is permitted by writing through it.
If pinned provider source cannot establish this allocation/lifetime contract,
B2 is rejected rather than casting to a guessed typed layout.

Build SECURITY_ATTRIBUTES with nLength=size_of of its GENERATED type, lpSecurity-
Descriptor set to that borrowed void pointer and bInheritHandle=0. Path uses
OsStrExt::encode_wide collected into owned Vec<u16>, rejects NUL before append,
checked bound, appends exactly one terminator. No temporary UTF16 pointer escapes.
Existing directory is failure; no open-or-create authority. On success return the
created obligation immediately, before ACL/provider/open validation in the caller.
Caller retains it on every subsequent failure. No creation of accounts, files,
processes, SCM objects or secrets occurs within this directory-only bridge call.

Capture OS error immediately after a failed BOOL/zero result before any further
FFI. BridgeError exposes static classes, not raw path/SID/context; no invalid output
is returned. Invalid-handle seam exists only in private cfg(test) wrappers and
must not construct a std::File owning an invalid handle. Exact6 call-site audit
and tests cannot expand this allowlist. A need for ANY process FFI or native
relative-open/section query requires a new Root decision.

## Approved source exception and exact graph handoff v0.4

Root recorded B/B2 SOURCE ONLY approval in rust-migration SoT commit
5a64daaf278672605363041ca0fcf5eabc73c1f8 after Authority review. C remains NOT_RUN;
the production producer identity gate remains Unsupported. Native-security caller
integration and all actual ACL/sharing/provider tests are still required. W/T/R
is accepted design, not actual execution evidence.

Authored source now exists in the four approved bridge src files, with exactly
six scoped unsafe wrappers and three pure boundary tests, all NOT_RUN. Raw source
hashes/consumer inventory are recorded alongside the separate retained-capture
handoff in private 20261107141000-windows-runner-retained-capture/source-handoff.json.
No manifest/shared graph/lock/native-security manifest has been written by owner.
Root should install this exact manifest when integrating the accepted graph:

```toml
[package]
name = "humanymous-windows-object-bridge"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish = false

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "=0.61.2", features = ["Win32_Foundation", "Win32_Storage_FileSystem", "Win32_Security"] }
windows-permissions = "=0.2.4"

[lints.rust]
unsafe_code = "deny"
missing_docs = "warn"

[lints.clippy]
all = "warn"
```

Register member crates/windows-object-bridge and, if using workspace dependency
centralization, humanymous-windows-object-bridge={path="crates/windows-object-bridge"}
with a cfg(windows) workspace edge in native-security only. There is no caller
activation in this source handoff. Linux builds see an empty cfg(windows) crate
and no Windows target dependency; Linux success cannot qualify Windows queries.
Use the scoped exception policy rather than lints.workspace=true, which would
inherit the unchanged workspace forbid. Root verifies all feature/lock changes.

Source implementation follows Authority's final delimiters: std last_os_error
immediately on native failure; no separate GetLastError/process FFI; local fixed
drive query stays with existing SAFE winsafe caller. Successful final-path output
requires 0<len<capacity and NUL at len; required-size growth is capped INCLUDING
terminator and retried three times maximum. Provider observation and forgeable
wire identity do not authorize a carrier. Owned path allocation is prepared before
CreateDirectoryW so its successful return immediately yields a created obligation.

## Allocation-bound correction and privacy regression source v0.5

Root and independent Convention review found a real source gap AFTER the14pure
receipt on the earlier create hash: path.to_owned preceded validation and
encode_wide().collect consumed the entire input before checking the32768limit.
The passing receipt remains exact to its old source and never qualifies that
unbounded prevalidation allocation claim.

New source uses encode_wide().take(32768).collect, rejects empty/interior-NUL or
len>=32768, appends one terminator, THEN clones the now bounded input path BEFORE
CreateDirectoryW. This preserves both bounded additional input copying and the
allocation-before-successful-create cleanup obligation. Existing caller Path
allocation is outside the bridge; it is not copied unboundedly a second time.
An additional pure case covers32767ASCII accepted with32768includingNUL,
32768ASCII rejected, and surrogate pairs exactly at/over the UTF16code-unit limit.
No global allocator instrumentation, new unsafe, feature or FFI is introduced.

The wire roundtrip unit test proves no construction privacy property. New Rustdoc
examples separately provide an external positive import/claim roundtrip without
calling a native query, plus an external compile_fail example trying to construct
the private native-observation fields. These need explicit Rustdoc execution and
diagnostic review; an arbitrary compilation failure must not be mistaken for the
intended E0451 privacy error. Four pure bridge cases and both doctests on this
NEW source are authored NOT_RUN. Existing14pure evidence is not silently reused.
