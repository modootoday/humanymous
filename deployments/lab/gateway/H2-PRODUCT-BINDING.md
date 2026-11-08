# H2 product-binding Cap pilot (Unlock B)

Status: Cap CLI `--w2-h2-product-binding` is code-defined under claimed
`test/lab/runner/**`. Cap-ready pack still transfers **contract docs only**.
Cap topology (compose + Dockerfile for gateway serve + `required.original_h2`)
is Root-allocated and currently missing → Cap CLI refuse-closes; Cap burn=0
until Root supplies topology. product=0.

## What this unlock is

Hosted **gateway** H2 product-binding measurement: serve path + route
`required.original_h2` + owner integration. Distinct from capture-component
`observe_h2` five-case (run 42479169280 = `pass_component_hosted` only).

## Code-defined Cap CLI (GRANT.pilot_command)

```text
cargo run --locked -p humanymous-lab-runner -- run --evidence BUNDLE --w2-h2-product-binding
```

## Already landed (not product)

| Item | Counts as |
| --- | --- |
| R14 serving consumer @ `5ce40904` | observation-gated authorize |
| Hosted capture `observe_h2` run 42479169280 | `pass_component_hosted` |
| Cap=1 Clippy / fail-closed pure | component |
| Unlock C `--authority-node-ipc` Cap | component only; no reburn |

## Missing Root Cap topology (refuse-closed)

| Path | Role |
| --- | --- |
| `deployments/lab/compose.h2-product-binding.yaml` | Desktop/hosted-honest Cap compose |
| `build/rust/h2-product-binding-lab.Dockerfile` | Cap image for serve+authorize pilot |

## One-grant-apply

1. Review this pack + `TRANSFER.md` + `transfer-manifest.json`.
2. `GRANT.json` sets `pilot_command` to the Cap CLI argv above.
3. `transfer-probe` then `transfer-apply --grant GRANT.json`.
4. Cap=1 only after Root Cap topology lands; until then Cap burn=0 with
   refuse-closed Cap CLI citing missing paths. Never invent
   `pass_product_clean_room`. Capture feeder forbidden.

## Fail-closed

| Condition | Expect |
| --- | --- |
| `pilot_command` absent/empty / lacks `--w2-h2-product-binding` | refuse apply / blocked_claim |
| Cap topology missing | Cap CLI refuse; Cap burn=0 |
| Gateway freeze preimage drift | abort Cap |
| Capture feeder as product-binding | refuse |
| Readiness `original_h2:true` without measured binding | refuse |
| Authority `--authority-node-ipc` Cap spam reburn | refuse |

## N06

Readiness / product / matrix G1 reconsideration only after measured binding +
Root N06 join — never from Clippy/capture-component alone.
