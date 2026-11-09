# Clean-room product-matrix Cap=0 Cap-ready structural feeder

Observed: 2026-11-09T21:58:12Z · tip `7dbd69708062` · session `cursor-integrator`
Cap burn=**0** · Cap free before=**false** · Cap free after realloc=**true** · product=**0**

## Structural delta (NOT a spent twin)

| Spent | This feeder |
| --- | --- |
| `--clean-room-product` + compose.clean-room-product.yaml | `--clean-room-product-matrix` + compose.clean-room-product-matrix.yaml |
| CR Cap1 topology spent | New require-full allowlist + e2e EXIT0 + AUTH/AT refuse binds |

## Landed

| Surface | Result |
| --- | --- |
| Cap CLI `--clean-room-product-matrix --require-full` | scaffolded + Cap=1 path after realloc |
| Cap-ready pack | `cap-ready/clean-room-product-matrix/candidate-pack/` |
| TRANSFER-PROBE | `clean_room_product_matrix_preimage_ready=true` |
| GRANT + TRANSFER-APPLY | live Cap topology written |
| Cap realloc | `20261110065100-cursor-clean-room-product-matrix-cap-realloc/CAP-REALLOC.json` |

## Honesty

- Never invent pass_product_clean_room / Cap free / BrowserReady / AUTH/AT
- Detection freeze
- Cap twin spam forbidden (G2/G5/CR/G6/RB)
