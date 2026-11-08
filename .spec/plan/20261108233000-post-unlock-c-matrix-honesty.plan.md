---
id: plan:root/20261108233000-post-unlock-c-matrix-honesty
title: Post-Unlock-C matrix honesty + Unlock B Cap (component EXIT 0)
status: active
created: 2026-11-08T14:30:00Z
updated: 2026-11-08T16:50:00Z
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
| 2 | N06 readiness / owner join | **open (Cap=0)** — hosted Cap EXIT0 component @ run **42500919761**; Cursor `--n06-owner-join-readiness` binds receipt → readiness (`n06_owner_join=false`); Root auth handoff absent; Cap free after EXIT0; no invent join |
| 3 | Unlock A / G4 CEF/AT | **blocked / Cap CLI refuse-closed** @ `eebcf64d` — Cap CLI `--g4-cef-at` code-defined; no `UNLOCK-A-APPLIED`; exact missing: path release `crates/browser/**` and/or `scripts/ci/cef-pilot/**`, Cap CLI GRANT pilot, `HUMANYMOUS_CEF_PILOT_BUNDLE`; Cap burn=0; Cap free |

## 2. Decision rules

R1. **Cap burn only with structural unlock/delta.** Unlock B Desktop measure Cap
burned once @ `4a5d0135`. Hosted venue Cap burned once EXIT0 @ **42500919761**.
Authority `--require-full` reburn forbidden. Desktop / hosted Cap reburn
forbidden without new structural delta. Cap free after EXIT0 for Unlock A.
N06 owner-join Cap=0 without Root auth handoff.

R2. **Component ≠ product.** Cap EXIT 0 + matrix `pass_component_desktop` /
`pass_component_desktop_measure` / `pass_component_hosted_measure` ≠
`pass_product_clean_room`.

R3. **No invent Unlock A / readiness / hosted / Root tokens.** Do not forge
`UNLOCK-A-APPLIED`, invent readiness `original_h2:true` product green, claim
`desktop_as_hosted`, or invent Root ownership / auth handoff tokens.

R4. **Detection freeze.** No scoring/threshold/hard-rule edits.

R5. **No idle CONSOLIDATED-ROOT-ASK re-push.** Prefer Root N06 auth handoff or
Unlock A GRANT/path release (Cap free).

## 3. Surfaces (private evidence + this plan)

Private (`.spec.local/`, not Git):

- `…/20261109004100-cursor-w2-h2-measure-topology/` — Desktop measure Cap EXIT 0
- `…/20261109003000-cursor-n06-original-h2-measure/cap-ready-hosted-venue/` — N06 GRANT+apply
- `…/20261109010000-cursor-n06-hosted-venue-apply/` — Cap=0 Desktop refuse receipt
- `…/20261109005000-cursor-n06-owner-join/` — Cap=0 owner-join closer + Root ask
- `…/20261109013400-cursor-n06-hosted-venue-cap-realloc/` — Cap EXIT0 run 42500919761
- `…/20261108164500-cursor-clean-room-matrix/matrix.json` — cell honesty remap
- Cap-ready g4 pack: `…/cap-ready/g4-cef-at/` (waiting Root)

Tracked: this plan + gateway contract/docs + lab-runner N06 hosted Cap CLI +
owner-join readiness Cap CLI + `compose.h2-hosted-venue.yaml` / hosted
Dockerfile(+dockerignore).

## 4. Verification

- Branch `codex/rust` @ `d483c40f`+ (hosted Cap EXIT0 + N06 join readiness)
- Cap `--w2-h2-product-binding` Desktop measure: EXIT **0** (prior; no reburn)
- Cap `--w2-h2-hosted-venue` Specfold: EXIT **0** run **42500919761** (component)
- Cap `--n06-owner-join-readiness`: Cap=**0** (Root auth absent; readiness only)
- `desktop_as_hosted=false`; `n06_owner_join=false`; product: **0**
- Detection freeze intact; no authority / Desktop W2 / hosted Cap reburn; no Unlock A invent

## 5. Residual / next

Hosted Cap component green recorded; Cap **free** after EXIT0 (no reburn of
hosted venue without structural delta). N06 owner join Cap=0 until Root auth.

Next Root-gated closer:

1. Root N06 owner-integration auth handoff + exact path release
   (`configuration.rs` / `service/readiness.rs` / `application.rs`) — **or**
2. Unlock **A** GRANT + exact path release (`crates/browser/**` and/or
   `scripts/ci/cef-pilot/**`) + Cap CLI `--g4-cef-at` pilot +
   `HUMANYMOUS_CEF_PILOT_BUNDLE` then Cap=1 matching g4 pack

No W2 Desktop measure / hosted Cap / authority Cap reburn without new structural
delta. Never invent `pass_product_clean_room`, `desktop_as_hosted`, or Root
ownership tokens. Capture `observe_h2` still forbidden as product-binding.
