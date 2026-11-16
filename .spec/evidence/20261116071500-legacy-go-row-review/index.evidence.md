---
id: evidence:root/20261116071500
title: Legacy Go responsibility row review (FM11)
status: active
created: 2026-11-16T06:40:00Z
updated: 2026-11-16T06:40:00Z
rel:
  depends_on: [plan:root/20261116060213, sot:root/milestones]
---

# Legacy Go responsibility row review (FM11)

## What was measured

Each of the 314 rows in `.agents/retirement/legacy-go-owner-routing.json`
(one Go file per row) was reviewed against the Rust tree at source commit
`6a578ee`. The result is `.agents/retirement/legacy-go-row-review.json`. The
routing file stays unchanged as the baseline.

Method:

- Four read-only review passes, one per batch: `cmd/*`, `internal/gate`,
  core (audit, network, scoring, signals, detect, collector), and the rest.
- A mechanical verifier checked every row. It confirmed row order and coverage,
  the disposition vocabulary, that each cited Rust path exists, that each cited
  test exists as `fn <name>` in its file, and that each `ported_unit` row cites
  at least one test. It found 0 problems.
- The 259 unique cited tests were matched by function name against a full
  `cargo test --workspace --locked` run. 256 passed (counting the TLS drain
  test, which was fixed in this session and then passed 15/15 reruns). 3 did
  not run: two are `#[ignore]` (native publication, hosted transport), and one
  is a `wasm_bindgen_test`. Each row records this per test in `test_verification`.

| Disposition | Rows |
| --- | ---: |
| `ported_unit` | 104 |
| `partial` | 90 |
| `missing` | 118 |
| `not_applicable` | 2 |
| `retire_candidate` | 0 |

`internal/detect/bridge.go` was downgraded from `ported_unit` to `partial`,
because its only cited test is the wasm one.

The largest `missing` groups:

- `cmd/server`: 22 missing, 18 partial.
- `internal/gate`: 45 missing, 39 partial.
- `internal/mlcorrect`: 16 missing.
- `internal/logger`: 6 missing.
- `internal/watermark`: 4 missing.
- `internal/resource`: 4 missing.
- `cmd/gate`: 5 missing.

## How to read it

- `ported_unit` means the responsibility and its unit tests exist in Rust. It
  is not runtime or product proof. FM11 still needs actual evidence per row, or
  a reviewed retirement.
- There are no `retire_candidate` rows. `plan:root/20261107205817` states that
  no retirement is proposed, and a retirement requires a quoted SoT basis. The
  118 `missing` and 90 `partial` rows need either a Rust port or an
  owner-reviewed retirement decision, and that decision is Root's.
- Reviewer notes to recheck:
  - `engine.go` (`ported_unit`) and `hardrules.go` (`partial`) share one
    untested net-policy path.
  - `httpx/json.go` (`not_applicable`) and `codec_lsb.go` (`missing`, despite
    a stub) are uncertain.
  - Wargame round `r161` conflicts with the Rust Transfer-Encoding refusal.
- Nothing was deleted. Deleting Go stays owner-gated.
