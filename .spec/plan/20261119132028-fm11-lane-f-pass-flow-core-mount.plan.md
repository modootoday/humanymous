---
id: plan:root/20261119132028
title: FM11 lane F Pass flow Core mount
status: active
created: 2026-11-19T13:20:28Z
updated: 2026-11-19T13:20:47Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/core_server/pass.rs]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application, plan:root/20261119120953, plan:root/20261119114223]
---

# FM11 lane F Pass flow Core mount

## Goal

Mount the lane C Pass flow (`gateway::pass_flow`) into the lane A Core server
(`gateway::core_server`) through the `PassRoutes` mount point, so `/pass`,
`/api/pass/new`, `pow`, `attest`, `solve` and `kpi` are served as Go
`cmd/server` `routes` served them at oracle 981d25c instead of answering 404.
Branch `claude/fm11-lane-f`. This closes the open follow-up recorded in the
lane A and lane C plans and in the `cmd/server/pass_handler.go`,
`handlers.go` and `server.go` ledger rows.

Bounds:

- Detection freeze: no score, weight, threshold, hard-rule predicate, verdict
  band or signal ID changes.
- Accessibility hard rule: only `MotorEvidence::Off` is served;
  `PassMount::new` refuses `GoParity`.
- The Pass golden `pass-flow-go-golden.json` is replayed, never regenerated.
- The public Core listener and binary role (B01) stay with lane A.

## Steps

- [x] `gateway::core_server::pass::PassMount`: the `PassRoutes` adapter over
  one `PassFlow` behind its own lock. The Core hands it:
  - the session resolved by Go `cookieValue`;
  - the legacy `passFP` (stable JA4 prefix of the connection's own
    ClientHello and the trust-gated client subnet);
  - the request time read under the Core state lock;
  - a `StoreBackend` over the Core collector store, the frozen engine and the
    live feed.
- [x] `CoreServer::with_pass_flow(step_up)` mounts the flow with the Core
  master key and web root and the optional shared step-up key.
- [x] Fail closed: a limiter, store or scoring failure, an unavailable Core
  history and a poisoned Pass lock answer 503. A panicking Pass handler is
  caught before the Core state guard drops, so it cannot poison the Core.
- [x] Go JSON bytes: the flow writes its bodies with `go_json` (an integral
  float is `0`), and the `/api/pass/new` challenge and PoW objects keep Go
  struct field order.
- [x] Replay every recorded Go Pass request as a wire request through
  `read_request`, `CoreServer::handle` and `encode`, plus the page, the
  accessible default, the fail-closed paths and a loopback socket.
- [x] Ledger rows, the `core_server` module doc and the lane A and lane C
  plans updated; qualification contract regenerated.
- [ ] Owner review.
- [ ] Owner decision on `MotorEvidence::GoParity` (lane C accessibility
  conflict); until then the served Pass never gates on motor evidence.
- [ ] Product e2e under Docker (`red-blue-validate`) once the Core binary
  role exists.

## Record

- Byte parity: outside the motor-evidence steps, the status, content type,
  Set-Cookie presence, security headers and exact body bytes equal Go for
  every recorded request. The final stored signals and frozen-engine scores
  equal Go outside the motor sessions.
- Motor-evidence steps: seven steps in `duplicate_trace_and_real_event` and
  `behavior_tells_fused_soft` got Go answers from `realEventOK` or the
  behavior tells. Each is pinned to the accessible answer, and so are the
  five affected sessions' stored signals, which carry no `l4.*` tell.
- `feedPassOutcome` has no consumer on the Core plane: the self-correcting
  control plane is the admin plane's (plan B09).
- A scoring failure cannot be forced through the Core's default engine. The
  503 on scoring failure is pinned at the flow level by the lane C tests, and
  the adapter passes the flow's answer through unchanged.
