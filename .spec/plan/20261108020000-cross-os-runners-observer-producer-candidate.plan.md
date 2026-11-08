---
id: plan:root/20261108020000
title: Concrete Windows observer producer frontier and hosted ServiceHost admission
status: draft
created: 2026-11-08T02:00:00Z
updated: 2026-11-08T02:00:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/adapter-authority, sot:root/native-security, sot:root/cli-installation]
---

# Delivered code and exact boundary

Root requested independent concrete work while Tools cutover is pending. Three
Rust review candidates are in evidence:root/20261106223747 under
observer-producer-candidate/: mod.rs, windows_process.rs, windows_service.rs.
They implement actual safe OS queries and retained ownership, not an invented
VerifiedObserverPreparation. No common Cargo/live source/compiler/local SCM,
account, service, token mutation or provider operation occurred. No extra unsafe,
new crate, callback signer, public proof field or sibling factory is proposed.
Candidates are Rust1.99.1/edition2024 source for integration REVIEW; formatter,
typecheck, Clippy and execution remain NOT_RUN. No compiled/production-ready claim.

Current native_preparation() has no input and always returns Unsupported; PRIVATE
EMPTY NativeProof remains empty even in tests. Owner construction/advance are
test-only. A token-query result cannot replace this proof. Authorization checks
expected caller DATA only. Runtime prepare_observed is private retention, not a
consuming public admission API. CLI HostAuthorizationData and Session::prepare
are DATA/fixtures, not original installation/channel/W owners. Standalone
ServiceHost::bind cannot become observed-positive admission. CLI reports its
private fail-closed candidate calls native_preparation once before secret/listener/
Ready and retains/cancels any unexpected proof; this is not a positive path.

# Concrete safe modules

windows_process.rs uses existing pinned winsafe0.0.29 kernel/advapi. OpenProcess
returns CloseHandleGuard<HPROCESS>, noninherited QUERY_LIMITED_INFORMATION|
SYNCHRONIZE. Retain that exact process and initial primary token; query native PID,
unscaled creation FILETIME and TokenUser SID, poll WaitForSingleObject(Some(0)),
then recheck through the SAME held process and a newly queried current primary
token. No cached PID reopen, foreign terminate, token duplicate/adjust, image-path
trust or environment/PATH lookup. Own thread-token presence/query uncertainty
refuses; enabled SeDebug refuses without disabling/re-enabling any privilege.

Caller supplies the ORIGINAL Instant deadline. Checks before/after native steps
never create a fresh budget or reconstruct a wire Instant. Synchronous kernel
calls can still block: independent original-budget outer supervision is required.
Private fields/no raw getter/no Clone/Debug/serde prevent wire construction. PID
input and matches_claim output are locator/DATA, not role authority. Repeated SID
queries do not attest loaded memory or privileged token immutability.

windows_service.rs uses pinned windows-service0.8.1 read-only SCM methods: manager
CONNECT, exact owned service QUERY_STATUS|QUERY_CONFIG|READ_CONTROL. Only
OWN_PROCESS+Running supplies a PID locator; pending, stopped, shared, paused and
unknown states refuse. Running means TransportPrepared, never application Ready.
Hold the same Service object. ServiceConfig::executable_path contains SCM's binary
COMMAND LINE as PathBuf, not a parsed verified executable. Exact protected BASE/
argv comparison must reuse the owning parser, never split by whitespace or open
that whole field as a bare path.

Concrete finding: same-service owner/DACL query needs NO new authored unsafe.
A private borrowing AsRawHandle adapter over Service::raw_handle feeds existing
windows-permissions0.2.4 GetSecurityInfo(SE_SERVICE,Owner|Dacl). AsRawHandle is safe;
no BorrowedHandle::from_raw_handle, ownership conversion or CloseHandle occurs.
Service retains its CloseServiceHandle lifecycle. Descriptor DATA still requires
original installer policy. The upstream safe wrapper can panic on successful-null
descriptor; retain fail-closed independent outer process supervision.

mod.rs joins initial SCM config/status, held process and repeated SAME-service
config/status before returning ScmInstanceObservation. Repeated observations are
not an atomic config lock or ABA-proof installation. Prelaunch file/ancestor
guards and native start receipt remain mandatory. Visibility stays within
native_security. No conversion to NativeProof/PreparedObserverSession/Ready exists.

# Precise pending decisions and implementations

D1: Root authorizes Windows-only Runtime native-security edge
windows-service=0.8.1, already pinned elsewhere. No version bump/common Cargo edit
here. Existing WinSafe/windows-permissions suffice for these modules; no Platform
unsafe loan is needed. Wire a real probe caller before strict dead-code checks;
the candidate includes no dead_code allowance or lint exemption.

D2: original installer I -> finite publisher P delegation/initial key pin remain
absent. Require protected installation/SCM owner/BASE/artifact selection, actual
stable P service principal and missing-existing-key-never-regenerates policy.
Descriptor query is not that transaction. Existing WinCred read-service-secret
and synthetic credential-pilot cannot create production issuers. LaunchTicket/
cursor/recovery formats remain proposals: first one-shot hosted attempt needs
explicitly accepted immutable BASE+attempt selection, not an unsigned shortcut.

D3: cross-SID query feasibility requires actual hosted evidence. Microsoft warns
about different-account token queries. P<->S, P<->O and S<->O must each work under
their OWN tokens, exact reviewed query grants and SeDebug disabled. Denial stays
Unsupported. Controller elevation, configured account, impersonation SID,
PID-only claim or same-account success cannot replace the actual edge. Self ACL
query grants are explicit role initialization, never verification-time repair.

D4: existing Authority Windows transport uses one SID/GA and generic std
read/write. Distinct-role endpoints need ONE owning two-principal implementation,
not copied Native transport. Existing interprocess safe options, kernel peer-PID
methods and OpenOptionsExt::access_mode support a candidate exact-rights path.
Proposed client0x12019b contains data/EA/attribute read/write, READ_CONTROL and
SYNCHRONIZE, excludes0x4 CREATE_PIPE_INSTANCE; no GENERIC_WRITE shortcut. Require
exact server owner/full rights, peer client-only rights, first-instance,
local-only, noninheritance and actual peer PID joined to retained primary-token
process. Recheck before every release. Safe integration, cancellation and partial
write behavior remain unimplemented/unqualified. Legacy single-SID verifier must
stay strict, not accept another policy through weakening.

D5: signed Prepared/both-native-acceptances/Commit and one-use process reservation
are absent. Immutable public publication files need descriptor-at-create,
same-held identity/ancestors and finite no-clobber journal custody. Existing six
Platform wrappers do not include file-at-create ownership conversion. Prior B1
proposes exactly ONE private unsafe CreateFileW+OwnedHandle::from_raw_handle scope,
AT-CREATE descriptor/CREATE_NEW/share0; still UNAPPROVED. Safe WinSafe HFILE create
exists but no reviewed std-owned conversion was proved. No post-create ACL repair,
open-existing fallback, bool receipt or generic signing callback. Accepted journal
genesis/Unknown/capacity requirements remain independently mandatory.

D6: W requires actual independently authenticated peer/key/node/writer continuity
BEFORE unseal and through final audit flush. ExternalWitness::new accepts trusted
broker inputs; its key equality does not establish these facts. Separate process/
key alone is not independence. Authority owns Offer/actual ACK and SAME irreversible
epoch. Ready needs final native/liveness check plus bounded nonblocking enqueue
serialized with revocation. No lock spans await/query/flush/audit. WouldBlock waits
outside gate then reacquires/rechecks; Partial/Unknown revokes and retains owners,
never resends or infers no bytes. These are missing owning implementations, not
fields to fabricate in the candidates.

# Actual hosted Windows test plan

All pending. Genuine disposable Windows2025 x64 actor, exact immutable source/run/
artifact admission independent of CI environment claims. Controller retains own
cleanup/manager delegation; actual P/S/O tokens execute the edges. No local SCM,
accounts, ACT or foreign process operation is authorized. Original-budget outer
Job/lifecycle ownership stays separate from labels/PIDs. Root allocates all slots.

1. After D1/path loan: fmt FIRST, locked scoped check/strict Clippy; list THREE
   candidate tests. Zero PID and expired deadline refuse before native open.
   actual_self_observation_retains_handle_and_rechecks_primary_token performs real
   self queries under ordinary non-SeDebug token. SELF success is not cross-SID,
   SCM owner, publication or production loader success. No ignore/privilege fallback.
2. Reuse CLI's existing hosted native actor and fresh owned service namespace,
   protected scratch and explicitly source-bound executable selection. Run SCM
   query/descriptor and process join UNDER EACH P/S/O identity, recording all SIX
   directed query edges separately. Permission denial is unavailable, not skip/pass.
3. Wrong SID/creation/grant/BASE, retained process exit, SCM stop/restart, pending/
   shared state and config/artifact/ancestor replacement refuse. Held old process
   remains dead despite name/PID reuse. Never force reuse by killing foreign PIDs.
   Deterministic held-old-instance case required; opportunistic reuse is separate.
4. After D2-D6 implementations: actual trusted install, stable P store, both native
   claims, immutable Commit, authenticated O endpoint, independent W, unseal,
   writer preparation, Offer/ACK, atomic Ready. Assert no secret/origin/Ready before
   each gate. Missing observer/ACK/W/key, replay, interrupted publication, blocked
   I/O and peer loss revoke on original deadlines, retain uncertainty, never retry.
5. Positive ServiceHost pilot correlates independent public readiness with exact
   runtime/Authority epoch/W facts. Authenticate management status, one protected
   request, first loss and closed new effects. Retain W through drain/final flush;
   separate authorized SCM stop/remove. Status credential grants no stop/kill/
   erasure. Actual exits, Job-empty, worker joins, audit flush, cleanup/restored
   delegation are separate facts. Forced timeout retains unproven74 convention.
6. Restart/upgrade/rollback and dual-control administrative gates remain required;
   reject old publication without custody reset. Warm restart is not cold boot.
   Linux/macOS provider/manager/channel stay Unsupported pending own code/venues.
   Hosted native components cannot clear Docker FullProduct unavailable.

# Source ownership loan requested from Root

Minimum first loan: NEW
crates/sdk/humanymous-runtime/src/native_security/process/observer/** plus ONLY
module declaration in native_security/process/mod.rs. Root keeps Cargo/feature
edits and approves D1. No whole Runtime/Platform/CLI ownership requested.

After primitive probes/joint interface acceptance, SEPARATE loan for private
authority/startup/observer/native/** and reviewed observer.rs/state.rs, owned by
Root/Authority. Producer stays below private observer boundary; no sibling factory.
Runtime owner supplies real consuming observed-admission API; CLI wires checked
DATA to it without standalone bind fallback. CLI/Delivery separately allocate
hosted fixture/actor/workflow paths. D4 transport/D6 W/Ready stay in owning contexts,
not duplicates in native_security. Native's current claim is spec/evidence only.

Verdict: safe query modules READY FOR INTEGRATION REVIEW; actual production
producer/ServiceHost NEEDS-WORK on D1-D6. Independent candidate progress does not
mark the whole provider/journal goal blocked. No proof-field forge, test factory,
unsafe expansion, install-trust weakening or dual-control exception accepted.

Primary references checked:
[OpenProcessToken](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocesstoken),
[SCM status/PID validity](https://learn.microsoft.com/en-us/windows/win32/api/winsvc/nf-winsvc-queryservicestatusex),
[named-pipe specific rights](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights).
Pinned API/current-source hashes: observer-producer-candidate/source-inputs.json.
