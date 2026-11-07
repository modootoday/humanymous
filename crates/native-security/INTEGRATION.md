# Native security integration delta

Existing regular-file installer/audit/custody profiles remain strict and shared.
New Windows-only APIs:

- `process::WindowsIdentity::{elevated,enabled_administrator}` expose observed
  facts without changing installer policy.
- `storage::verify_windows_pipe_privacy<H:AsRawHandle>(&H,&str)` reuses the
  protected owner-only descriptor verifier. Transport independently validates
  actual named-pipe type/server PID; regular-file checks are not relaxed.
- `storage::{ServiceRole::{Executable,StateDirectory},grant_service_access,
  verify_service_access,revoke_service_access,resolve_service_sid}` define
  explicit SCM activation permission changes and verification.

Grant/revoke signature: `(&Path, service_sid:&str, ServiceRole)->io::Result<()>`.
Verify: `(&File, owner_sid:&str, service_sid:&str, ServiceRole)->io::Result<()>`.
Resolve: `(service_name:&str)->io::Result<String>`, accepts only owned
`humanymous-lab-[a-z0-9-]{1,48}` names and queries `NT SERVICE\name` natively.
The service SID must have the exact S-1-5-80 plus five-u32 structure. Existing
owner-private non-reparse objects are required before any grant. An exact existing
grant is idempotent. Unknown ownership/ACL is refused, never repaired. Set and
verify use the same opened handle. Revoke requires the exact prior service grant.

Executable gets only RX; state directory gets Modify+file/directory inheritance.
Owner remains full control; no SYSTEM/Admin exception is added to the emitted
DACL. Strict audit/custody still reject service-granted objects. The activation
state parent is not a ready audit directory: the service account must create its
own owner-private protected child state before strict product startup. CLI owns
SCM registration/dispatcher/signed descriptor/transaction; this crate never does.

Windows9unit/native-file tests and warnings-denied Clippy passed in the local
source snapshot, including actual owned-file grant/revoke, wrong-SID refusal and
ordinary-profile rejection. That ACL test uses a synthetic SID and proves native
descriptor transitions, not an actual SCM service account/process. CLI needs its
own privileged disposable service pilot; Linux/macOS/remote peer remain separate.
No shared Cargo manifest/lock changes are needed for these APIs.

Follow-up locally tested count10 includes actual state-directory role transition
and strict custody rejection; protection-bit extraction is common to all roles.
Grant/revoke now require absolute local fixed-drive paths, matching the installer
boundary; UNC/verbatim/ambiguous/remote roots are refused before mutation.

Protected service input: `secret::ServiceSecretReference::new(service_name,id)`
and `secret::read_service_secret(&reference) -> io::Result<Zeroizing<Vec<u8>>>`.
Windows requires the resolved owned virtual service SID to equal actual TokenUser,
uses an explicit native Credential Manager store and exact target
`humanymous/service/{service}/{id}`, checks existing Local persistence before and
after reading, and accepts1..4096 bytes. Caller must validate UTF8 if needed.
Backend errors are redacted, no UI/argv/env/file fallback, no credential creation.
Non-Windows returns Unsupported. Separate metadata reads are not atomic/CAS and
same-principal concurrent mutation is not excluded. Actual virtual-account WinCred
read remains hosted not_run. Root reported selector/size/metadata pure3pass before
the subsequent directory-factory addition; do not inherit that result for new code.

Windows initializer `storage::create_service_private_directory(parent,child,
installer_owner_sid,service_name) -> io::Result<PathBuf>` requires actual service
principal and exact delegated StateDirectory parent, trusted ancestors and signed
original owner SID. It creates a new canonical child only, checks non-reparse
directory/actual owner, sets protected owner-only OICI via the same handle and
verifies protected and inheritable policies before returning. It never repairs an
existing child or writes secrets. Failure after creation may leave an empty partial
directory; no path is blindly deleted. Parent-owner replacement remains a premise.
Disposable service fixtures must clean their own protected children before exit;
installer cleanup must report denial rather than take ownership or relax ACLs.
No actual local OS initialization/read is authorized. New helper pure/hosted
verification is pending the root-assigned Cargo/hosted queues.

`create_service_private_subdirectory(parent,child,service_name)` accepts only an
already service-owned protected and inheritable parent, then reuses the same
new-child initializer. It does not reinterpret an installer-owned parent as private.
Custody and WAL may use separate protected children; inherited ACL alone does not
establish the required protected bit.

Feature `credential-pilot` exposes `secret::pilot::DisposableServiceContext::new(
service_name,run_id,source_sha)` and `ServiceSecretFixture::create(&reference,
Zeroizing<Vec<u8>>,&context)`, followed by required `finish(self)->io::Result<()>`.
Context validates only nonsecret syntax/target binding, not hosted identity or a
signature; CLI must authenticate signed fixture metadata and external run/source/
actor evidence establishes execution. SCM CI environment inheritance is not
assumed. IDs must be `pilot-` plus32 lower-hex random digits, generated by the caller.
Only the actual virtual service principal may create/read/delete its exact fixture
entry. Existing/unknown entries or changed metadata/secret are refused. Creation
precheck/write and verified delete are not atomic/CAS against another same-user
writer; an exclusive disposable context is an explicit premise. Drop cleanup is
best effort only; finish observes absence or returns failure. Production default
does not enable this feature and the production reader remains read-only. All
new fixture mutation and subdirectory behavior still requires actual hosted proof.
