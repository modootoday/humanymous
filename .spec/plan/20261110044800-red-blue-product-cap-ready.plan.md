---
id: plan:root/20261110044800-red-blue-product-cap-ready
title: Cap=0 Cap-ready --red-blue-product require-full feeder (not Cap twin)
status: active
created: 2026-11-09T19:48:00Z
updated: 2026-11-09T19:48:00Z
rel:
  depends_on:
    - plan:root/20261110043600-production-status-live-cap0
    - sot:root/clean-room-usage-matrix
    - sot:root/program-roadmap
  governs: []
---

# Cap=0 Cap-ready red-blue-product require-full feeder

Brief: Cap free=false / Cap contended. Cap burn=0. Land Cap-ready
`--red-blue-product --require-full` topology + TRANSFER helper + GRANT.TEMPLATE
with matching preimages so the **next Cap=1 after Cap free restores** is a
direct product/red-blue feeder — not another G2/G5/CR/G6 component twin.
Never invent `pass_product_clean_room` / Cap free / AUTH/AT / BrowserReady.

## Cap=0 landed

| Item | Path / note |
| --- | --- |
| Cap CLI | `--red-blue-product` (+ `--require-full` for product feeder) |
| Cap=0 honesty | tip-bound missing product / red-blue cell enumeration |
| Cap-ready pack | `.spec.local/root/evidence/20261110044800-cursor-red-blue-product-cap-ready/cap-ready/red-blue-product/candidate-pack/` |
| GRANT.TEMPLATE | matching preimages; unlock=`RB` |
| ROOT-ONE-STEP | standing-autonomy transfer-apply recipe |
| transfer-probe | ready=true expected while live destinations absent |

## Still missing for pass_product_clean_room (honesty)

- 47 clean-room-required non-product cells (tip-bound matrix)
- red_blue_up blocked; red_blue_attack/e2e_assert/gate_e2e/swarm unrun
- m3-full-product required_unavailable / fail
- clean-room-product hold_component only
- AUTH/AT absent; Cap free=false

## Exact next Cap=1 (when Cap free restores)

1. `cp GRANT.TEMPLATE.json GRANT.json` + `transfer-apply`
2. Cap realloc `humanymous.cursor-red-blue-product-cap-realloc/1`
3. Cap=1 once: `--red-blue-product --require-full` (jobs=1)
4. Then Docker product oneshot `bash scripts/e2e-docker.sh` / `make e2e` for `red_blue_*`

product=0 until measured `pass_product_clean_room`.
