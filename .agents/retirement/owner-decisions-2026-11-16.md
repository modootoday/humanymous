# Owner decisions on Go/Rust parity, 2026-11-16

The owner decided these items for roadmap units 24 and 37
(`plan:root/20261116060213`). The Go baseline is commit 981d25c. None of them
changes a signal ID, weight, threshold, hard-rule predicate or verdict band:
the detection freeze still holds.

## 1. Unit 37: packages without Go vectors are accepted gaps

The 25 Go packages listed under `gaps` in `scripts/ci/parity-surfaces.json`
have no Go-recorded golden vector. The owner accepted them as gaps: no new Go
exporter is written. Each package's parity evidence is the Rust unit tests that
the FM11 ledger (`.agents/retirement/legacy-go-row-review.json`) maps to its
rows. Rows still `missing` or `partial` stay open in that ledger. This decision
does not mark them ported.

`delivery policy` now refuses a gap that has no `acceptance.decision` or whose
`acceptance.record` does not name this file.

## 2. Intended Go/Rust differences: the Rust behavior is accepted

For each item below, the owner accepted the current Rust behavior over Go
parity. Every item fails closed and is pinned by an existing Rust test. Each
affected ledger row's `notes` now ends with a reference to this file.

| Item | Go at 981d25c | Rust (accepted) |
|------|---------------|-----------------|
| Ten DIFF rows of the FM11 class A sweep (`fm11-class-a-sweep.md`) | `enforce.go` `Verdict.action` passes an unknown verdict on a public safe-method route; address-keyed registry (SoT-31); in-memory audit ring and `Recent(n)`; no-action `Emit`; bounded async log queue | Unknown fails closed; no address-keyed registry; no memory ring; `emit_and_act` only; synchronous log emission |
| Lone `Transfer-Encoding: chunked` (issue #20) | `smuggleScan` accepts it (R169) | `400 http_framing`; Content-Length-only request bodies |
| Ban store (issue #23, rows 1–6) | 4th strike and `Add(...,0)` make permanent bans; `cidr:` checked only at HTTP; `Lift` removes anything; raw keys; unbounded | No automatic permanent ban; permanent and `cidr:` bans need dual control in the store; keys canonicalized; bounded store that refuses when full |
| Permanent-ban lift (issue #23, decision 2) | `Lift` removes it | No lift action. A permanent ban is removed only through erasure. Adding a control `Action` remains a separate owner decision |
| Shadow observers, trust guard, credential verifiers, logging, Pass store (issue #24) | As listed in the issue | As listed in the issue |
| RIT bootstrap | `cmd/server` `verifyRIT` emits `l5.rit.bootstrap` once, then `l5.rit.absent` | `integrity::request::verify` follows `internal/integrity.Verify`: `l5.rit.ok` with value `"bootstrap"` |
| Rate configuration (`internal/gate/config.go`) | `rlWindow`/`rlSoft`/`rlHard` fall back to defaults | `rate::Config::new` refuses zero; there are no defaults |

Wiring any of these into serving composition is not decided here. It stays a
gap in the ledger.
