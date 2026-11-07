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
The binary's owned raw config and unlock buffers are zeroizing; readiness never
serializes them. Inline component hmac_key_hex/signing_seed_hex fields are
Zeroizing<String> from deserialization, including when source/witness/private-
directory preflight refuses before consuming them. Successful inline handoff
moves those wrappers directly, without nested wrappers or encoded-string clones.
This is CA-SECRET-01 encoded-key drop ownership remediation, not production
promotion of inline keys. Caller-retained original JSON, serde intermediates,
transport/provider/allocator/kernel copies have separate lifetimes; field types
do NOT prove their erasure or universal RAM erasure. Audit remains Deserialize
only, without Debug/Serialize. JSON string fields are unchanged; direct Rust
struct constructors must supply Zeroizing<String> instead of String.
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

## L3/L4 final-consume contract (existing protocol0.1)

BoundResponse is a non-Clone single-owner RESPONSE, not server nonce state or an
atomic native origin permit. verdict()/expires() are inspection only. Consumers
must retain the actual BoundResponse through body preparation, awaits, event-loop
dispatch and GIL/GVL entry, then consume it at the trusted actual host application
invocation boundary. There must be no further await/queue before that invocation.
An ALLOW copied into an integer, bool or enum before a later wait is not a live
handoff grant. DENY/CHALLENGE can be consumed and returned without origin.
Pending::decide(&self) is currently repeatable; L4 must not re-decide, generate a
fresh nonce or recreate a result to bypass expiry/cancellation or obtain a second
origin handoff. Server replay state remains independently authoritative.

check_handoff(expires) only checks local clocks/budget. It cannot replace final
opaque-result consumption after the last host wait, or certify host liveness and
native epoch atomicity. Actual host qualification requires late-connect/queued-
callback expiry and cancellation controls proving origin was NOT invoked. New
kernel/native-epoch effects still require the separately qualified same epoch
gate; protocol0.1 components do not mint that missing production capability.
The existing Authority API suffices for this final-consume correction; no public
origin/Ready factory, callback identity proof or new wire format is authorized.

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
// Root accepted Ready composition in Runtime; there is no Authority method
// accepting a Runtime-private evidence type or caller-provided activation bool.
// The consuming PreparedObserverSession/transport transfer is pending joint review.
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

#### Proposed shared irreversible epoch ownership (not an accepted API)

Authority owns the neutral observer epoch/gate core. Runtime already depends on
Authority; Authority MUST NOT depend on Runtime. Native-security owns the opaque
VerifiedBootstrapEpoch and retained actual process/token/manager evidence. An
Authority-private bootstrap composition constructor consumes that proof, separate
verified W capability/lifetime and trusted deadlines exactly once. There is no
public live/new/from-ID/from-bool/test-provider constructor. Missing native/W
proof cannot produce a production epoch; pure fixture gates never mint that type.

Proposed ownership split:

```rust
// Names/signatures require independent native/runtime review before code.
struct ObserverEpochOwner; // Unique; owns pins, witness and monitor cancellation/join.
struct ObserverEffectGate; // Restricted clone; SAME shared epoch, never a new owner.
struct ObserverRevoked; // First cause and monotonic deadline; not a stop authorization.
struct ObserverEpochCloseFacts; // Monitor join, worker drain, flush and native exits separate.

ObserverEpochOwner::effect_gate(&self) -> ObserverEffectGate;
ObserverEpochOwner::revoked(&self) -> Future<ObserverRevoked>;
ObserverEpochOwner::close(self, absolute_deadline: Instant)
    -> Future<ObserverEpochCloseFacts>;
```

Only the unique owner/native monitor can accept bootstrap phase transitions or
publish first loss. Its restricted gate clones cannot reactivate, extend expiry,
change scope, authorize stop or create another epoch. Owner Drop synchronously
revokes before releasing pins; it is NOT proof of async monitor/worker join. The
monitor is owned, cancellation-aware and explicitly joined within the conserved
stop deadline. Never detach it or report Stopped when it has not returned.

The same shared gate is retained by Authority credential-release operations and
Runtime application-Ready/origin adapters. Internal phases distinguish verified
publication+W/preparation, permitted secret Offer, actual observer receipt, and
Active. Origin admission is forbidden before Active. Readiness and volatile
contexts carry this same epoch; first loss irreversibly invalidates all of them
at one linearization point, without iterating or resetting persistent controls.
ShutdownRequest/watch channels can notify cleanup but cannot construct this proof.

Effect acceptance is a synchronous bounded nonblocking operation serialized with
revocation, not a separately returned permission followed by async I/O. Concrete
Runtime adapters must place the actual nonblocking socket connect/write acceptance
inside that critical section. Authority places actual secret-byte enqueue there.
WouldBlock accepts no bytes: release gate, wait outside, reacquire and recheck.
No gate lock spans await, token query, fsync, audit, blocking connect or join.
A future-returning/generic asynchronous callback is not this API. If a synchronous
adapter callback is used, it is trusted reviewed effect code, NOT an identity or
readiness proof; its exact bounded return type and call sites require review.
Previously accepted/partial kernel effects remain ambiguous after revocation;
cancel/revoke their credential, never claim that bytes cannot arrive later.

Root accepted Ready COMPOSITION in Runtime. Runtime privately constructs its
actual listener/readiness evidence; native proof,
durable audit and observer receipt are independently required. Authority cannot
import a Runtime proof type or accept activate(true). Proposed consuming interface:
Authority transfers VerifiedObserverPreparation (actual native publication/W,
same gate and restricted bounded transport), never a synthetic Ready proof.
It contains NO observer credential receipt: that receipt comes later, after
permitted unseal/preparation and Offer. PreparedObserverSession denotes that later
receipt-bound phase, never a prerequisite that creates a pre-unseal ACK cycle.
Runtime-private prepare_observed must consume and validate that native epoch and
independent W BEFORE any unseal/writer open. Existing ServiceHost::bind already
opens audit/writer state and MUST NOT serve as an unverified TransportPrepared
path. Only after that proof may preparation acquire actual listeners/capture/TLS/
audit resources. Its unique ObservedStartupOwner retains the prepared session,
host resources, transport, credential, pins, W and monitor throughout all stages.
The same owner's later provisioning transition obtains the actual receipt-bound
PreparedObserverSession; a caller cannot attach a claimed receipt or readiness.
activate_observed consumes that owner into RunningObservedService. Runtime
privately creates RuntimeReadyReceipt and emits ServiceReady only after gated
activation; that receipt never crosses back into Authority. Exact consuming types,
transport ownership, failure retention and revocation races remain P0 joint review;
this direction does not approve a native AcceptedObserverEpoch producer.
Failure returns an owned non-Clone ObservedStartupFailure with its stage and the
SAME cleanup owner; it cannot discard or detach transport/secret/pins/W/monitor.
Revoke immediately, retain ReadyDelivery as NotSubmitted/Partial/Submitted/Unknown
facts, then explicit bounded close/cancel/join/drain/final-flush reports separate
outcomes. Drop revokes but never asserts those asynchronous operations completed.
Partial Ready permits no origin, automatic resend, new epoch or watchdog reset.
The final complete Ready enqueue AND transition to Active must serialize with
loss on the same gate: no full-frame-then-later-activate race. Previously accepted
Ready remains a historical receipt after loss, never a continuing live authority.
The exact sealed activation adapter/evidence boundary remains P0; these rules do
not fabricate an implemented constructor. Expensive checks/audit happen outside
the gate, followed by final epoch/deadline/native-liveness validity at bounded
activation acceptance. A revoked epoch cannot be activated after an await.

Required races: first loss versus secret enqueue, origin connect/write and Ready;
WouldBlock/reacquire; cancellation during monitor wait; expired witness/peer pins;
owner Drop with gate clones retained; blocked/unreturned worker; partial write
before revocation; duplicate phase/old epoch; no mutation authority from status.
Every admitted effect is ordered before loss or refused after it. No test-only
factory or isolated shutdown-watch test qualifies the production native epoch.

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

## Draft public publisher authorization kernel v2

Version2-draft; NOT ACCEPTED, NOT IMPLEMENTED. Root assigned this concrete
parallel predecessor before full native/provider admission. It replaces neither
NativeProof nor actual issuer/store/SCM/W evidence. Earlier /1 Prepared/Commit
and /0.2 Hello formats are historical PROPOSALS, not fallback choices for v2.
This first exact record profile uses Windows binary SIDs. Linux/macOS actual
principal/provider encodings remain required OPEN backends, not emulated by
packing a UID or login identity into this Windows profile. Portable pure parsing
of these bytes does not qualify Windows or another OS admission.
Freeze this exact byte contract through independent native/runtime/CLI critique
and root acceptance BEFORE Rust codec/signature implementation.

### Scope, ownership and graph

Authority owns bounded PUBLIC record parsing, canonical encoding, strict signature
verification and data-binding comparisons. Native-security owns original protected
installer root, delegated publisher identity/store, retained actual S/O/P instances,
endpoint ACL, atomic claim custody and owned monitor/close evidence. Audit owns
independent W key/continuity; native/broker owns its actual instance/lifetime.
Runtime owns actual prepared resources/private Ready composition. No reverse edge,
DTO authority, public activate(bool), generic identity callback or production key
creation. Pure successful verification MUST NOT construct NativeProof, either
observer phase type, Ready, management authorization or native healthy capability.

Smallest Rust slice: startup::authorization::{codec, binding, verification} with
bounded byte readers and public-data structs. No issuer signing API or service
process. Proposed types: ParsedDelegation, ParsedPrepared, ParsedCommit and
SignatureCheckedAuthorization. Parsed records are untrusted data. The latter
means ONLY strict signatures plus explicit data-binding checks succeeded against
caller-supplied ExpectedAuthorizationData; that context is also NOT native proof.
No conversion/trait implementation into startup::observer proof-bearing types.
No signing seeds, secret callbacks, Debug/serde of native owners or public raw
native handle getters. Public-data field access is not an admission grant.

Proposed API, not yet accepted code:
- parse_bundle(delegation:&[u8], prepared:&[u8], commit:&[u8]) -> bounded parsed data;
- verify_bundle(parsed, expected:&ExpectedAuthorizationData, now_ms:u64)
  -> Result<SignatureCheckedAuthorization, AuthorizationError>;
- explicit expected data includes original installer verification key, exact
  selected delegation commitment, configured scope/install epoch/audiences,
  expected actual-instance projections, grant/credential epochs and policy limits.
  Neither a record public key nor this caller data can establish native trust.

Reuse existing workspace ed25519-dalek3.0.0 strict verify_strict, sha2 and existing
hex/base64 implementations; no new cryptographic primitive or external version.
Authority currently has sha2/hex but NO direct dalek/base64 edges. Root must approve
and integrate the existing workspace edges before implementation; do NOT repurpose
Audit checkpoint signatures as arbitrary grant verification. Smallest first kernel
may omit Hello text encoding until its existing-base64 edge is available. No
Cargo/lock/native-owner source edit is authorized by this draft.

### Common binary and signature rules

All integers unsigned big-endian; exact fixed byte widths; no varints, floats,
normalization, implicit default, trailing bytes or alternate field ordering.
Record artifact = exact magic8 + body_length:u16 + body + Ed25519 signature64.
Signature message = exact ASCII per-kind domain + ONE NUL + body_length:u32BE
+ exact body. Strict Ed25519 verification uses the expected original I key for
Delegation, and the exact I-authorized P key for Prepared/Commit. Reject weak or
noncanonical keys/signatures. No caller verifier callback or record-selected
algorithm. All unknown versions/purpose/flags/record kinds refuse before promotion.

Canonical names: u8 byte length16..63, EXACT native-security owned grammar:
ASCII humanymous-lab- followed by a nonempty tail of at most48 lowercase ASCII
letters, digits or hyphens. Reuse that accepted grammar, never general ASCII or
case folding; valid maximum-name vectors use63bytes, NOT64. P/S/O names differ.
Canonical binary Windows SID: u8 length8..68,
revision1, subauthority count0..15 and exact length8+4*count, exact bytes. Native
projection must match actual queried primary TokenUser, never configured account
or pipe owner alone. Reject PID0 and creationFILETIME0; equality against retained
instances remains mandatory outside this codec. Digests/keys32, epochs16, nonce/
grant/delegation identifiers32; fresh nonzero CSPRNG identifiers are an issuer
obligation, not entropy proof from observing bytes.
Pure parsing/binding MUST reject all-zero installation/P/S/O/credential epochs,
grant/delegation IDs and Hello nonce. Nonzero bytes still do NOT prove CSPRNG
freshness, uniqueness, native identity or actual one-claim consumption.
The mandatory v2 comparisons require pairwise DISTINCT P/S/O principal SIDs and
I SID != P SID, not merely different service names. ExpectedAuthorizationData
must contain the original protected installer's canonical SID as well as P/S/O
projections so pure binding checks can reject same-SID/different-name cases.
I's commitment must match that SID and installation epoch. Native admission must
independently enforce those actual-principal inequalities against the protected
original installer record and retained role tokens; expected caller data is not
proof. Additional I SID != S/O SID (all FOUR distinct) remains an explicit
PROPOSAL pending native installer/backend applicability and Root review; it is
NOT accepted production policy or an implicit extra codec admission condition.
No implementation may silently waive mandatory P/S/O or I/P separation, or
claim this data comparison proves issuer store/elevation authorization.
I and P Ed25519 verification keys MUST also differ byte-for-byte, with distinct
recomputed key fingerprints: reusing I's private key as P's key would let P sign
delegations despite the stated issuer separation. Pairwise distinct P/S/O live
PIDs are required even if caller-supplied creation times differ; their full actual
instance projections must match the retained native owners. I is the original
installer identity record, not a requirement that its old process remain alive.

### Closed expected-data schema and equality matrix

ExpectedAuthorizationData is a closed bounded Rust DATA struct, not a trust
constructor or a wire-selected policy. Proposed exact members:
- installer: public_key32, canonical_SID(8..68), installation_epoch16,
  selected_delegation_artifact_commitment32, delegation_id32,
  delegation_not_before_ms:u64, delegation_not_after_ms:u64;
- publisher: owned_name(16..63), canonical_SID, public_key32, artifact_digest32,
  actual_PID:u32, actual_creation_FILETIME:u64, publisher_epoch16;
- service and observer: fixed role1/2, owned_name, canonical_SID, actual_PID,
  actual_creation_FILETIME, role_epoch16, BASE_plan_digest32, artifact_digest32;
- scope: installation/tenant/origin/revision, each1..128 ASCII alphanumeric or
  ._:-, plus application_authority1..1024 printable ASCII excluding @/#? and
  backslash, EXACT existing Authority scope grammar, no case normalization;
- attempt: grant_id32, credential_epoch16, startup_cap_ms:u32,
  earliest_issued_ms:u64, latest_not_after_ms:u64.

Validate lengths, namespace/SID/profile inequalities, nonzero IDs/epochs and time
intervals BEFORE hashing/allocating variable data. No arbitrary extra fields,
unbounded expected-scope string or JSON-derived native context. Reference now_ms
is the separate explicit DATA argument, not an implicit trusted current clock.

| Signed field group | Required data comparison/check |
|---|---|
| version/magic/kind/domain/purpose/flags/mask | Exact protocol constants, including Commit mask3; mask remains an assertion |
| I key fingerprint/principal/install epoch | Recompute from expected I key/SID/epoch; exact equality and I/P separation |
| Delegation record selection/ID/window | Complete artifact hash equals selected expected commitment; exact ID/window |
| P key/name/SID/artifact | Exact expected publisher fields; verify P signatures only under that delegated key |
| scope/installation/pair policy commitments | Recompute all exact bounded expected fields, including both BASE plans/artifacts |
| delegation links in Prepared and Commit | Both equal the SAME complete verified Delegation artifact commitment |
| P instance commitment/PID/creation/epoch | Recompute commitment plus exact expected actual-instance projections |
| S/O names/SIDs/PIDs/creation/epochs/plans/artifacts | Exact expected role fields, pairwise names/SIDs/PIDs distinct |
| grant/credential epoch | Exact expected nonzero attempt data; never selected from incoming records |
| Prepared artifact link | Hash COMPLETE exact Prepared artifact including signature; exact Commit equality |
| delegation validity | Exact selected window, not_before < not_after; bounds every attempt time |
| Prepared/Commit times and cap | Signed ordering/reference checks below, earliest_issued <= issued and not_after <= latest_not_after; both caps equal exact positive expected cap |
| Hello repeated fields/pins | Match verified record data plus exact recomputed role-instance commitments; nonzero Hello nonce |

No signed security field is ignored. A mismatch returns static typed error, not
partial success with an omitted policy comparison. Golden mutation tests cover
each row independently; native projection checks remain separate actual evidence.

### Installer delegation record

Magic HMBGD002. Domain humanymous.observer-bootstrap.authorization/2/delegation.
Body EXACT order (maximum393bytes; artifact467bytes):
1. version:u16=2; installation_epoch16; bootstrap_scope_commitment32;
2. original_installer_principal_commitment32; installer_key_fingerprint32;
3. publisher_SID(length+bytes); publisher_service_name(length+bytes);
4. publisher_public_key32; publisher_installed_artifact_digest32;
5. service_observer_audience_policy_commitment32; delegation_id32;
6. not_before_ms:u64; not_after_ms:u64; purpose:u8=1(ObserverStatus); flags:u8=0.

I is the ORIGINAL installation/controller trust anchor; P is a distinct finite
service principal. Native-protected original root/SCM-owner/delegation record must
select the expected I key and EXACT current delegation commitment independently
of this packet. I authorizes P's exact service SID/name/key/artifact and exact S/O
audience policy; P cannot self-sign a replacement delegation, broaden audiences,
reset an existing key or learn its trust anchor from its own record. No automatic
key creation on missing/read failure; no interactive-current-user or machine-wide
secret fallback. Initial I key/publisher observation and unattended P store access
are STILL native provisioning/probe requirements, not implemented by certificates.

### Pair preparation record

Magic HMBGP002. Domain humanymous.observer-bootstrap.authorization/2/prepared.
Body EXACT order (maximum710bytes; artifact784bytes):
1. version:u16=2; delegation_artifact_commitment32;
2. bootstrap_scope_commitment32; bootstrap_installation_commitment32;
   publisher_key_fingerprint32; grant_id32; publisher_instance_commitment32;
3. publisher_PID:u32; publisher_creation_FILETIME:u64; publisher_epoch16;
4. service_epoch16; observer_epoch16; credential_epoch16;
5. service_name(length+bytes); observer_name(length+bytes);
6. service_SID(length+bytes); observer_SID(length+bytes);
7. service_PID:u32+creation_FILETIME:u64; observer_PID:u32+creation_FILETIME:u64;
8. service_BASE_plan_digest32; observer_BASE_plan_digest32;
   service_installed_artifact_digest32; observer_installed_artifact_digest32;
9. issued_ms:u64; not_after_ms:u64; startup_cap_ms:u32;
10. purpose:u8=1; flags:u8=0.

Names/SIDs/artifacts/base-plan digests MUST match the I-delegated pair commitment
and native/configured expected data. Instance fields must match actual retained
P/S/O projections; claims alone are not those projections. Installed artifact
binding is launch-file custody under the separately accepted trusted-OS/parent
premise, NOT loaded-memory attestation. BASE plan means EXACT installer-protected
base plan. LaunchTicket/cursor, combined plan digest and role_plan fallback are
NOT accepted or encoded by this kernel. Their absence cannot be patched with a
legacy digest or mutable unsigned launch metadata.

### Pair commit record

Magic HMBGC002. Domain humanymous.observer-bootstrap.authorization/2/commit.
Body EXACT order (201bytes; artifact275bytes):
1. version:u16=2; delegation_artifact_commitment32; grant_id32;
2. prepared_artifact_commitment32; publisher_key_fingerprint32;
3. service_epoch16; observer_epoch16; credential_epoch16;
4. commit_issued_ms:u64; not_after_ms:u64; startup_cap_ms:u32;
5. purpose:u8=1; flags:u8=0; native_acceptance_mask:u8=3.

Repeated identities/epochs/delegation/key/grant match exactly. Commit expiry/cap
must equal Prepared, never extend it; commit_issued is >= Prepared issued and
< expiry. The signed mask is a P ASSERTION, not proof that native S/O ACK actually
occurred. Native publisher must retain and authenticate both actual acceptance
owners before committing. Prepared alone, cross-grant/delegation Commit, changed
key/epochs, partial Commit or wrong record kind never permits secret release.

### Exact commitment encoders

Each hash is SHA256(exact ASCII domain + ONE NUL + exact bytes below):
- /2/key: raw Ed25519 public key32 (used for I and P fingerprints).
- /2/delegation-record and /2/prepared-record: COMPLETE corresponding artifact,
  including magic, length and signature, with exact parse-to-EOF.
- /2/installer-principal: canonical SID(length+bytes) then installation_epoch16.
- /2/scope: installation, tenant, origin, revision, application_authority, each
  u32BE UTF8 length plus exact already-validated bytes, no normalization.
- /2/installation: scope32 + I key fingerprint32 + installation_epoch16 + exact
  S name(length+bytes) + O name(length+bytes).
- /2/pair-policy: role1(S) + S name/SID(length+bytes) + S artifact32 + S BASE plan32,
  then role2(O) + corresponding O fields. No optional fields or reordered tuple.
- /2/publisher-instance: P SID(length+bytes) + PID:u32 + creation:u64 + P epoch16.
- /2/role-instance: role1 or2 + canonical SID + PID:u32 + creation:u64 + role epoch16
  + exact BASE plan32. Used by Hello sender/receiver pins, not native attestation.

Every /2/... above is prefixed in FULL by
humanymous.observer-bootstrap.authorization (for example
humanymous.observer-bootstrap.authorization/2/pair-policy). Scope/installation
commitments are BOOTSTRAP POLICY bindings and NEVER replace stable kill/control/
retained-intent installation/node identities or reset persistent controls.

### New explicit wire profile and limits

Propose protocol humanymous.observer-bootstrap/0.3, control HMBP0003, grant.format
humanymous.observer-bootstrap.authorization/2. No /0.2/HMBP0002, /1, mixed record,
hex-profile or prior shared-secret Hello fallback. This is an explicit new format
proposal; earlier names were not an implemented compatibility obligation.

Hello retains EXACT closed ordered fields: protocol, phase(hello),
purpose(observer-status), bootstrap_scope, grant_id, service_epoch, observer_epoch,
credential_epoch, sender_pin, receiver_pin, hello_nonce, grant. Existing four
commitment/ID strings are64lowerhex; three epochs/hello_nonce32lowerhex. Grant
has EXACT order format, delegation, prepared, commit. ALL THREE complete artifacts
are canonical UNPADDED BASE64URL (RFC4648 URL alphabet, no '=' or whitespace,
strict canonical re-encode equality including unused trailing bits). At maximum
sizes Delegation/Commit final symbols have two unused low bits; Prepared has four.
Mutating any unused bit MUST refuse even when a permissive decoder yields the
same bytes. Cover every length remainder, not only these maximum-size vectors.
This replaces
hex because the complete delegation MUST also fit inside Hello, never a side file
or unbounded attachment. Binary caps467/784/275 -> encoded caps623/1046/367.
Independent compact ASCII maximum =2036artifact chars +100grant syntax +632other
metadata =2768bytes. Enforce received metadata<=3072 BEFORE JSON parse/decode,
frame<=4096, aggregate startup<=32768. Strict unknown/duplicate/missing/type and
canonical key order/no whitespace/escapes. Final encoded size is checked, not
inferred from arithmetic. Offer ONLY has64lowerhex secret bytes; all other phases
zero. Binary record parsing also checks per-kind size BEFORE allocation and exact
EOF. No truncation or giant declared-size allocation.

### Lifetimes, one claim and cross-SID rights

Pure verification compares signed times/caps against explicit expected policy and
now_ms DATA; it does not certify that caller clock/native manager is trusted.
Require not_before <= issued <= commit_issued < not_after <= delegation expiry,
and commit_issued <= now_ms < not_after for the explicit reference-time check.
Require a positive cap within an explicitly configured protocol/policy bound, no default
lease.
All time arithmetic is checked: reject reversed intervals or overflow BEFORE
subtraction/conversion; require cap <= checked(not_after-issued) and exact
Prepared/Commit cap equality. Reference now==expiry refuses. No wrapping or
saturating arithmetic may turn invalid/expired data into accepted authorization.
The expected maximum cap must itself be explicit/positive, never a default lease.
Native owner separately conserves the minimum original local monotonic
startup deadline and signed wall remainder, detects rollback and never renews via
frame/retry/ACK. Key/delegation/pair/instance changes require fresh explicit native
transaction. No old record/cursor resurrection or private-key reset on restart.

One-claim reservation is NATIVE-owned atomic create-new+retained-lock per actual
role process instance, NOT per grant ID or new Rust context. Codec/signature success
has NO consumption power. Prepared/native pair verification -> reserve process
slots -> actual native P acceptance -> immutable signed Commit -> actual S/O
publication verification. Cancellation/error before Commit burns; after Commit
unknown publication stays ambiguous, no automatic reissue/retry. Old roles must
be stopped/observed before explicit fresh pair/grant, without erasure/counter reset.
A private finite model may test ordering, but cannot create native slot evidence.

Cross-SID minimum native queries: PROCESS_QUERY_LIMITED_INFORMATION|SYNCHRONIZE|
READ_CONTROL; primary TOKEN_QUERY|READ_CONTROL under actual P/S/O tokens, not
fixture controller I/T. No SeDebug/admin/current-user fallback, DUP/VM/terminate/
WRITE_DAC permission to peer. P native-manager start/stop/query is exact S/O only.
Both O-owned bootstrap and S-owned management use exact owner/client-specific pipe
rights excluding FILE_CREATE_PIPE_INSTANCE/GENERIC_WRITE, reject remote/default/
extra-instance access. Existing file bridge/cooperative Job proofs do not establish
these rights, actual primary TokenUser, original installation owner or W independence.

### Verification, critique and promotion gates

L1 first kernel: fixed golden bytes/domain/signatures/commitments for min/max names
and SIDs; independently derived lengths; strict malformed/weak/noncanonical
signature tests; every bound field mutation; wrong I/P key/delegation/pair/scope/
role/artifact/BASE plan/instance/nonce/epoch/expiry, Prepared-as-Commit, mixed old
formats and partial/extra bytes. Hello strict base64url/JSON/size/predecode tests;
expiry equality/rollback/reference policy; private one-claim model races and no
public native/Ready conversions. Seeds in deterministic fixtures ONLY, no issuer
key API. Locked scoped tests/check/Clippy/fmt require resource allocation.

L2 genuine hosted ephemeral P/S/O roles: original protected I delegation, actual
cross-SID token/process handles and rights negatives, exact launch-file custody,
provider/store/key absence and cold service/reboot persistence, authenticated
acceptance/Commit uncertainty, claim marker/concurrent constructor/restart/replay,
wrong principal/PID/creation and endpoint squatting/remote negatives, actual W
identity/lifetime, bounded monitor cancel/join+flush and retained cleanup facts.
No local accounts/SCM/elevation/provider read is authorized by this design.
L3 Runtime/CLI/.NET composition must consume those ACTUAL owners before unseal,
then receipt/Ready/failure ownership on SAME gate. L5 product Docker remains
separate. No pure codec/signature pass is full R1/native/Ready/hosted qualification.

Single-author panel: Blue separates data from native ownership; Red rejects
self-issued delegation/record substitution and per-grant claim bypass; SRE retains
absolute clocks/uncertain Commit/cleanup and original key; DPO treats public SID/
instance data as pseudonymous/private raw evidence, never anonymous public logs;
Evaluator keeps signatures, actual native proof and product gates separate.
Independent native/runtime/CLI adversarial critique and ROOT acceptance are still
REQUIRED. P0 exact native trust/delegation/provision/clock/rights/W/consumption and
Ready producer remain OPEN. P1 existing dalek/base64 graph edges, golden vectors
and exact namespace compatibility require review before implementation.
