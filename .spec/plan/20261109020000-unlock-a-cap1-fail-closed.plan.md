---
id: plan:root/20261109020000-unlock-a-cap1-fail-closed
title: Unlock A G4 CEF/AT Cap=1 fail-closed (missing CEF bundle)
status: active
created: 2026-11-08T16:58:00Z
updated: 2026-11-08T16:58:00Z
rel:
  depends_on:
    - plan:root/20261108233000-post-unlock-c-matrix-honesty
    - plan:root/20261108220600-cap-ready-g4-w2-harness
    - plan:root/20261108215600-g4-at-cef-blocked-claim
    - sot:root/program-roadmap
  governs: []
---

# Unlock A Cap=1 fail-closed (missing CEF bundle)

Brief: Unlock **A** GRANT+transfer-apply bound on claimable
`scripts/ci/cef-pilot/**` (collaboration scope released path). Cap CLI
`--g4-cef-at` status **bound**. Cap=1 once @ `72d0f8ad` EXIT **1**
`fail_closed_missing_cef_bundle` — `HUMANYMOUS_CEF_PILOT_BUNDLE` unset; no CEF
binary invented. Cap **spent**. `product=0`. Detection freeze. No N06 / W2 /
authority Cap reburn.

Private evidence (gitignored `.spec.local/`):

- `…/20261109020000-cursor-unlock-a-cap-ready/` — UNLOCK-PROBE, CAP1-RECEIPT,
  CAP1-CONSOLE, CAP-REFUSE, CLOSER
- `…/cap-ready/g4-cef-at/` — GRANT.json, transfer-manifest (11 entries),
  UNLOCK-A-APPLIED, TRANSFER-APPLY

## Decision rules

R-A1. **Claimable path release ≠ CEF binary.** Scope
`scripts/ci/cef-pilot/**` authorizes identity Cap-ready candidates + GRANT
apply. Cap day still refuses without `HUMANYMOUS_CEF_PILOT_BUNDLE`.

R-A2. **Cap=1 once after Unlock A bound.** Fail-closed EXIT1 records Cap burn;
reburn only with structural delta (fresh CEF bundle / Root allocation).

R-A3. **Component refuse ≠ product.** Never invent `pass_product_clean_room` or
`BrowserReady`.

R-A4. **No N06/W2/authority Cap reburn** without new structural delta.

## Verification

- Branch `codex/rust` @ `72d0f8ad`+
- `unlock-probe`: `g4_cef_at_cap_cli_status=bound`
- Cap `--g4-cef-at`: EXIT **1** missing CEF bundle env
- product: **0**

## Next

1. Root: set `HUMANYMOUS_CEF_PILOT_BUNDLE` to a fresh private CEF bundle (+ optional
   interactive AT Windows tuple) then Cap reburn only with structural delta
2. Else Root N06 owner-integration auth handoff (Cap=0 readiness without auth)
