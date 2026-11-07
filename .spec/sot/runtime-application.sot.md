---
id: sot:root/runtime-application
title: Runtime application and session owner
status: active
created: 2026-11-07T00:20:25Z
updated: 2026-11-07T00:20:25Z
verify: [cargo test -p humanymous-runtime --locked, cargo clippy -p humanymous-runtime --all-targets --locked -- -D warnings]
rel:
  governs: [crates/runtime/**]
  depends_on: [sot:root/rust-migration, sot:root/adapter-authority, sot:root/server-adapter-expansion]
---

# Runtime application and session owner

## Rule

Version 0.1 is an experimental native loopback HTTP/1.1 application owner, not
full Core/Gate parity. Configuration arrives over private inherited stdin;
readiness goes to private piped stdout. Bind only explicit loopback IP addresses
for listener and owned origin. There is no public administration endpoint.
Do not widen to public plaintext listeners before a separate TLS/capture contract.

One process owns installation/tenant/authority/revision, exact method/path route
inventory, bounded volatile sessions, shared frozen Engine and durable audit Log.
Restart invalidates every session. Issue random 256-bit session capability and
independent random integrity key at POST /_humanymous/session (empty body).
Do not log either. Admission is bounded before issue; sessions have a fixed TTL,
capacity and no live eviction. Clock rollback poisons the epoch.

POST /_humanymous/collect ingests a bounded shared ClientReport using the session
capability and a rotating token for its exact body. It is client-asserted evidence,
not an authenticated browser collector. Only its client fields are merged; the
client cannot supply server/network/scoring fields. Session ID must match the
capability. Raw report/body/headers are volatile and never included in audit.
Reject unknown/duplicate/non-L1-L4 signal IDs, server collection claims, invalid
verdict/confidence; rebuild layer/weight/score/expected metadata using the shared
registry constructor, never client-supplied numeric scoring fields.
There is no browser collector or Pass recovery shipped by this slice.

Every collect/protected request requires Authorization: Bearer <session>,
X-Humanymous-Counter, X-Humanymous-Bucket, X-Humanymous-Deadline and
X-Humanymous-Token. Counter starts at 1. Use existing integrity::request seed,
canonical, compute_token and verify without changing their frozen byte protocol.
The application-specific signed text is domain `humanymous.runtime/0.1\n`,
then length-prefixed UTF-8 installation, tenant, origin identity, revision, authority, method,
exact origin-form target, decimal deadline (each `byte_length:value\n`), then
every observed header name and value in wire order/casing using the same
length-prefixing, excluding only X-Humanymous-Token, then one empty line and
the existing canonical headers/session/body. Application cookies and all other
forwarded fields are therefore bound too. This is a new application binding,
not a relabelled Node adapter protocol. Key provisioning returns key and initial
counter0 only to the initiating loopback client. No bootstrap token grace.
Counter advances atomically after valid authentication, before collection,
scoring/audit/origin. Valid authenticated attempts remain consumed on later error.
No automatic retry, cache or failover after ambiguous origin effects.

HTTP framing is strict bounded HTTP/1.1 with one request/connection and close:
16KiB headers, 100 unique case-insensitive headers, 64KiB body, mandatory canonical
Content-Length (including zero), exact Host. Reject TE/upgrade/Expect/trailers,
obs-fold, control/non-ASCII header values, duplicate headers, absolute targets,
percent/backslash/dot-segment paths and reserved unknown routes. Query remains
part of the integrity binding. Only fixed GET/POST/PUT/PATCH/DELETE/HEAD methods.
No stream, multipart reconstruction, decompression or response transformation.

Observe native HTTP version/header order/casing from the actual socket. Header
values never manufacture client evidence. TLS/H2 are absent on this listener;
required raw TLS, original H2 or browser collector returns422. RIT-required routes
are supported by the session owner, without claiming resistance to a coherent
T4 browser/client. All protected routes require RIT in this first profile.
Route requirements and active policy revision are server configuration, never
client assertions. Use shared network::report and scoring::Engine exactly once.
Only ALLOW can connect to the configured owned loopback origin, after successful
durable audit and a repeated deadline check. CHALLENGE is a blocked decision,
not human success. DENY/errors/unsupported/expired/truncated requests have zero
origin calls. Audit failure poisons the epoch. Audit stores minimized scope,
opaque request ID, revision, verdict and commitment; never raw report or tokens.

Forward complete observed body and safe end-to-end headers, stripping runtime
credentials and hop-by-hop framing, rebuilding Host/Content-Length/Connection.
Origin response is a bounded opaque HTTP/1.1 frame returned once, preserving
duplicate Set-Cookie. Reject ambiguous response framing and unsupported upgrades
or transfer encoding; an origin error can already have an effect and is never
retried. Socket deadlines bound reads/writes; synchronous disk append may exceed
the budget, but deadline is checked before origin. Cancellation after origin
dispatch cannot undo an origin effect. Independent origin bypass denial and
TLS, process isolation, real-browser/keyboard/AT controls and Docker product
e2e are later mandatory qualification gates. No speed/motor requirement exists.

### Review reflected

Blue: reuse one engine/integrity/network/audit implementation. Red P0: method,
target, tenant, body, policy and deadline binding; no first-request grace; strict
framing and replay reservation. SRE P1: loopback-only plaintext scope, bounded
admission/storage/deadlines, audit poison and no retry. DPO P1: no raw audit;
session TTL and volatile storage. Evaluator P0: client assertion is not browser
collector; fixture ALLOW is not browser/human success. Residual T4 and independent
witness/antirollback custody are explicit. Review verdict: ready for bounded
implementation; blocked for full product qualification.

## Why

The existing local adapter authority has no application session owner. This
slice adds a real listener/session/integrity/collection/enforcement composition
without pretending to implement absent raw planes or introducing a detector.

## Verification

Unit and native TCP tests must prove scope/method/target/body/replay/deadline,
collection session isolation, absent-plane refusal, capacity/clock/audit poison,
audit-before-origin and truncated request no-origin using real origin counters.
Use explicit fixture control reports only for bounded ALLOW wiring. Prepare
ordinary-browser/keyboard/AT cases as not_run until Rust browser collection exists.
Docker product e2e is the completion authority; native tests are not that gate.
