---
id: plan:root/20261119094520
title: FM11 residual 18 gate serving composition
status: active
created: 2026-11-19T09:45:20Z
updated: 2026-11-19T09:45:20Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/application/edge.rs, crates/sdk/humanymous-runtime/src/gateway/configuration/edge.rs, crates/sdk/humanymous-runtime/tests/fixtures/gateway/edge-gates-go-golden.json]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application]
---

# FM11 residual 18 gate serving composition

## Goal

Run the existing edge-gate kernels on the live gateway `Application`, so the
served path matches the Go oracle's measured behaviour for the FM11 ledger
rows `internal/gate/guard.go`, `forward_fidelity_test.go` and
`ban_integration_test.go` (now `ported_unit`), and narrow `gates.go`,
`wargame_escalation_r161_test.go` and `prodhardening_test.go` (still
`partial`), on branch `claude/fm11-residual-18`. Detection freeze: no score,
weight, threshold, hard-rule predicate, verdict band or signal ID changes; the
gates enforce existing kernel decisions and ban-store state only. Scoring
rows (settings overlay, classify/resolve, settings engine, Pass scoring,
admin settings overlays) are untouched.

## Steps

- [x] Capture `edge-gates-go-golden.json` from the frozen oracle 981d25c: the
  banned response and record sequence, the spoof deny per forged header, the
  probe bodies and the add-only security headers.
- [x] `gateway::application::edge`: ban gate then spoof-header gate for
  proxied requests after the reserved control routes, records appended before
  any effect, Go `http.Error` refusals (`http::write_text`/`text_frame`);
  `Application::install_bans` (needs the PROXY v2 client address) and
  `ReadySwitch`; `GET /_humanymous/healthz` and `/_humanymous/readyz`.
- [x] `configuration::Edge`: gate modes (default enforce), opt-in HSTS (TLS
  listener only) and the origin-cloak key (owner decision C10); readiness
  shows only whether cloaking is enabled.
- [x] Attach `X-Hmny-Origin-Auth` on every origin request of the plaintext
  and TLS paths; add-only security headers on complete proxied responses.
- [x] Served-path tests `src/gateway/tests/edge.rs` on real loopback
  listeners (counting origin, PROXY v2 edge, admin plane sharing the store,
  dual-control R162 commit on unix).
- [x] Update the ledger rows, `docs/reviews.json`, the runtime-application
  SoT status lines and the qualification contract.
- [ ] Provision a ban store from the service binary (needs a rate-limit
  configuration document) and share one monotonic epoch with the admin plane.
- [ ] Serve Prometheus metrics (`TestAdminMetricsExposition`).
- [ ] Owner review of the pull request, including the default-enforce
  wiring that owner decision 2026-11-16 left undecided.

## Record

Capture, 2026-11-19, in a cloud session container with its own Docker Engine
(rule 96 cloud container venue; not a GitHub-hosted runner):

- Image `mirror.gcr.io/library/golang:1.25.3@sha256:6d4e5e74f47db00f7f24da5f53c1b4198ae46862a47395e30477365458347bf2`,
  `--network none`, `GOTOOLCHAIN=local`, `GOPROXY=off`, `GOFLAGS=-mod=readonly`,
  read-only module cache.
- Source: `git archive 981d25c go.mod go.sum internal pkg` plus a scratch
  in-package test `internal/gate/zz_fm11s18_capture_test.go` (SHA-256
  `e47a1b4588011a68798e862236ae79e56a61e5c2b35c48533c19a5b463d54ba1`), run as
  `go test -count=1 -run ^TestFM11S18Capture$ ./internal/gate/`.
- The capture program is not committed, so the bytes are bound but not
  reproducible from the baseline tree alone (`go-recorded-differential`).
- Raw output `capture.json`
  `3333ddcb262b8c4eae3e37e133010fdef5c3355fcd9cb80ba46fa189f7f7e5a7`; a run on
  the host Go 1.25.3 toolchain was byte-identical. The committed fixture is the
  raw output with a `provenance` object prepended, written with `jq --indent 1`
  (SHA-256 `49dfe8b69b882fcd75c497e9aedde20e772e71d1b63970b6c02a8b769a1ce900`).
- The eight Go tests behind the rows (`TestAutoBanEndToEnd`,
  `TestFingerprintAutoBan`, `TestConsoleBanManagement`,
  `TestWargameR162_CommitThenEdgeDenies`,
  `TestProxyForwardsClientRequestFaithfully`,
  `TestProxyBlocksSpoofedTrustHeaders`, `TestSecurityHeadersAddOnly`,
  `TestReadinessProbe`) pass on the oracle.
- Never regenerate these fixtures to hide drift.
