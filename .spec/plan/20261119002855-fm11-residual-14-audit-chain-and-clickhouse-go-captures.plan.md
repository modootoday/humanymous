---
id: plan:root/20261119002855
title: FM11 residual 14 audit chain and ClickHouse Go captures
status: active
created: 2026-11-19T00:28:55Z
updated: 2026-11-19T00:29:16Z
rel:
  governs: [crates/sdk/humanymous-runtime/tests/fixtures/audit/chain-go-golden.json, crates/sdk/humanymous-runtime/tests/fixtures/audit/clickhouse-go-golden.json]
  depends_on: [sot:root/rust-migration]
---

# FM11 residual 14 audit chain and ClickHouse Go captures

## Goal

Record the Go oracle captures behind the FM11 ledger rows
`internal/audit/chain.go` and `internal/audit/sink_clickhouse.go` (pull request
#139, branch `claude/fm11-residual-14`), so `scripts/ci/parity-surfaces.json`
can bind both vectors to a named record with provenance
`go-recorded-differential`. Detection freeze: no scoring, weight, threshold or
signal ID is involved.

## Steps

- [x] Capture `chain-go-golden.json` and `clickhouse-go-golden.json` from the
  frozen oracle 981d25c.
- [x] Replay them in Rust: `tests/audit/chain_parity.rs`
  (`sealed_records_and_signed_heads_match_go`, `tamper_detection_matches_go`),
  `src/audit/log/stamp.rs` (`stamped_timestamp_and_event_id_prefix_match_go`)
  and `src/audit/projection/tests/parity.rs` (`insert_targets_match_go`,
  `batch_bodies_match_go_bytes`, `request_scenarios_match_go`).
- [x] Register both vectors and `chain_parity.rs` under the `audit-records`
  surface of `scripts/ci/parity-surfaces.json`. A Go package may be covered by
  only one surface, and `internal/audit` already is, so no separate surface is
  added. The ClickHouse replay is a library unit test, not a `*_parity.rs`
  file, so the manifest binds its vector digest and this record names its test.
- [ ] Owner review of pull request #139.

## Record

Capture, 2026-11-19, in a cloud session container with its own Docker Engine
29.9.2 (rule 96 cloud container venue; not a GitHub-hosted runner):

- Image `mirror.gcr.io/library/golang:1.26.3-alpine@sha256:91eda9776261207ea25fd06b5b7fed8d397dd2c0a283e77f2ab6e91bfa71079d`,
  `--network none`, `GOTOOLCHAIN=local`, `GOPROXY=off`, `GOFLAGS=-mod=readonly`,
  read-only module cache.
- Source: `git archive 981d25c go.mod go.sum internal/audit` plus a scratch
  in-package test `internal/audit/zz_fm14capture_test.go` (SHA-256
  `bde49464cde1ec3129fa53727024bd73f2c9fbf79c8e015b04dd55200a6b8104`), run as
  `go test -count=1 -run ^TestFM14Capture$ ./internal/audit/`.
- The capture program is not committed, so the bytes are bound but not
  reproducible from the baseline tree alone (`go-recorded-differential`).
- Raw outputs: `chain.json`
  `46fa90f33f26dd3a38eeb79c182e18579b2ba938cf15b1474c6ef9222990e2b4`,
  `clickhouse.json`
  `8634722f4f1366adbc60980703234d739879730bcd2a2dde4a7c321f12c5b0fa`. Each
  committed fixture is the raw output with a `provenance` object prepended,
  written with `jq --indent 1`; the manifest records the committed digests.
- Never regenerate these fixtures to hide drift.
