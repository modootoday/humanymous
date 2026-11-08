---
id: plan:root/20261108233000-post-unlock-c-matrix-honesty
title: Post-Unlock-C matrix honesty + Unlock B Cap (component EXIT 0)
status: active
created: 2026-11-08T14:30:00Z
updated: 2026-11-08T15:35:00Z
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
(evidence `20261109001300`). `product=0`. `original_h2_measured=false`.
Never invent `pass_product_clean_room`.

## 1. Preference probe (fail-closed)

| # | Closer | Result |
| --- | --- | --- |
| 1 | Unlock B / G1 W2 H2 product-binding | **Cap EXIT 0 component** — allowlist cites `deployments/lab/gateway/h2-product-binding-contract.json`; prior EXIT 1 @ `20261109000700`; reburn justified by structural allowlist delta; `counts_as=pass_component_desktop`; product=0 |
| 2 | N06 readiness join | **blocked (diagnosed)** — Cap EXIT 0 left `original_h2_measured=false` because composition Cap is `network_mode=none` oneshot (binary+contract only); Desktop≠hosted; missing Root-hosted measure join cited in contract + lab-runner refuse; Cap=0 this turn (no invent measure) |
| 3 | Unlock A / G4 CEF/AT | **blocked / Cap CLI refuse-closed** @ `eebcf64d` — Cap CLI `--g4-cef-at` code-defined; no `UNLOCK-A-APPLIED`; exact missing: path release `crates/browser/**` and/or `scripts/ci/cef-pilot/**`, Cap CLI GRANT pilot, `HUMANYMOUS_CEF_PILOT_BUNDLE`; Cap burn=0 |

## 2. Decision rules

R1. **Cap burn only with structural unlock/delta.** Unlock B Cap burned once for
allowlist delta. Authority `--require-full` reburn forbidden. `--w2-h2-product-binding`
reburn forbidden without new structural delta.

R2. **Component ≠ product.** Cap EXIT 0 + matrix `pass_component_desktop` ≠
`pass_product_clean_room`.

R3. **No invent Unlock A / readiness.** Do not forge `UNLOCK-A-APPLIED` or invent
readiness `original_h2:true` / CEF path release.

R4. **Detection freeze.** No scoring/threshold/hard-rule edits.

R5. **No idle CONSOLIDATED-ROOT-ASK re-push.** Cap free; prefer Unlock A or
measured hosted H2 for N06.

## 3. Surfaces (private evidence + this plan)

Private (`.spec.local/`, not Git):

- `…/20261109001300-cursor-w2-h2-product-binding-cap1/` — Cap EXIT 0 receipt
- `…/20261109001600-cursor-post-b-closer/` — N06 blocked + Unlock A prep
- `…/20261108164500-cursor-clean-room-matrix/matrix.json` — cell
  `desktop-linux|none|w2-h2-product-binding|lab_runner_require_full`:
  `unrun` → `pass_component_desktop` (earned); M3 stays `required_unavailable`
- Cap-ready g4 pack: `…/cap-ready/g4-cef-at/` (waiting Root)

Tracked: this plan.

## 4. Verification

- Branch `codex/rust` @ `64def57a` (allowlist cites gateway contract)
- Cap `--w2-h2-product-binding`: EXIT **0**; 6/0 cases; Desktop baseline match
- `unlock-probe`: A=false B=true; w2 Cap CLI=`bound`; authority=`blocked_claim`
- `original_h2_measured=false`; `pass_product_clean_room=0`; product: **0**
- Detection freeze intact; no authority Cap reburn

## 5. Residual / next

N06 diagnose (this turn): Cap=0; strengthened refuse cites
`missing_for_original_h2_measured` (contract + lab-runner). Cap free.
Unlock A Cap CLI refuse-closed @ `eebcf64d`. Next Root-gated closer:

1. Root-hosted gateway H2 **measure** topology + pilot (exact join in
   `deployments/lab/gateway/h2-product-binding-contract.json`
   `missing_for_original_h2_measured`) → then Cap=1 once with that structural
   delta → `original_h2_measured` only if measured — **or**
2. Unlock **A** GRANT + exact path release (`crates/browser/**` and/or
   `scripts/ci/cef-pilot/**`) + Cap CLI `--g4-cef-at` pilot +
   `HUMANYMOUS_CEF_PILOT_BUNDLE` then Cap=1 matching g4 pack

No W2/authority Cap reburn without new structural delta.
Never invent `pass_product_clean_room`. Capture `observe_h2` still forbidden as
product-binding.
