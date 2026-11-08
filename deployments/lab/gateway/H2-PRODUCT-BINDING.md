# H2 product-binding Cap pilot (Unlock B)

Status: Cap CLI `--w2-h2-product-binding` runs a **TLS listen+client measure**
topology (loopback under `network_mode=none`). `original_h2_measured` flips only
from a validated measure receipt. Never invent `pass_product_clean_room` or
claim `desktop_as_hosted` as hosted product. Detection freeze.

## What this unlock is

Gateway H2 product-binding measurement: serve path + route
`required.original_h2` + live TLS/H2 observation. Distinct from capture-component
`observe_h2` five-case (run 42479169280 = `pass_component_hosted` only).

## Code-defined Cap CLI (GRANT.pilot_command)

```text
cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --w2-h2-product-binding
```

## Cap topology

| Path | Role |
| --- | --- |
| `deployments/lab/compose.h2-product-binding.yaml` | Cap compose (`profile: h2-product-binding`) |
| `build/rust/h2-product-binding-lab.Dockerfile` | Cap image (measure binary + oneshot) |
| `build/rust/h2-product-binding-lab.Dockerfile.dockerignore` | Cap freeze build context |

Measure binary: `humanymous-h2-product-binding-measure` (features
`gateway,hosted-pilot`). Emits receipt schema
`humanymous.lab-h2-product-binding-measure-receipt/1`.

## Already landed (not product)

| Item | Counts as |
| --- | --- |
| R14 serving consumer @ `5ce40904` | observation-gated authorize |
| Hosted capture `observe_h2` run 42479169280 | `pass_component_hosted` |
| Cap=1 Clippy / fail-closed pure | component |
| Unlock C `--authority-node-ipc` Cap | component only; no reburn |
| Desktop lab TLS measure (this Cap) | component measure only |

## Fail-closed

| Condition | Expect |
| --- | --- |
| Cap topology missing | Cap CLI refuse |
| Gateway freeze preimage drift | abort Cap |
| Capture feeder as product-binding | refuse |
| `original_h2_measured=true` without measure receipt | refuse |
| Authority `--authority-node-ipc` Cap spam reburn | refuse |
| Desktop measure claimed as hosted product | refuse |

## N06 / honesty after measure

Desktop lab loopback measure may set `original_h2_measured=true` when the
receipt validates (TLS/H2 status ≠ 422 `required_capability`). That is **not**
`desktop_as_hosted`, **not** readiness product green, **not**
`pass_product_clean_room`.

Still Root for N06 product lift:

1. Owner-integration readiness join (SoT N06 / R14-D6)
2. Hosted venue qualification (Desktop ≠ hosted product)
3. Matrix G1 admission after measured binding
