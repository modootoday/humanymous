# Cap=0 tip-bind — RB Cap=1 EXIT2 feeder honesty

| Field | Value |
| --- | --- |
| Tip | `0329ef44` / `codex/rust` |
| Cap | **0** (tip-bind only; Cap free **false**, spent) |
| Cap=1 source | `20261110050000-cursor-red-blue-product-cap-realloc` |
| Cap=1 exit | **2** (6 passed / 0 failed) |
| `counts_as` | `red_blue_product_require_full_feeder_not_product` |
| `product` | **0** |
| `pass_product_clean_room` | **0** |
| AUTH / AT | absent (Root) |
| Unlock C | skipped (`blocked_claim`) |

## Honesty

- Cap=1 feeder ≠ product
- Never invent `pass_product_clean_room` / Cap free / BrowserReady / AUTH/AT
- Detection freeze held

## Next

Docker `red-blue-validate` / `e2e-docker` oneshot for `red_blue_*` cells; Root AUTH/AT when present.
