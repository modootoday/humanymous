---
id: plan:root/20261119112843
title: FM11 lane B gate composition
status: active
created: 2026-11-19T11:28:43Z
updated: 2026-11-19T11:28:43Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/application/maintenance.rs, crates/sdk/humanymous-runtime/src/gateway/tests/maintenance.rs, crates/sdk/humanymous-runtime/src/gateway/tests/integration.rs, crates/sdk/humanymous-runtime/src/gateway/logging/inherited.rs]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application]
---

# FM11 lane B gate composition

## Goal

Close the recorded gaps of the gate process composition rows of the FM11
ledger (`.agents/retirement/legacy-go-row-review.json`) where the Go oracle at
981d25c can be matched without a new owner decision, and record the rest
truthfully. Lane B of the Step 3 bulk delivery mode of
`plan:root/20261117142053`; base is pull request #149 (`claude/fm11-residual-18`
2d9892be). Detection freeze: no score, weight, threshold, hard-rule predicate,
verdict band or signal ID changes. Owner-reserved items (dual control,
permanent bans, erasure execution, kill switch commit semantics) are not
loosened.

## Steps

- [x] Composite sweep: `Application::gc` (legacy `Server.GC`) over every
  kernel the serving owner composes, run by both accept loops on the legacy
  one-minute maintenance tick. Row `internal/gate/gc_test.go` to
  `ported_unit`.
- [x] Integration flow: the Go `TestProxyEnforcementAndAudit` bot beacon,
  edge block without origin contact and sealed-chain verification through
  `Application::serve` (`gateway::tests::integration`). Row
  `internal/gate/integration_test.go` stays `partial`: HTML injection is not
  composed on the served path (owner/SoT decision).
- [x] R166 on a chain-backed admin plane: `GET integrity` answers 200 with the
  chain verifying after the meta-audited ban list. Row
  `internal/gate/wargame_escalation_r161_test.go` to `ported_unit`; R169 is the
  owner-accepted difference of decision 2 (issue #20), pinned by the HTTP
  parser test.
- [x] Upstream error: owner decision C10 (issue #75) already fixed 503 with a
  typed code as the documented difference from Go's empty 502, so row
  `internal/gate/upstream_error_test.go` moves to `ported_unit` with the
  event-id stamping test added.
- [x] Gate operational logging in the runtime binary: the five `-log-*` flags
  as the inherited configuration's `logging` object, opened before custody,
  with Go's configured and `runtime.fatal` records. Row `cmd/gate/logging.go`
  has no open gap left; `cmd/gate/main.go` narrows its flag gap.

## Open owner questions (not decided here)

- Public edge listener and separate admin listener (plan B01).
- Prometheus metrics, gate default modes, ban-store provisioning and a shared
  ban clock (issue #75).

## Record

Go oracle runs, 2026-11-19, in a cloud session container with its own Docker
Engine 29.9.2 (rule 96 cloud container venue; not a GitHub-hosted runner):

- Image `mirror.gcr.io/library/golang:1.26.3-alpine@sha256:91eda9776261207ea25fd06b5b7fed8d397dd2c0a283e77f2ab6e91bfa71079d`,
  `--network none`, `GOTOOLCHAIN=local`, `GOPROXY=off`, `GOFLAGS=-mod=readonly`,
  read-only module cache.
- Source: `git archive 981d25c go.mod go.sum internal cmd pkg`, unmodified; no
  capture program was added.
- `go test -count=1 -v -run '^(TestVerdictStoreGC|TestBanStoreGCSweepsStrikesAndExpired|TestSweepDetectorGC|TestServerGCRunsWithoutPanic|TestProxyEnforcementAndAudit|TestUpstreamErrorAudited|TestKillSwitchDualControlAndEffect|TestConfigVersionSignedAndStable|TestWargameR166_MetaAuditOnBanList|TestWargameR169_TEChunkedAloneOK)$' ./internal/gate/`:
  all ten PASS.
