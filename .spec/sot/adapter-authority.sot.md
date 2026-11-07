---
id: sot:root/adapter-authority
title: Adapter authority
status: active
created: 2026-11-06T22:50:11Z
updated: 2026-11-07T03:30:26Z
verify: [manual]
rel:
  governs: [crates/authority/**]
  depends_on: [sot:root/adapter-service-contract]
---

# Adapter authority

## Explicit other-user qualification receipt

The Linux owned-socket negative parent must verify both successful child exit
and an exact executed unix_unprivileged_connect_probe test receipt with1passed/
0failed/0ignored. A zero-test, missing or skipped helper is not socket denial.
The real child requires UID65534 and PermissionDenied. Execute this case only
inside the owned Docker profile with UID/GID switching capabilities; it grants
no host privilege, network egress, public admin access or relaxed peer identity.
The ordinary helper remains explicitly ignored until its parent provisions it.

## Rule

### Follow-up contract v0.2: accepted and component-verified

2026-11-07 KST serial coordination: audit checkpoint d001ee6 releases gate-edge
to authority for the shared adapter client and generic local IPC boundary, then
runtime reclaims it for actual management/session wiring. Node and nginx owners
accepted this shared client instead of duplicating IPC validation. Rust1.99.1 /
edition2024 only; root owns common Cargo/lock/toolchain. Historical results below
remain scoped to their original source and are not current qualification.

Configuration gains optional `adapter: "node" | "nginx"` (default node). Public
Configuration.adapter is integrity::adapter::AdapterProfile. Credentials are
bound to this provisioned profile; envelope data never selects it. Reuse
parse_with_profile, not a copied parser. Node id/version/modes remain unchanged;
Nginx is humanymous.nginx/0.1.0-experimental.1, nginx-raw or nginx-normalized.
Nginx connectionId is always null, normalized peer is null. Raw means observed
application HTTP1 order, never original TLS/H2. capture/stream remain null;
no raw plane is enabled by a profile name.

Accepted and implemented client API: authority::client::ClientOptions holds
endpoint/credential &str, server &Peer, timeout_ms u64, adapter AdapterProfile;
ExpectedBinding holds id/tenant/origin/revision &str and deadline u64.
async decide(&ClientOptions, &[u8], ExpectedBinding) returns a non-Clone
BoundResponse with verdict()/expires() and consuming consume()->Result<Verdict>.
Verdict is Allow/Deny/Challenge. Request bytes are shared-parser verified;
response is exact seven fields, duplicate/unknown/missing fields rejected,
scope/nonce/revision/expiry matched. Non-200, framing/peer/state/clock failures
are typed static 503 refusals. No retries, redirects, cache or TCP fallback.
The supervisor provides actual authority PID/SID or UID, not HTTP peer claims.
Verify private socket paths and kernel server peer, or native pipe PID and actual
owner/protected DACL via shared native-security. Windows client uses identification
SQOS and no unbounded connect/flush wait. consume rechecks wall and monotonic
expiry; adapters also check cancellation and expiry immediately before origin.

Accepted generic IPC seam: LocalHandler: Send+Sync+'static supplies
handle(&self, LocalRequest, LocalContext)->impl Future<Output=Result<LocalResponse,
Rejection>>+Send. LocalRequest owns method, exact target, ordered service headers,
authorization and complete bounded body. LocalContext supplies actual NativePeer,
clock/deadline/cancellation and admission lifetime. Scope comes from the handler's
trusted configuration, never transport headers. LocalResponse is status plus
bounded JSON only. Listener::bind_ipc(endpoint,&Peer) and serve_handler(handler,
Limits{capacity,rate_per_second,timeout_ms}) reuse the native ACL/peer/admission
boundary. Existing decision bind/serve APIs remain compatible and reject admin
paths. Runtime owns a distinct management endpoint/credential and route allowlist,
CSRF/header policy, signed approvals and operations-only settings. Transport alone
does not authorize administration. It owns response writes; write failure closes
without appending another error frame. Windows cancellation closes its own pipe
without leaving unbounded FlushFileBuffers/limbo work; slow-peer regressions are
required. Response streaming remains the runtime/adapter origin relay's concern.

RIT/typed provenance dependency: runtime owns Sessions and canonical/authorize.
It must join the exact adapter scope, nonce, method/target/headers/body commitment,
connection/stream context and credential epoch to a consumed session receipt.
Client assertions, a signed subset, cookies/auth commitments and native capture
are separate evidence axes. v0.1 strips internal headers and has no session receipt
field, so required.rit/collector/raw capture remain 422 without a verified native
provider. An in-process one-use reference is preferred; future wire v0.2 requires
an audited opaque one-use grant, RequestBinding commitment, session/protocol/
consumed counter/expiry and authenticated peer/adapter epoch. No arbitrary IPC
report or boolean can mint this evidence. Browser Fetch's cookiefree auth-only
profile is separate from native ordered-header RIT and does not cover HttpOnly or
path-scoped application cookies. Stream uploads/upgrades/gRPC/trailers remain
unsupported by v0.1; bounded complete request/SSE-response profiles are distinct.

R15 accepted and implemented shutdown addition: serve_handler_until(handler,
Limits, shutdown:impl Future<Output=()>+Send, drain_timeout_ms:u64) returns
io::Result<()>. Tracked connections are canceled/closed and joined; global cancel
reaches LocalContext clones, and all admission permits must return before Ok.
A detached worker holding context prevents a false graceful acknowledgement.
Drain timeout is TimedOut, never success. Drop cancels/aborts but cannot promise
graceful drain. Runtime retains custody/service state until actual until success
or the final remaining worker ends. No partial-response503 is appended on stop.

Audit/runtime contracts accepted: reuse the new audit CheckpointWitness and
WitnessServiceClient plus durable signed ControlAuthority/ActionPermit; no new
signing, RIT or scoring implementation here. A shared Log+Custody lifetime owner
is not yet implemented. Authority inline-key startup remains experimental debt;
do not duplicate runtime::startup::SealedAudit or call it sealed-qualified.
Remaining native remote-client/multi-user, full runtime, provider and product
e2e requirements stay tracked in the follow-up plan until source-bound execution.

Component verification: Windows25 scoped tests, release LocalHandler3 and Linux
network-none Docker22 plus explicit other-UID parent1 pass. Two default Linux
privileged/helper ignores are not included in passed counts. Actual Windows
pipe grant/PID, no credential disclosure,144 nonreaders, exact response binding,
profile scope and canceled blocking-work lifetime are tested. Synthetic local
management is transport conformance only; runtime control/session qualification
and actual remote Windows host denial are still not_run. Current public summary
is evidence:root/20261107053239; private logs retain failed and earlier sources.

### Historical v0.1 baseline

Version 0.1.0 implements the accepted Node service contract without changing
scoring. One process owns one installation, tenant, origin, revision and peer
identity. Its only endpoint is POST /adapter/v0.1/decision; unknown/admin routes
are denied. nginx's current conformance protocol is not accepted as Node.

Read bounded startup configuration through private inherited stdin, then emit
readiness and a fresh 256-bit base64url bearer only through private piped stdout.
The supervisor must deliver that secret only to the configured adapter. No
operator-selected/reused bearer exists. Every restart rotates credentials.
Readiness includes the current application-http capability manifest and exact
route inventory readiness for node-raw versus fetch-normalized. Routes requiring
absent planes are reported unready even while the IPC listener is running.
In-memory atomic nonce retention through deadline plus skew is sufficient only
inside that fresh credential epoch. Nonces are never evicted to admit capacity;
capacity exhaustion rejects new decisions. Scope is checked before reservation.
Any consumed nonce remains consumed after computation/audit/response failure.
Observed wall-clock rollback poisons the epoch so evicted nonces cannot become
valid again; recovery rotates the credential by restarting the process.

Use safe library IPC APIs. Unix sockets require absolute filesystem paths,
canonical private owner-controlled parent, 0600 socket, actual peer UID and no
symlink/reused socket removal. Parent 0700 protects macOS where socket mode alone
does not authorize clients. Windows pipes require exact local pipe names,
protected generated DACL restricted to the configured SID, remote rejection,
first-instance protection and actual OS client PID equality. SID ACL creation
and PID matching are distinct checks. PID reuse/handle forwarding and trusted
same-user processes are residuals; no header establishes locality. Native
multi-user and actual remote-host rejection remain qualification gates.

All inherited configuration/audit paths belong to a trusted supervisor. Audit
directories must be private and non-reparse/non-symlink. Windows directory ACL
inspection must fail closed if it cannot establish the configured SID owner and
no grants to other principals. No permissive default DACL is accepted for pipes.
No runtime unsafe code is allowed in this crate.

Strict HTTP/1.1 framing is bounded to 16 KiB headers, 100 headers, exact decimal
Content-Length and 1,600,000 body bytes. Only one request per IPC connection;
Connection: close on every response. Reject TE, duplicate singleton headers,
obs-fold, control characters, upgrade, Expect, pipelined extra buffered bytes,
wrong method/target/media type. One fixed connection lifetime bounds slow input
and output; global/per-peer slots and fixed-window rate limit bound admitted
work. OS backlog/ACL are additional controls, not unbounded task creation.

Reuse integrity::adapter::parse before any secondary projection of validated
fields. Reuse network::report::build and scoring::engine::Engine. Header values
are application observations only: never populate collector client evidence
from User-Agent. Production executable only implements application-http profile
with explicit absent TLS/H2/collector/RIT. Every configured route declares exact
method/path and required raw TLS/H2/header-order/collector/RIT capabilities.
Unsupported required capabilities return 422, unknown routes and stale revision
409, scope mismatch 403, credentials 401, malformed input 400, capacity 429,
audit/engine/state/clock failure 503. Reserved /_humanymous subtree never forwards.
Route inventory must be nonempty and collision-free. RIT-required routes cannot
claim bootstrap grace without a Rust-owned browser/session contract.

The library accepts a trusted in-process evidence provider, never IPC-provided
session reports. Its report must be bound to the already validated request by
the provider. Test-only fixture evidence proves ALLOW audit ordering only; it is
not a production profile. Future capture providers require a new normative
authenticated connection/session binding contract before shipping.

Persist all decisions through the existing audit::log::Log before returning
the exact seven bound response fields. Store only nonce, scope, revision,
verdict and a SHA256 envelope commitment in audit (no raw body/headers/token).
Deadline is checked again after durable append; expired work yields no 200.
Audit recovery uses supplied trusted head/keys, never ephemeral replacement
identities. Independent witness anchoring and power-loss acceptance remain
deployment gates. Administration/erasure/kill-switch/permanent bans are not
implemented here and cannot be invoked using the decision bearer.

### Review reflected

Blue: one shared engine/network/parser/audit authority; no duplicated tells.
Red: restart token reuse, capability forgery, route drift and framing ambiguity
become explicit rejects. SRE: bounded admission, deadline, poisoned audit and
private lifecycle. DPO: no raw requests/credentials in audit or evidence.
Evaluator: fixture ALLOW, native process, Docker and provider results are
separate; missing browser/admin qualification is not described as completed.

## Why

Adapters previously had only a mock authority. A local executable must turn the
accepted protocol into actual authenticated, replay-safe, durable decisions,
while honestly preserving the missing original-client observations.

## Verification

Cargo unit/integration/native child-process tests cover schema/auth/scope,
replay/restart/body/capture/route/deadline/admission/audit failure. MSRV 1.99.1 and
advisory scan cover new dependencies. Docker product lab and remote native tuples
remain distinct evidence. See implementation plan and crates/authority/INTEGRATION.md.

### Native Windows storage privacy

Replace the embedded PowerShell ACL probe with one shared safe Windows security
descriptor implementation in audit storage, also consumed by sealed custody.
Read the descriptor from the opened non-reparse file handle, compare the actual
owner SID and every ACE, reject null/empty DACLs and unsupported/foreign ACEs.
Authority directories require a protected DACL; custody directories retain their
explicit owner-only file/directory inheritance requirement. A file policy is
distinct from either directory policy. Do not mutate ACLs during verification.
The small protected-flag decoder consumes native DACL-only SDDL, never arbitrary
user permission text: Microsoft documents P as SE_DACL_PROTECTED at
https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-string-format.
No inline PowerShell or new repository unsafe block is permitted. Unix ancestor
checks and trusted-supervisor path-race premises remain unchanged. Stricter
rejection of empty/unsupported ACE forms must be tested and documented; this is
not a new detection rule. Current approved toolchain/MSRV is1.99.1; prior1.88 execution records remain
historical context.

## Python and Ruby provisioned profiles (2026-11-07)

The shared integrity profile canon additionally provisions PythonAsgi with
humanymous.python / asgi-normalized / config python-asgi, PythonWsgi with
humanymous.python / wsgi-normalized / config python-wsgi, and RubyRack with
humanymous.ruby / rack-normalized / config ruby-rack. Version is
0.1.0-experimental.1. Configuration names select policy independently of wire
contents. Unknown names reject. Existing omitted configuration remains Node.
Readiness uses explicit profile equality, including nginx; normalized routes
requiring raw header order are unavailable. TLS, original H2, collector and RIT
requirements remain unavailable. ASGI preserves ordered duplicate pairs without
original casing claims; WSGI/Rack CGI maps lose order and duplicates. This
configuration inventory is not adapter runtime qualification. Client API, native
peer checks, IPC and scoring are unchanged. Python/Ruby runtime validation is
NOT_RUN pending the standard hosted delivery gates. Prior component passes apply
only to their recorded source. Shared execution policy is defined by
sot:root/program-roadmap and sot:root/delivery, not duplicated here.

## Shared private credential ownership

Credential owns Zeroizing<String>; generated entropy uses Zeroizing<[u8;32]>.
from_private_provisioning consumes Zeroizing<String> without cloning and accepts
exactly64 lowercase hex ASCII bytes, the existing generated grammar. It rejects
noncanonical casing, whitespace, controls and length errors with a static code.
Exact Bearer scheme and existing constant-time fixed-length comparison remain.
No Debug/Serialize/Clone is added. Representation validation does not prove
entropy, supervisor identity, role/scope separation or fresh restart epochs.
Management must use independently provisioned credentials, never adapter/session
credentials. Caller-owned request/wire copies are outside this type's erasure
guarantee. Runtime owns integration and actual hosted qualification; no inline
key startup or physical-memory erasure claim follows from Zeroizing ownership.

## Apache and ASP.NET Core provisioned profiles

Root-owned integrity canon adds Apache and DotnetAspNetCore. Trusted configuration
names apache and aspnetcore bind respectively humanymous.apache/apache-normalized
and humanymous.dotnet/aspnetcore-normalized, version0.1.0-experimental.1. Exact
normalized readiness keys apache_normalized and aspnetcore_normalized never imply
raw TLS/H2/header ordering/collector/RIT. Peer, connection, capture and stream
remain null at this wire profile. Config selection is independent of wire ID or
mode; mixed identity/mode rejects. Go continues the existing HTTP Gate profile;
no new Go producer is inferred. Implementation source does not qualify actual
Apache MPM/hooks or ASP.NET host behavior. New source execution is NOT_RUN.

## Audit-owned sealed lifetime adapter

Audit29ee901 provides log::SealedLog owning custody/WAL/key guards together.
Authority implements its local DurableAudit trait directly for that shared type,
delegating append and preserving I/O errors before any decision response. No
key/lifetime wrapper is copied into authority. Composition callers must open
SealedLog through audit's existing unlock/witness contract before listening and
retain ownership through drain/flush. This adapter alone does not migrate the
authority binary's inline-key startup or qualify external witness independence.

## Executable sealed startup transport contract

The authority executable opens its configured audit source before binding IPC.
Optional audit.keys specifies existing custody directory and an explicit tagged
unlock: inherited-private-pipe with Zeroizing passphrase, or os-service-store
with nonsecret service_name/credential_id. Protected unattended Windows input
reuses native-security::secret::read_service_secret and audit UnlockCredential
Bytes; unsupported platforms reject without plaintext/inline fallback. Controlled
parent/broker/owned hosted lab may select the inherited mode. The caller must
verify parent ownership/provenance externally: JSON, nonterminal stdio or an
arbitrary Read is not attestation. No argv/env/plaintext file fallback is added.
Raw config and unlock buffers are zeroizing; readiness never serializes them.
Readiness labels inherited transport as external-provenance-required, reports
external_parent_attested=false, and never equates it with OS-store attestation.

SealedLog retains custody and WAL ownership for the full authority lifetime.
No keys are provisioned by startup, and an unlock/recovery failure never selects
another source. Mixed inline/sealed, missing source and unknown unlock modes
reject. Required witness absence rejects before opening directories or consuming
unlock; broker injection uses startup::open_audit(...,Option<PinnedWitness>).
The standalone binary has no witness broker transport, so requiring a witness
there currently refuses before listening. Inline legacy keys remain explicitly
inline-component-nonproduction. This is not production sealed qualification.

## Draft bootstrap v0.2 — authenticated one-use observer provisioning

Version0.2.0-draft. Channel implementation status: DRAFT, NOT IMPLEMENTED.
Topology is selected by root471058e9 in the Rust migration SoT; R2 launcher
architecture and R3 loss semantics are also accepted as recorded below. Native-
security critique, CLI/runtime lifecycle agreement and root contract review are
implementation gates. No scoring, decision bands, browser session, unseal-secret
handoff, writable settings, erasure, kill or ban capability is added.

### B01. Concrete process architecture and ownership

Root selected three runtime roles and two persistent MAIN identities: finite
authorized publisher P; persistent observer O that IS the bootstrap receiver/broker AND the
actual management client; independent service host S that SENDS its fresh observer
credential. Broker!=observer forwarding is Unsupported. ManagementPolicy pins
actual O, never P's short-lived PID, S or a purported observer field in an envelope.

Root has APPROVED the initial Windows launcher architecture: the authorized
installer/controller configures two exact owned
SCM own-process service roles. SCM launches S using the protected service-host
entry and O using new Rust `humanymous observer-host --plan PATH --plan-sha256 DIGEST`.
O has its own observer virtual-service principal, distinct from S. The plan is
bounded nonsecret protected metadata; its hash does not prove issuer authority.
For unattended Windows, root additionally accepted a finite one-shot registered
P publisher service: THREE registered roles, only TWO persistent processes S/O.
The original installer/controller authorization is distinct from P's stable
publisher service principal. A protected original installer/root/SCM-owner record
must bind that exact publisher principal, artifact and ORIGINAL public key; no
self-issued trust, missing-key reset or production key creation is authorized.
The previous interactive-P TokenUser==installer/root-owner assumption does not
apply unchanged to this delegated service publisher.
P may start/wait the exact S/O TransportPrepared instances, then publish and exit.
Avoid dependency cycles: S/O must not wait for P service stop while P waits for
their acceptance. Boot/recovery orchestration must specify fresh instances/grants,
conserved absolute deadlines and failure cleanup. Actual service-principal key
availability across service stop/start and reboot needs genuine hosted proof;
no interactive-profile or machine-DPAPI fallback. This architecture direction
does not qualify unattended recovery or authorize local SCM/account mutation.
No observer-host command, second SCM implementation or grant publisher exists by declaration.
Root approval does not authorize actual local SCM/account mutation or qualify runtime.
The ordinary fixture instead uses retained supervisor-owned child handles under
ordinary principals. It is not genuine SCM or cross-principal qualification.

| Actor | Expected native identity/launcher | Ownership and stop authority |
|---|---|---|
| P | Finite publisher under stable delegated service principal, distinct from original installer/controller | Uses original protected delegation to publish exact role/epoch authorization; never receives management authority or becomes a persistent peer |
| O | Proposed independent observer SCM role; actual principal, PID, retained native instance and approved artifact | Owns receiver, client credential, authenticated liveness channel and witness supervision through final service flush |
| S | Independent SCM own-process service; actual principal, PID, retained native instance and protected launch artifact | Owns sealed writer, fresh server credential, pending/active management listener, public ingress and activation/revocation; SCM owns process lifetime |
| W | Distinct witness producer, supervised by O with retained actual child instance and independently pinned key | Subordinate native child, not a third MAIN role or an alias for O; remains usable until writer flush; no organizational-independence claim |

Rejected alternatives: installer as long-lived observer; a separate broker ACK
forwarded to another observer; arbitrary callback sink(service,id,&str); secret-
only reconnect/PID rebinding. An external short-lived status client to a separate
O would need its own authorized request contract, not forwarded observer credentials.
Initial v0.1 assumes no hot rebind or implicit renewal. Replacement/restart needs
fresh authorization, instance binding, grant/credential epoch and policy decision.

### B02. Native proof and noncircular trust prerequisite

Both ends verify actual native PRINCIPAL + PID + PROCESS INSTANCE from the accepted
connection against retained expected native pins. Windows requires actual peer
TokenUser SID evidence (and explicit service-role identity where distinct), not a
claim that the existing DACL queried that token. Unix requires UID and peer PID;
unavailable PID/instance proof is Unsupported, not UID-only success. PID reuse is
resisted with a retained native process handle/pidfd or reviewed equivalent, never
by reopening an arbitrary numeric PID. Native manager discovery must match exact
owned service, installed artifact and protected launch policy before pinning.
Verify before any secret read/write and again before activation/privileged effect.

Legacy Peer/NativePeer are insufficient: Unix retains UID only, Windows PID only.
Current UDS0700/0600 and same-SID protected pipe policy are NOT controller/service
cross-principal delegation. Native-security must provide exact owner/allowed-peer
policy for BOTH bootstrap O-owned/S-accessible and management S-owned/O-accessible
endpoints. No world/group/default grants, ACL repair during verification, remote
UNC/TCP fallback or impersonation privilege is inferred. Failure to obtain peer
TokenUser/process query rights must refuse, not substitute a configured SID string.
Connection-time credentials do not attest ongoing writers or absence of delegated
handles. Same-principal hostile code/handle delegation and privileged OS compromise
remain explicit ceilings; challenges do not solve them.

P must establish a native-authenticated protected trust record: installation,
service/observer roles, allowed principals/artifacts, purposes and epoch policy.
Each actor validates it INDEPENDENTLY through native-security; JSON/hash/path or
arbitrary Read/Write is not attestation. Expected instances come from verified
native manager/owned-launch handles, not the incoming frame. Root accepted PUBLIC
signed targeted authorization in place of the initial shared-secret Hello. The
signed record is not a bearer credential: both actual native instance/principal
proofs remain mandatory. A grant cannot supply its own trusted publisher key.

The concrete candidate is WindowsScmSignedGrantV1 in plan:root/20261107103828:
explicit originally authorized publisher key in the exact delegated service
principal's protected native store, protected original installer/SCM-owner trust
root, and a public signed record targeting retained
actual S/O instances. Existing read_service_secret is read-only and does NOT
implement this publisher. Architecture acceptance does not implement or qualify
the actual issuer, publication, discovery, key persistence or native proof. If
that backend or required native proof is absent, refuse BEFORE unseal, secret
transfer, writer/listener activation or public readiness. Same-user fixtures cannot
qualify the missing production source. No initial key or witness auto-provisioning.

### B03. Exact trusted binding and typed API proposal

Immutable binding fields: version; installation, tenant, origin, revision and
application authority; exact service name, observer role and credential ID;
purpose=observer-status; service/observer native-instance epochs; fresh credential
epoch and grant ID; sender=S and receiver=O native pin IDs; bootstrap expiry,
credential-lease expiry and trusted budget policy. Broker==observer is actual
verified equality, never a third wire PID. Expected scope is provisioned, not
inferred. Service/selector grammar reuses native-security's exact owned namespace;
existing application scope limits stay unchanged. Grant/credential epochs and
challenge nonces are CSPRNG-fresh. Native-instance epochs must be tied to actual
retained instance evidence, not self-asserted random text.

Proposed authority::bootstrap API (not existing code or frozen signatures):

```rust
// Native-security owns the private constructors and actual provenance checks.
struct VerifiedServiceGrant;
struct VerifiedObserverGrant;
struct VerifiedProcessInstance;
struct VerifiedWitnessCapability;
struct ObserverWitnessLease;
struct VerifiedManagementEndpoint;
struct SharedSealedAudit;
struct RuntimeActivationEvidence;
struct VerifiedServiceStopReceipt;
// Authority owns validated scope, protocol and secret/lifetime guards.
struct BootstrapBinding;
struct ServiceBootstrapSender;
struct AuthenticatedObserver;
struct ObserverBootstrapReceiver;
struct PendingObserverListener;       // S owns fresh credential and disabled listener.
struct ServiceProvisionedObserver;    // S owns server lease; O receipt is not Ready.
struct ObserverProvisionedClient;     // O owns received client credential; ACK sent.
struct ServiceObserverLease;
struct ObserverClientLease;
struct ObserverCredentialReceipt;
struct ServiceReadyReceipt;
struct LifecycleFacts; // Cause, drain, flush, join/reap and native-exit facts stay separate.
enum ObserverCloseOutcome { Closed, Unknown } // unknown cleanup maps to exit74.

ServiceBootstrapSender::connect(binding: BootstrapBinding,
    grant: VerifiedServiceGrant, cancellation: Cancellation)
    -> Future<Result<AuthenticatedObserver, BootstrapError>>;
ObserverBootstrapReceiver::bind(binding: BootstrapBinding,
    grant: VerifiedObserverGrant) -> Result<Self, BootstrapError>;
ObserverBootstrapReceiver::receive_once(self, witness: ObserverWitnessLease,
    cancellation: Cancellation)
    -> Future<Result<ObserverProvisionedClient, BootstrapError>>;
AuthenticatedObserver::observer_identity(&self) -> &VerifiedProcessInstance;
AuthenticatedObserver::witness(&self) -> &VerifiedWitnessCapability;
PendingObserverListener::bind(&authenticated,
    management_endpoint: VerifiedManagementEndpoint)
    -> Result<Self, BootstrapError>;
AuthenticatedObserver::provision_once(self, pending: PendingObserverListener,
    sealed_audit: SharedSealedAudit, witness: VerifiedWitnessCapability,
    cancellation: Cancellation)
    -> Future<Result<ServiceProvisionedObserver, BootstrapError>>;
ServiceProvisionedObserver::observer_receipt(&self) -> &ObserverCredentialReceipt;
ServiceProvisionedObserver::announce_ready(self,
    runtime_activation_evidence: RuntimeActivationEvidence)
    -> Future<Result<ServiceObserverLease, BootstrapError>>;
ObserverProvisionedClient::wait_service_ready(self, cancellation: Cancellation)
    -> Future<Result<ObserverClientLease, BootstrapError>>;
ServiceObserverLease::revoked(&self) -> Future<()>;
ObserverClientLease::request_status(&mut self) -> Future<Result<Status, Error>>;
ObserverClientLease::await_service_stop(&mut self, stop_budget: StopBudget)
    -> Future<Result<VerifiedServiceStopReceipt, LifecycleFacts>>;
ObserverClientLease::close_after_service_stop(self,
    stopped: VerifiedServiceStopReceipt, stop_budget: StopBudget)
    -> Future<ObserverCloseOutcome>;
```

Grants, credentials and leases are non-Clone/non-Debug/non-Serialize with private
provenance constructors; no raw stream/credential-store escape is public. S's
PendingObserverListener creates its own fresh Credential and carries native bound-
listener evidence for the SAME scope/O instance, not a caller's success bool. O
never owns S's writer/server credential lease. Runtime must define the real
activation-evidence constructor; observer ACK cannot construct ServiceReady.
O transfers actual W lifetime ownership into receive_once; it remains retained by
client/receiver state through final writer flush. Wire witness references do not
mint VerifiedWitnessCapability: S independently obtains that proof through the
native/audit path. Witness pin/key/lease IDs are distinct binding fields.
observer-status confers NO service-management mutation, permanent kill or erasure.
await_service_stop only observes, and close_after_service_stop closes O's OWN
resources after verified flush/native stop. Normal S stop requires a SEPARATE
verified native-manager/service-lifecycle authorization, not this credential or
an arbitrary stop flag. Closing O's liveness is loss/failure, not authorized S stop.
receive_once sends O's receipt BEFORE wait_service_ready. No readiness circular
wait. Service unlock and witness authentication remain separate shared primitives.

### B04. Secret frames, phases and distinct budgets

Use shared native transport internally, not application HTTP DTOs that currently
clone non-zeroizing Authorization strings. Root accepted a NEW explicit protocol
version/domain with no legacy fallback. Proposed exact wire selection for review:
protocol humanymous.observer-bootstrap/0.2, signature domain
humanymous.observer-bootstrap.signed-grant/1, control magic ASCII HMBP0002
(8bytes), phase byte, zero flags byte, u16 BE metadata length, exact UTF-8 strict
JSON metadata, then only the phase's fixed secret slot. Metadata<=3072bytes;
frame<=4096; aggregate STARTUP bytes<=32768. Reject lengths before allocation,
duplicate/unknown/missing fields, extra bytes, phase changes and schema ambiguity.
No compression, redirects, fallback or EOF-as-message-framing. Reject HMBP0001,
legacy capability Hello, unknown versions/domains and any mixed schema. Hello
carries the bounded PUBLIC signed targeted authorization plus exact native-bound
metadata; it has NO secret slot. Offer alone carries a fixed64lowerhex observer
credential. Exact canonical signed-record encoding remains subject to native-owner
review; these proposed identifiers are not independently qualified wire formats.
The COMPLETE signed public authorization, including every claim, signature and
its encoding overhead, MUST fit INSIDE Hello's3072byte metadata cap. There is no
separate grant slot, unbounded file attachment or continuation bypass. Validate
declared lengths before allocation and before releasing any observer credential.
For the native v0.2 Prepared/Commit proposal, this means BOTH signed records,
their linkage and encoding together inside that single cap, not3072 per record.
Prepared and Commit must have unambiguous signed record-kind/domain separation;
Prepared alone, wrong-kind or cross-grant Commit cannot authorize secret release.

| Phase | Metadata cap | Secret slot bytes |
|---|---|---|
| Hello (including complete signed authorization) | 3072 | 0 |
| Challenge | 3072 | 0 |
| Offer | 3072 | Exactly64 ASCII lowercase hexadecimal bytes |
| CredentialReceived | 3072 | 0 |
| ServiceReady | 3072 | 0 |
| Liveness/stop/flush control | 3072 maximum; exact schema still pending | 0 |

Each complete frame is exactly12bytes of fixed header plus the declared metadata
and the phase-specific slot; the4096byte frame ceiling also applies. Reject any
nonzero secret slot outside Offer, wrong Offer slot length/alphabet, unknown phase
or trailing bytes. Buffered trailing bytes are not accepted as an implicit next
valid frame; the expected protocol transition must admit each subsequent frame.
Before implementation native-security must freeze the EXACT signed bytes: domain
separator and length, canonical field encoding/order and numeric rules, signature
and key-fingerprint encoding, and strict duplicate rejection. A domain string
alone is not a canonical signature format or an implementation-ready contract.
No response echoes the credential or hashes a potentially weak supplied secret. Fresh
observer credentials are generated internally with the shared CSPRNG primitive.
Owned buffers zeroize on ALL exits; no physical-memory/kernel/allocator guarantee.
Errors/logs/readiness/argv/env/files never include raw secret or parser input.

Startup order:
0. S/O TransportPrepared: SCM_RUNNING means native process/discovery readiness
   ONLY. AppReady, ingress, unseal and writer remain false; independent absolute
   startup watchdog and Stop handler are armed. Existing application-Ready
   publisher MUST NOT run early. Doctor/status/installation preserve this
   distinction. P pins both actual instances and publishes targeted authorization.
1. S Hello -> O: native pins and exact signed public grant/binding; no credential.
2. O Challenge -> S: fresh nonce and separately verified witness capability binding.
3. S validates required witness BEFORE writer open/unseal; opens shared SealedLog,
   binds DISABLED pending management listener and records typed preparation event.
4. S Offer -> O: fresh observer credential, exact scope/epochs/both challenge nonces.
5. O CredentialReceived -> S: actual intended observer owns client credential.
   This ACK is sent before public readiness and proves neither S activation nor W.
6. S rechecks native/liveness/deadlines/witness, durably commits typed control event,
   validates actual runtime listeners/capabilities and activates; S ServiceReady -> O.
7. O uses only the purpose-scoped management API after actual S ServiceReady.

One absolute STARTUP deadline, proposed<=30seconds, covers admission, accept/
connect, all startup I/O, ACK and Ready. It is never extended by a frame. Persistent
observer PROCESS lifetime is distinct from startup. Proposed credential lease TTL
must be explicitly configured by trusted policy; NO universal24hour cap/default
or renewal/restart policy is accepted. Omit/unknown policy refuses production
activation. No in-place renewal/rebind is assumed. Authenticated liveness
round trips have separately bounded deadlines/rate and strict epoch/sequence;
they do not renew grants or extend lease TTL. Stop/drain/flush/join use existing
StopBudget, not the startup timer. Every budget checks monotonic expiry and wall
rollback. One pending grant/one active observer epoch; bounded queue/frame/rate,
no detached worker outside cancellation/admission ownership. Flush/kernel enqueue
is not application ACK or completed stop. Liveness/stop frames are secret-free,
individually bounded; their traffic does not reset startup or authorization clocks.

### B05. Consumption, audit, ambiguity and lifetime/stop table

O atomically consumes the matching grant after actual S/signature/binding checks,
before Offer. S consumes its sender grant after verified O Challenge, before Offer.
An unrelated invalid native peer cannot burn another grant. After reservation,
any bad phase/scope/nonce, timeout, cancellation, entropy or audit error burns that
attempt. No retry/replay/cache/durable-record resurrection. A new explicit attempt
requires fresh authorized grant/credential epoch/nonces and valid current pins.
Missing required witness remains fail-closed before unseal; W Ready key/node data
is continuity content only, not actual peer identity/deadline/independence evidence.
S pins actual W and its independently authorized public key using shared audit/
native primitives. No reader, signer or witness transport is forked in bootstrap.

Preparation/activation records use audit-owned typed minimized operational events,
not fabricated SessionReport/score decisions. Audit failure prevents activation.
Crash after append is an ambiguous consumed attempt, not instructions to replay.
No exactly-once distributed delivery claim. Lost ACK/Ready leaves uncertainty;
close the authenticated liveness channel and burn/revoke rather than resend.
S owns server credential/listener revocation; O owns client credential/receiver/
witness lifetime. Loss is detected by authenticated channel, actual process pin or
hard expiry, not instantaneous omniscience. Secret possession never authorizes
new-PID reconnect. Observer receipt is not service readiness or witness liveness.

| Phase | O owns/does | S owns/does | Required conclusion |
|---|---|---|---|
| Trust/native discovery | Verify protected role/grant, actual S, receiver and W | Verify protected role/grant, actual O and separate W capability | Missing native source/proof is Unsupported before unseal |
| Offer/receipt | Own zeroizing client credential; ACK exact receipt immediately | Own sealed writer and disabled pending server credential/listener | ACK before public Ready; no broker forwarding |
| Activate/live | Retain receiver/client/W; await Ready; maintain authenticated liveness | Commit audit, activate actual listeners, announce Ready, pin O in management | Separate O receipt and S readiness; no silent renew/rebind |
| Separately authorized S stop | Observe S stop; retain W and supervision | Verify independent native-manager lifecycle authority, revoke admission, drain and flush | observer-status grants no stop authority |
| Observer loss/epoch expiry | Preserve actual cause and unknown scope | Irreversibly revoke readiness/admission/volatile contexts, cancel/join and flush while W usable | Failed epoch even if cleanup succeeds; no silently degraded writer |
| Final flush | Verify actual S flush outcome, not EOF/own ACK | Freeze further writes; authenticate FlushComplete after successful shared flush | W remains alive through this point |
| Final cleanup | Observe actual S flush and SCM instance stop, then stop/cancel/join/reap OWN W/workers | Close writer/custody only after own workers/flush obligations; SCM owns S lifecycle | Stopped only with evidence for each actor |
| Timeout/peer death/unknown worker | Bound waits and preserve cause/unknown scope; no false graceful report | Fail closed; do not reuse token or assert flush without completion | Unknown/74, explicit controller recovery; never fake Stopped |

SCM S is not an owned disposable child. A separate verified native-manager lifecycle
authorization is required to request its stop; the observer lease only observes.
O cannot call Child::kill on a numeric SCM PID. Ordinary fixture
owned children are a distinct permitted kill/reap scope. O must not stop W merely
because bootstrap EOF or its own client is closing while S could still flush.
Unexpected O/W death can make flush fail; that is Unknown, not graceful success.
A bounded timeout cannot prove that a blocking worker returned. LifecycleFacts and
cleanup evidence must retain that uncertainty; Drop is not proof of join/reap.

Root accepted the first-loss semantics in runtime-application.sot.md, section
observer/broker-loss v0.1. One shared irreversible epoch gate synchronizes origin
connect/write acceptance against revocation: no check-then-await race and no lock
held over await or fsync. No new origin effect may begin after revocation
linearizes; prior partial effects remain unknown, without retry, replay or counter
reset. epoch_expired, observer_lost and witness_lost remain distinct causes.
W availability and bounded cancellation/join must be established before secret
read; a cached Ready receipt cannot replace current witness proof or an independent
watchdog. An unreturned worker remains Unknown/74, never Stopped.

### B06. Wiring, verification matrix and review blockers

Authority owns protocol/framing/state/leases and shared transport extensions.
Native-security owns verified role/grant issuance, both process-instance proofs,
manager discovery and exact cross-principal access. CLI owns new observer-host/
launcher/controller supervision. Runtime owns S readiness/loss/flush integration.
Audit owns typed events and W protocol. Promoting native-security from Windows-only
to shared dependency may require root's manifest/feature window; no manifest or
new cryptographic implementation is authorized by this draft.

| Gate | Required proof | Non-coverage |
|---|---|---|
| Pure state/codec | Exact schemas/bindings/direction, concurrent one-use/replay, partial/oversize frames, all transition cancels, separate deadlines/TTL, ACK-before-Ready, audit refusal/restart | Mock proof/duplex I/O is not native identity |
| Actual native pair | Both UID/SID/PID/instance, wrong principal/PID/instance, reuse, endpoint owner/ACL/link replacement, missing provider, backpressure/lost ACK, bounded cleanup | Unsupported tuples stay Unsupported; no same-UID-only success |
| Ordinary owned fixture | Real O/S/W children, retained actual handles, authenticated witness before writer bind, Offer/receipt before Ready, status request, flush-before-W-stop | No SCM/account mutation or production OS-store attestation |
| Genuine hosted SCM | Actual distinct O/S service roles, authorized protected grant publication/discovery, native proof, install/upgrade/rollback/replay/stop/remove, independently restored controller rights | ACT/job labels/inherited Write do not qualify |
| Failure/lifetime | Missing O/ACK/W, wrong scope/purpose, death during each phase, lost Ready/flush ack, blocked worker, restart/same PID replacement, no old-secret reconnect | Timeout/74 is not Stopped or closed custody proof |
| Product/evidence | Owned Docker Core/Gate/observer/W topology; exact source/OS/engine/run and per-case denominators/raw hashes; no secret leaks | Native components, ignored/skipped/mock or unreturned raw are separate |

Critique reflected so far: topology/API inversion corrected (S sender/server lease,
O receiver/client lease); same-UID/PID text limitations explicit; cross-principal
transport gap explicit; finite delivery is not exactly-once; separate startup,
lease/liveness and drain budgets; ACK-before-Ready; W identity independent of ACK;
SCM lifetime not owned-child termination. These requirements reflect actual early
CLI/native-security feedback, not final approval.

P0 R1: Native-security must settle actual noncircular protected grant issuer,
publication/discovery, process pin/TokenUser/peer PID primitives and supported ACL
profile. None is implemented by old Peer/NativePeer or the read-only store API.
Root accepted signed public targeted authorization and distinct TransportPrepared
architecture only; native primitive probe preparation is authorized, not complete
R1/channel implementation. Remaining P0 gates: actual cross-SID primary TokenUser
query/retained handles without SeDebug/admin fallback; actual create-new SCM/root
owner binding and existing-key no-reset; typed authenticated P acceptance without
management authority; process-wide one-claim, rollback and conserved deadlines;
actual loaded-artifact binding; atomic release/epoch gate; and explicit unattended
restart/republication. Root accepted the finite one-shot P service direction, not
actual service-principal WinCred availability or production key provisioning.
Original interactive installer's private WinCred must not be assumed available
to that service or after logout/reboot. No implicit renew, replacement key or TTL policy.
Linux and macOS require separate backends and remain unqualified by this candidate.
R2 architecture accepted by root: two SCM-owned own-process O/S roles under
distinct service principals, with W supervised by O. The new observer-host command,
native implementation and actual SCM qualification remain unimplemented; no local
SCM/account mutation follows from architecture acceptance.
P0 R3 implementation: root accepted irreversible first-loss semantics, but actual
typed activation-evidence constructors, atomic admission gate, witness lifetime,
typed events and bounded cancellation/join/Unknown74 still require implementation
and independent review. Semantic acceptance is not native qualification.
P1 R4: Exact Rust constructors, native support matrix and proposed timer/TTL limits
need final review. No P0/P1 finding is resolved merely by listing it here. Final
native-security adversarial verdict and root contract acceptance precede channel
implementation; there is no runtime/SCM qualification from this draft.
