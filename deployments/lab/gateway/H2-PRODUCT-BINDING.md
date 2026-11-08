# H2 product-binding Cap pilot (Unlock B)

Status: Cap CLI `--w2-h2-product-binding` is code-defined under claimed
`test/lab/runner/**`. Cap topology (compose + Dockerfile + dockerignore) is
Cap-ready for preimage-matching transfer-apply. Cap=1 burns composition Cap
only; never invent `pass_product_clean_room` or readiness `original_h2:true`.

## What this unlock is

Hosted **gateway** H2 product-binding measurement: serve path + route
`required.original_h2` + owner integration. Distinct from capture-component
`observe_h2` five-case (run 42479169280 = `pass_component_hosted` only).

## Code-defined Cap CLI (GRANT.pilot_command)

```text
cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --w2-h2-product-binding
```

## Cap topology

| Path | Role |
| --- | --- |
| `deployments/lab/compose.h2-product-binding.yaml` | Cap compose (`profile: h2-product-binding`) |
| `build/rust/h2-product-binding-lab.Dockerfile` | Cap image (gateway runtime + oneshot) |
| `build/rust/h2-product-binding-lab.Dockerfile.dockerignore` | Cap freeze build context |

## Already landed (not product)

| Item | Counts as |
| --- | --- |
| R14 serving consumer @ `5ce40904` | observation-gated authorize |
| Hosted capture `observe_h2` run 42479169280 | `pass_component_hosted` |
| Cap=1 Clippy / fail-closed pure | component |
| Unlock C `--authority-node-ipc` Cap | component only; no reburn |
| Cap topology composition Cap | component only |

## One-grant-apply

1. Review this pack + `TRANSFER.md` + `transfer-manifest.json`.
2. `GRANT.json` sets `pilot_command` to the Cap CLI argv above.
3. `transfer-probe` then `transfer-apply --grant GRANT.json`.
4. Cap=1 once with `--w2-h2-product-binding`. Component ≠ product.
   Never invent `pass_product_clean_room`. Capture feeder forbidden.

## Fail-closed

| Condition | Expect |
| --- | --- |
| `pilot_command` absent/empty / lacks `--w2-h2-product-binding` | refuse apply / blocked_claim |
| Cap topology missing | Cap CLI refuse |
| Gateway freeze preimage drift | abort Cap |
| Capture feeder as product-binding | refuse |
| Readiness `original_h2:true` without measured binding | refuse |
| Authority `--authority-node-ipc` Cap spam reburn | refuse |

## N06

Readiness / product / matrix G1 reconsideration only after measured binding +
Root N06 join — never from Clippy/capture-component/composition Cap alone.
