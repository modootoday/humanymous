---
id: plan:root/20261107055000
title: Browser client session and actual transport integration
status: active
created: 2026-11-07T05:50:00Z
updated: 2026-11-07T05:50:00Z
rel:
  depends_on: [sot:root/client-collector, sot:root/runtime-application]
---

# Client session and transport

- [x] Accept exact scope/bootstrap/one-flight/clock/expiry/invalidation contract.
- [x] Accept shared response ACK with runtime owner; bind actual status and consumed
  counter to pending request MAC. ACK is not ALLOW/body/collector attestation.
- [x] Implement strict bootstrap, shared request binding, zeroizing session/token
  buffers, one-flight reservation, authenticated completion and ambiguous invalidation.
- [x] Pin ACK SHA256/HMAC vector independently checked with .NET.
- [x] Native collector26/integrity20+legacyparity3 and actualDockerFirefox58 pass;
  host/WASM all-target Clippy pass after owning helper fixes its WASM boundary.
- [ ] Own browser Fetch Request/Headers/AbortController and a single whole-request
  deadline through complete buffered response; same-origin/omit/error/no retry.
- [ ] Verify real final Request representation and existing app Authorization;
  reject forbidden/reserved headers, cross-origin, streaming and ambiguous paths.
- [ ] Verify ACK only after complete bounded body, cancellation/drop cleanup and
  reentrancy/concurrency; invalidate on partial body/network/timeout/redirect.
- [ ] Join real runtime bootstrap/browser profile/ACK and actual owned Docker lab.
- [ ] Bind actual collector version/epoch/coverage to fresh server-issued context;
  keep required.collector422 until accepted provenance and actual browser pilot.
- [ ] Complete route-scoped cookie/identity profile, distribution/bootstrap and
  full product/human/keyboard/AT qualification; initial cookie-free is not final scope.

The initial runtime body limit64KiB is independent of the shared codec1MiB bound.
Bootstrap has exactly10 fields; no hidden budget negotiation. A pre-consumption
413 has no ACK and invalidates the client epoch. Server/client integration is not
proven by synthetic ACK unit tests or native/Browsers' pure lifecycle tests.
