---
id: plan:root/20261108233000-post-unlock-c-matrix-honesty
title: Post-Unlock-C matrix honesty + Unlock B Cap-ready probe (Cap=0)
status: active
created: 2026-11-08T14:30:00Z
updated: 2026-11-08T14:30:00Z
rel:
  depends_on:
    - plan:root/20261108230100-bottom-up-inner-outer-parallelization
    - plan:root/20261108221000-clean-room-matrix-cap-driver
    - plan:root/20261108220600-cap-ready-g4-w2-harness
    - sot:root/program-roadmap
    - sot:root/structural-development
  governs: []
---

# Post-Unlock-C matrix honesty + Unlock B Cap-ready probe

Brief: After Unlock C Cap=1 `--authority-node-ipc` EXIT 0 component @ `f6df9e01`
(evidence `20261108232200`), Cap is free again. Preference order: Unlock **B**
(W2 H2 product-binding) → Unlock **A** (G4) → highest claimable P0 residual.
B and A remain Root-blocked (no `UNLOCK-*-APPLIED`). This turn takes the residual:
honest matrix remap of authority require_full to `pass_component_desktop`. Cap=0.
product=0. Never invent `pass_product_clean_room`.

## 1. Preference probe (fail-closed)

| # | Closer | Result |
| --- | --- | --- |
| 1 | Unlock B / G1 W2 hosted H2 product-binding | **blocked** — no `UNLOCK-B-APPLIED.json`; no Root-named gateway product-binding allocation; Cap-ready pack advanced with gateway preimage rehash only |
| 2 | Unlock A / G4 CEF/AT | **blocked** — no `UNLOCK-A-APPLIED.json`; Cap-ready honesty only |
| 3 | P0 residual | **taken** — matrix honesty for Unlock C Cap component |

## 2. Decision rules

R1. **Cap burn only with structural unlock/delta.** Unlock C Cap already burned;
authority `--require-full` reburn forbidden. Cap conserved for B preferred else A.

R2. **Component ≠ product.** Cap EXIT 0 + matrix `pass_component_desktop` ≠
`pass_product_clean_room`.

R3. **No invent Unlock B/A.** Do not forge `UNLOCK-*-APPLIED` or invent readiness
`original_h2:true` / CEF path release.

R4. **Detection freeze.** No scoring/threshold/hard-rule edits.

R5. **No idle CONSOLIDATED-ROOT-ASK re-push.** Wake records Cap free + prefer B.

## 3. Surfaces (private evidence + this plan)

Private (`.spec.local/`, not Git):

- `…/20261108233000-cursor-post-c-closer/` — `CLOSER.json`, `UNLOCK-PROBE.json`,
  `W2-CAP-READY-PROBE.json`, `INDEX.md`
- `…/20261108164500-cursor-clean-room-matrix/matrix.json` — cell
  `desktop-linux|none|authority-node-ipc|lab_runner_require_full`:
  `unrun` → `pass_component_desktop`
- `…/20261108301100-cursor-clean-room-matrix-driver/matrix-cap-plan.live.json` —
  `blocked_claim=0`, `serialize_cap_when_unlocked=0`, authority disposition
  `hold_component`, `cap_cli_status=bound`
- Cap-ready wake refresh: `goal-wake.json`, `cap-ready/INDEX.md`,
  `w2-h2-product-binding/harness.json` (`preferred_next_after_unlock_c=true`)

Tracked: this plan.

## 4. Verification

- Branch `codex/rust` @ `f6df9e01` (Unlock C Cap HEAD)
- `unlock-probe`: A=false B=false C=true; authority Cap CLI=`bound`
- `matrix-plan`: authority require_full → `hold_component` (no Cap serialize)
- Cap burn this turn: **false**; product: **0**
- Detection freeze intact

## 5. Residual / next

Root authorize **B** (hosted gateway H2 product-binding + N06 + `UNLOCK-B-APPLIED`)
preferred, else **A** (G4 CEF/AT path release + `UNLOCK-A-APPLIED`). Then Cap=1
matching Cap-ready pack once. Cap free now.
