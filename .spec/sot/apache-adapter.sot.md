---
id: sot:root/apache-adapter
title: Apache native adapter lifecycle and authority boundary
status: active
verify: manual
created: 2026-11-07T07:26:00Z
updated: 2026-11-07T07:26:00Z
rel:
  governs: [crates/apache-adapter/**, adapters/apache/**]
  depends_on: [sot:root/rust-migration, sot:root/adapter-service-contract]
---

# Apache native adapter

Version0.1 experimental target; no Apache tuple is currently qualified.
Authored implementation/tests are Rust1.99.1 edition2024. Native ABI unsafe is
not approved. Safe lifecycle implementation does not establish module support.

## Required lifetime contract

Child initialization occurs after fork. A child owner binds actual process ID,
generation and supported MPM. Parent-created runtimes, credentials and threads
must not be reused after fork. Prefork admits one request; worker/event admit
a configured bounded number. Admission is atomic and is retained through input,
authority await and origin handoff. Graceful child shutdown closes new admission
and invalidates outstanding work. A replacement child owns a new generation.

Each request owns bounded copied input, exact method/target/authority/header
observations and a single monotonic budget. No request_rec/APR pool pointer may
cross an await, thread or pool cleanup. Pool cleanup cancels the request, and
origin handoff consumes an authenticated authority response exactly once.
The safe request owner is !Send/!Sync, enforcing host-thread replay affinity.
A separate idempotent cancellation signal can be retained by pool cleanup while
authority work is pending. Cancellation retains admission until request drop;
it cannot revive work or mint an authorization. Native cleanup registration is
still unimplemented and must be measured separately from these pure contracts.
Subrequests and internal redirects do not inherit authorization; initial native
coverage rejects them until separately qualified. Cache quick handlers and
request rewrites must not bypass guard or change an authorized request binding.

Normalized Apache observations use provisioned Apache profile only:
humanymous.apache/0.1.0-experimental.1, apache-normalized, rawHeaderOrder=false,
peer/connectionId/capture/stream=null. Do not impersonate Node or claim original
TLS/H2/header order. Shared framework encoding and authenticated authority client
own validation and decisions. No scoring or transport/crypto clone.

## Adversarial requirements

P0: serving before security hooks, mutable target after decision and inherited
subrequest decisions are bypasses. Required: initial hook coverage plus final
binding check, no cache path, explicit subrequest rejection.
P0: forked runtime locks and retained APR pointers cause stale/process-unsafe
access. Required: child-only construction, owned copies and cleanup cancellation.
P1: truncated/overflow body must poison request, never authorize prefix bytes.
P1: request capacity must remain held until final effect, including cancellation.
P1: event MPM does not make blocking authority IPC asynchronous. Initial blocking
worker behavior must have explicit deadlines and measured capacity; no event-loop
scalability claim. Authenticated ALLOW alone grants no management/recovery access.

## Qualification

Pure units cover lifecycle races, process mismatch, capacity restoration,
body truncation/overflow and one-shot transitions. Cargo runs only in root slot.
Actual native ABI, startup/reload, each MPM, real binary body replay, concurrent
requests, authority outage/timeout/expiry, cancellation, spoofed headers and
origin witness denial require source-bound free hosted GitHub Docker execution.
Record exact httpd/APR/libc/architecture/module/toolchain/image identity and all
failed/unsupported/ignored/not_run cases. Compose belongs to product-lab owner.
Native ABI approval, workspace registration, shared Apache profile and hosted
runner integration are required dependencies, not implicit waivers.

References: https://httpd.apache.org/docs/2.4/developer/request.html and
https://httpd.apache.org/docs/2.4/developer/modguide.html.
