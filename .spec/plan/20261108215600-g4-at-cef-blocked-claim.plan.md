---
id: plan:root/20261108215600-g4-at-cef-blocked-claim
title: G4 human/keyboard/AT (CEF) — blocked_claim honesty receipt
status: active
created: 2026-11-08T12:56:24Z
updated: 2026-11-08T12:56:24Z
rel:
  depends_on:
    - sot:root/browser-ui
    - sot:root/runtime-application
    - sot:root/container-test-environments
    - plan:root/20261108213000-g2-collector-provenance
    - plan:root/20261108210000-r14-h2-serving-consumer
  governs: []
---

# G4 AT/CEF — blocked_claim (no Cap burn)

Brief: After G2 Cap=1 Desktop browser-product recording (`e711a632` /
`05fd59cb`), coordinator next gap G4 human/keyboard/assistive controls is
**Root-only**. Cursor records `blocked_claim` and does not invent CEF/AT
execution, motor gates, or product promotion. Detection freeze; a11y non-motor
preserved. Cap=1 conserved.

## 1. Problem

Interactive AT/CEF/IME qualification is required for product/M3 a11y cells, but
no Root execution allocation or path release admits those pilots to
`cursor-integration`. Desktop a11y component feeders already exist as
`pass_component_desktop` only. Cap spam cannot clear `required_unavailable`.

## 2. Decision rules

G4-D1. **blocked_claim.** Cursor does not claim `browser-ui` interactive CEF/AT
execution while Root allocation and path release are absent.

G4-D2. **Non-motor.** Never gate Pass/CHALLENGE on motor richness or speed.

G4-D3. **Component ≠ product.** Existing Desktop a11y / browser-product Cap=1
results stay `pass_component_*` only; not `pass_product` /
`pass_product_clean_room`.

G4-D4. **No Cap spam.** Do not reburn Cap=1 a11y/browser-product `--require-full`
without a new Root structural delta.

G4-D5. **Residual.** Next Cursor-owned residual after G4 block is W2/G1 hosted
H2 product binding; that residual is also Root-blocked under N06 until hosted
gateway H2 binding + owner integration are allocated (see R14 residual and
private `W2-H2-PRODUCT-BINDING-ROOT-ASK.md`).

## 3. Verification

- Private receipts under
  `.spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/`
  (`g4-blocked.json`, `G4-ROOT-ASK.md`, `W2-H2-PRODUCT-BINDING-ROOT-ASK.md`,
  `goal-wake.json`).
- Cap burn this turn: **false**.
- Matrix cells: unchanged `required_unavailable`.
- `product=0` / `pass_product_clean_room=0`.

## 4. Residual / next gap

Root must allocate either (a) G4 CEF/AT interactive pilots with exact path
release, or (b) W2 hosted H2 product-binding measurement + N06 readiness join.
Cursor waits; no architecture invention. Detection freeze.
