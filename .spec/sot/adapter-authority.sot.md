---
id: sot:root/adapter-authority
title: Adapter authority
status: active
created: 2026-11-06T22:50:11Z
updated: 2026-11-06T23:07:47Z
verify: [manual]
rel:
  governs: [crates/authority/**]
  depends_on: [sot:root/adapter-service-contract]
---

# Adapter authority

## Rule

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
replay/restart/body/capture/route/deadline/admission/audit failure. MSRV 1.88 and
advisory scan cover new dependencies. Docker product lab and remote native tuples
remain distinct evidence. See implementation plan and crates/authority/INTEGRATION.md.
