---
id: plan:root/20261108233000-post-unlock-c-matrix-honesty
title: Post-Unlock-C matrix honesty + Unlock B Cap (component EXIT 0)
status: active
created: 2026-11-08T14:30:00Z
updated: 2026-11-08T18:05:00Z
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

Unlock **A**: Cap realloc Desktop baseline + host-only cleanup + Cap=1
`--g4-cef-at` EXIT **0** component executed + cleanup_passed (evidence
`20261109020800`); Cap free=**false**; BrowserReady=**false**; product=**0**.

## 1. Preference probe (fail-closed)

| # | Closer | Result |
| --- | --- | --- |
| 1 | Unlock B / G1 W2 H2 product-binding | **Cap EXIT 0 Desktop measure** @ `4a5d0135` — `original_h2_measured=true` from validated TLS listen+client receipt; `counts_as=pass_component_desktop_measure`; product=0; **not** hosted / **not** N06 join |
| 2 | N06 readiness / owner join | **open (Cap=0)** — hosted Cap EXIT0 component @ run **42500919761** + Unlock A Cap EXIT0 @ `8f8a128d` bound into Cursor `--n06-owner-join-readiness` → readiness (`n06_owner_join=false`; Cap free=false; BrowserReady=false); Root auth handoff absent; no invent join / BrowserReady |
| 3 | Unlock A / G4 CEF/AT | **Cap realloc + Cap=1 EXIT0** — Desktop baseline + host-only Cap cleanup; Cap CLI `--g4-cef-at` with `HUMANYMOUS_CEF_PILOT_BUNDLE` + `HUMANYMOUS_DOCKER_DESKTOP_BASELINE`; `g4_cef_at` **executed**; cleanup_passed (evidence `20261109020800`); Cap free=**false**; BrowserReady=false; product=0; no invent CEF binary / `pass_product` |

## 2. Decision rules

R1. **Cap burn only with structural unlock/delta.** Unlock B Desktop measure Cap
burned once @ `4a5d0135`. Hosted venue Cap burned once EXIT0 @ **42500919761**.
Unlock A Cap burned (missing CEF) then Cap **realloc** after host bundle
discovery EXIT1 @ `20261109020200`, then Cap **realloc** after Desktop
baseline + host-only cleanup EXIT0 @ `20261109020800`. Authority
`--require-full` reburn forbidden. Desktop / hosted Cap reburn forbidden
without new structural delta. Further A Cap reburn needs Root interactive AT.
N06 owner-join Cap=0 without Root auth handoff.

R2. **Component ≠ product.** Cap EXIT 0 + matrix `pass_component_desktop` /
`pass_component_desktop_measure` / `pass_component_hosted_measure` ≠
`pass_product_clean_room`. Staged `g4_cef_at` executed ≠ product / BrowserReady.

R3. **No invent CEF binary / readiness / hosted / Root tokens.** Unlock A apply
used claimable scope paths only; host-discovered private CEF tree is not an
invented binary. Do not invent readiness `original_h2:true` product green,
`desktop_as_hosted`, or Root ownership / auth handoff tokens.

R4. **Detection freeze.** No scoring/threshold/hard-rule edits.

R5. **No idle CONSOLIDATED-ROOT-ASK re-push.** Prefer Root interactive AT or
N06 auth handoff (A Cap spent EXIT0).

## 3. Surfaces (private evidence + this plan)

Private (`.spec.local/`, not Git):

- `…/20261109004100-cursor-w2-h2-measure-topology/` — Desktop measure Cap EXIT 0
- `…/20261109003000-cursor-n06-original-h2-measure/cap-ready-hosted-venue/` — N06 GRANT+apply
- `…/20261109010000-cursor-n06-hosted-venue-apply/` — Cap=0 Desktop refuse receipt
- `…/20261109005000-cursor-n06-owner-join/` — Cap=0 owner-join closer + Root ask
- `…/20261109013400-cursor-n06-hosted-venue-cap-realloc/` — Cap EXIT0 run 42500919761
- `…/20261109020000-cursor-unlock-a-cap-ready/` — Unlock A Cap=1 EXIT1 missing env
- `…/20261109020200-cursor-unlock-a-cef-bundle-realloc/` — CEF found + Cap realloc Cap=1 EXIT1
- `…/20261109020800-cursor-unlock-a-desktop-baseline-realloc/` — Desktop baseline Cap=1 EXIT0
- `…/20261108164500-cursor-clean-room-matrix/matrix.json` — cell honesty remap
- Cap-ready g4 pack: `…/cap-ready/g4-cef-at/` (**UNLOCK-A-APPLIED**; Cap spent)

Tracked: this plan + `20261109020000-unlock-a-cap1-fail-closed.plan.md` + prior
gateway/lab-runner Cap surfaces.

## 4. Verification

- Branch `codex/rust` @ `b60f809d`+
- Cap `--w2-h2-product-binding` Desktop measure: EXIT **0** (prior; no reburn)
- Cap `--w2-h2-hosted-venue` Specfold: EXIT **0** run **42500919761** (component)
- Cap `--n06-owner-join-readiness`: Cap=**0** (Root auth absent; readiness only)
- Cap `--g4-cef-at`: Cap realloc + Cap=**1** EXIT **0** component executed + cleanup_passed
- Cap free: **false**; BrowserReady=**false**; `desktop_as_hosted=false`; `n06_owner_join=false`; product: **0**
- Detection freeze intact; no authority / Desktop W2 / hosted Cap reburn

## 5. Residual / next

Unlock A Cap **spent** (free=false) after Desktop-baseline Cap burn EXIT0 @
`8f8a128d`. Matrix A/B/C remapped earned-only component cells; M3/product
unavailable; BrowserReady remains false (interactive AT Root-gated; no non-Cap
structural invent). N06 owner join Cap=0 binds hosted Cap EXIT0 + Unlock A
EXIT0 status until Root auth. Cap=0 `--n06-owner-auth-handoff-honesty` binds
those receipts + prior N06 readiness closer and refuse-closes without inventing
`n06_owner_join=true`. Cap=0 G2/G5 honesty gates land Desktop collector +
custody/witness components without inventing product / authenticated transport.
Cap=0 BrowserReady honesty refuse-closes without inventing AT.
Matrix-plan after A/B/C: `serialize_cap_when_unlocked=0`. No
hosted/W2/authority/A Cap reburn without structural delta. Cap free=**false**.

**Decision @ `e38ce6c5` tip audit: `hard_stop_root_ask` for AUTH/AT.** Cap=0
honesty CLIs remain closed. Cap=0 **G2 Cap CLI Cap-ready structural feeder**
landed @ tip `8a129ab7`+ (Cap CLI `--g2-collector-product-binding` scaffold +
Cap-ready topology pack; Cap realloc **eligible** not executed; Cap free still
**false**; Cap burn=0). Cap=1 forbidden while Cap free=false without Cap
realloc + live Cap topology TRANSFER. Do not duplicate CONSOLIDATED-ROOT-ASK.

Cap=0 CONSOLIDATED-ROOT-ASK (supersedes Cap-free-era packet):
`.spec.local/root/evidence/20261109025000-cursor-cap0-hard-stop-root-ask/CONSOLIDATED-ROOT-ASK.md`

Exact Root closer (mutually sufficient):

1. Place `AUTH-HANDOFF.json` schema `humanymous.root-n06-owner-join-auth-handoff/1`
   with `n06_owner_join_authorized=true`, `product=0`, `pass_product_clean_room=0`
   at `.spec.local/root/evidence/root-n06-owner-join-auth-handoff/` (or grants twin);
   release gateway readiness/application paths only if Cursor applies measured join.
2. Else place `AT-HANDOFF.json` schema `humanymous.root-interactive-at-windows-tuple/1`
   with `browser_ready_authorized=true`, `product=0`, `pass_product_clean_room=0`
   at `.spec.local/root/evidence/root-interactive-at-windows-tuple/` (or grants twin);
   release `crates/browser/**` / `scripts/ci/cef-pilot/**` / `crates/apps/humanymous-browser/**`
   only if Cursor applies measured BrowserReady.

Bound receipts: tip `e54c51e6` (hard-stop) / honesty tip `e38ce6c5`; hosted Cap
**42500919761**; Unlock A Cap EXIT0; BrowserReady honesty `34493ff0`; G2/G5
honesty `14268686`. Tip-reconfirm receipt:
`.spec.local/root/evidence/20261109025000-cursor-cap0-hard-stop-root-ask/HARD-STOP-RECEIPT.json`
(matrix tip-bound; AUTH/AT still absent). G2 Cap-ready feeder:
`.spec.local/root/evidence/20261109030000-cursor-g2-cap-ready-feeder/` + plan
`20261109030000-g2-collector-product-binding-cap-ready.plan.md`.

Never invent `pass_product_clean_room`, CEF binary, `desktop_as_hosted`,
BrowserReady, or Root ownership tokens. Capture `observe_h2` still forbidden as
product-binding. Evidence:
`.spec.local/root/evidence/20261109022100-cursor-cap0-g2-g5-honesty/`,
`.spec.local/root/evidence/20261109023000-cursor-cap0-browser-ready-honesty/`,
`.spec.local/root/evidence/20261109024500-cursor-cap0-n06-owner-auth-honesty/`,
`.spec.local/root/evidence/20261109030000-cursor-g2-cap-ready-feeder/`,
and
`.spec.local/root/evidence/20261109025000-cursor-cap0-hard-stop-root-ask/`.
