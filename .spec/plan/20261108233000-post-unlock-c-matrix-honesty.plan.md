---
id: plan:root/20261108233000-post-unlock-c-matrix-honesty
title: Post-Unlock-C matrix honesty + Unlock B Cap (component EXIT 0)
status: active
created: 2026-11-08T14:30:00Z
updated: 2026-11-08T16:58:00Z
rel:
  depends_on:
    - plan:root/20261108230100-bottom-up-inner-outer-parallelization
    - plan:root/20261108221000-clean-room-matrix-cap-driver
    - plan:root/20261108220600-cap-ready-g4-w2-harness
    - sot:root/program-roadmap
    - sot:root/structural-development
  governs: []
---

# Post-Unlock-C matrix honesty + Unlock B Cap

Brief: Unlock C Cap=1 `--authority-node-ipc` EXIT 0 component @ `f6df9e01`
landed earlier. Unlock **B** Cap topology + Cap=1 `--w2-h2-product-binding`
reburn after freeze-allowlist structural delta @ `64def57a` EXIT **0** component
(evidence `20261109001300`). Measure topology Cap @ `4a5d0135` EXIT **0** with
`original_h2_measured=true` (Desktop lab receipt; evidence `20261109004100`).
**Measured Desktop lab ≠ hosted product.** N06 hosted Cap Specfold EXIT **0**
run **42500919761** @ `d483c40f` = `pass_component_hosted_measure` only
(≠ product / ≠ N06 join). Owner-join Cap=0 without Root auth. `product=0`.
Never invent `pass_product_clean_room` / `desktop_as_hosted`.

## 1. Preference probe (fail-closed)

| # | Closer | Result |
| --- | --- | --- |
| 1 | Unlock B / G1 W2 H2 product-binding | **Cap EXIT 0 Desktop measure** @ `4a5d0135` — `original_h2_measured=true` from validated TLS listen+client receipt; `counts_as=pass_component_desktop_measure`; product=0; **not** hosted / **not** N06 join |
| 2 | N06 readiness / owner join | **open (Cap=0)** — hosted Cap EXIT0 component @ run **42500919761**; Cursor `--n06-owner-join-readiness` binds receipt → readiness (`n06_owner_join=false`); Root auth handoff absent; no invent join |
| 3 | Unlock A / G4 CEF/AT | **bound + Cap=1 EXIT1** — GRANT+apply claimable `scripts/ci/cef-pilot/**`; `UNLOCK-A-APPLIED`; Cap CLI `--g4-cef-at` bound; Cap burn=1 EXIT **1** `fail_closed_missing_cef_bundle` (evidence `20261109020000`); no CEF binary invented; Cap **spent** |

## 2. Decision rules

R1. **Cap burn only with structural unlock/delta.** Unlock B Desktop measure Cap
burned once @ `4a5d0135`. Hosted venue Cap burned once EXIT0 @ **42500919761**.
Unlock A Cap burned once EXIT1 @ evidence `20261109020000` (missing CEF
bundle). Authority `--require-full` reburn forbidden. Desktop / hosted / A Cap
reburn forbidden without new structural delta (A needs fresh CEF bundle).
N06 owner-join Cap=0 without Root auth handoff.

R2. **Component ≠ product.** Cap EXIT 0 + matrix `pass_component_desktop` /
`pass_component_desktop_measure` / `pass_component_hosted_measure` ≠
`pass_product_clean_room`.

R3. **No invent CEF binary / readiness / hosted / Root tokens.** Unlock A apply
used claimable scope paths only; do not invent `HUMANYMOUS_CEF_PILOT_BUNDLE`,
readiness `original_h2:true` product green, `desktop_as_hosted`, or Root
ownership / auth handoff tokens.

R4. **Detection freeze.** No scoring/threshold/hard-rule edits.

R5. **No idle CONSOLIDATED-ROOT-ASK re-push.** Prefer Root CEF bundle for A
reburn (structural delta) or N06 auth handoff.

## 3. Surfaces (private evidence + this plan)

Private (`.spec.local/`, not Git):

- `…/20261109004100-cursor-w2-h2-measure-topology/` — Desktop measure Cap EXIT 0
- `…/20261109003000-cursor-n06-original-h2-measure/cap-ready-hosted-venue/` — N06 GRANT+apply
- `…/20261109010000-cursor-n06-hosted-venue-apply/` — Cap=0 Desktop refuse receipt
- `…/20261109005000-cursor-n06-owner-join/` — Cap=0 owner-join closer + Root ask
- `…/20261109013400-cursor-n06-hosted-venue-cap-realloc/` — Cap EXIT0 run 42500919761
- `…/20261109020000-cursor-unlock-a-cap-ready/` — Unlock A Cap=1 EXIT1 receipts
- `…/20261108164500-cursor-clean-room-matrix/matrix.json` — cell honesty remap
- Cap-ready g4 pack: `…/cap-ready/g4-cef-at/` (**UNLOCK-A-APPLIED**; Cap spent)

Tracked: this plan + `20261109020000-unlock-a-cap1-fail-closed.plan.md` + prior
gateway/lab-runner Cap surfaces.

## 4. Verification

- Branch `codex/rust` @ `72d0f8ad`+ (Unlock A Cap burn receipts private)
- Cap `--w2-h2-product-binding` Desktop measure: EXIT **0** (prior; no reburn)
- Cap `--w2-h2-hosted-venue` Specfold: EXIT **0** run **42500919761** (component)
- Cap `--n06-owner-join-readiness`: Cap=**0** (Root auth absent; readiness only)
- Cap `--g4-cef-at`: Cap=**1** EXIT **1** missing `HUMANYMOUS_CEF_PILOT_BUNDLE`
- `desktop_as_hosted=false`; `n06_owner_join=false`; product: **0**
- Detection freeze intact; no authority / Desktop W2 / hosted Cap reburn

## 5. Residual / next

Unlock A Cap **spent** (EXIT1 refuse-closed missing CEF bundle). N06 owner join
Cap=0 until Root auth. No hosted/W2/authority Cap reburn without structural delta.

Next Root-gated closer:

1. Root: provision `HUMANYMOUS_CEF_PILOT_BUNDLE` (fresh private CEF) + optional
   interactive AT Windows tuple — Cap A reburn only with that structural delta
2. Else Root N06 owner-integration auth handoff + exact path release
   (`configuration.rs` / `service/readiness.rs` / `application.rs`)

Never invent `pass_product_clean_room`, CEF binary, `desktop_as_hosted`, or Root
ownership tokens. Capture `observe_h2` still forbidden as product-binding.
