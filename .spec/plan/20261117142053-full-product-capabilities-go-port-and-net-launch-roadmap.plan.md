---
id: plan:root/20261117142053
title: "Full-product capabilities, Go port and .NET launch roadmap"
status: draft
created: 2026-11-17T14:20:53Z
updated: 2026-11-17T14:20:53Z
rel:
  depends_on: [plan:root/20261116060213]
---

# Full-product capabilities, Go port and .NET launch roadmap

## Goal

On 2026-11-17 the owner chose, for the open items in issue #65:

- build the nine missing full-product capabilities as roadmap work rather than
  signing exceptions;
- port the unported legacy Go rows to Rust before the Go tree is removed;
- finish the .NET launch path now rather than deferring it.

`codex/rust` is complete only when `delivery ready` passes on every axis at HEAD
and `validate / merge-ready` is green. This plan orders the work. Each step lands
as its own `claude/<slug>` pull request. Detection freeze, Docker-only product
e2e, Rust 1.99.1 / edition 2024 and the no-forgery rule apply throughout.

## Step 0: merge deadlock (owner)

Every pending pull request fails the required check `validate / merge-ready`. That
check is red because of base-branch defects which those same pull requests fix:

| Defect | Fixed by |
| --- | --- |
| runtime-lib inventory mismatch | #41 |
| retired Go images | #59 |
| `publish=false` on all crates | #61 |
| missing Node qualification receipt | #64 |
| documentation review inventory | #63 |
| Go advisories | #58, #44 |

The `codex-rust-protected` ruleset has no bypass actors, so no pull request can
merge. Until the full product qualifies (Step 2), `merge-ready` cannot be green at
all. The owner must either add a bypass actor (for example the repository admin
role in pull-request mode) or temporarily require only the per-axis jobs a pull
request touches. After that, merge order is #41, #40, #42, #58, #44, #59, #61, #63,
#64, #60. Agents rebase and re-run Validate after each merge.

## Step 1: .NET launch path

Accounting rows 24/72/73 are the dotnet hosted/package ignored tests. They need a
real launch executor (`actor/prebuilt.rs`) and a pilot declaration in
`scripts/ci/native-pilots.json`, mapped by name like the other pilots. Owner input:
the launch API shape (hosted process or package entry point) recorded in #65.

## Step 2: full-product capabilities

Order follows dependency. Each capability turns one `required_unavailable` entry in
`test/lab/runner/src/orchestration.rs` into an evaluated product case. The flag
`full_product_qualified` becomes true only when every case passes in Docker.

| # | Capability | Product cases | Needs |
| --- | --- | --- | --- |
| 2a | full Gate TLS/H2/capture | gate, raw-tls-h2 | certificate source (Step 3 B02) |
| 2b | runtime binary authenticated witness transport | attack-catalog, swarm | — |
| 2c | cross-namespace origin/audit effect binding | origin-isolation, audit-restart | owner G6 grant |
| 2d | session-bound real browser collector | collector-parity, missing-signals-closed | browser image |
| 2e | Pass recovery | pass | — |
| 2f | human, keyboard and assistive controls | human-control, challenge-keyboard, challenge-assistive | Windows AT venue for the native part |
| 2g | native OS qualification | native axis | Step 1 |
| 2h | signed remote target attestation | deployment | owner signing key (secret) |
| 2i | provider deployment | deployment, documentation-examples | owner provider account |

Red-blue product (RB) and clean-room product matrix (PM) runs need owner-authored
grants, per `transfer_applicator.rs`. Agents never author grants.

## Step 3: legacy Go port (FM11 ledger)

`.agents/retirement/legacy-go-row-review.json` has 95 `partial` and 52 `missing`
rows. Port them by plan slice:

| Slice | Rows | Notes |
| --- | --- | --- |
| B02 certificate source (ACME, BYO, dev self-sign) | cmd/server, cmd/gate tls/acme/cert | also unblocks 2a |
| B23 Redis client, ledger, rate limit | internal/redis, internal/gate redis* | |
| B24 Redis/ClickHouse audit projection | internal/audit sinks | |
| B11 playground, livefeed, launch parity | cmd/server playground/launch, internal/livefeed | lab-only replacement |
| B21 admin console; B15 loader injection; B05 demo | internal/gate console/inject, cmd/server demo | |
| resource gate, embed/media | internal/resource | emits existing signal IDs only |
| watermark LSB, recovery, issuance | internal/watermark | |
| mlcorrect-train promotion gate | cmd/mlcorrect-train | SoT model-training-correction |
| attacker clients | cmd/redteam h2raw, cmd/tlsparrot | defense-only, this deployment |
| remaining `partial` rows | internal/gate, cmd/server, others | close the recorded gaps |

**Detection freeze:** the settings overlay, classify and resolve rows,
`settings_engine` and `pass_scoring` feed or emit scoring inputs. They are ported
only after the owner explicitly unfreezes or approves an exact-parity port. No
signal ID, weight or threshold changes otherwise.

When every row is `ported_unit` or explicitly oracle-only, remove the Go sources,
`go.mod` and `go.sum` in a needs-human pull request, keeping the oracle pinned at
981d25c.

## Step 4: final validation

Docker red-blue-validate (state source, image, topology and runner), workspace
`cargo test --locked`, fmt and Clippy, wasm32 collector check, an actual browser
pilot, and `delivery ready` on every axis. Summarise in #65 and update #22.

## Record

- 2026-11-17: the owner chose roadmap implementation, porting Go before removal,
  and handling .NET now. A merge attempt on #41 was refused: "Required status
  check validate / merge-ready is failing". The ruleset has no bypass actors.
