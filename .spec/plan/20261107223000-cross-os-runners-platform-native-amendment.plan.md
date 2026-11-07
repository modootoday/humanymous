---
id: plan:root/20261107223000
title: Separate Platform file creation and owned supervisor native amendment
status: draft
created: 2026-11-07T22:30:00Z
updated: 2026-11-07T22:30:00Z
rel:
  depends_on: [sot:root/structural-development, sot:root/cross-os-runners, plan:root/20261107220000]
---

# Scope and accepted first durability profile

Root accepts process-crash/reopen ONLY on admitted local NTFS and cold O restart
RecoveryOnly/inert: no reconstructed Instant/resume/create/delete/new budget.
Stronger power-loss/directory/rollback guarantees remain Unsupported. These are
primitive-profile decisions, not final product acceptance. Installer/genesis and
actual provider still must be implemented and qualified; pre-existing journal
state alone is not a substitute for that deliverable.

This is a SEPARATE proposed Platform allowlist/feature amendment. Existing six
windows_object unsafe wrappers remain unchanged until explicit Root+X approval.
No code/unsafe/Cargo/runtime I/O is authored here. Runtime remains unsafe-forbid;
testing remains unsafe-forbid. No caller bool, raw-handle/PID/wire factory or new
private helper package. Root owns all actual source moves/graph changes.

# A: safe existing API investigation (source-read only)

Read pinned local registry winsafe0.0.29 and windows-permissions0.2.4 source.
winsafe HFILE::CreateFile is safe and accepts Option<&SECURITY_ATTRIBUTES>,
CREATE_NEW disposition and returns CloseHandleGuard<HFILE>. This is PARTIAL
positive evidence, not an end-to-end safe proof. Relevant exact source hashes:

| Versioned source | SHA256 |
| --- | --- |
|winsafe kernel/handles/hfile.rs|5894f4605d650f88242238a33e139514c6c2af3b40ace97815bef856bfba36d1|
|winsafe kernel/traits.rs|f7407754377c56c1d63fc5d8a11ad8dd24a614224873640390c834405cccbd4d|
|winsafe kernel/guards.rs|d11d5d05f29f736827c9dc9c2f9c3f2d4850ddc6998289baf51cf2a813765cf8|
|winsafe kernel/structs.rs|9176ffac71471695d81e925410a9b0154736692c5feb7d55ef92d4ea735a0106|
|winsafe advapi/funcs.rs|c7116d557878305681bc42c5a046afc8a8e47209acb589e18e11fbf5e253a44a|
|windows-permissions structures/sd.rs|1a921bd8f360bc88f4fef9952628a2cca8bd6d72d05a57c8bf95f9aa3455b48f|
|windows-permissions wrappers/mod.rs|d24e0840485d364d9924b63da206d992b8f5ce94175e1fe1845dd0db27e21080|
|windows-permissions windows_secure.rs|442cc3c92c1922930c7094cd9f827a3007f13ac700bc1be4f24f32a39f62c475|

windows-permissions SD is opaque borrowed native storage; winsafe uses a different
absolute SECURITY_DESCRIPTOR with raw pointer fields. A valid lifetime-bound
descriptor adapter would need proof, not a struct cast. No AsHandle/OwnedHandle
conversion for the returned guard was found in that pinned source. Guard::leak
is safe but reconstructing std OwnedHandle/File from its pointer is unsafe.
Current bridge same-held identity/ACL/sync_all path uses std File. No complete
safe direct replacement was proved by this inspection; no such proof is claimed.
No CreateJobObject/JobObject API was found in winsafe0.0.29 source. Existing runner
Job operations are native unsafe quarantined in actor.rs, not a Runtime factory.

Rust1.99.1 [ProcThreadAttributeList](https://doc.rust-lang.org/std/os/windows/process/struct.ProcThreadAttributeList.html)
and [CommandExt::spawn_with_attributes](https://doc.rust-lang.org/std/os/windows/process/trait.CommandExt.html)
remain nightly experimental for raw attributes; no toolchain/nightly escape is
proposed. A stable safe existing upstream path can still replace B if proved with
the complete ownership/descriptor/Job contract; no new dependency is assumed.

# B1: smallest FILE-create amendment

Proposed module remains Platform windows_object, with separate opt-in
windows-file-create feature implying windows-object, default[], no testing/SDK
implication. Reuse current windows-sys0.61.2 Foundation/Storage/Security and
windows-permissions0.2.4; no new package/dependency or Runtime unsafe.

Safe API proposal:
create_new_file_with_descriptor(path: &Path, sd: &SecurityDescriptor)
-> Result<CreatedFile, FileCreateFailure>; CreatedFile has private std File plus
locator/creation obligation, no caller constructor/Clone/serde/raw getter. A
consuming into_file(self) transfers the ACTUAL File owner to the safe journal;
it grants no O/ACL/provider/Ready status. No policy callback or caller native facts.

ONE proposed new private unsafe function create_file_owned, containing exactly
CreateFileW plus std OwnedHandle::from_raw_handle ownership adoption. Precompute
bounded UTF16/owned locator/valid borrowed SD before call. SECURITY_ATTRIBUTES
borrows the existing valid SD only for that synchronous call, inherit=false;
GENERIC_READ|GENERIC_WRITE|READ_CONTROL, share0, CREATE_NEW, normal disk flags with
OPEN_REPARSE_POINT, no template/overlapped/delete-on-close/OPEN_ALWAYS fallback.
Reject NULL/INVALID_HANDLE_VALUE before adoption; capture native error immediately.
On success adopt handle exactly ONCE with no fallible work/early exit between
kernel return and ownership. Safe File::from(OwnedHandle) transfers ownership.
Later validation failure retains actual CreatedFile; never close one owner twice,
open an existing collision or repair its ACL. Native FileID is read on THIS File.

[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
supports descriptor-at-creation and CREATE_NEW refusal. Return of a real File
closes the file create/open identity gap; it does NOT fix the existing directory
bridge's locator-only gap or attest actual supervisor provenance/provider trust.

Installer/genesis route enabled by B1: use an admitted creation-safe O parent
(not necessarily owner-only directory), FIXED protected configured journal slot,
descriptor-at-create O-only FILE, then same-held identity/ACL/provider readback
and synced genesis before any target-root ACK/create. This avoids requiring the
target private directory to exist first. Before call W retains the exact fixed
bootstrap locator/anchor/original budget in its independently supervised attempt.
Existing/torn/unknown slot is quarantined, never a new random journal path or
reinitialized ledger. W restart must keep the SAME protected installer selector;
missing history/slot identity is not permission to bypass old Unknown. Genesis
bootstrap interruption may retain an unknown empty FILE obligation; it is not
silently deleted. Actual installer/W admission and that fixed-slot protocol still
require implementation/review/venue proof, not a caller-provided ACK.
The NEW owned file alone may be set_len(1MiB), synced, initialized with bounded
GenesisIntent+sync then GenesisCommit+sync; never truncate/resize an existing
candidate. Crash before full genesis leaves the same fixed slot quarantined.
set_len bounds logical size, not physical allocation/power-loss durability.

# B2: real owned Job and process manufacture, staged explicit expansion

Proposed independent windows-supervisor feature with Windows-only existing
windows-sys Threading/JobObjects/Foundation features, default[], not testing.
No account/token/SCM/UI privilege APIs. Module deny unsafe, individual approved
function-local allow only. No OpenProcess(PID), OpenJobObject(name), attach to
foreign Job, raw-handle importer, unsafe Send/Sync or generic native-call callback.

OwnedJob stores private OwnedHandle and policy observations. OwnedNativeProcess
stores actual process/thread OwnedHandles and native PID from PROCESS_INFORMATION,
no external PID/assigned bool constructor. Observations mean kernel resources,
not NativeProof/admitted launch. Keep actual Job+process owners on all post-create
failures; termination request/handle Drop does not prove exit or Job-empty.

Exact proposed private native function scopes (10, separate from B1's1):

| Function | Native unsafe operations and lifetime contract |
| --- | --- |
|create_job_owned|CreateJobObjectW(NULL,NULL), validate nonnull, adopt OwnedHandle exactly once; unnamed/uninherited, no existing-name ambiguity.|
|configure_job|SetInformationJobObject with fixed initialized EXTENDED_LIMIT struct, KILL_ON_JOB_CLOSE, no breakaway bits; borrow owned Job.|
|query_job|QueryInformationJobObject ONLY two closed typed classes: extended policy or basic accounting; fixed-size valid output and length checks. No arbitrary class/buffer.|
|terminate_owned_job|TerminateJobObject on SAME owned handle only; request observation, not completion.|
|is_owned_process_in_job|IsProcessInJob borrowing actual owned process+Job, private observation; no caller bool promotion.|
|make_job_attribute_list|InitializeProcThreadAttributeList bounded probe/init, UpdateProcThreadAttribute JOB_LIST only. Bounded aligned storage and owned handles remain live through process creation.|
|drop_job_attribute_list|DeleteProcThreadAttributeList exactly once after last creation use; preserve allocation and borrowed Job lifetime through deletion.|
|create_process_in_job|CreateProcessW with valid initialized STARTUPINFOEX/PROCESS_INFORMATION, EXTENDED_STARTUPINFO_PRESENT and JOB_LIST, explicit executable/bounded command/environment, inherit=false. Adopt BOTH valid returned handles immediately and retain on later failure.|
|poll_owned_process|WaitForSingleObject(handle,0); only signaled then GetExitCodeProcess. Timeout is Pending, failure Unknown; exit code259 can be real terminal only after signal.|
|terminate_owned_process|TerminateProcess on SAME actual owned process handle for retained partial failure only; never PID lookup and never completion proof.|

This is a proposed total of11 new narrowly named unsafe function scopes (B1+B2),
not permission to add them; original six remain unchanged. No Assign-after-spawn
production guarantee: JOB_LIST at creation avoids a child running before Job
association. Microsoft documents
[JOB_LIST](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)
and [Job limits](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
Unsupported nested Job/OS/attribute policy refuses, no breakaway/fallback. Creating
a kill-on-close Job alone does not manufacture an assigned/supervised process.
Existing cooperative runner assignment may remain legacy component code until
replacement qualifies; its barrier proof is not a generic adversarial launch.

Safe facade proposal: OwnedJob::create(), spawn_in_job(&mut self, PreparedLaunch)
-> Result<OwnedNativeProcess, RetainedSpawnFailure>, query/poll/stop borrowing
actual owners. PreparedLaunch is bounded validated DATA until native factory;
source/token/executable/parent/clock admission must be independently discharged
by Root's legitimate producer seam. Native process creation alone cannot assert O.
No pipe/token/credential inheritance in first primitive; output transport needs
separate reviewed explicit handle list if later required, not inherit-all.

Reuse ONE existing lifecycle/clock/retained-failure controller. Root must choose a
sealed backend adaptation for std Child versus these native owned handles; do
not copy testing::OwnedDirectChild/controller state into production or make the
production feature depend on testing. This backend integration is an explicit
source/visibility decision, not an extra permanent helper package.

# Negative and actual venue gates before qualification

File pure/compile negatives: bounded/NUL/UNC/device/ADS input, malformed descriptor
policy, no raw-handle/field construction, no implicit created-success factory.
Actual Windows fixture: same-held FileID/SD readback at creation; collision file/
directory/reparse unchanged; duplicate owner/close error path retained; exclusive
share denial; wrong owner/provider/parent rejects; genesis interruption retains
slot rather than creating a second journal. Native failure injection is labeled
controlled, not proof of every actual disk/driver failure.

Job pure/compile negatives: no raw PID/Job/assigned factory; invalid buffers/size/
attribute counts fail pre-effect; closed flags prohibit breakaway/inherit-all.
Actual Windows GHA job: fixed Rust helper, real PID/owned handles and JOB_LIST
membership observed; real descendant bounded under same Job; forced post-create
query failure retains owner; stop request followed by actual process signal+Job0
within ORIGINAL budget; nested/attribute failure no fallback. Parent independent
watchdog remains required for blocking kernel calls. Foreign processes untouched.

Root+X must approve B1 separately, B2 exact operations/ABI/lifetimes and backend
ownership before source changes; Root/Delivery allocate immutable GHA component
venue and installer genesis afterward. Existing Windows CEF cancellation/native
workspace SKIP gives no such proof. No native I/O/compiler/GHA dispatch here.
The final product deliverable still includes legitimate O/installer association,
native8 families, actual provider/genesis/recovery and consumer integration.
