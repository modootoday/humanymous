---
id: plan:root/20261107103828
title: Draft Windows native bootstrap grant publisher and process pins
status: draft
created: 2026-11-07T10:38:28Z
updated: 2026-11-07T10:38:28Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/adapter-authority, sot:root/native-security, sot:root/rust-migration]
---

# Native bootstrap R1 backend proposal v0.4

DRAFT ONLY. No backend, API, privilege change, channel, manifest edge or native
execution is accepted by this document. Root approved roles and loss semantics,
not R1. Authority/CLI/Runtime must independently critique this exact proposal.

## Selected candidate and authorization root

Propose WindowsScmSignedGrantV1: a native-verified installation record and a
per-install publisher signing key in the actual publisher's Windows credential
store issue a PUBLIC signed, one-use authorization record for the exact live
S/O pair. Initial bootstrap has no shared bearer secret to distribute. The
service-generated observer credential remains secret and crosses only the later
mutually authenticated channel. This deliberately requires Authority/Root to
replace the draft shared-secret Hello premise; it is not a silent protocol fallback.

Reject machine-DPAPI ciphertext as a role-bound grant by itself: any local user
who obtains a machine-scope blob can decrypt it. Reject arbitrary plaintext
secret files, environment/argv secrets, callback signers and private-flag claims.
No custom cryptographic primitive: proposal uses existing workspace Ed25519/SHA256
with a new domain-separated canonical grant format, subject to explicit root
manifest/feature integration. No dependency or feature is changed here.

The original installer/controller I explicitly creates a NEW installation trust
root. I is distinct from the stable publisher service principal P. The original
installer authorization must bind the protected root and exact SCM object ownership
to the delegated P principal, artifact and original signing key; P is not assumed
to own objects created by I. Exact protected delegation APIs remain a P0 gate. Existing
installations require the original pinned publisher key, never replacement-key
generation after read failure. Public-key/role/scope metadata is an installer-
owned protected file, bound to exact SCM image/argument/plan-digest configuration;
recipients derive the expected owner from native service-object ownership and
verify the actual opened file/ancestors. They never trust an owner SID, public
key or plan hash first received over bootstrap. Initial operator authorization
is this explicit owned installation/native SCM registration boundary, not HTTP
input or proof of human identity. Same-owner hostile code and OS administrators
are outside this boundary; it is not remote attestation.

A new PublisherKeyStore profile uses the existing native keyring backend under
P's ACTUAL account and a fixed installation/purpose-specific target, Local
persistence and exact metadata checks. It is separate from service unseal
read_service_secret and audit/witness keys. Creation is explicit new-install
only; an exclusive native publisher lock serializes absent-check/create since
WinCred does not supply a claimed atomic create-if-absent operation. Missing or
changed existing metadata/key fails, never resets. Production keys are not
fixture-cleaned or rotated by Drop. Native-security exposes no arbitrary signing
callback or raw signing seed; only validated role-pair claims can be signed.

## Exact claim and public publication

Grant binds domain/version, installation/application-scope commitment, original
publisher-key fingerprint, exact owned S/O service names, role-plan and installed
artifact digests, both native TokenUser SIDs/PIDs/creation observations, fresh
service/observer/grant/credential epochs, purpose ObserverStatus, issuance/expiry
and startup budget. Grant ID is fresh256-bit OS entropy. Signature bytes and the
claim are public authorization metadata, not a bearer credential; possession
without BOTH actual native role proofs never authorizes a connection.

P create-news bounded regular non-reparse prepared and commit grant files in its protected grant
root; P owns writes, only exact S/O principals get required read/traverse rights.
No world/group/default grant or current.grant overwrite. Name derives from the
fresh grant ID already in protected role plans. Bound actual handle owner/DACL,
file identity, length and content before parsing; validate exact canonical schema
and issuer signature. No public key learned from that file is its trust root.
Publish/read failures return Unsupported/Refused before unseal or observer secret.
P does not receive observer management credentials. Grant-file cleanup does not
claim secret erasure; replay protection is not filesystem deletion.

## Concrete query-rights profile, not SeDebug fallback

At explicit native-role startup, each S/O configures ONLY its OWN process/token
object access for its provisioned peer and P. This is a reviewed installation/
role initialization operation, never ACL repair during verification. Preserve
required owner/System/admin baseline authority and reject unknown broad ACEs.
New peer/P rights: process QUERY_LIMITED_INFORMATION|SYNCHRONIZE|READ_CONTROL;
primary token TOKEN_QUERY|READ_CONTROL. Do not grant DUP_HANDLE, VM read/write,
create-thread/process, terminate, token duplicate/assign/impersonate or WRITE_DAC
on the other role. No SeDebug/elevation/impersonation fallback is permitted.

This rights profile is a PROPOSAL requiring actual cross-account qualification:
OpenProcessToken documentation warns about different/elevated-account access.
Explicit DACLs are not assumed sufficient across integrity/protected-process
boundaries. If actual TokenUser query cannot work without forbidden privilege,
this production profile remains Unsupported. Do NOT substitute pipe owner,
effective impersonation identity or configured service account for independently
queried primary TokenUser. A reciprocal Identification-only pipe proof would be
a DIFFERENT contract requiring explicit root review, not an automatic fallback.

P/S/O open and RETAIN process handles, query actual TokenUser, creation times,
process ID, liveness and native manager binding; current-self checks reject an
impersonated thread. Creation time/text alone does not pin an instance. Hold the
native handles through grant acceptance, then S/O retain each other's handles
through the lease; P exits only after the publication protocol below; native acceptance is not ServiceReady.
Never close the pin and later reopen its numeric PID. A dead/unqueryable handle
or mismatch invalidates the transaction and cannot release credential bytes.

## Discovery state and circular-readiness decision

SCM QueryServiceStatusEx PID may be invalid in START_PENDING/STOP_PENDING and is
never valid when STOPPED. Do NOT use those states as trusted process discovery.
Root has now accepted the architectural separation of a new native
TransportPrepared phase: role security setup and nonsecret discovery endpoint
are complete; SCM reports RUNNING, while APPLICATION READY/ingress/unseal/writer
activation remain false and the absolute startup watchdog stays armed. Native
manager PID is accepted only in documented valid state, joined to exact role
configuration, retained process handle and actual TokenUser. Doctor/status must
not translate this SCM process-host state into application readiness.

TransportPrepared semantics are accepted; its typed/native implementation is not. No fabricated
START_PENDING PID, native peer self-report, WMI alias or Ready-before-grant cycle
may replace it. Ordinary owned-child handles avoid this discovery issue but do
not qualify the SCM profile.

## Two endpoint policies and secret-release gate

Bootstrap is O-owned with S client access; management is S-owned with O client
access. Exact protected owner/SID policies are separate; P gets no management
access from publisher status. Local named pipes reject remote clients, require
first-instance ownership and non-inherited handles. Client rights are individual
read/write-data/attributes/READ_CONTROL/SYNCHRONIZE as needed, EXCLUDING
FILE_CREATE_PIPE_INSTANCE. FILE_GENERIC_WRITE/GENERIC_WRITE must not be used as
a shortcut: they include that create-instance right. Client open must request
matching specific rights, not std read/write defaults that request broader masks.

Native-security opens/binds the actual named endpoint from the protected plan,
checks actual owner/DACL/type and kernel client/server PID, joins it to retained
primary-token process proofs and the signed authorization record. No constructor
accepts arbitrary Read/Write, raw inherited handle, caller SID/PID or success bool
as proof. Recheck liveness/binding before each credential release/activation.
Connection-origin identity does not attest absence of delegated handles or each
later writer; same-principal delegation remains the explicit threat ceiling.

## Proposed opaque API and state ownership

Names are proposed, not existing implementations:

```rust
InstallationBootstrapRoot::create_new_authorized(native_install_request)
    -> Result<InstallationBootstrapRoot>;
InstallationBootstrapRoot::open_existing(native_role_startup)
    -> Result<InstallationBootstrapRoot>;
NativeRoleSelfAccess::prepare(&root, exact_role) -> Result<NativeRoleSelfAccess>;
NativeRolePair::discover_transport_prepared(&root, startup_budget)
    -> Result<NativeRolePair>; // retained handles + actual primary-token proofs
BootstrapGrantPublisher::open(root, publisher_lock) -> Result<Self>;
BootstrapGrantPublisher::publish_once(&mut self, pair, trusted_policy)
    -> Result<PublishedBootstrapGrant>; // no generic sign(bytes)
VerifiedServiceGrant::claim(root, self_access, publication, current_role)
    -> Result<VerifiedServiceGrant>;
VerifiedObserverGrant::claim(root, self_access, publication, current_role)
    -> Result<VerifiedObserverGrant>;
VerifiedBootstrapEndpoint::connect_or_bind(role_grant, startup_budget)
    -> Result<VerifiedBootstrapEndpoint>;
```

All provenance-bearing constructors perform the actual native checks. Root,
self-access, pair, role grants and endpoint own nonextractable handles and are
non-Clone/non-Debug/non-Serialize. Per-process role context permits one claim;
reservation/error/drop burns it for that process epoch. Sender/receiver protocol
also consume once; concurrent constructor calls cannot mint multiple live grants.
Restart changes native instance and requires a new authorized record. Records
cannot revive a consumed in-memory lease or substitute for the peer's continued
liveness. Deadline/cancel closes only owned handles/endpoints; it never kills a
numeric SCM PID. Authority retains endpoint+pins inside server/client leases.
Witness capability remains separately verified by audit/native ownership; an O
Challenge field is NOT a native proof of W or its independent key/lifetime.

## Existing versus new work and support matrix

Existing: safe current-token/owned-service-name checks, native keyring backend,
file/privacy primitives, shared bounded capture and current authority transport.
NEW/unimplemented: publisher-key profile, installation-root/SCM-owner verifier,
self query-rights policy, retained foreign-process+primary-token pins, typed grant
signer/parser/store, two-principal endpoint policy and opaque role constructors.
Transport logic must be moved/reused, not forked. Safe wrapper coverage and any
new feature/dependency edge require explicit root integration; no authored unsafe
exception is silently created.

| Profile | Current state | Required before support |
|---|---|---|
| Windows2025 x64 distinct NT SERVICE S/O | Unsupported; concrete proposed target | Root accepted the protocol/phase semantics; actual cross-token rights without SeDebug and complete native/hosted gates remain |
| Ordinary Windows owned children | Component design only | Actual retained child/native token tests; never label SCM/cross-role store qualification |
| Linux system/user manager | Unsupported for this backend | Separate protected publisher/provider, native-instance and cross-UID transport contract; no WinCred/SCM assumptions |
| macOS launchd | Unsupported for this backend | Separate unattended provider/manager/native-instance/access contract; no GUI/keychain fallback |
| ACT/emulated hosted labels | Not genuine hosted; no account/SCM mutation | Cannot qualify this profile |

## Independent critique and exact verification gates

Self-critique verdict NEEDS-WORK: P0 query-rights feasibility and native TransportPrepared implementation
are unproven; P0 original installation/publisher trust must survive
actual service/file replacement tests; P1 signed-public-grant versus old secret
Hello requires explicit Authority/Root decision; P1 safe API/feature integration
not reviewed. Merely naming these mechanisms does not mark R1 resolved.

Before channel implementation, isolate actual native probes for cross-SID process/
primary-token reads with declared rights and no SeDebug/admin fallback; prove
valid-state SCM discovery while application readiness stays false. Failure means
Unsupported, not relaxed identity. Pure gates: canonical signature binding,
unknown/duplicate/oversize fields, wrong key/scope/role/epoch/process time, expired
record, missing existing publisher key, concurrent claim/burn/restart, exact ACL
masks and no credential bytes in serialization/Debug/errors. Actual native gates:
wrong role/TokenUser, instance exit/reuse, token/process query denial, endpoint
squatting/extra pipe instance, remote client, object/ancestor/plan/artifact swap,
publisher disappearance, stale signed record, cancellation and no leaked handles.
Genuine hosted SCM runs are separate from ordinary fixtures. No product pass,
service-status readiness, witness independence or physical-erasure claim follows.

## Primary references checked

- [Process object access](https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights)
- [OpenProcessToken access and account warning](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocesstoken)
- [SCM PID validity by state](https://learn.microsoft.com/en-us/windows/win32/api/winsvc/nf-winsvc-queryservicestatusex)
- [Named-pipe rights and create-instance alias](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)
- [Process handles and identifiers](https://learn.microsoft.com/en-us/windows/win32/procthread/process-handles-and-identifiers)
- [Credential metadata/persistence](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw)
- [Machine DPAPI scope](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata)


## v0.2 independent critique reflected: exact publication lifecycle

This section supersedes the v0.1 ambiguous acceptance/wait wording. Authority,
CLI and Runtime independently reviewed v0.1 hash6b97bda5 and returned needs-work.
No reviewer approved R1. Their requirements below are normative for this proposal.

Before launching either role, P creates immutable role plans containing fresh
GrantId, exact grant-root names, P native-instance descriptor, planned S/O role
names/epochs and the ORIGINAL publisher-key fingerprint. Plans do NOT pretend
to know S/O PIDs before launch. After both actual TransportPrepared instances are
native-pinned, P signs the pair claims. There is no plan rewrite to insert PIDs.
Both services' actual configurations must still identify those exact role plans.

P owns a separate bounded nonsecret native acceptance listener, created before
launch with exact P-owner/S/O-client ACL. It conveys NO observer credential and
is not the management endpoint. Its actual server/client PID, primary TokenUser,
retained instances and installation/grant purpose are checked by the same native
backend, including P's own explicit query grants to the two roles. No inherited
pipe or reported receipt object authenticates the sender. P remains a publisher,
never an ObserverStatus peer. This extends the actual permission-probe matrix
to P<->S and P<->O as well as S<->O; success is not assumed.

Publication states are Prepared -> BothNativeClaimsAccepted -> Committed.
P writes the signed PREPARED record; S/O verify source/signature/current native
pair and reserve their process-wide slots, but cannot release a secret or activate
from Prepared. Each returns NativeGrantAccepted over P's authenticated listener:
version, grant/role epochs, exact publication digest and both instance commitments.
The native backend constructs this receipt only after claim/pins succeed. It is
NOT CredentialReceived, witness proof or service readiness. P validates the actual
native sender and exact receipt, keeps both process pins, and only after both
acceptances creates a SECOND immutable signed COMMIT record over the prepared
digest and same grant/epochs. Both roles must validate that exact commit before
their native grant becomes usable. No forwarded ACK or shared success bool.

Before Commit, cancellation/deadline/error burns the attempt; absence of a valid
commit prevents secret release. After the commit boundary, publisher failure or
lost notification is AMBIGUOUS publication, not proof of revocation or ServiceReady.
No automatic fresh issuance/retry follows. Independent operator recovery must
observe stopped old roles and explicitly provision fresh plans/instances/grants.
A returned Published receipt describes signature/file durability and native
acceptances only; it is never observer provisioning or runtime readiness.
File sync/namespace uncertainty is reported honestly. A system crash changes the
native process instances; old prepared/commit artifacts cannot authorize restart.

Initial profile has NO unattended automatic SCM recovery/rebind. Configure no
implicit restart loop; a single-role exit triggers accepted firstloss handling
in the other role and never admits a replacement PID from an old record. An
explicit authorized provisioner transaction stops/observes BOTH old instances,
creates fresh immutable plans and grant ID/epochs, starts/pins the new pair and
publishes again. S/O never gain issuer signing authority. Any future unattended
recovery orchestrator is a separately approved role/permission/lifecycle design.

## v0.2 single claim, clocks and effect linearization

One claim is per actual PROCESS INSTANCE, not per new Rust object. Use an atomic
create-new claim marker under the role's verified owner-only native state root;
name derives only from actual PID/creation instance, not caller-selected grant
ID. Retain its native lock; leave a spent marker on cancellation/error/Drop.
Every constructor and linked adapter uses this same OS-backed slot. No new root
object, duplicate module instance or concurrent constructor may mint another
claim. Startup never clears markers to retry. Explicit maintenance only after
native proof that the prior role stopped may remove old instance state. Bounded
claim-history capacity fails closed; it does not delete unknown live claims.

P starts one local monotonic publication deadline before role launch. Each role
starts its independent absolute startup watchdog on native entry. Signed plans
carry trusted issuance/not-before/not-after and the accepted startup cap; grants
cannot extend plan expiry. Effective remaining budget is the minimum of local
monotonic remaining and positive signed wall-clock remainder. Every phase uses
that same absolute deadline; no accept/read/retry resets it. Reject backwards wall
observations, now before signed issuance, expired records, nonfinite/oversized
budgets and inability to establish trusted time. All actors are on the same
machine, but monotonic instants are NOT serialized or compared as portable values.
Clock rollback and publisher delay cannot renew a role startup interval. Lease
TTL/liveness/stop policy remains separate and requires explicit Root policy;
24hours is not an accepted default.

The signed record does not make check-then-await safe. A native role grant carries
one shared, irreversible epoch/revocation owner retained into Authority and
Runtime. Secret offer writes and activation must use that SAME live epoch object,
not a reconstructable epoch string or a cached liveness bool. Native token/instance
checks precede admission; final liveness and bounded nonblocking kernel enqueue
serialize with revocation. WouldBlock exits the gate without accepting bytes;
wait outside, reacquire and recheck. A queued/partial operation that linearized
before loss remains potentially disclosed/ambiguous; cancel and revoke the
associated credential, never claim no bytes could arrive later. No lock spans
await, blocking token query, flush or audit I/O. Exact transport/activation evidence
constructors and race tests remain implementation acceptance gates.

Witness is a separate P0 typed dependency: actual native W peer/owned-instance
proof plus audit-pinned key/writer/node continuity and a bounded owned lifetime
must produce VerifiedWitnessCapability before S opens unseal/writer state. An O
Challenge field, generic ExternalWitness constructor or signed S/O grant alone
cannot mint it. Native-security and audit must jointly define its constructor;
missing proof returns Unsupported before secret use. The backend does not claim
W independence merely because W is a different process.

## v0.2 supported-installer and image-proof limits

Permission to create/configure a service does NOT establish its actual owner.
This candidate accepts only the installer profile where native service-object
ownership and protected root owner really match P as specified. It performs no
ownership repair to force that match. Actual SCM-owner behavior is a required
probe; if default registration does not satisfy the policy, this profile remains
Unsupported until an independently reviewed authorization profile replaces it.

Installed artifact digest and protected SCM configuration are deployment checks,
NOT loaded-image attestation. An opened path hash alone does not prove executed
pages or prevent every process/file replacement race. Hold verified file identity
handles under the explicit same-owner/privileged-OS threat ceiling; join native
manager PID, retained live process instance and actual primary TokenUser before
publication. A mismatched or replaced file/configuration refuses. Do not add
"running binary authenticated by SHA" to public facts without a separate reviewed
native executable-image proof mechanism.

Required new negative gates: only one role restarts; P dies before/after commit;
one acceptance/commit missing; repeated root/context/module claims; claim marker
loss/corruption/pending state; past/future clock rollback; plan/grant budget reset;
SCM owner differs despite create permission; artifact path changes after hash;
revocation races first credential enqueue/activation; forwarded W proof; loaded-
image overclaim. R1 verdict remains NEEDS-WORK, not accepted or implemented.


## v0.3 Root decisions and exact signed-byte proposal

Root accepted public targeted authorization as REPLACEMENT for the shared-secret
Hello and accepted TransportPrepared SCM_RUNNING distinct from AppReady. This
acceptance is SPEC/probe preparation only. Do not call the existing final readiness
publisher early: native Stop handling and independent startup watchdog stay armed;
public ingress, unseal, writer and application readiness remain false. Doctor,
installation/status and receipt consumers must distinguish these phases. Root did
not accept a universal lease TTL, native proof, publisher backend or channel code.

Authority wire selection is humanymous.observer-bootstrap/0.2, header HMBP0002,
signed-grant format humanymous.observer-bootstrap.signed-grant/1. Reject HMBP0001,
mixed versions, missing commit and all downgrade attempts. Hello contains the
COMPLETE prepared AND commit records below in metadata, no secret slot or side
attachment. Offer alone has its fixed64-lowerhex observer credential slot.

### Canonical records (proposal requiring independent review)

Use binary public records with exact parse-to-EOF and strict verification; no JSON
or concatenated variable strings are signed directly. Numeric fields are unsigned
big-endian, with no text numeric spellings. Fixed byte lengths are exact. All
reserved flags are zero; purpose is the single byte1=ObserverStatus. Reject all
other values. UTF-8 names use the existing owned ASCII service-name grammar,
1-byte length1..64 and exact bytes, never normalization/case folding. SID fields
use 1-byte length8..68 and native canonical binary SID bytes; subauthority count
must exactly determine length and match the actual queried token SID.

Prepared body in EXACT order:
1. version u16=1;
2. application-scope commitment32, installation commitment32,
   publisher-key fingerprint32, grant ID32, publisher-instance commitment32;
3. service epoch16, observer epoch16, credential epoch16;
4. service name(length+bytes), observer name(length+bytes);
5. service SID(length+bytes), observer SID(length+bytes);
6. service PID u32 + creation FILETIME u64, observer PID u32 + creation FILETIME u64;
7. service role-plan digest32, observer role-plan digest32,
   installed service artifact digest32, installed observer artifact digest32;
8. issued UTC milliseconds u64, not-after UTC milliseconds u64,
   startup cap milliseconds u32;
9. purpose u8=1, reserved flags u8=0.
Maximum prepared body652bytes. The creation observations join RETAINED native
handles; they are not a standalone process-instance proof or loaded-image hash.

Prepared signature message is exact ASCII domain bytes
humanymous.observer-bootstrap.signed-grant/1/prepared followed by ONE NUL byte,
u32 BE body length, then exact body. Use the existing strict Ed25519 verification
primitive and expected ORIGINAL pinned publisher key. No key from the record
replaces that key. Prepared artifact is ASCII HMBGP001(8bytes), u16 BE body length,
exact body, raw Ed25519 signature64. Maximum artifact726bytes.

Prepared record commitment is SHA256 of ASCII
humanymous.observer-bootstrap.prepared-record/1 plus ONE NUL and the COMPLETE
prepared artifact, including its signature. Publisher fingerprint is SHA256 of
ASCII humanymous.observer-bootstrap.publisher-key/1 plus ONE NUL and the raw
32-byte expected Ed25519 public key. Commitment encoders below are also proposals requiring exact trusted DTO review;
no caller hash or arbitrary bytes may mint a native proof. They cannot be silently
implemented by Debug, JSON map iteration, string joins or path text alone.

Commit body in EXACT order: version u16=1; grant ID32; prepared record commitment32;
publisher-key fingerprint32; service epoch16; observer epoch16; credential epoch16;
commit-issued UTC milliseconds u64; not-after UTC milliseconds u64;
startup cap milliseconds u32; purpose u8=1; reserved flags u8=0;
native-acceptance mask u8=3(exact S and O). Exact body169bytes.
Commit signature message uses domain
humanymous.observer-bootstrap.signed-grant/1/commit plus ONE NUL, u32 BE length
and exact body. Commit artifact is HMBGC001(8bytes), u16 BE body length, body,
raw signature64, maximum243bytes. Every repeated field must equal prepared and
trusted binding; commit expiry/cap must not extend prepared/role-plan deadlines.
Prepared signature/kind can NEVER authorize Commit. Partial/missing commit,
wrong record kind, cross-grant digest/epoch/key and extra bytes are refusals.

Hello grant object has only format, prepared and commit fields. Artifacts are
canonical LOWERCASE hex, even length, no whitespace/padding/alternate encoding.
Full artifact hex max1452+486=1938bytes; grant JSON overhead<=128bytes; all other
Hello metadata<=896bytes. Thus total<=2962bytes, still enforce the actual complete
metadata<=3072/frame<=4096/startup<=32768 checks BEFORE allocating or decoding.
Do not permit3072bytes PER record or an unbounded file/extra secret slot. Full
application identities remain in protected configuration; wire scope is an exact
commitment compared against that configuration, not a repeated unbounded origin.
Lengths, domains, SID encoding, fixed-order fields and both signatures need pure
cross-language/cross-platform fixtures before acceptance. No signature-format code
is authored by this proposal.

## v0.3 native probe preparation and known safe API gaps

Read existing pinned source, without executing OS mutations: winsafe0.0.29 has
safe OpenProcess/GetProcessId/GetProcessTimes/GetExitCodeProcess/OpenProcessToken/
GetTokenInformation, and safe SCM open/create wrappers. Its current service wrapper
exposes no complete query-status/config surface needed here. Existing CLI already
pins windows-service0.8.1 with safe Service::query_status/query_config; proposing
that exact Windows-only native-security edge requires a Root manifest window.
windows-permissions0.2.4 supplies safe kernel/file/service security descriptor
operations. A safe AsRawHandle adapter over an OWNED winsafe guard can expose
borrowed raw values without implementing unsafe code or ownership conversion.
No direct windows-sys unsafe call in native-security is authorized. If safe wrapper
coverage is missing for an operation, report it and seek a reviewed dependency/
boundary decision; do not smuggle an unsafe exception through a probe.

Proposed isolated genuine-hosted probe (NOT the secret channel): exact two owned
virtual service roles each verify self TokenUser and install the declared process/
primary-token QUERY-only policy; P and peers query/retain the actual objects,
compare valid-state SCM PID, creation observations, current TokenUser and role
configuration, then report bounded nonsecret facts. Exercise missing rights,
wrong SID, process exit, startup watchdog, remote/default/extra-instance pipe ACL
rejection and no application readiness/unseal/writer activation. No management
credential, publisher seed, witness token or production grant is transmitted.
It must record actual enabled privileges and prove no SeDebug/elevation fallback.
Actual source/run/OS/role/controller rights restoration and service cleanup are
required. No local SCM/account mutation is authorized by this draft.

## v0.3 unattended publication decision still open

The original interactive installer's private WinCred is not assumed available
after logout, service restart or boot. The v0.2 manual reprovision policy is
fail-closed but DOES NOT qualify unattended operation. A concrete new candidate
for Root decision is a registered finite one-shot P publisher role, with stable
publisher service principal, per-install native key store and protected delegated
issuer record; P is invoked at boot/recovery and exits after publication. This
adds a third registered SCM role, while S/O remain the two persistent MAIN
processes. Root has NOT approved that launcher/privilege extension by approving
two S/O roles. The original installer owner and authorized runtime publisher SID
would be distinct explicit fields, joined by native protected delegation; never
silently replace the original issuer key or assume CreateService means ownership.

Before this candidate can replace the manual policy, specify and independently
review original key initialization/pinning, exact publisher token/service rights,
authenticated finite recovery trigger, exclusive pair-stop/pin/new-plan/publication
transaction, both-role failure/partial commit handling and actual key availability
across publisher service stop/start without interactive credentials. Original
installer/publisher logout and machine-boot claims require their own evidence.
No signer is moved into S/O, no stored plaintext password, no arbitrary recovery
command or permanent additional broker is inferred. Until Root chooses and these
gates pass, unattended recovery remains Unsupported and R1 remains unaccepted.


### Exact commitment encoder proposal

All SHA256 domains below are exact ASCII followed by ONE NUL. Hash incrementally
within the already validated configuration bounds. String encoding is u32 BE UTF-8
byte length then exact validated bytes; no implicit Unicode normalization or case
folding beyond the existing application grammar.

ApplicationScopeCommitment domain is
humanymous.observer-bootstrap.scope/1. Fields in order: installation, tenant,
origin, revision, application_authority (five length-prefixed strings).
InstallationCommitment domain is
humanymous.observer-bootstrap.installation/1. Fields: ApplicationScopeCommitment32,
original pinned publisher fingerprint32, immutable installation epoch16,
service name(u8 length+bytes), observer name(u8 length+bytes).
PublisherInstanceCommitment domain is
humanymous.observer-bootstrap.publisher-instance/1. Fields: actual native SID
(u8 length+binary bytes), actual PID u32 BE, actual creation FILETIME u64 BE,
publisher epoch16. Role acceptance instance commitment domain is
humanymous.observer-bootstrap.role-instance/1. Fields: role u8(1=S,2=O), native SID
(u8 length+bytes), PID u32 BE, creation FILETIME u64 BE, role epoch16,
protected role-plan digest32. Every input is obtained from the native verified
root/process context or original provisioned scope, never accepted as its own
proof from a wire hash. Retained handles and ownership checks remain mandatory;
these encodings do not become native attestation merely by being canonical.


## v0.4 closed Hello DTO and independent format review

Authority, CLI and Runtime independently checked c934d71a arithmetic and distinct
signature/linkage rules; they found no arithmetic blocker. They did NOT approve
R1 or implementation. Required golden signatures/byte/hash vectors remain NOT_RUN.

Proposed exact Hello object, in this field order for the bounded canonical encoder:
protocol, phase, purpose, bootstrap_scope, grant_id, service_epoch, observer_epoch,
credential_epoch, sender_pin, receiver_pin, hello_nonce, grant. Values: protocol is
humanymous.observer-bootstrap/0.2; phase=hello; purpose=observer-status;
bootstrap_scope/grant_id/sender_pin/receiver_pin are exactly64-lowerhex strings;
three epochs and hello_nonce are exactly32-lowerhex strings. Grant has exactly
format, prepared, commit in that order; format is
humanymous.observer-bootstrap.signed-grant/1 and artifacts follow the exact hex
bounds above. No other type, array, field or nesting is accepted. All repeated
identity/epoch/commitment values must equal native/trusted claims, not select them.

Canonical metadata emits compact ASCII JSON with this key order, no whitespace,
escapes or alternate spellings. Receiver first enforces actual received length,
strict duplicate/unknown/missing/type validation, then compares to the SAME bounded
canonical encoding. Reordered/whitespace/escaped equivalents are rejected in this
proposed wire profile, not normalized into another accepted byte representation.
This explicit canonical-wire decision requires Authority agreement before code.
At maximum record sizes the exact compact object is2653bytes:1938 artifact hex,
83 grant-object syntax bytes,632 bytes outside that object. Still enforce actual
complete3072 metadata,4096 frame and32768 startup ceilings; the128/896 component
budgets count ALL corresponding JSON syntax. No per-record cap substitution or
unbounded RawValue/side attachment. Canonical parser/encoder requires golden min/
max SID/name/clock vectors and every-field mutation tests before format freeze.

InstallationCommitment is a BOOTSTRAP POLICY-SCOPE commitment (it includes the
revision-bearing application scope). Rename its Rust type to
BootstrapInstallationCommitment, separate from stable runtime KillTarget/control
installation/node commitments. Never substitute it for kill/intent/nonce/counter
identity or reset retained state when revision/policy changes.

## v0.4 approved unattended architecture direction, not qualification

Root approved a THIRD registered finite one-shot publisher service P, in addition
to persistent S/O. Only S/O are persistent MAIN processes. Original installer/
controller I is a distinct authorization subject from stable publisher principal
P. This supersedes the v0.1 same-P-owner assumption for the unattended candidate;
it does NOT silently change the supported native owner profile.

I creates and protects the original installation/root/SCM-owner record, binding
exact publisher service SID, approved artifact and ORIGINAL authorized public-key
fingerprint. P's signing-key store is under its ACTUAL stable service account,
not the interactive installer's logon/profile. P must verify that its actual
principal/artifact/native role matches the protected delegation. Neither P nor
S/O may pin a key first learned from a grant. Original creation/initial public-key
pinning is an explicit installer transaction with independently native-authenticated
publisher observation; existing-install missing key never regenerates. Actual
SCM owner can differ from permission to create: record and verify the accepted
owner/installer delegation profile without ownership repair. Exact native
initializer/delegation methods remain an API acceptance gate, not a caller bool.

P service boot/recovery operation is finite and exclusively serialized per
installation. Its explicit native-manager capability may start/stop/query ONLY
exact S/O roles; no observer management credential, arbitrary service command,
raw PID termination, DELETE/WRITE_DAC or generic service reconfiguration is added.
Configure no S/O dependency waiting for P to STOP. P starts/waits both roles in
TransportPrepared, publishes, then exits; S/O do not wait for P's service exit.
Initial setup/boot activation and the exact authenticated recovery trigger are
installer-owned configuration, not requests learned over a public endpoint.

Proposed recovery transaction: acquire native publisher lock; identify old exact
pair from authenticated publication state; request bounded native stop and observe
old retained instances stopped; fail closed on unknown workers/stop state; choose
fresh grant ID and role/credential epochs; publish fresh launch metadata BEFORE
starting either role; start/pin both; complete Prepared/native-acceptance/Commit.
Only one role restarting is not admitted; both are stopped/reprovisioned. S/O
never become signers. No automatic resume of an ambiguous prior commit. Retained
control/custody/counters are not reset by this lifecycle.

To avoid granting P SERVICE_CHANGE_CONFIG merely to replace per-attempt arguments,
propose an immutable installer-owned BASE role plan plus a publisher-signed
immutable per-attempt LaunchTicket. A bounded signed current-ticket cursor, under
exact publisher write policy, selects that ticket. Role plans keep the original
publisher key/scope/native-role policy; ticket carries fresh grant/epochs/P native
instance and hard expiry, never S/O PID before launch. Ticket/cursor signature,
actual owner/access and exact scope are verified before native claim; stale or
partial metadata never authorizes unseal. role_plan_digest in Prepared would be
a domain-separated commitment to base-plan digest AND launch-ticket digest.
THIS ticket/cursor format and exact recovery trigger remain proposals requiring
independent review before changing the frozen record semantics/creating APIs.
They are not an unsigned mutable plan, SCM dependency trick or key-reset fallback.

Actual service-principal key create/read/delete and availability across stop/start,
logoff and machine boot require separate genuine-hosted/native evidence. A warm
restart test is not a reboot result. If the platform cannot retain/access the
key without an interactive profile or forbidden privilege, report Unsupported.
No machine-DPAPI, plaintext-password/file, interactive prompt or personal keychain
fallback. No production issuer key is created by probe permission.

## v0.4 exact native primitive probe slice for Root integration review

Scope: no secret channel, signature issuer or production key. Compile-reviewed
helpers in native-security and thin CLI-owned service probe entrypoints reuse
existing SCM lifecycle, rather than duplicate account or transport code. The
existing Windows actor supplies an ordinary fixture controller I/T on a genuine hosted VM,
distinct from the actual stable P service principal. Cross-SID query and pipe
edges execute under actual P/S/O service tokens, with enabled privileges, denial
checks and cleanup outcomes recorded separately per edge. Ordinary or elevated
controller observations cannot substitute for any service-principal edge.
scoped manager delegation and independent restoration; no local SCM/account test.
Fresh owned namespace identifies S/O/P fixture registrations and source/run.

Proposed native-security feature bootstrap-native-probe (default off) needs:
- existing pinned winsafe kernel/advapi and windows-permissions for self-token,
  owned safe process/token guards, descriptor inspection/query grants;
- NEW Windows-only windows-service=0.8.1 edge for actual query_status/query_config;
- NEW sha2 workspace edge for bounded artifact verification; full signing codec
  additionally needs Ed25519 but is OUT of this first probe;
- existing credential-pilot feature only for scoped TEST credential lifecycle.
No feature/manifest change is made here. Native-security remains free of authored
unsafe; report any missing safe primitive before source implementation.

Proposed operations: NativeProbeSelfAccess::prepare_exact_role; hold_peer_from_scm
(valid TransportPrepared state only); query actual primary TokenUser, native
creation/ID/liveness and privilege facts; verify role/endpoint-specific ACL and
actual peer PID on the exact opened pipe; then stop/restore owned role state.
Return ObservedNativePeerProbe, never production VerifiedServiceGrant. No public
constructor takes numeric PID or a caller identity claim as proof. Probe receipts
separate native query success, expected-role match, SCM phase, installed-artifact
check and loaded-memory-attestation=false; raw identities remain private.

Native launch-image binding proposal: hold verified installed executable files
with write/delete sharing denied BEFORE manager start, and hold relevant native
ancestor identities; start only newly created/stopped exact roles under the
publisher lock, retain native start receipt and actual process creation window.
After valid-state SCM discovery, query actual process image path and join its
opened file identity to the held prelaunch artifact. Reject replacement/config
change. This is a LAUNCH-FILE binding under the same-owner/privileged-OS ceiling,
not a claim that loaded pages cannot change. Exact APIs/file-share behavior and
actual swap/old-process tests must pass before that proof type exists.

P fixture credential probe uses only a fresh pilot selector/synthetic test bytes
under actual P service SID, with explicit planned persistence across one stop/
start and explicit matching cleanup. Existing fixture RAII auto-cleanup must not
be silently defeated or promoted to a production write API: exact cross-instance
fixture handoff/adoption/cleanup capability needs review before authoring. Read/
cleanup mismatch leaves Unknown and never deletes unrelated stored bytes. Root
can allocate this probe only after that fixture contract/feature edge is accepted.
Cold reboot, original key initialization and unattended orchestration stay separate.
