---
id: plan:root/20261121201500
title: Lane H delegated gate decisions
status: active
created: 2026-11-21T20:15:00Z
updated: 2026-11-21T20:15:00Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/admin_plane.rs, crates/sdk/humanymous-runtime/src/gateway/application/gates.rs, crates/sdk/humanymous-runtime/src/gateway/application/edge.rs, crates/sdk/humanymous-runtime/src/gateway/service.rs, crates/sdk/humanymous-runtime/src/gateway/service/admin.rs, crates/sdk/humanymous-runtime/src/gateway/configuration.rs]
  depends_on: [sot:root/runtime-application, plan:root/20261119180000, plan:root/20261119160000]
---

# Lane H delegated gate decisions

## Goal

Port the decisions the owner delegated in issue #75 comment 7017558784
(2026-11-19) that lane J did not take: #2 (the rest of provisioning), #3
(gate default modes), #4 (verdict-token issuer and route presets), #5
(Prometheus metrics) and #7 (HTML injection). Lane J recorded #6 and #8 to #12;
`plan:root/20261119160000` named these "lane H" without a plan, and the SoT and
ledger kept calling them open. They are decided; this plan orders the ports.

Detection freeze: no score, weight, threshold, hard-rule predicate, verdict
band or signal ID changes. Modes, metrics, tokens and injection are not
scoring inputs. A11y gating is untouched.

## Steps (one pull request each, in this order)

- [x] Record the decisions in `sot:root/runtime-application` and correct the
  ledger gaps that called them open (2026-11-21).
- [ ] #5 metrics: `GET metrics` on the admin listener with Go's
  `hmn_gate_*` names, HELP/TYPE text, label sets and `prometheusLabel`
  escaping, pinned against a Go capture. Sources the service has: uptime,
  active bans, kill switch, effective monitor, settings overlay, pending
  age, apply and store-error counters, config version, operational log
  queue and sink counters. Audit-chain gauges need the chain attached to the
  admin listener (`AdminPlane::with_chain`), which is a step of its own; Go
  runtime gauges (goroutines, heap, sys) have no Rust counterpart and are a
  recorded deviation.
- [ ] #3 gate modes: Go's default module modes at 981d25c as a golden, the
  effective settings overlay consumed on the request path for `gate.ban`,
  `gate.verdict_token`, `gate.recon_sweep`, `gate.inject` and
  `gate.attest_floor`.
- [ ] #2 sweep gate: install the sweep detector from configuration in the
  mode #3 selects.
- [ ] #4 verdict tokens and presets: compose the `hmn_vt` issuer behind a
  configured key (none: upgrades stay denied, HR-26) and route presets with
  the attestation floor. The open #78 stack (B12 route presets) is the
  starting point and is rebased onto `codex/rust` here.
- [ ] #7 HTML injection: an opt-in, default-off configuration that composes
  `gateway::inject::Injector` for inject routes and serves its loader.

## Verification

Each step: scoped `cargo test --locked -p humanymous-runtime`, Clippy and fmt,
a Go golden where Go behaviour is ported, SoT and ledger updates in the same
pull request. Product Docker cases are separate gates.

## Not done here

Anything the delegation kept with the owner: ruleset bypass or admin merge,
releases, tags and secret values; execution grants; unfreezing scoring;
physical assistive-technology verification; paid services.
