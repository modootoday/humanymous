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

## Hosted venue Cap topology (N06 — transferred; Cap venue pending)

Live Cap topology transferred (GRANT → transfer-apply). Cap CLI:
`--w2-h2-hosted-venue`. Cap=1 only when `actual_hosted_linux=true`
(Specfold/github-hosted). Desktop Cap refuse-closed (Cap=0; no
`GITHUB_ACTIONS` spoof).

| Path | Role |
| --- | --- |
| `deployments/lab/compose.h2-hosted-venue.yaml` | Hosted Cap compose (not Desktop loopback) |
| `build/rust/h2-hosted-venue-lab.Dockerfile` | Hosted Cap image |
| `build/rust/h2-hosted-venue-lab.Dockerfile.dockerignore` | Hosted Cap build context |

Owner-join readiness schema:
`humanymous.lab-h2-n06-owner-join-readiness/1` (always
`n06_owner_join=false` until hosted measure + Root join). Evidence:
`.spec.local/root/evidence/20261109010000-cursor-n06-hosted-venue-apply/`.

## Already landed (not product)

| Item | Counts as |
| --- | --- |
| R14 serving consumer @ `5ce40904` | observation-gated authorize |
| Hosted capture `observe_h2` run 42479169280 | `pass_component_hosted` |
| Cap=1 Clippy / fail-closed pure | component |
| Unlock C `--authority-node-ipc` Cap | component only; no reburn |
| Desktop lab TLS measure @ `4a5d0135` / `20261109004100` | `pass_component_desktop_measure` only |

## Fail-closed

| Condition | Expect |
| --- | --- |
| Cap topology missing | Cap CLI refuse |
| Gateway freeze preimage drift | abort Cap |
| Capture feeder as product-binding | refuse |
| `original_h2_measured=true` without measure receipt | refuse |
| Authority `--authority-node-ipc` Cap spam reburn | refuse |
| Desktop measure claimed as hosted product | refuse |
| `n06_owner_join=true` without hosted venue + `actual_hosted_linux` | refuse |
| Hosted venue Cap on Desktop | refuse (`actual_hosted_linux=false`) |
| W2 Desktop measure Cap reburn without structural delta | refuse |

## N06 / honesty after Desktop measure

Desktop lab loopback measure may set `original_h2_measured=true` when the
receipt validates (TLS/H2 status ≠ 422 `required_capability`). That is **not**
`desktop_as_hosted`, **not** readiness product green, **not**
`pass_product_clean_room`, **not** N06 owner join.

Still Root for N06 product lift:

1. Specfold/`actual_hosted_linux` Cap=1 once with `--w2-h2-hosted-venue` (topology already transferred)
2. Owner-integration readiness join (SoT N06 / R14-D6) after hosted measure
3. Matrix G1 admission after measured hosted binding
