---
id: plan:root/20261108233000-post-unlock-c-matrix-honesty
title: Post-Unlock-C matrix honesty + Unlock B Cap (component EXIT 0)
status: active
created: 2026-11-08T14:30:00Z
updated: 2026-11-08T15:55:00Z
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
**Measured Desktop lab ≠ hosted product.** N06 owner join still open.
`product=0`. Never invent `pass_product_clean_room` / `desktop_as_hosted`.

## 1. Preference probe (fail-closed)

| # | Closer | Result |
| --- | --- | --- |
| 1 | Unlock B / G1 W2 H2 product-binding | **Cap EXIT 0 Desktop measure** @ `4a5d0135` — `original_h2_measured=true` from validated TLS listen+client receipt; `counts_as=pass_component_desktop_measure`; product=0; **not** hosted / **not** N06 join |
| 2 | N06 readiness / owner join | **open (Cap=0)** — Desktop measure landed; hosted venue Cap topology absent; `actual_hosted_linux=false` on Desktop; Cap-ready hosted-venue pack cites exact destinations; no invent join |
| 3 | Unlock A / G4 CEF/AT | **blocked / Cap CLI refuse-closed** @ `eebcf64d` — Cap CLI `--g4-cef-at` code-defined; no `UNLOCK-A-APPLIED`; exact missing: path release `crates/browser/**` and/or `scripts/ci/cef-pilot/**`, Cap CLI GRANT pilot, `HUMANYMOUS_CEF_PILOT_BUNDLE`; Cap burn=0 |

## 2. Decision rules

R1. **Cap burn only with structural unlock/delta.** Unlock B Desktop measure Cap
burned once @ `4a5d0135`. Authority `--require-full` reburn forbidden.
`--w2-h2-product-binding` Desktop measure reburn forbidden without new structural
delta. Hosted venue Cap=1 only after hosted topology transfer + `actual_hosted_linux`.

R2. **Component ≠ product.** Cap EXIT 0 + matrix `pass_component_desktop` /
`pass_component_desktop_measure` ≠ `pass_product_clean_room`.

R3. **No invent Unlock A / readiness / hosted.** Do not forge `UNLOCK-A-APPLIED`,
invent readiness `original_h2:true` product green, or claim `desktop_as_hosted`.

R4. **Detection freeze.** No scoring/threshold/hard-rule edits.

R5. **No idle CONSOLIDATED-ROOT-ASK re-push.** Prefer Root hosted venue join Cap
or Unlock A GRANT/path release.

## 3. Surfaces (private evidence + this plan)

Private (`.spec.local/`, not Git):

- `…/20261109004100-cursor-w2-h2-measure-topology/` — Desktop measure Cap EXIT 0
- `…/20261109003000-cursor-n06-original-h2-measure/cap-ready-hosted-venue/` — N06 Cap-ready
- `…/20261109005000-cursor-n06-owner-join/` — Cap=0 owner-join closer
- `…/20261108164500-cursor-clean-room-matrix/matrix.json` — cell honesty remap
- Cap-ready g4 pack: `…/cap-ready/g4-cef-at/` (waiting Root)

Tracked: this plan + gateway contract/docs + lab-runner N06 hooks.

## 4. Verification

- Branch `codex/rust` @ `4a5d0135`+ (Desktop measure topology)
- Cap `--w2-h2-product-binding` measure: EXIT **0**; `original_h2_measured=true`
- `desktop_as_hosted=false`; N06 owner join **still open**
- Hosted venue Cap topology **absent** → Cap=0 this turn
- `pass_product_clean_room=0`; product: **0**
- Detection freeze intact; no authority Cap reburn; no Unlock A invent

## 5. Residual / next

N06 owner-integration (this turn): Cap=0; lab-runner fail-closed gates +
receipt schema + Cap-ready hosted venue pack landed under claim. Cap free.

Next Root-gated closer:

1. Root transfer hosted Cap topology
   (`deployments/lab/compose.h2-hosted-venue.yaml` +
   `build/rust/h2-hosted-venue-lab.Dockerfile`) + Specfold/`actual_hosted_linux`
   Cap=1 once with fail-closed hosted measure → then owner-integration readiness
   join (SoT N06 / R14-D6) — **or**
2. Unlock **A** GRANT + exact path release (`crates/browser/**` and/or
   `scripts/ci/cef-pilot/**`) + Cap CLI `--g4-cef-at` pilot +
   `HUMANYMOUS_CEF_PILOT_BUNDLE` then Cap=1 matching g4 pack

No W2 Desktop measure / authority Cap reburn without new structural delta.
Never invent `pass_product_clean_room` or `desktop_as_hosted`. Capture
`observe_h2` still forbidden as product-binding.
