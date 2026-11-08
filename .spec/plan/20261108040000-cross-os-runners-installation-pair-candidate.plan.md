---
id: plan:root/20261108040000
title: Actual read-only installation and six-edge native query candidates
status: draft
created: 2026-11-08T04:00:00Z
updated: 2026-11-08T04:00:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/native-security, sot:root/adapter-authority, plan:root/20261108023000, plan:root/20261107223000]
---

# Actual code delivered, no live loan implied

Root accepted D2/D3 read-only composition direction and requested ACTUAL code in
owned evidence. Bundle evidence:root/20261106223747/installation-pair-candidate
contains installation.rs, pair.rs, mod.rs and reused a7286fab Windows leaves.
These are Rust1.99.1/edition2024 source candidates, not a new design-only response.
source-inputs.json binds current Runtime/Platform preimages, original a728 leaf
hashes, API sources, candidate hashes and exact six source test functions.
No live/Root-private source, common Cargo/lock or graph was edited. No formatter,
compiler, native query/process, SCM/account/key/provider mutation or hosted run.

Proposed destination is private native_security/process/observer/**. All five
paths are NEW relative to the pinned current Runtime tree; they compose the
existing current_identity/resolve_service_sid/volume policies. This replaces the
a728 observer module composition when Root chooses to integrate, not a second
parallel implementation. Original retained leaf implementation is copied as a
review candidate with explicit small extension, never silently claimed live.

# installation.rs performs real query work

InstallationLocators contains only exact root, three direct-child BASE paths and
P/S/O owned names; no caller SID, PID, key, original-issuer flag or native proof.
Names/BASEs must be distinct. One QueryClock captures the first Instant before
validation/I/O; every file/service/edge shares its original deadline without a
renewal or caller-provided Instant. Cap is an observation-profile DATA input,
positive and at most30s; it is NOT the production original supervisor authority.

The code queries actual unimpersonated current_identity and refuses elevated/
enabled-admin caller; checks explicit local fixed drive; opens every selected
ancestor through nonreparse handles with desired0x20001/share3/noDelete; opens
three BASE Files with desired0x120089/share1/noWrite/noDelete. It checks actual
object type/reparse flags and queries owner/DACL through GetSecurityInfo. BASE
DATA is bounded64KiB, never printed; this is a probe bound, not a changed CLI
plan schema. Files are retained in the owner BEFORE subsequent inspection/read,
so later failures retain them and any partial nonsecret read. Actual root/BASE/
SCM object owner equality is checked. Three actual SCM objects/configurations are
queried through the same held service objects. No configured account is used as
primary TokenUser proof, and binary-command-line DATA is not an opened artifact.

Successful InstallationObservation remains observations only. admit_original
ALWAYS returns OriginalInstallerProducerUnavailable with the actual observation
owner/clock retained. Its OriginalInstallerProof is PRIVATE EMPTY, without test
factory, public constructor, bool conversion or NativeProof link. The independently
protected original-I selection/initial P-store/delegation producer does not exist
in current Runtime and has not been replaced by common-owner equality. No P key
is read/created/regenerated; missing existing P key cannot cause initialization.

namespace_admission_available explicitly returns
HeldNamespaceAdmissionUnavailable: actual FileID/NTFS/provider/link/ancestor
in-place mutation exclusion and prelaunch BASE/artifact/SCM transaction are not
manufactured. Same-handle recheck compares metadata/owner/length only, not content
immutability, atomic namespace identity or complete DACL admission. Merely reading
DACLs does not approve their policy. No production delegation succeeds here.

# pair.rs performs actual caller-specific directed queries

The Edge enum has exactly P->S,S->P,P->O,O->P,S->O,O->S. Each invocation can query
only its selected directed edge under the ACTUAL caller role. Resolve exact native
service SIDs; require P/S/O pairwise distinction; compare current primary SID and
ordinary token policy to selected caller. Require caller's actual SCM RUNNING
OWN_PROCESS PID equals current process ID before AND after peer queries; same-SID
arbitrary fixture/controller is not substituted. It retains caller service/process,
peer SCM/process/token, initial config and actual security observations. Borrowing
the InstallationObservation keeps root/BASE/service handles and original clock.

Peer PID is a locator returned from same held valid-state SCM, never an input
authority. Reused WinSafe leaf retains exact process and initial primary token,
queries native PID/creation FILETIME/TokenUser/liveness, rechecks same process and
fresh attached primary token, rejects own thread impersonation/enabled SeDebug.
The extension requests process0x121000/token0x20008 and obtains SAME-handle process
and token descriptors via SAFE borrowing AsRawHandle -> GetSecurityInfo
(SE_KERNEL_OBJECT). No raw handle getter/ownership conversion/unsafe source.

query_descriptor implements a strict protected query-only DACL checker: actual
target primary SID owner mandatory; owner's full process0x1fffff/token0xf01ff;
optional SYSTEM/admin full baseline at most once each; exactly TWO distinct peer
query grants with masks0x121000/0x20008. No inherited/deny/object/unknown ACE,
duplicate/broad peer/alias or missing peer is accepted. No SetSecurityInfo or
query-time repair exists. This is the proposed narrow D3 query profile, not proof
that existing Windows default process/token descriptors satisfy it. Missing rights
or absent separately reviewed self-query initializer refuses. Actual six hosted
edges and owner baseline feasibility remain NOT_RUN; no controller fallback.

Returned EdgeObservation is NOT RetainedRolePair authority and has no conversion
to DelegatedInstallation, NativeProof, prepared session or Ready. Repeated config/
status observations detect observed change, not ABA-proof locking. Full launch-
file association/original issuer/channel/W checks stay mandatory. Parent failures
retain successfully returned native pins. Inside a leaf before successful return,
safe RAII closes its own partial query handles on error; that is local handle
closure, never process termination, child reap, monitor JOIN or service cleanup.
No stronger universal failure-retention/cleanup fact is claimed.

# Exact unsupported/manufacturer boundaries and integration requirements

- Original installer/held BASE selection authority: PRIVATE EMPTY proof and
  unconditional OriginalInstallerProducerUnavailable; actual observations do not
  fill it. Existing signing DATA verifier is not this producer.
- Namespace/native provider/prelaunch artifact transaction: explicit
  HeldNamespaceAdmissionUnavailable; existing Runtime has no normal Platform
  windows-object dependency supplying that whole capability. No new edge added.
- Role-self exact query ACL initializer/cross-SID feasibility: absent; query checker
  refuses incompatible descriptors, never installs rights or escalates privileges.
- Original independent outer Job/budget owner: absent production manufacturer.
  QueryClock is only one local observation clock. Synchronous calls may block;
  externally allocated hosted supervisor must supply the original outer bound.
- Runtime's Windows-only windows-service0.8.1 edge and actual private probe caller
  still require Root integration. Candidates have no allow(dead_code); strict
  integration needs real caller wiring rather than lint suppression.
- D4 frame schemas/channel/native issuer/W/OfferACKReady and B1/B2 are NOT approved
  by D2/D3 source preparation. No non-Windows backend exists: explicit UnsupportedOs.

# One exact B1 proposal for Root+X, NO unsafe source

Propose this ONE safe-facing Platform windows_object bridge, matching the prior
separate amendment; no generic policy callback or raw handle input:

```rust
pub fn create_new_file_with_descriptor(
    path: &std::path::Path,
    descriptor: &windows_permissions::SecurityDescriptor,
) -> Result<CreatedFile, FileCreateFailure>;
```

CreatedFile privately owns the actual std File plus exact creation locator and
creation obligation; consuming into_file transfers that owner, never O/provider/
ACL/Ready authority. FileCreateFailure privately retains Option<CreatedFile> for
failures AFTER actual creation, plus a static classification. No Drop deletion,
automatic existing-file adoption or successful cleanup claim.

Preconditions: Root+X approve the explicit ONE-wrapper allowlist/opt-in
windows-file-create feature (implies windows-object, default-off); caller already
has the admitted current O and independent original clock/outer supervisor, exact
fixed state slot and held parent/ancestors. Descriptor memory is valid borrowed
native storage for the call; policy is validated by the safe caller, no layout
cast from unrelated WinSafe descriptor. Precompute bounded UTF16/name allocations
before creation, reject embedded NUL/nonabsolute/unsupported locator. No followup
fallible allocation may lose an actual created-file obligation.

Exact proposed private unsafe function allowlist is ONLY:

1. CreateFileW with desired GENERIC_READ|GENERIC_WRITE|READ_CONTROL=0xc0020000,
   share0, SECURITY_ATTRIBUTES pointing to the borrowed descriptor and
   bInheritHandle=false, CREATE_NEW, FILE_ATTRIBUTE_NORMAL|FILE_FLAG_OPEN_REPARSE_POINT,
   null template. INVALID_HANDLE_VALUE is failure; capture safe last_os_error
   immediately. Never OPEN_ALWAYS/OPEN_EXISTING/truncate/retry-new-name/fallback.
2. std::os::windows::io::OwnedHandle::from_raw_handle for that exact successful,
   non-null/non-invalid newly owned disk-file handle, ONCE; then safe File::from.

No GetSecurityInfo/query/read/write/sync, process/Job, ACL repair, privilege change
or CloseHandle call is added to this unsafe scope; safe ownership supplies close.
After return, safe caller performs SAME-file type/reparse/ID/provider/owner/DACL/
link/ancestry checks, journal capacity initialization and required sync. Initial
genesis needs independent O state and cannot be authorized by its eventual target
root ledger. Existing-only recovery is not installer/genesis completion. B1 source
and B2 supervisor wrappers remain UNAPPROVED; this signature is not a source loan.

Hosted B1 cases after approval: peer observer cannot access file at FIRST
observable creation; exact descriptor present immediately, actual same-held ID/
ACL checks; collision preserves existing bytes/owner with no fallback; injected
post-create refusal retains obligation/no deletion; original deadline expiration
does not create another slot. Actual NTFS and independent outer Job required.

# Exact validation plan and evidence boundary

Six source test functions: one original-clock profile test, two new pair tests
(six fixed edges; terminate-right/missing-peer descriptor negatives), and three
retained a728 process tests (zero PID, expired deadline, actual self query). All
NOT_RUN. Descriptor parsing is a synthetic Windows component, not live cross-SID
SCM proof; actual self query is distinct from ordinary pure negatives.

Root first allocates matching isolated integration: fmt FIRST, locked scoped
check/strict Clippy, exact list and these component tests. Only separately
authorized genuine Windows hosted fixture may set role-self query grants and
run all SIX required actual directed edges. Assert wrong caller/controller,
missing READ_CONTROL/TokenUser, SeDebug enabled, pending/shared SCM, old held
process exit and SCM config change all refuse. Independent original outer Job,
restored manager delegation, process exits and service cleanup are separate facts.
Original-I/key-store/namespace/publication and complete ServiceHost Ready/native
journal goals remain OPEN; component success cannot close them or FullProduct.
