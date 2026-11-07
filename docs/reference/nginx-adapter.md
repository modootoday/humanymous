---
layout: doc
title: Experimental native nginx adapter
description: Linux ingress, private provisioning and native lifecycle qualification limits.
permalink: /reference/nginx-adapter.html
reviewed: 2026-11-07
---
# Experimental native nginx adapter

**Diátaxis quadrant:** Reference. **Audience:** nginx and application integrators.

Review the [deployment boundaries](supported-topologies.md) before selecting
this experimental profile.

## Experimental native nginx component

A Rust nginx module now exists for an initial Linux HTTP/1.1 profile. It uses
an upstream nginx C engine through the pinned ngx-rust SDK; it is not a Rust
replacement for the engine. Default library unit tests do not qualify the native
ABI, nginx binary/build flags or a deployed configuration.

The `humanymous_guard` directive consumes supervisor-provisioned private
configuration. The authority must independently select the nginx producer
profile. The ingress projection binds an exact method/path inventory, Host,
complete body and scope, then reuses the shared authority client. A generic
parser accepting nginx envelopes is not proof of native module execution.

The declared profile bounds complete uploads to 1 MiB. Non-HTTP/1.1 requests,
file-backed/incomplete bodies, transfer-coded uploads, trailers, upgrades and
gRPC are outside its support. Internal/forwarding assertions are sanitized;
original TLS/H2 capture, HTML/CSP insertion and response-body filtering are not
provided. Operator configuration remains trusted: prevent post-authorization
rewrite and origin bypass, disable proxy retry/cache, and apply the declared
buffering and upstream-control-header restrictions before claiming deployment
isolation or unchanged handler-owned streaming.

DENY maps to 403. Source returns a no-store 409 JSON
`challenge_unavailable` response for CHALLENGE; that is not an accessible
recovery service. The older status-only 35-group result does not cover the later
JSON/timer changes. A separate hosted Linux Docker run at source `8132117`
passed 35 scenario groups, including the JSON response, repeated keepalive
challenges and an SSE cancellation case that observed upstream closure.
Those component observations do not qualify later source or dependency changes.
Historical positive ALLOW pilots used
explicit test-only synthetic evidence; the actual production application-HTTP
profile's 409/no-origin observation does not prove a successful human flow.
Independent witness, cross-user isolation, sustained backpressure and full
browser/human/assistive-technology qualification remain separate outstanding gates.

### Current pilot report boundary

The current component contract uses report schema `humanymous.nginx.pilot/0.2`.
Its 35 required IDs describe scenario groups in one Rust pilot process, not 35
Rust test-harness tests: the HEAD/204 group contains two HTTP behaviors, and the
concurrency group accounts for 32 requests. A changed contract is not covered
merely because an older report also counted 35 groups.

Consumers require the exact unique group set, zero failed/ignored/not-run groups,
successful process exit and source, module, engine, lock, package and immutable
image identities. They must reject duplicate JSON keys and unknown top-level
fields rather than rewrite producer results into passing cases. A failed process
may produce no success report; retain that failure and unknown or unreached
counts. Per-case private diagnostics must not be copied wholesale into public
summaries. The report remains a component result with
`full_product_qualified: false`; trusted synthetic ALLOW and process cleanup do
not establish a complete browser, independent custody or production deployment.

The producer's completed-group check validates its own exact group-name set;
it is not an independent validator of an incoming untrusted report. The recovered
`8132117` summary names source, module, engine, lock and image identities, but
does not export a package-manifest hash or full private case diagnostics.
Do not fill those evidence gaps with the package writer's declared checks.

### nginx provisioning and request projection

Private provisioning is bounded to 64 KiB. The Linux reader requires a canonical
absolute path, a private parent under the shared authority transport policy,
no-follow file opening, a regular file owned by the current effective UID and
exact `0600` permissions. The expected Unix authority UID must match that UID.
The supervisor still owns ancestor mounts and removal of obsolete secret files;
these checks do not establish protection from every hostile same-user process.
Do not place credentials in request headers, public logs, argv or environment.

The configuration declares scope, exact authority, a 1–30000 ms budget and at
most 1024 unique method/path routes. Routes exclude the reserved namespace,
CONNECT, dot segments and ambiguous query/fragment/percent/backslash forms.
Projection checks that the target pathname equals the current location path,
that exactly one Host matches the provisioned authority, and that complete body
bytes fit the 1 MiB bound. Shared nginx-profile parsing verifies the resulting
envelope; ordinary duplicate header order is distinct from forbidden duplicate
framing or Host fields. Original peer, connection, TLS/H2 and capture evidence
are not manufactured from this envelope.

The default library feature set does not load nginx. Native exported-module and
pilot features have different responsibilities; the pilot entrypoint refuses
non-Linux targets rather than reporting zero executed tests as success. Executed
native ABI, peer/ACL and request-lifecycle evidence remains separate from these
source-level configuration and projection checks.

### nginx dispatch and resource limits

The preaccess handler captures the sanitized request and completes its bounded
body before scheduling an authority task. Immediately before continuing nginx
processing it compares a new method/target/path/header/body snapshot with the
stored one and consumes the bound decision, including expiry checks. This is
not protection from a later trusted module or operator rewrite; the configured
post-authorization route/origin boundary still needs deployment evidence.

The source permits at most 16 concurrent authority tasks per worker process.
That slot is acquired after body collection; it is not a global limit of 16
buffered uploads or all active nginx connections. Configure and validate nginx
connection/body/resource limits as well. A single request budget covers body
reception and the decision phase, with wall-clock and elapsed-time checks.
Neither the budget nor a 10 ms polling interval is a human interaction deadline
or a guaranteed response-time bound under load.

Native snapshots reject file-backed bodies and bound the in-memory body chain.
Header names/values must be representable as UTF-8 and pass the shared profile;
preserving ordinary header order does not imply arbitrary raw header-byte
fidelity. Input/header/route failures may return 400, body snapshot failure 413,
and capacity/dependency/expiry failures 503. These are this native component's
paths, not the Node adapter's uniform input-error mapping or generic Draft v0
status rules. Pool cleanup aborts owned tasks and deletes owned timers in source;
actual cancellation, finalization, reload and ABI lifetime safety need native
execution and review against the exact engine/build flags.

## Historical njs mock fixture

The archived njs fixture is a different mock protocol and authority. Its root
worker, fixed fixture credential, five-header projection and 64 KiB body limit
are historical test conditions, not current native deployment instructions.
Archived commands and old Node-only schema assumptions do not describe the
current Rust nginx profile. Do not infer native peer authentication, audit,
collector recovery or production qualification from that fixture's old results.

The mock lowercases and sorts its five projected headers, rejects duplicate
request headers and content-encoded uploads, and uses a private pilot message
format. Those choices are not the current native nginx-raw profile, ordinary
duplicate-header preservation or binary/content-encoded body contract. Its
origin-call counter and early SSE chunk test exercise only that local fixture;
they do not establish native replay durability, independent audit custody,
long-stream backpressure or accessible human recovery.
