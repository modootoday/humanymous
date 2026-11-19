---
id: plan:root/20261119025817
title: FM11 residual 17 attest floor and admin chain Go captures
status: active
created: 2026-11-19T02:58:17Z
updated: 2026-11-19T02:59:12Z
rel:
  governs: [crates/sdk/humanymous-runtime/tests/fixtures/gateway/attest-floor-and-admin-chain-go-golden.json]
  depends_on: [sot:root/rust-migration]
---

# FM11 residual 17 attest floor and admin chain Go captures

## Goal

Record the Go oracle capture behind the FM11 ledger rows
`internal/gate/attestfloor_test.go`, `admin_integrity_empty_test.go`,
`admin_offline_verify_test.go` and `ws4_test.go` (now `ported_unit`), and
`admin_test.go` and `killswitch_test.go` (still `partial`), on branch
`claude/fm11-residual-17`. Detection freeze: no scoring, weight, threshold,
verdict band or signal ID is involved. `settings_runtime.go`, its test and
`admin_settings.go` implement the settings overlay and its effective-rate
validation, so they are left unchanged.

## Steps

- [x] Capture `attest-floor-and-admin-chain-go-golden.json` from the frozen
  oracle 981d25c.
- [x] Replay it in Rust as library unit tests:
  `src/gateway/attest_floor/tests/parity.rs` (floor HTTP cases, the step-up
  exchange, receipt reasons, the WebAuthn fan-out cap and the attested
  catch-all refusal) and `src/gateway/admin_plane/chain/tests.rs` (empty-chain
  integrity, keys and offline verification, proof and checkpoint refusals, the
  audit stream, incident resolution and the enumeration cap). Neither is a
  `*_parity.rs` file, so `scripts/ci/parity-surfaces.json` is unchanged.
- [x] Pin each deliberate difference in a test: a proof Rust cannot encode
  answers 500 without a cookie; an unadmitted fan-out observation withholds the
  upgrade on every route; an unadmitted incident lookup counts as over the cap;
  signed-head tree sizes follow the existing audit::log witness cadence (1, 5,
  9 instead of Go's 4, 8).
- [ ] Wire the exchange, fan-out cap and chain routes into the serving path
  (Runtime route owner).
- [ ] Owner review of the pull request.

## Record

Capture, 2026-11-19, in a cloud session container with its own Docker Engine
29.9.2 (rule 96 cloud container venue; not a GitHub-hosted runner):

- Image `mirror.gcr.io/library/golang:1.25.3@sha256:6d4e5e74f47db00f7f24da5f53c1b4198ae46862a47395e30477365458347bf2`,
  `--network none`, `GOTOOLCHAIN=local`, `GOPROXY=off`, `GOFLAGS=-mod=readonly`,
  read-only module cache.
- Source: `git archive 981d25c go.mod go.sum internal pkg` plus a scratch
  in-package test `internal/gate/zz_fm11r17_capture_test.go` (SHA-256
  `7510b7d33d556958b2bb6a9f008cc7a15f69b978cb075a3fd394f6c428b7c34f`), run as
  `go test -count=1 -run ^TestFM11R17Capture$ ./internal/gate/`.
- The capture program is not committed, so the bytes are bound but not
  reproducible from the baseline tree alone (`go-recorded-differential`).
- Raw output `capture.json`
  `cc4f1e81d77aee31aa5d391d6ca87f3fa4cdf344d80c27c6fcedea7824fd4371`; two
  consecutive runs were byte-identical. The committed fixture is the raw output
  with a `provenance` object prepended, written with `jq --indent 1`
  (SHA-256 `8d06ecb0b05178915c7865e9aba079916dd181176742b7890b7cdbdddeac2c77`).
- Never regenerate these fixtures to hide drift.
