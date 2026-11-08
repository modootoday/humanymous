# H2 product-binding Cap pilot (Unlock B)

Status: Desktop Cap CLI `--w2-h2-product-binding` ran a **TLS listen+client
measure** (loopback under `network_mode=none`) with
`original_h2_measured=true` from a validated receipt
(evidence `20261109004100`). That is **Desktop lab measure only** — not hosted
product, not `desktop_as_hosted`, not N06 owner join, not
`pass_product_clean_room`. Detection freeze.

## What this unlock is

Gateway H2 product-binding measurement: serve path + route
`required.original_h2` + live TLS/H2 observation. Distinct from capture-component
`observe_h2` five-case (run 42479169280 = `pass_component_hosted` only).

## Code-defined Cap CLI (GRANT.pilot_command)

```text
cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --w2-h2-product-binding
```

## Cap topology (Desktop lab measure — landed)

| Path | Role |
| --- | --- |
| `deployments/lab/compose.h2-product-binding.yaml` | Desktop Cap compose (`profile: h2-product-binding`) |
| `build/rust/h2-product-binding-lab.Dockerfile` | Cap image (measure binary + oneshot) |
| `build/rust/h2-product-binding-lab.Dockerfile.dockerignore` | Cap freeze build context |

Measure binary: `humanymous-h2-product-binding-measure` (features
`gateway,hosted-pilot`). Emits receipt schema
`humanymous.lab-h2-product-binding-measure-receipt/1`.

## Hosted venue Cap topology (N06 — Specfold Cap EXIT0 component)

Live Cap topology transferred (GRANT → transfer-apply). Cap CLI:
`--w2-h2-hosted-venue`. Specfold Cap EXIT **0** run **42500919761** @
`d483c40f` = `pass_component_hosted_measure` only — **not** product, **not**
N06 owner join. Desktop Cap refuse-closed (no `GITHUB_ACTIONS` spoof). No
hosted Cap reburn without structural delta. Cap free again for Unlock A.

| Path | Role |
| --- | --- |
| `deployments/lab/compose.h2-hosted-venue.yaml` | Hosted Cap compose (not Desktop loopback) |
| `build/rust/h2-hosted-venue-lab.Dockerfile` | Hosted Cap image |
| `build/rust/h2-hosted-venue-lab.Dockerfile.dockerignore` | Hosted Cap build context |

Owner-join readiness Cap CLI: `--n06-owner-join-readiness` (Cursor-owned;
binds hosted Cap EXIT0 → readiness; Cap=0 without Root auth handoff). Schema
`humanymous.lab-h2-n06-owner-join-readiness/1` always
`n06_owner_join=false` until Root auth handoff. Evidence:
`.spec.local/root/evidence/20261109013400-cursor-n06-hosted-venue-cap-realloc/`,
`.spec.local/root/evidence/20261109005000-cursor-n06-owner-join/`.

## Already landed (not product)

| Item | Counts as |
| --- | --- |
| R14 serving consumer @ `5ce40904` | observation-gated authorize |
| Hosted capture `observe_h2` run 42479169280 | `pass_component_hosted` |
| Cap=1 Clippy / fail-closed pure | component |
| Unlock C `--authority-node-ipc` Cap | component only; no reburn |
| Desktop lab TLS measure @ `4a5d0135` / `20261109004100` | `pass_component_desktop_measure` only |
| Hosted venue Cap EXIT0 run 42500919761 @ `d483c40f` | `pass_component_hosted_measure` only |

## Fail-closed

| Condition | Expect |
| --- | --- |
| Cap topology missing | Cap CLI refuse |
| Gateway freeze preimage drift | abort Cap |
| Capture feeder as product-binding | refuse |
| `original_h2_measured=true` without measure receipt | refuse |
| Authority `--authority-node-ipc` Cap spam reburn | refuse |
| Desktop measure claimed as hosted product | refuse |
| Hosted Cap component claimed as product / N06 join | refuse |
| `n06_owner_join=true` without Root auth handoff | refuse (Cap=0) |
| Hosted venue Cap on Desktop | refuse (`actual_hosted_linux=false`) |
| W2 Desktop measure Cap reburn without structural delta | refuse |
| Hosted venue Cap reburn without structural delta | refuse |

## N06 / honesty after hosted Cap EXIT0

Desktop lab loopback measure may set `original_h2_measured=true` when the
receipt validates (TLS/H2 status ≠ 422 `required_capability`). Hosted Cap
EXIT0 run **42500919761** is **component** only. Neither is
`desktop_as_hosted`, readiness product green, `pass_product_clean_room`, or
N06 owner join.

Still Root for N06 product lift:

1. ~~Specfold/`actual_hosted_linux` Cap=1 once with `--w2-h2-hosted-venue`~~ **done** (component EXIT0)
2. Owner-integration readiness join authorization + exact path release (SoT N06 / R14-D6) — Root auth handoff; Cap=0 until then
3. Matrix G1 / `pass_product_clean_room` admission after hosted measure + owner join

Exact Root ask paths: `configuration.rs`, `service/readiness.rs`,
`application.rs`, network-capture SoT N06, R14-D6 plan.
