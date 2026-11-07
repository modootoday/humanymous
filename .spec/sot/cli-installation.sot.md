---
id: sot:root/cli-installation
title: Explicit CLI provisioning boundary
status: active
created: 2026-11-07T00:00:00Z
updated: 2026-11-07T00:00:00Z
verify:
  - cargo test -p humanymous-cli
rel:
  governs: [crates/cli/**]
---

# Contract

## Signed service deployment v1

New service-* commands verify a detached strict Ed25519 signature over
`humanymous.release.v1\0` followed by the exact UTF-8 descriptor bytes. The
operator independently pins the public key. A descriptor binds schema, version,
positive monotonic sequence, native OS/architecture, artifact SHA256, owned
`humanymous-lab-` name, argument array and loopback diagnostic readiness address
and exact response. Unknown/duplicate fields, wrong target, invalid signatures,
path/link collisions and mismatched bytes fail closed. This does not substitute
for release/OIDC provenance, revocation or a trusted key-distribution channel.

Service deployment requires an existing private user-owned root. Planning is
read-only and creates no lock. Mutation requires explicit --activate. Immutable
generations use the existing installer; OS registration is a separate context.
Linux uses runtime-only systemd user units; macOS uses user launchd registration;
Windows uses a demand-start own-process virtual service account and explicit
service-role ACLs. No implicit elevation, account password, autostart enablement
or general system-service management is permitted. CLI never creates a host user.
Service artifacts must implement the declared native service protocol; existing
Core/Gate stdin runtime is not silently treated as service-compatible.

State persists signed current/previous generations and a high-water sequence.
Upgrade rejects lower/equal sequences; explicit rollback selects only its known
signed predecessor and retains the high-water mark. Unknown services/configs and
changed installed files block mutation. A create-new lock and pending record
serialize writes; crash residues fail closed rather than guessing recovery.
Failed readiness removes the new registration and restores the old generation;
cleanup/restore failure is reported and retains pending state. Remove first
verifies all known files and only removes owned registrations/generations.
Power-loss atomicity, concurrent same-owner hostile mutation, and automatic stale
lock recovery are not claimed. The operator owns the private root exclusively.

Readiness is a bounded loopback diagnostic plus native service state. It grants
no management authority and does not prove IPC peer, browser qualification,
complete product health or credentials. Never put secrets in descriptor/argv/env.
Actual runtime qualification additionally requires ServiceHost private input,
sealed credentials, authenticated peer and graceful drain integration. All
reported service fixtures retain product_qualified=false. OS pilots are explicit
opt-in in a disposable owned test context; unsupported/blocked is not success.

Official API references: [systemctl user/runtime](https://freedesktop.org/software/systemd/man/latest/systemctl.html),
[Apple user agents](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html),
[Windows services wrapper](https://docs.rs/windows-service/0.8.1/windows_service/).

The legacy provisioning-only contract below remains valid for non-service commands.

### Exact signed schema and command compatibility

The descriptor is a JSON struct (not an intermediate map); every field is required
and unknown or duplicate fields are errors. Maximum exact document size is16384
UTF-8 bytes. Signature is128lowercase hex characters; independently supplied
trust key is64lowercase hex characters. Signature input is domain bytes plus
exact document bytes, including any whitespace; no canonicalization is invented.

| Field | Type and accepted boundary |
|---|---|
| schema | literal `humanymous.release.v1` |
| version | nonempty string ≤64bytes, no controls; display only |
| sequence | positive u64; upgrade must exceed local highest accepted sequence |
| os, arch | exact native Rust OS/architecture names; Windows/Linux/macOS only |
| artifact_sha256 |64lowercase hex characters; artifact ≤256MiB |
| service_name | `humanymous-lab-` plus1..48 lowercase ASCII/digit/hyphen characters |
| arguments | ≤32tokens, each≤1024bytes without controls; no secrets |
| readiness | numeric loopback SocketAddr with nonzero port; no DNS |
| ready_response | nonempty ≤512byte control-free exact diagnostic response |

`service-plan`, `service-install`, `service-upgrade` take --manifest, --signature,
--trust-key, --artifact, --destination. Mutators require --activate. --dry-run is
read-only and mutually exclusive with --activate. `service-doctor`,
`service-rollback`, `service-remove` accept only --destination and --trust-key;
rollback/remove also require --activate or support --dry-run. Duplicate options
and ignored extra options fail. Doctor performs actual native manager and
loopback diagnostic queries; dry-run never performs either. The legacy unsigned
doctor/install/rollback commands retain their original provisioning semantics.

State `humanymous.deployment.v1` contains schema/trust_key/highest/current/
previous/retired. Each generation carries exact signed document/signature,
sequence/digest and optional native identity(configuration fingerprint/service
SID). At most16known generations are retained; later upgrades refuse rather
than silently deleting history. Generations contain the existing install receipt
and artifact (`artifact.exe` only for new Windows service generations; legacy
Windows provisioning retains `artifact`). Unix definitions are private files in
separate generation-specific definition directories. `deployment.next` uses
create-new then same-directory rename. `pending.json` records operation only;
it contains no credential. Unknown ACL/content/registration, pending residue,
stale lock, unavailable manager, deadline or cleanup failure returns nonzero.
Diagnostic strings are contextual I/O errors, not a stable machine error enum.
Success reports are versioned JSON and always contain product_qualified=false.
The local sequence floor is not an independently durable anti-rollback service.

### Cross-chat interfaces and acceptance status

Windows runner/native-security owner accepted and implemented (26d9925) these
Windows-only APIs: `storage::ServiceRole::{Executable,StateDirectory}`,
`grant_service_access(&Path,&str,ServiceRole)`,
`revoke_service_access(&Path,&str,ServiceRole)`,
`verify_service_access(&File,owner_sid,service_sid,ServiceRole)` and
`resolve_service_sid(service_name)`. All return io::Result; role grants use the
actual resolved virtual-service SID and preserve existing strict profiles.
Executable gets only RX; StateDirectory is an activation handoff directory with
Modify, never strict audit/custody storage. A service must create its own
owner-only protected custody child before strict runtime startup. CLI currently
uses Executable only. This boundary is implemented and Windows fixture-tested;
SCM virtual-account execution still needs the dedicated actor pilot.

Windows positive service pilot needs an ordinary, non-admin installer token with
SCM CREATE_SERVICE delegated by a separate disposable-VM controller. Merely
running the pilot elevated is incompatible and must fail the existing installer
policy. CLI does not elevate, create a host user, weaken installer permission
checks or use LocalSystem. It creates only its own demand-start virtual-account
service. Exact native identity/configuration is queried through windows-service
0.8.1; unexpected manager changes block teardown or replacement.

Runtime owner accepted the proposed (not yet CLI-integrated) API:
`ServiceHost::read_configuration(impl Read)` consumes a bounded Zeroizing private
frame; async `ServiceHost::bind(Configuration)` owns sealed startup/listeners;
`readiness()` returns typed `humanymous.runtime.readiness/0.1`; `run(watch receiver
of Option<ShutdownRequest>)` returns ExitReport. ShutdownRequest declares bounded
drain_timeout_ms; readiness reports listen/profile/listener_bound/audit_durable/
product_qualified=false/state/nonsecret policy commitment; ExitReport reports
stopped/durable/drained/interrupted. CLI will add verified artifact digest and
generation nonce outside the runtime report. Secret input remains inherited
pipe or OS-protected credential/authorized local IPC, never argv/env or a new
plaintext file. SCM Stop/Unix termination must become the runtime shutdown
channel. Persistent credential bootstrap and graceful runtime drain remain open;
the current fixture has no product credentials and cannot qualify these gates.

Delivery/Windows owners received the exact opt-in Rust `service_process` pilot
commands. Normal Windows pilot has two actual process/refusal cases; ignored
native lifecycle case is a separate unexecuted denominator until a disposable
manager actor runs it. The report schema `humanymous.cli-service-pilot.v1` binds
os/arch/artifact_sha256 and distinct case/status plus native_service_registered,
activated/readiness_verified/upgrade_verified/rollback_verified/cleanup_verified
as actually observed. Neither mock manager unit tests nor ordinary SCM denial
may become positive service-install evidence. Linux/macOS real results and six
native tuples are CI owner follow-up, not inferred from Windows compilation.

The CLI provisions a caller-supplied regular artifact after verifying its SHA-256
against an independently obtained digest. It does not claim that this artifact is
a working Core/Gate server. It never executes the artifact. Doctor reports native
OS and checks the same prerequisites as installation. No network operations occur.

Install requires an absolute destination directly below an existing private,
caller-controlled parent, a regular non-linked artifact, its expected digest and
explicit acknowledgement that local management binding and IPC ACLs of the artifact
were independently verified. This acknowledgement is a deployment precondition,
not proof of runtime conformance. Windows UNC, device paths and non-fixed drives are rejected before filesystem reads. Unix mounts are an operator-controlled local-filesystem prerequisite; the portable implementation cannot prove mount locality. No default server configuration is invented.
Windows uses the shared native-security crate for read-only token, drive and
handle/DACL inspection; no PowerShell/.NET helper or shell fallback is used.
Unknown native queries and impersonated threads fail closed. Linux/macOS require parent owner matching the process and mode0700(owner read/write/execute,
no group/other access). Windows requires protected DACL with allow entries restricted to current
user, SYSTEM and Administrators; elevated tokens are refused. Other OSes fail closed.

Dry-run performs reads and prerequisite diagnostics only: no mkdir, write, lock,
network, privilege changes or service changes. Actual provisioning creates only
the destination, artifact and fixed receipt. Existing destinations are accepted
only when their complete content matches the requested installation. Files are
never overwritten. Rollback removes only an exact verified managed installation;
changed/extra files block deletion. Installation failure removes only files created
by this invocation and reports cleanup failures. A crash may leave a partial tree;
it is deliberately not accepted as a completed install.

Threat review: private parent control is mandatory because portable path checks
cannot defeat concurrent hostile replacement of directories. No symlink/reparse
point in any traversed path is allowed. Concurrent commands in a trusted parent
may fail safely; create-new and create-dir arbitrate ownership. Rollback assumes
exclusive administrative access during its transaction. Crash-atomic removal and
power-loss durable installation are not provided. System service registration and
privilege elevation are not implemented and are rejected, including explicit flags.

## Acceptance and residuals

Unit tests cover digest mismatch, containment, collision, mutation, repeat install,
rollback and partial write cleanup. Windows temp-fixture pilot must show unchanged
dry-run snapshots and repeat/rollback. Linux/macOS pilots, runtime binding/IPC
conformance, signed artifact release provenance and actual workspace checks remain
integration gates; a local fixture is not deployment/e2e authority.

Official implementation references: [Rust filesystem race guidance](https://doc.rust-lang.org/stable/std/fs/),
[Get-Acl](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-acl?view=powershell-7.4),
[ACL inheritance rules](https://learn.microsoft.com/dotnet/api/System.Security.AccessControl.FileSystemAccessRule).



## Native implementation follow-through

humanymous-cli owns installation orchestration and explicit transactional files.
Its installation/permissions context delegates native facts and role-specific
DACL semantics to humanymous-native-security. Installer SYSTEM/Administrators
exceptions do not apply to audit/custody profiles. Parent owner full self/child
rights and protected DACL remain mandatory. Null/empty/unknown/deny ACE forms
are refused. GetDriveType proves only its native fixed classification, not
hardware identity. The actual process token must not be elevated or have enabled
administrator authority; an impersonated thread is unsupported. Unix geteuid
replaces the id subprocess. There is no authored shell source in crate runtime
or tests. A supplied artifact still is not executed or registered as a service.
The Docker process lab also invokes the actual CLI for doctor, dry-run snapshots,
install/repeat/rollback, digest mismatch and mutation refusal; this remains a
bounded Linux non-root provisioning pilot, not a service-install qualification.


Doctor also rejects a Unix parent with owner read/execute but no write permission
rather than reporting an install prerequisite that cannot be met. It checks
mode0700without performing a probe write; read-only mounts/quota/power-loss and
concurrent same-owner actors remain independent operational premises.
