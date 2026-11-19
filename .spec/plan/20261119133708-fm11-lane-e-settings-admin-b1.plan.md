---
id: plan:root/20261119133708
title: FM11 lane E settings admin plane B1 exact-parity port
status: active
created: 2026-11-19T13:37:08Z
updated: 2026-11-19T13:37:08Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/admin_plane/settings.rs, crates/sdk/humanymous-runtime/src/gateway/admin_plane/settings/tests.rs, crates/sdk/humanymous-runtime/src/gateway/admin_plane.rs, crates/sdk/humanymous-runtime/src/gateway/settings/overlay.rs, crates/sdk/humanymous-runtime/src/gateway/settings/overlay/wire.rs, crates/sdk/humanymous-runtime/src/gateway/settings/overlay/timestamp.rs, crates/sdk/humanymous-runtime/tests/fixtures/gateway/settings-admin-go-golden.json]
  depends_on: [sot:root/rust-migration]
---

# FM11 lane E settings admin plane B1 exact-parity port

## Goal

Port the legacy ledger rows `internal/gate/admin_settings.go`,
`settings_runtime.go`, `admin_settings_test.go` and `settings_runtime_test.go`
(baseline 981d25c) under owner decision B1 (issue #75 comment 6979694428):
the SoT-39 settings write plane on the admin listener. It covers propose
with a server-computed mutation class (A/B/C/D) and lifetime, a pending
approval for B, C and D, the class C confirmation phrase, the overlay list,
dry-run with low-N usability, rollback and clear-overlay CAS, approval-backed
commit, overlay expiry, scoring/route/net-policy overlay content and runtime
validation of the effective overlay.

The frozen `gateway::settings::overlay` port validates, classifies, resolves
and versions the overlays, so no default, bound, weight, threshold,
hard-rule predicate, verdict band or signal ID is added or changed.
Dual-control stays where Go requires it, and every refusal fails closed.

Branch `claude/fm11-lane-e`.

## Steps

- [x] Capture `settings-admin-go-golden.json` offline from the oracle through
  its admin handler.
- [x] Slice 1: `gateway::admin_plane::settings` (`AdminPlane::with_settings`,
  the six routes in Go table order, the `settings.overlay` and
  `settings.overlay.rollback` approval commits, expiry, the runtime check
  and the hot reconfiguration of the shared ban store's limiter, counters and
  pending age). It comes with the small overlay-library helpers it needs:
  the propose and rollback body decoders, compact overlay JSON, and
  whole-second UTC timestamps. Replayed request by request against the
  golden, plus unit tests for each fail-closed difference. Ledger rows
  moved.
- [ ] Owner review.
- [ ] Compose the admin plane, including `with_settings`, into
  `gateway::service`, and consume the effective overlay on the request path.
  This is open, and owner-gated with B01 and with issue #75 for the
  settings-driven module modes.
- [ ] Prometheus exposition of the settings counters, the gauges and the
  configuration-version label. Owner pending in issue #75.

## Record

Capture, 2026-11-19, in a cloud session container with its own Docker Engine
(rule 96 cloud container venue; not a GitHub-hosted runner):

- Image `mirror.gcr.io/library/golang@sha256:91eda9776261207ea25fd06b5b7fed8d397dd2c0a283e77f2ab6e91bfa71079d`
  (the 1.26.3-alpine digest), `--network none`, `GOTOOLCHAIN=local`,
  `GOPROXY=off`, `GOFLAGS=-mod=readonly`, read-only module cache.
- Source: `git archive 981d25c go.mod go.sum cmd internal pkg`, plus a scratch
  in-package test `internal/gate/zz_lanee_capture_test.go` (SHA-256
  `3c319ae3b98c9aa36f671b190e5f9b860eb6273d823045fced01ca8e77c6488a`). It
  was run as `go test -count=1 -run ^TestLaneECapture$ ./internal/gate/` and
  drives `srv.AdminHandler()` with bearer tokens.
- The capture program is not committed (`go-recorded-differential`). Raw
  output `capture.json` has SHA-256
  `e0a91b487d590def3237dcb9d503709c77e2670fcd167ed5ec5165812aae0add`. The
  committed fixture is that output with a `provenance` object prepended.
- Some Go values are nondeterministic, and the replay handles each one:
  - Overlay ids are injected.
  - Approval ids are mapped.
  - Wall-clock `createdAt` and `expiresAt` are masked. Lifetimes are pinned
    by the captured `expiryForClass` table and a unit test.
  - The schema hard-rule order and the approvals list order (a Go map and
    random ids) are compared without regard to order.
- `internal/gate` has no parity surface in `scripts/ci/parity-surfaces.json`,
  so the golden is not registered there.
- Never regenerate this fixture to hide drift.

## Remaining gaps (ledger)

- `settings_runtime.go`: Prometheus exposition. The values are kept by
  `Settings::stats` and `AdminPlane::settings_pending_age`.
- `settings_runtime_test.go`: neither of these is mapped:
  - the six per-gate monitor-mode tests, because the hosted gateway does not
    consume the effective overlay;
  - `TestSettingsMetricsReflectApply`, because its exposition is owner
    pending.
