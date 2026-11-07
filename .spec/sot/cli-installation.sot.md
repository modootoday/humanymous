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

## Active ServiceHost integration tranche

The explicit CLI goal remains active through signed deployment, native services,
protected unattended input, ServiceHost supervision and current-source hosted
qualification. CLI owns crates/cli only; the root owns Cargo/lock/toolchain and
dispatch. The native-cli work lane is an operational CLI ownership label, not
permission to edit the browser/framework crate crates/native-cli.

The runtime-service feature links the existing runtime ServiceHost and authority
credential types instead of duplicating request/audit/custody logic. A signed
runtime host-plan binds service identity, generation nonce, executable digest,
non-secret configuration digest/path, typed credential selectors, diagnostic
readiness and stop budgets. Host-plan bytes are SHA256-pinned in signed launch
arguments. The configuration file must have an empty unseal passphrase; only an
authorized OS-secret provider fills its existing Zeroizing field in memory.
There is no plaintext passphrase file, argv/env secret or interactive fallback.

The OS-secret reader is native-security/Windows owner responsibility. The CLI
owns selector validation and consumes the provider's zeroizing result; runtime
owns bounded configuration validation and existing sealed-custody consumption.
Missing/locked/unsupported/unqualified providers are errors, not an empty secret.
Current service principal must be tested; an installer actor's store is not a
virtual service account's store. Private inherited input additionally needs real
pipe type/locality/owner evidence; a file/DACL check alone is insufficient.

Shared runtime StopBudget17ad9e6 validates strict drain+supervisor reserve less
than actual manager stop time. CLI must consume it and enforce an independent
native watchdog deadline for synchronous custody/read/flush; an async timeout
alone cannot preempt those calls. On expiry the owned host exits nonzero and
must not emit a successful stopped/drained/flushed report. Windows SCM Stop and
Unix termination map to the same channel. Success requires runtime ExitReport's
actual stopped/drained/audit_flushed values, not receipt/loopback readiness.
The startup bound remains armed until synchronous readiness publication returns,
including native SCM status or nonsecret output flush. Listener bind alone cannot
disarm it. If stop arrives during publication, publication completion preserves
the stricter existing deadline instead of clearing or extending it. The watchdog
never holds its state lock across the potentially blocking publisher call.
Final successful foreground JSON delivery, including synchronous serialize/write/
flush, likewise runs before Stopped/disarm and under the existing stop deadline.
Windows final SCM Stopped publication uses the same completion ordering. Delivery
does not acquire the supervisor state lock or obtain a fresh deadline. Validation
rejects an unproven phase/drain/flush before emitting a successful final report;
delivery failure cannot commit supervisor Stopped. New regression and ordering
changes require their own source-bound gates. Validation and completion each
check the original monotonic deadline under the state lock; completion at or
after expiry rejects without committing Stopped, even if the watchdog thread has
not yet run. The hard-expiry branch performs no stderr/log write before exit74.
Deterministic pure tests inject expiry before and during delivery; actual blocked
stdout/stderr process-boundary qualification remains pending and is not implied
by scoped compile/pure/Clippy results.
Observer management e6dd38d uses the existing authenticated native transport and
independent Credential; it is not exposed through browser/application routes.

This tranche is authored/NOT_RUN until root grants bounded pure-unit Cargo and
current-source hosted native-process/service validation. Prior fixture checks
and remote42348518353/63f69ae do not qualify the new source.

Host-plan schema `humanymous.runtime-host.v1` requires service_name, installation,
generation, executable_sha256, configuration path/configuration_sha256,
unseal_credential_id, observer (null or typed endpoint/peer/credential_id), numeric
loopback diagnostic, ready_response, startup_ms, stop and mode. Stop contains
drain_ms/supervisor_reserve_ms/manager_stop_ms; the current manager contract is
6000ms on Windows and5000ms on Linux/macOS. Schema/duplicate/unknown fields and
noncanonical selectors reject. Both SHA256 fields and the outer host-plan digest
are mandatory. Executable/plan metadata reads also have a5second independent
preflight deadline before the plan-controlled startup watchdog begins.

Windows additionally requires installer_owner_sid in the digest-pinned launch
document. The actual virtual service principal reads that bounded nonsecret
document and verifies its original installer owner plus the exact shared native
Executable read/execute grant against the same opened non-reparse handle. This
role is used only for the immutable launch document; it grants no write access.
Runtime configuration instead requires the current service's strict owner-only
File policy, including inherited private-file ACLs. Installer-only token checks
must not be reused as service-input authorization. Trusted ancestor ownership
and same-principal replacement remain explicit premises.

Returned shared ExitFailure facts are preserved in the bounded failure envelope:
stage, drained and audit_flushed remain independent. Other failures report unknown
phase facts, stopped=false and product_qualified=false without serializing their
arbitrary error payload. Stop channel replacement remains valid after a returned
runtime drops its receiver, so that closure cannot replace the original typed
failure or suppress the watchdog wakeup. Windows SCM still reports nonzero exit;
recoverable Windows failure-envelope transport remains a qualification dependency.

CLI consumes native-security::secret::{ServiceSecretReference::new(service,id),
read_service_secret} returning Zeroizing bytes. Shared selectors restrict actual
owned service namespace and credential IDs. Windows requires actual TokenUserSID
equal the resolved virtual service SID and existing Local-persistence metadata;
an installer user's store is not a service store. Non-Windows providers currently
return Unsupported. Native Windows execution must use SCM; foreground mode cannot
silently launch protected service code outside that dispatcher. No arbitrary
stdin redirect is admitted as a private pipe based on a DACL check alone.

The CLI calls shared ServiceHost::bind/bind_with_witness and, with an authorized
PrivateObserverProvisioner, bind_with_management. Observer credentials are freshly
generated using the shared Credential type and delivered only through that trusted
sink after the native listener binds. The ordinary command has no such sink and
explicitly refuses observer configuration rather than reusing a stored process
epoch or emitting its credential in public readiness. This production observer
provisioning connection remains open. Required witness absence rejects before
retrieving any unseal secret. Public readiness and exit envelopes retain exact
generation/executable digest and product_qualified=false; no secret is serialized.

The hosted runtime-service-pilot feature is separate from production runtime-service
and may enable only native-security's credential-pilot and existing audit fixture
APIs. Proposed DisposableServiceContext(service,run_id,full_source_sha) is a
nonsecret syntactic marker, not cryptographic hosted attestation. CLI pins that
metadata in signed fixture launch input; root's external run/source/artifact/actor
evidence remains mandatory. SCM does not inherit a job's environment by assumption.
The actual service principal creates a random unique test credential, reads it
through the production reader, initializes only new fixture custody, runs the real
ServiceHost and cleans its own fixture state/credential after custody drops.
Production persistent custody is never removed on ordinary stop.

An actual runtime lifecycle qualification must cover install, upgrade, known
predecessor rollback and removal through ServicePlan with real ServiceHost and
protected input; generic diagnostic-fixture transitions are separate evidence.
Retained signed generations may run again, so a disposable pilot receipt uses
the signed bounded base component followed by the independently observed actual
process PID. Every write remains create-new. Parent receipt binding includes the
signed nonce/source/run/generation/executable and exact observed PID, plus native
stop status and separate credential/state cleanup. PID reuse or missing receipt
fails qualification rather than overwriting a previous instance's record. The
rollback replay floor must remain at the highest accepted sequence, and a refused
replay must leave the running PID unchanged. No production custody is deleted.
The private final receipt also retains the actual complete nonsecret HostReadiness
envelope. For every positive instance the parent compares the entire runtime
listen/profile value with shared service::standalone_profile applied to its
independently reconstructed signed fixture input. Subset capability checks or a
ready boolean cannot replace that comparison. Independent native PID/status,
loopback listener and no-origin-effect checks remain necessary: an expected
profile is not runtime attestation by itself. Missing-secret startup retains a
null readiness manifest, not a fabricated expected one.

WinCred has no atomic create-only or compare-and-delete primitive in the selected
safe API. A cooperative mutex cannot exclude arbitrary same-principal writers.
The pilot therefore requires exclusive disposable service context, a unique random
ID and explicit existing-entry/metadata/value checks; unknown or changed entries
reject cleanup, and no general atomic no-overwrite guarantee is claimed. The native
owner's create-new private-directory factory validates the activation parent and
initializes only the new child's protected service-owner ACL. Existing children
are never repaired; parent-owner replacement resistance is not claimed. Strict
custody/WAL children require their own protected service-owner parents. Cleanup
runs under that same service principal; takeown or ACL repair cannot fake success.

## Accepted execution-policy reference

The user subsequently authorized actual local Windows/WSL build, test and runtime
measurements against GitHub timings/resources. This supersedes the earlier
hosted-only blanket below; root owns the measured replacement policy. During
root's benchmark quiet window no new Cargo/native/runtime/model/Docker validation
commands may start. Existing owned commands finish and their slots are returned;
no foreign process is terminated. Coding/documentation may continue in owned paths.
The CLI goal's specific prohibition on changing local user services/accounts still
applies unless explicitly revoked. Owned standalone runtime measurements do not
authorize SCM installation or account mutation by inference. Product e2e remains
Docker-only, and no idle pipeline may be self-allocated.
Local container execution follows the Docker engine identity section of
[[sot:root/rust-migration]]: Windows Docker Desktop is the default, with explicit
desktop-linux selection and observed engine identity. No implicit WSL-daemon
substitution is allowed. This does not change real hosted Linux engine identity.
Root subsequently allowed lightweight fmt, scoped Specfold checks and owned Git
transactions during the quiet window; Cargo/runtime validation still requires
the allocated benchmark or validation slot.

The single execution-policy canon is [[sot:root/program-roadmap]]; workflow
implementation and evidence rules are [[sot:root/delivery]]. CLI does not define
a separate scheduler, quota, artifact-retention or impact-classification policy.
Under the earlier policy, local CLI work was source editing, fmt, bounded scoped Clippy and genuinely pure
signature/schema/parser/state-decision unit checks under the common Cargo limit.
Actual filesystem permission/ACL, process/port/IPC, service-manager and native OS
tests previously required the appropriate public standard hosted runner, with capability and
ordinary-actor prerequisites proved rather than inferred from its label.

Existing service state-machine tests use a FakeManager plus real loopback Probe,
temporary files and Windows role ACLs. They are hosted component tests despite
their Rust unit-harness location; default whole-package tests are not a pure
local fastcheck. The positive native-service case, protected credentials/runtime
ServiceHost connection and six-tuple qualification remain not_run/unqualified
until current-source hosted execution supplies actual evidence. Earlier Windows
results retain their original source hashes and do not qualify later changes.
Only sanitized CLI summaries/logs/hashes may be recovered publicly; missing
required private raw evidence must be disclosed. Human/AT/IME and other special
environment evidence cannot be replaced by a hosted service fixture. Required
gates remain in force while impact classification is unfinished.

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

The current shared API is service::read_configuration(impl Read), async
ServiceHost::bind(Configuration), readiness() and run_report(watch receiver of
Option<ShutdownRequest>). ExitReport reports stopped/drained/audit_flushed;
ExitFailure preserves returned drain/flush facts independently. The active
integration tranche above consumes these APIs with verified executable/generation
metadata and native stop supervision. Its source and hosted fixture are authored;
current-source actual runtime qualification remains open. The historical generic
service fixture has no product credentials and cannot qualify those gates.

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
