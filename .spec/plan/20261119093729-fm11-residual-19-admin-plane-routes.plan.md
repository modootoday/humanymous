---
id: plan:root/20261119093729
title: FM11 residual 19 admin plane routes
status: active
created: 2026-11-19T09:37:29Z
updated: 2026-11-19T09:37:29Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/admin_plane/policy.rs, crates/sdk/humanymous-runtime/tests/fixtures/gateway/admin-policy-console-go-golden.json]
  depends_on: [sot:root/rust-migration]
---

# FM11 residual 19 admin plane routes

## Goal

Port the remaining admin-plane behaviour behind the FM11 ledger rows
`internal/gate/wargame_escalation_r81_test.go` (R113),
`wargame_escalation_r121_test.go` (R131, R141) and `incident.go` (now
`ported_unit`), and `admin_test.go` and `admin.go` (still `partial`), on
branch `claude/fm11-residual-19`. Detection freeze: the policy route only
reads the boot policy, the kill-switch flag and the active overlay; no score,
weight, threshold, hard-rule predicate, verdict band, preset or signal ID
changes. Settings, metrics and virtual-USB routes, the kill-switch commit and
the erasure commit without a ledger-issued permit stay out of scope.

## Steps

- [x] Capture `admin-policy-console-go-golden.json` from the frozen oracle
  981d25c.
- [x] Serve the read-only `GET policy` route and the console page in
  `gateway::admin_plane::policy`, add `X-Frame-Options: DENY` to every admin
  response, and replay the capture in `src/gateway/admin_plane/policy/tests.rs`.
- [x] Port R113 and R131 against the existing chain-backed keys and integrity
  routes (`src/gateway/admin_plane/chain/tests.rs`), R141 and the extended
  R146 over the served listener (`src/gateway/admin_plane/tests/wargame.rs`)
  and TestAdminConsoleAndFeatures (`src/gateway/admin_plane/tests.rs`).
- [x] Pin each deliberate difference in a test: the console is an
  authenticated `GET` read and no bearer token is ever injected (Go served it
  to any method without authentication and could inject a dev token); policy
  routes follow prefix byte order (Go map order varied); the DPO erasure
  commit answers 409 without a staged ledger-issued erasure permit
  (owner-reserved dual control, unchanged).
- [ ] Wire the plane, its policy inputs and the console front end into the
  serving path (Runtime route owner).
- [ ] Owner review of the pull request.

## Record

Capture, 2026-11-19, in a cloud session container (not Docker; not a
GitHub-hosted runner):

- Toolchain `golang.org/toolchain@v0.0.1-go1.25.3.linux-amd64` from a
  read-only local module cache, `GOTOOLCHAIN=local`, `GOPROXY=off`,
  `GOFLAGS=-mod=readonly`; no network was used by the test.
- Source: `git archive 981d25c go.mod go.sum internal pkg` plus a scratch
  in-package test `internal/gate/zz_fm11r19_capture_test.go` (SHA-256
  `3cda0d5c2468c74018aee7f69b3d39c35cad24041184553d3e70b06b9edc2321`), run as
  `go test -count=1 -run ^TestFM11R19Capture$ ./internal/gate/`.
- The capture program is not committed, so the bytes are bound but not
  reproducible from the baseline tree alone (`go-recorded-differential`).
- Go lists policy routes in map iteration order, which differed between runs;
  the capture re-joins the rows with `(default)` first and the rest sorted by
  prefix, and masks the random STH public key.
- Raw output `capture.json`
  `ffdbe1e418f4f0c8365b65427e069d2e7142b58615a34de73c4011f1908067e3`; three
  consecutive runs were byte-identical. The committed fixture is the raw output
  with a `provenance` object prepended, written with `jq --indent 1`
  (SHA-256 `3453ff25f782d42c1258f113464eb04b8c3476f27caffdaa8bc0fde09dc7e2f6`).
- Never regenerate these fixtures to hide drift.
