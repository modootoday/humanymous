---
id: plan:root/20261109020000-unlock-a-cap1-fail-closed
title: Unlock A G4 CEF/AT Cap=1 (missing CEF then realloc with host bundle)
status: active
created: 2026-11-08T16:58:00Z
updated: 2026-11-08T17:10:00Z
rel:
  depends_on:
    - plan:root/20261108233000-post-unlock-c-matrix-honesty
    - plan:root/20261108220600-cap-ready-g4-w2-harness
    - plan:root/20261108215600-g4-at-cef-blocked-claim
    - sot:root/program-roadmap
  governs: []
---

# Unlock A Cap=1 — CEF missing then host-bundle realloc

Brief: Unlock **A** GRANT+transfer-apply bound on claimable
`scripts/ci/cef-pilot/**`. Cap CLI `--g4-cef-at` **bound**.

1. Cap=1 @ `72d0f8ad`/`0026c293` EXIT **1** `fail_closed_missing_cef_bundle`
   — env unset; no CEF binary invented (evidence `20261109020000`).
2. Host search found complete private CEF runtime (CI artifacts/caches empty).
   Cap **realloc** (structural: bundle available) + Cap=1 once with
   `HUMANYMOUS_CEF_PILOT_BUNDLE` set → `g4_cef_at` component **executed**;
   process EXIT **1** `component_executed_cleanup_failed` (Desktop baseline
   env unset). Cap **free=false**. `product=0`. Detection freeze. No N06 / W2 /
   authority Cap reburn.

Private evidence (gitignored `.spec.local/`):

- `…/20261109020000-cursor-unlock-a-cap-ready/` — first Cap EXIT1 missing env
- `…/20261109020200-cursor-unlock-a-cef-bundle-realloc/` — search, realloc,
  Cap1 receipts (component executed)
- `…/cap-ready/g4-cef-at/` — GRANT.json, UNLOCK-A-APPLIED, TRANSFER-APPLY

## Decision rules

R-A1. **Claimable path release ≠ CEF binary.** Scope
`scripts/ci/cef-pilot/**` authorizes identity Cap-ready candidates + GRANT
apply. Cap day still refuses without `HUMANYMOUS_CEF_PILOT_BUNDLE`.

R-A2. **Cap=1 once after Unlock A bound; realloc only with structural delta.**
Host-discovered complete private CEF tree is structural for Cap realloc.
Further Cap reburn needs new delta (Desktop baseline cleanup EXIT0 / Root AT).

R-A3. **Component ≠ product.** `g4_cef_at` executed ≠ `BrowserReady` ≠
`pass_product_clean_room`.

R-A4. **No N06/W2/authority Cap reburn** without new structural delta.

## Verification

- Branch `codex/rust` @ `0026c293`+
- Cap `--g4-cef-at` realloc Cap=1: EXIT **1** component executed + cleanup_failed
- `HUMANYMOUS_CEF_PILOT_BUNDLE` present for realloc Cap (host private evidence)
- Cap free: **false**; product: **0**

## Next

1. Optional Cap realloc only with Desktop baseline structural delta for cleanup
   EXIT0, or Root interactive AT Windows tuple
2. Else Root N06 owner-integration auth handoff (Cap=0 readiness without auth)
