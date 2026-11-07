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
