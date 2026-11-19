---
id: plan:root/20261119230000
title: Owner gate provisioning and modes
status: active
created: 2026-11-19T23:00:00Z
updated: 2026-11-19T23:00:00Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/application/modes.rs, crates/sdk/humanymous-runtime/src/gateway/tests/modes.rs, crates/sdk/humanymous-runtime/tests/fixtures/gateway/gate-modes-go-golden.json]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application, plan:root/20261119160000]
---

# Owner gate provisioning and modes

## Goal

Carry out owner decisions #3, #2, #4 and #7 of issue #75 comment
7017558784 (lane H), one commit each in that order, on the B01 #1 base
`claude/owner-b01-edge` e732faa0 (PR #156). Go oracle 981d25c.

- #3: per-gate default modes (Go goldens) and the settings overlay consumed
  on the request path, as Go `settings_runtime.go` and `gateModuleMode`.
- #2: config-driven provisioning of the ban store, Web Bot Auth key
  directory, PAT issuers, WebAuthn credentials and the sweep detector.
  Unconfigured installs nothing; configured but unreadable or invalid fails
  startup closed.
- #4: the key-gated `hmn_vt` verdict-token issuer and the route presets that
  carry the attestation floor, so the verdict-token gate and the floor audit
  run on the served path. No key: upgrades stay denied (HR-26).
- #7: HTML loader injection, opt-in and off by default, with Go's CSP and
  length handling.

Detection freeze: no score, weight, threshold, hard-rule predicate, verdict
band or signal ID changes; modes are not weights or thresholds. A11y gating
is untouched. Human-gated paths are not edited.

## Steps

- [x] #3 modes: `application::modes` reads the admin settings plane's active
  overlay slot per request; ban, spoof-header and sweep gates consume it;
  Go capture `gate-modes-go-golden.json`; service composition test.
- [ ] #2 provisioning.
- [ ] #4 verdict-token issuer and route presets with the attestation floor.
- [ ] #7 HTML injection.

## Record

Go oracle capture, 2026-11-19, cloud session container with its own Docker
Engine (rule 96 cloud container venue; engine id checked before use):

- Image `mirror.gcr.io/library/golang@sha256:91eda9776261207ea25fd06b5b7fed8d397dd2c0a283e77f2ab6e91bfa71079d`
  (go1.26.3 linux/amd64), `--network none`, `GOTOOLCHAIN=local`,
  `GOPROXY=off`, `GOFLAGS=-mod=readonly`, read-only module cache.
- Source: `git archive 981d25c go.mod go.sum internal pkg`, plus one scratch
  in-package capture test `internal/gate/zz_fm11lh_capture_test.go`
  (SHA-256 `d96cc579f635afdfea9417482e9cb3db64864e7ff7dd438dab11d7e968286688`,
  not committed).
- `go test -count=1 -run '^TestFM11LHCapture$' ./internal/gate/`: PASS twice;
  the outputs were identical once the wall-clock `Date` response header was
  removed (SHA-256 of the masked output
  `b71c8803bc13726c33119b26ae94bcf94262e367b592588ac869f70ca4187b02`),
  stored with a provenance object as the fixture.
- #3 Rust: `gateway::tests::modes` 5 passed;
  `gateway::tests::service::exposure::unix_edge_gate_modes_follow_the_admin_settings_overlay`
  passed over real loopback sockets and real custody. Fixture controls, no
  browser, human or Docker product e2e.
