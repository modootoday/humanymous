---
id: plan:root/20261119180000
title: FM11 lane J owner policy, ML activation and ledger
status: active
created: 2026-11-19T18:00:00Z
updated: 2026-11-19T18:00:00Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/admin_plane/models.rs, crates/sdk/humanymous-runtime/src/gateway/admin_plane/models/tests.rs, crates/sdk/humanymous-runtime/tests/fixtures/gateway/ml-bundle-go-golden.json, crates/sdk/humanymous-runtime/src/gateway/settings/configsign.rs, crates/sdk/humanymous-runtime/src/gateway/tests/signed_policy.rs]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application, sot:root/model-training-correction, sot:root/ip-reputation, sot:root/pass-protocol]
---

# FM11 lane J owner policy, ML activation and ledger

## Goal

Record and implement the owner decisions delegated in issue #75 comment
7017558784 (2026-11-19): #6, #8, #9, #10, #11 and #12, and the accepted
review deviations of PRs #150 and #151. Lane J of the Step 3 bulk delivery
mode of `plan:root/20261117142053`; base is `codex/rust` ba926ddc.

Detection freeze: no score, weight, threshold, hard-rule predicate, verdict
band or signal ID changes.

## Decisions

| # | Decision | Where |
| --- | --- | --- |
| 6 | Keep the Rust signed one-way node kill. Go's reversible global monitor mode is not ported (accepted deviation). | `sot:root/runtime-application` |
| 8 | Records bind the signed effective-policy version (Go `configVersion`) in place of `scope.revision`. | `sot:root/runtime-application` |
| 9 | `MotorEvidence::GoParity` is never served (accessibility hard rule); it stays `Off`. | `sot:root/pass-protocol` |
| 10 | The proxy-VPN and Tor exit feeds stay shadow-only. | `sot:root/ip-reputation` |
| 11 | A credential never stands in for request integrity. Credential gates run after the integrity checks (stricter than Go). | `sot:root/runtime-application` |
| 12 | Training never activates a model. Promote and rollback are explicit, audited operator actions on the admin plane. | `sot:root/model-training-correction`, `sot:root/runtime-application` |

Accepted deviations from PRs #150 and #151: `/static/` answers 404 for `\`,
`:`, NUL and device names; Pass answers 503 on a duplicate trace plus a
scoring failure; the settings audit record is written before the change; a
Web Bot Auth keyid is named only when the kernel reports it; credential gates
run after the integrity checks.

## Steps

- [x] #12: `gateway::admin_plane::models` adds `GET models`, `POST
  models/promote` (an `ml.model.promote` pending action that a distinct
  approver commits) and `POST models/rollback` to the admin route table.
  `Models` pairs the core `BundleManager` (verification unchanged) with the
  shared inference `Runtime`. Each promote and rollback writes
  `config.changed` before the change; a failed write answers 503 and changes
  nothing. The service and listener code is not touched (lane G composes the
  admin listener).
- [x] #12 parity: Go capture of `BundleManager` and `mlserve.Score`
  replayed through the admin routes; activation freeze test over the frozen
  engine.
- [x] #8: `settings::configsign::SignedPolicy` and
  `Application::install_signed_policy`; the `runtime.decision`,
  `runtime.collect` and `upstream.error` records carry the signed version
  once installed.
- [x] #6, #9, #10, #11 and the deviations: SoT and ledger records only (no
  code path changes; `pass_flow` doc comments name decision #9).
- [x] FM11 ledger rows, totals and `by_package` recounted; the gateway
  qualification contract lists the 11 new tests.

## Activation and scoring

The served model only feeds the existing weight-0 `l4.ml.behavioral`
annotation. The activation test scores three reports with the frozen engine
before and after a promote. Risk, verdict, hard rule, contributors and every
hard-rule match are equal. The explanation gains an L4 layer row of
probability 0, as Go's did ("a live model shifts only the audit trace, never
a verdict"). No threshold provider is installed, so the annotation fire
threshold stays 0.5. Go's `SetThresholdProvider(ctrl)` at startup is not
ported (accepted deviation under #12).

## Not done here

- Composing `Models` and `SignedPolicy` into the service binary (lane G owns
  `gateway::service` and the admin listener).
- The controller's feature and shadow observers, the shadow model and the
  canary auto-rollback onto `Models`.
- Prometheus metrics, including the config-version label (issue #75).

## Record

Go oracle capture, 2026-11-19, in a cloud session container with its own
Docker Engine (rule 96 cloud container venue):

- Image `mirror.gcr.io/library/golang@sha256:91eda9776261207ea25fd06b5b7fed8d397dd2c0a283e77f2ab6e91bfa71079d`
  (go1.26.3 linux/amd64), `--network none`, `GOTOOLCHAIN=local`,
  `GOPROXY=off`, `GOFLAGS=-mod=readonly`, read-only module cache.
- Source: `git archive 981d25c go.mod go.sum internal pkg`, plus one scratch
  in-package capture test `internal/mlcorrect/zz_lanej_capture_test.go`
  (SHA-256 `ec63026cfdaad3dd55e8d52a48b784b54a39dac322d343a778043f3d3931a42f`,
  not committed). It drives `NewBundleManager`, `Stage`, `Promote` and
  `Rollback` with a fixed-seed Ed25519 key and records `Staged`, `Active`,
  `mlserve.Default().BundleVersion()` and `mlserve.Score` after each step.
- `go test -count=1 -run '^TestLaneJCapture$' ./internal/mlcorrect/`: PASS;
  two runs produced byte-identical output (SHA-256
  `fe17362b4ec313a119f11749be6fa1763df810752d7364269898925f4e42e993`),
  stored with a provenance object as
  `tests/fixtures/gateway/ml-bundle-go-golden.json`.
- Rust: `gateway::admin_plane::models` 10 passed, and
  `gateway::tests::signed_policy` 1 passed. A deliberate mutation of one
  golden p_bot value made `golden_lifecycle` fail; the fixture was restored
  and its hash re-checked. These are unit and fixture controls only: no
  browser, human, assistive-technology or Docker product e2e ran.
