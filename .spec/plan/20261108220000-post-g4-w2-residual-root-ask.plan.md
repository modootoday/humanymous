---
id: plan:root/20261108220000-post-g4-w2-residual-root-ask
title: Post-G4/W2 residual inventory — consolidated ROOT-ASK (Cap free)
status: active
created: 2026-11-08T13:01:00Z
updated: 2026-11-08T13:06:00Z
rel:
  depends_on:
    - plan:root/20261108215600-g4-at-cef-blocked-claim
    - plan:root/20261108213000-g2-collector-provenance
    - plan:root/20261108210000-r14-h2-serving-consumer
    - sot:root/program-roadmap
    - sot:root/pass-protocol
  governs: []
---

# Post-G4/W2 residual inventory — Cap free / wait Root

Brief: After G4 `blocked_claim` and W2/G1 hosted H2 product-binding Root-block
(N06) at `92571cd7`, Cursor re-inventoried residuals. **No Cursor-executable
product slice** remains. Consolidated ROOT-ASK records Cap free and waiting Root
G4 CEF/AT allocation **or** W2 H2 product-binding measurement. Detection freeze.
Cap burn=0. product=0. Never invent `pass_product_clean_room`.

## 1. Problem

Coordinator asked for the next Cursor-owned residual after G4/W2 block (G3, G5,
custody/runtime/browser binding, matrix honesty, Specfold cover only if in
scope). Cap=1 may burn only on a structural delta with honest fail-closed
receipts. Cap-spam of `--require-full` is forbidden when Cap is merely free.

## 2. Decision rules

R-D1. **All closers Root-blocked.** G3 (Pass recovery / SoT-36), G5
(authenticated witness transport), custody/runtime/browser product binding, and
live H2 readiness / G4 CEF-AT remain `root_only` or `blocked_claim`.

R-D2. **No Cap spam.** Cap is free and conserved. Do not reburn browser /
collector / controls / a11y / topology `--require-full` without a new Root
structural delta.

R-D3. **Matrix honesty hold.** `root-coordination` owns matrix paths; cells stay
`required_unavailable` / `pass_component_*`. Never invent
`pass_product_clean_room`.

R-D4. **Specfold out of scope.** require-go cover already
`pass_component_hosted` (run 42476157423). No further Specfold attach this turn.

R-D5. **Wake.** Cursor resumes only on W2 product-binding path release or W4 G4
CEF/AT allocation with Cap free and no Cap-spam reburn.

## 3. Verification

- Private receipts under
  `.spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/`:
  `post-g4-w2-residual-inventory.json`, `CONSOLIDATED-ROOT-ASK.md`,
  `consolidated-root-ask.json`, refreshed `goal-wake.json`.
- Cap burn this turn: **false**. Cap free: **true**.
- Matrix: untouched.
- `product=0` / `pass_product_clean_room=0`.

## 4. Residual / next gap

Root/coordinator must allocate either (a) G4 CEF/AT interactive pilots with
exact path release, or (b) W2 hosted H2 product-binding measurement + N06
readiness join. Cursor waits on Cap-free; no architecture invention; detection
freeze.

Follow-on Cap-ready prep (no Cap burn; no idle ROOT-ASK re-push):
`plan:root/20261108220600-cap-ready-g4-w2-harness` + private
`.spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/`.
