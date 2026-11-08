---
id: plan:root/20261108023000
title: Concrete D2 through D6 native producer decisions and ownership
status: draft
created: 2026-11-08T02:30:00Z
updated: 2026-11-08T02:30:00Z
rel:
  depends_on: [sot:root/adapter-authority, sot:root/native-security, sot:root/cli-installation, sot:root/cross-os-runners, plan:root/20261108020000, plan:root/20261107220000, plan:root/20261107223000, plan:root/20261107223500]
---

# Decision scope and precedence

Root requested concrete implementation choices for D2-D6, not a live source,
Cargo, compiler or hosted execution grant. This table is an actionable proposal
under the accepted architecture. All proposed types/signatures below are PRIVATE
implementation contracts, NOT existing APIs, proof constructors or public SDK
additions. Their actual producers must perform the stated native operations.
Owning evidence contains only the real query-source candidates from a7286fab;
they remain uncompiled and cannot mint NativeProof.

Canon precedence: adapter-authority B01-B05 plus its L1 authorization-v2 amendment
and exact existing startup/authorization implementation; native-security's
unsafe-forbid/token/ACL rules; CLI DATA-only authorization boundary; Root-accepted
first-loss/Runtime Ready ownership and journal process-crash/reopen/inert-recovery
decisions. Native-bootstrap R1 v0.4 supplies native role/launch proposal details.
Historical signed-grant/1, HMBP0002 and /0.2 are NOT fallbacks: use existing
authorization/2, /0.3 and HMBP0003. Do not confuse earlier R1 draft syntax with
accepted L1 DATA grammar. Root/Authority joint acceptance is needed for NEW
non-Hello frame schemas/phase tags before implementing those frames.

# One concrete signature and owning implementation per decision

Paths below are relative to crates/sdk/humanymous-runtime/src unless stated.
Each Result's failure retains any resources already acquired, first cause and
original budget; it is not a detached io::Error that silently discards custody.
None of these signatures accepts a caller bool, arbitrary provider trait,
callback signer, raw process handle or PID as authority.

| Decision | Proposed single concrete Rust signature | Owner and path | Existing safe APIs / explicit missing boundary | Actual hosted case | Acceptance class |
|---|---|---|---|---|---|
| D2 original I and delegated P | `fn open_delegated_installation(self_role: NativeRoleSelf, selection: HeldBaseSelection, original: OriginalStartupOwner) -> Result<DelegatedInstallation, RetainedInstallFailure>` | Native, private `native_security/process/bootstrap/installation.rs`; original provisioning remains CLI installer-owned | Existing current_identity, same-opened File/ancestor privacy and Platform identity queries; candidate SCM descriptor query. NEW original installation transaction and stable P native-store initializer required; no unsafe inherently needed for these reads. | Install once as I, then P/S/O independently open same protected root; forged owner/key/SCM/BASE and existing missing P key refuse before unseal; synthetic fixture does not establish unattended P key persistence. | Root+Native+CLI routine implementation/interface review within accepted architecture; no Human approval required for read-only implementation. Explicit NEW-install/key mutation is separately authorized in disposable hosted job. |
| D3 six native query edges | `fn observe_role_pair(installation: &DelegatedInstallation, original: &OriginalStartupOwner) -> Result<RetainedRolePair, RetainedQueryFailure>` | Native, private `native_security/process/bootstrap/pair.rs`, reuses `process/observer/**` candidates | Existing WinSafe owned process/token/time/wait, windows-permissions same-object descriptor, candidate safe SCM status/config. Root adds Windows-only windows-service0.8.1 edge. Explicit role-self process/token query DACL setup, never repair during verification. | Each P->S, S->P, P->O, O->P, S->O, O->S under actual caller token; missing rights/debug-enabled/primary-token denial/state mismatch/dead old instance refuse. | Root graph window plus Native/Authority review and separately allocated hosted fixture mutation. No privilege escalation decision or Human confirmation needed for this exact profile. |
| D4 fixed authenticated channels | `async fn open_role_channels(grant: CommittedNativeGrant, original: OriginalStartupOwner) -> Result<OwnedRoleChannels, RetainedChannelFailure>` | Authority, private `authority/startup/observer/native/channels.rs`; native-security supplies process/endpoint policy, existing `authority/transport/windows.rs` remains ONE native transport owner | Existing interprocess local/first-instance/noninheritance/peer-PID methods, std exact access_mode and windows-permissions opened-pipe descriptor. NEW concrete two-principal policy and native joins; no new unsafe identified. No generic stream injection. | Endpoint squatter, remote client, extra-instance attempt, wrong TokenUser/instance/publisher role, generic-write client and partial Hello refuse; correct S/O actual connection succeeds with no secret before both proofs. | Root+Authority+Native routine implementation after exact frame schema freeze. No Human policy choice needed; single-SID legacy policy remains strict. |
| D5 durable reservation and exact create | `fn create_reserved_root<'j>(journal: &'j mut SupervisorJournal, anchor: AdmittedParent, nonce: FreshRootNonce, original: OriginalBudgetOwner) -> Result<JournaledRoot<'j>, RetainedRootAttempt<'j>>` | Native journal `native_security/storage/root_journal/transaction.rs`; Platform owns file-at-create bridge, Root owns supervisor clock/Job seam | Reuse canonical fixed journal grammar, SAME File serial writer, sync_all, approved directory descriptor-at-create and opened-object inspection. B1 missing exact operation: descriptor-at-create CREATE_NEW disk FILE returned as std owned File with creation obligation, no open-existing fallback. ONE new private unsafe scope is only PROPOSED. | Crash/lost ACK between ReserveIntent/Commit/CreatePending/create/terminal sync; changed anchor/nonce/selector/original budget, torn record, second attempt/saturated16 slots refuse. Genesis FILE at-create and same-held identity/ACL tested on real NTFS. | NOT routine source work: explicit Root+X Platform allowlist/feature amendment per plan20261107223000 BEFORE B1 source. Existing canon does not explicitly mandate direct Human confirmation for that delegated review. Without acceptance B1 stays Unsupported. |
| D6 actual W, Offer/ACK and Ready | `async fn activate_observed(owner: ObservedStartupOwner<PreparedRuntimeResources>, session: PreparedObserverSession) -> Result<ActiveObservedHost, RetainedObservedFailure>` | Runtime private `gateway/service/observed/activate.rs`; Authority private observer/native producer owns preparation/session/epoch, Audit owns concrete W authentication/client | Existing Audit verification-only WitnessPipeClient and sealed writer, native retained child/process, existing private epoch retention. NEW native W capability, concrete Offer/ACK receipt producer, monitor cancellation/JOIN, nonblocking Ready enqueue and effect-commit serialization. No new Runtime unsafe or public SDK constructor. | W wrong key/node/writer or dies pre-unseal; lost O/ACK/partial Ready; Ready races loss; real authenticated status/protected request/closed later effects; W remains alive through actual final audit flush and separate authorized SCM stop. | Root+Runtime+Authority+Audit routine private implementation/joint interface review under accepted semantics. No Human policy choice or fabricated preparation. CLI positive entry remains separately reviewed; legacy standalone bind never substitutes. |

# D2 trusted input and exact initial installation choice

Choose the already accepted explicit original installer I transaction, not a
trust-on-first-use role. I's current native identity must pass existing ordinary
installer policy, with existing controller delegation restored independently.
The exact owned SCM registrations and immutable BASE selections are created by
that transaction. Their actual service-object owners and protected root owner
must match I's recorded original authorization. Do not equate CreateService
permission with owner identity; if native owner differs without an accepted
explicit delegation, refuse rather than repair owner or invent a whitelist.

At role start, locate only the fixed installer-selected service/root/BASE via
native service configuration, open/hold all selected objects, and verify their
same-native owners/ACLs/identity relationships before reading the public trust
record. The root owner must be bound to the original SCM owner/delegation; a SID
or key first learned from the trust file cannot authenticate that file itself.
If no independently protected original selection exists, no constructor exists:
Unsupported before store/key/secret/listener work. HostAuthorizationData is not
this selection. A separately signed grant is not its own installer trust root.

P is a finite stable service principal distinct from I, S and O as required by
current accepted equality rules (P/S/O pairwise distinct, I/P distinct). Do NOT
silently require all FOUR SIDs distinct; that stronger restriction remains a
proposal. I pins the ORIGINAL P verification key/delegation/artifact/pair/validity
and installation epoch. P's private seed is created only by explicit authorized
NEW installation under actual P's native store, not copied from interactive I.
Existing install missing/changed key refuses, never regenerates. No observer
management credential belongs to P. Initial key initialization needs a concrete
exclusive native publisher lock and protected publication transaction; currently
absent, not replaced by the read_service_secret API or a keyring callback.

First bounded hosted profile chooses initial one-shot provisioning only. Keep
unattended cold boot/recovery OPEN; do not implement the still-proposed mutable
ticket/cursor or assume a warm stop/start proves reboot persistence. Public
authorization uses existing canonical three-record v2 parser/verifier; native
selection supplies expected DATA internally after independent native checks.

# D3 exact six-edge rights and handle custody

Same rights for EACH directed edge P->S, S->P, P->O, O->P, S->O, O->S:
target process QUERY_LIMITED_INFORMATION|SYNCHRONIZE|READ_CONTROL =0x00121000;
target primary token TOKEN_QUERY|READ_CONTROL =0x00020008. No DUP_HANDLE,
VM read/write, terminate, create-thread/process, token assign/duplicate/impersonate,
WRITE_DAC/WRITE_OWNER, SeDebug or admin fallback. The a7286fab query candidate uses
the smaller query+synchronize/token-query subset; explicit descriptor-policy
probe must request the above READ_CONTROL bits, not claim the existing candidate
already validated cross-SID security descriptors.

Each role configures ONLY its OWN process and primary-token descriptors during
accepted TransportPrepared initialization, preserving the exact reviewed owner/
SYSTEM/admin baseline and adding only its TWO exact peer principals. Actual
GetSecurityInfo then checks those complete masks/forms; unsupported/extra ACEs
refuse. AsRawHandle borrowing adapters over owned WinSafe guards can feed existing
safe Get/SetSecurityInfo; no ownership conversion needed. Do not broaden this
setup into an arbitrary ACL API or automatic verification-time repair.

Query exact local SCM object (QUERY_STATUS|QUERY_CONFIG|READ_CONTROL), accept only
Running+OWN_PROCESS for this narrow profile, then retain exact process and token
handles across both status rechecks/publication/channel/lease. Verify primary
TokenUser, unscaled creation FILETIME, PID, zero-time liveness and launch-file
binding. Keep prelaunch executable/ancestor replacement guards and exact native
start receipt; image path or hash alone is not loaded-page attestation. Query
denial across any actual role token means Unsupported, regardless of controller
or same-account success. Six edges remain six required cases, not one aggregate.

# D4 fixed channels, packet limits and ownership

Choose exactly THREE purpose-separated channel roles: P-owned finite native
acceptance listener accessible only to S/O; O-owned bootstrap receiver accessible
only to S; S-owned observer-status management endpoint accessible only to O.
P has NO management access. O is the receiver AND actual management client; no
third broker forwards ACK or credential. Each server keeps full own rights,
exact peer clients use0x12019b, excluding CREATE_PIPE_INSTANCE0x4. P acceptance
can narrow to its exact nonsecret frame needs after source review but may not
add broader rights. Verify descriptor owner/DACL/type and kernel peer PID on the
ACTUAL opened endpoint, joined to retained native primary-token instances.

Local-only first-instance noninherited pipes; specific-rights client open with
Identification SQOS, never impersonate/query an effective peer token as primary
proof. Keep exact server/client handles, no raw stream passed by user. No std
generic write access, remote path, TCP fallback or retry/squatting takeover.
Opening a channel grants no publication/W/credential authority by itself.

Freeze startup framing as current v2 amendment: protocol
humanymous.observer-bootstrap/0.3, magic HMBP0003, authorization/2,12byte fixed
header, strict canonical Hello metadata including all THREE signed records,
metadata<=3072, frame<=4096 and cumulative startup<=32768 BEFORE allocation.
Existing max Hello2768 is a DATA vector bound, not transport proof. Header flags
zero, exact declared length and allowed current phase; no extra/trailing bytes,
mixed domains/versions, EOF framing, continuation or legacy downgrade. ONLY Offer
has the fixed64lowerhex secret slot; zeroizing ownership on all exits. Exact NEW
Challenge/Offer/CredentialReceived/Ready schemas and numeric phase tags need
Authority joint freeze; do not invent their values from this table.

One original startup clock begins before admission, cap selected from trusted
configuration (existing proposed ceiling30s is not permission to override a
smaller signed/configured cap). Include every native/open/read/unseal/W/ACK/Ready
step under that same deadline plus independently owned outer watchdog. Lease,
liveness and stop/drain budgets remain distinct but cannot renew original startup.
Incomplete frame/Unknown write retains ownership and burns attempt; never reconnect
with the same grant. P listener returns only native-claim acceptance, not secret
receipt. Signed Prepared/Commit bits assert publisher decisions, not actual ACK.

# D5 journal is physical custody, not signed authorization consumption

Reuse accepted O-only single-writer journal: stable selector/catalog, exact held
parent volume64/file128, generated lowercase nonce, O owner, original clock pair,
attempt ID/generation and journal epoch bind EVERY reservation/ACK. ReserveIntent
and Commit sync before local held reservation/ACK; CreatePending sync before the
existing descriptor-at-create DIRECTORY operation. Terminal intent/commit sync
before releasing reservation. No stale ACK from another anchor/selector/nonce/
owner/budget; B NotCreated cannot resolve A Unknown. Fixed16 attempt/namespace/
tombstone capacities reserve before effect, checked counters, no eviction.

The private-root journal protects filesystem reservations, not a substitute
ledger for authorization grant consumption. Signed grant one-use remains a
separate process-epoch claim/issuer publication decision; never store an arbitrary
grant in the root grammar or treat a durable filesystem ACK as NativeProof.

B1's exact missing operation concerns independent O STATE/genesis FILE and any
new immutable file requiring the same descriptor-at-create contract; it is not
permission for eleven supervisor wrappers. CreateFileW uses CREATE_NEW, explicit
descriptor AT CREATION, noninherited handle and share0; immediately adopt its
actual returned handle as OwnedHandle then File, retain creation obligation on
later failure. No open-existing/random-name recovery, truncate, TEMP staging or
post-create ACL repair. Inspector/ACL/provider/ancestor/link-count checks still
required on same actual File before initialization and sync. Existing-only journal
admission cannot be the final installer/genesis substitute. Torn/ambiguous records
quarantine; lost ACK leaves reserved Unknown. Cold O restart is RecoveryOnly/inert,
no reconstructed Instant, resume, new budget, create/delete or counter reset.

# D6 private proof production and Runtime activation sequence

Native-proof producer must be a descendant of authority/startup/observer, so only
that existing private boundary can assemble NativeProof/Owner/session. It consumes
actual native-security installation/pair/committed one-use channel owners and
Audit's native-authenticated W owner; never projection DATA or observations alone.
Do not widen state::Owner or make its model/advance production constructors.
Replace empty NativeProof only after this complete producer slice is reviewed;
current no-argument native_preparation remains Unsupported meanwhile.

O owns the actual W child/client/lifetime, with independently provisioned key and
same writer/node continuity. Native child pin plus authenticated Audit response
and original protected key selection establish a concrete W owner; generic
Read/Write, CheckpointWitness or ExternalWitness::new alone do not. Retain W
through actual final service flush; no W key auto-provisioning. Initial W spawn
needs reviewed at-creation owned Job/lifecycle custody; runner's existing actor
unsafe is not a Runtime producer. If existing approved production API is absent,
use separate B2 explicit boundary review, not a testing-helper import or detached
child. No Human permission is inferred from a hosted label.

Sequence: actual native+W preparation -> attach owning Runtime guard -> unseal ->
prepare writer and disabled management listener -> authenticated Offer -> O actual
CredentialReceived on same scope/epoch/pins -> PreparedObserverSession -> Runtime
resource validation -> bounded nonblocking Ready enqueue and Active at ONE shared
gate linearization point. No lock across await/query/flush/audit. WouldBlock waits
outside gate, then reacquires/rechecks native liveness/epoch/deadline. First loss
irrevocably closes credential/readiness/new effects; partial/unknown Ready retains
owners/no resend. Existing native queries may block, so final native checks must
be done before entering the enqueue critical section, with an independently owned
monitor and same gate preventing loss-vs-activation races. Query success followed
by an unguarded async send is insufficient.

ObservedStartupOwner privately owns the actual Runtime configuration/listeners/
writer/W/Authority resources; failure keeps them until explicit bounded drain,
flush, monitor join and owned-child reap. No Drop-as-reap/flush receipt. Normal
SCM stop needs separate verified lifecycle authorization, not observer-status
credential or liveness close; permanent bans/erasure/kill retain dual control.
Root/Runtime must select the existing executable composition entry to invoke
this private path. This proposal adds no new public SDK surface. CLI DATA fence
stays negative-only until Root accepts the actual consuming entry; standalone
bind is not a positive fallback. Private activation is not a shipping CLI API.

# Approval classification and next bounded slice

No choice in this proposal requires asking Human merely because implementation
is unfinished. Root may allocate D2-D4/D6 private implementation under already
accepted roles/trust/fail-closed semantics, with owning-context joint review and
separate hosted actor/resource grants. New Cargo edges are Root-owned routine
graph decisions, not Native permission to edit manifests. B1/B2 are explicit
Platform unsafe-allowlist amendments requiring Root+X acceptance per existing
proposal; they are NOT routine source loans and are NOT already approved. Current
canon does not state a separate direct Human approval requirement for that
delegated review. If Root lacks authority to amend the hard boundary, it must
seek Human decision before granting it; no candidate assumes that authority.

Changing accepted architecture/security policy DOES need an explicit owner/user
decision: SeDebug/admin/impersonation fallback, owner/ACL repair during admission,
weakening original-I trust/dual control, making unknown/native-missing pass,
turning status into service stop, public proof factories, stronger crash/erasure
guarantees or third-party targets. NONE is selected here. Scoring remains frozen.

Recommended next grant remains a7286fab's smallest concrete read-only source
slice plus Root's windows-service edge, with no proof construction or SCM mutation.
Then separately hosted exact SIX query edges and self-only tests can falsify D3
before investing in D4/D6 activation. Positive production producer remains OPEN;
this is finite implementation sequencing, not an Unsupported waiver or whole-goal
blocked status. All code/test/hosted items in this decision table are NOT_RUN.
