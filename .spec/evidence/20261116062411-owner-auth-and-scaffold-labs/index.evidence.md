---
id: evidence:root/20261116062411
title: Owner authorizations and product-lab scaffold finding
status: active
created: 2026-11-16T06:24:11Z
updated: 2026-11-16T06:24:11Z
rel:
  depends_on: [plan:root/20261116060213, sot:root/clean-room-usage-matrix]
---

# Owner authorizations and product-lab scaffold finding

## What was measured

### Owner decisions (2026-11-16, chat)

The repository owner confirmed that no other session edits humanymous. They
approved push to `codex/rust`. Acting as Root, they also approved two items.

1. **AUTH-HANDOFF.** Recorded as
   `.spec.local/root/evidence/root-n06-owner-join-auth-handoff/AUTH-HANDOFF.json`
   (schema `humanymous.root-n06-owner-join-auth-handoff/1`,
   `n06_owner_join_authorized=true`, `n06_owner_join=false`, product 0). A fresh
   `lab-runner cap0-chain --inspect-only` lists AUTH-HANDOFF as satisfied. The
   remaining blockers are AT-HANDOFF, Cap-free-GRANT and TRANSFER-manifest.
   `n06_owner_join` stays false until a hosted-venue measured join exists.
2. **Cap-free GRANT.** Approved in intent, but **not applied**:
   - `cap-realloc-apply` flips `cap_free` inside the Unlock A closer
     (`.spec.local/root/evidence/20261109020800-…/CLOSER.json`). That closer is a
     Windows-desktop run receipt and is absent from this checkout. Writing one
     would forge a measurement.
   - The approval has no measurement value anyway, because of the scaffold
     finding below.

### Product-lab images are scaffolds

Source inspection of `build/rust/*.Dockerfile` and `deployments/lab/product/`:

| Image | Base | Builds Rust | What it runs |
| --- | --- | --- | --- |
| clean-room-product-lab | alpine:3.20 | no | `clean-room-product-cap-pilot.sh`: `test -f` two files, then `cat` a pre-written `measure-receipt.scaffold.json` |
| clean-room-product-matrix-lab | alpine:3.20 | no | same pattern |
| red-blue-product-lab | alpine:3.20 | no | same pattern |
| g2-collector-product-binding-lab | alpine:3.20 | no | same pattern |
| g5-witness-transport-lab | alpine:3.20 | no | same pattern |
| g6-origin-audit-binding-lab | alpine:3.20 | no | same pattern |
| docker-e2e-product, browser-product-measured-e2e pilots | — | — | the same `*-cap-pilot.sh` + `*.scaffold.json` pattern |
| h2-product-binding, h2-hosted-venue, authority-lab, runtime-pilot, browser-product | rust:1.99.1 | yes | real Rust binaries |

So the Cap=1 "product" runs that earlier receipts recorded as spent
(G2/G5/CR/G6/RB/PM twins) echoed fixed JSON. They measured no product
behavior. Their own receipts already counted them as component holds, never
product, so `pass_product_clean_room=0` stays true. Even so, no Docker
image in the tree executes a composed Gate + origin + authority + audit +
browser + attacker product e2e in Rust. The scaffold scripts are also new
foreign shell executables, which conflicts with the Rust-only rule (FM11 debt).

## How to read it

- This is a source observation, not a run.
- IM3 and FM8 cannot be closed by granting Cap. They need a real Rust Docker
  product e2e harness, which does not exist yet; goal-plan unit 35 tracks it.
- AT-HANDOFF still requires an actual Windows interactive AT session.
