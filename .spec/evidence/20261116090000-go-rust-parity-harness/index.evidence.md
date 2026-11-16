---
id: evidence:root/20261116090000
title: Go↔Rust differential parity harness bound to the frozen Go baseline
status: active
created: 2026-11-16T09:00:00Z
updated: 2026-11-16T09:00:00Z
rel:
  depends_on: [plan:root/20261116060213, sot:root/milestones, sot:root/delivery]
---

# Go↔Rust differential parity harness bound to the frozen Go baseline

## What changed

`scripts/ci/parity-surfaces.json` lists every Go↔Rust differential surface
that exists in the Rust workspace. Each surface names:

- the FM11 ledger Go packages it compares;
- the Rust `*_parity.rs` tests and the `#[test]` functions that replay the
  vectors;
- each vector file with its SHA-256 and its provenance.

`delivery policy` now runs `parity::check`. It refuses the change when:

- the baseline is not `981d25cf71a5a54855d7e2b4359e832a9d5998e5`;
- a vector's bytes drift from the recorded digest;
- a named test function is missing or lacks `#[test]`;
- a tracked `*_parity.rs` file is not in the manifest;
- a Go package from `.agents/retirement/legacy-go-row-review.json` is neither a
  surface nor a named gap, or is both.

The CI workflow is unchanged: the policy job already runs `delivery policy`,
and the native job already runs the parity tests through `cargo test`.

## Coverage

10 surfaces cover 10 Go packages with 27 vector files: `internal/attestation`,
`internal/audit`, `internal/behavior`, `internal/integrity`,
`internal/mlserve`, `internal/network`, `internal/pow`, `internal/resource`,
`internal/scoring`, `internal/signals`. The other 25 ledger packages are
recorded as gaps with their FM11 row counts. They include `internal/gate`,
`cmd/server` and `internal/mlcorrect`.

## Provenance measured at the baseline

Measured in a Linux cloud container with Go 1.25.3 on an extract of the
baseline commit:

- `UPDATE_SCORING_GOLDEN=1 go test ./internal/scoring -run TestFreezeGoldenCorpus -count=1`
  passed. The regenerated `test/fixtures/scoring/*.json` files are
  byte-identical to the six Rust copies under
  `crates/sdk/humanymous-core/tests/scoring/fixtures/frozen/`. These vectors
  are marked `go-regenerated-at-baseline`.
- The other 21 vectors were recorded from the frozen Go oracle under
  `plan:root/20261106194012`. Their exporter was not retained, so the manifest
  binds their bytes but the baseline tree alone cannot regenerate them. They
  are marked `go-recorded-differential`.

## Limits

- The harness binds and replays recorded Go outputs. It does not run Go in CI.
- Gap packages have no differential vectors yet. Adding vectors for them needs
  a Go-side exporter at the baseline, which is foreign authored code. That
  needs an owner decision.
- An intended Go↔Rust difference still needs owner approval (unit 37, class R).
