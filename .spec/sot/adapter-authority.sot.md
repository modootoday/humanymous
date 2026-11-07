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
