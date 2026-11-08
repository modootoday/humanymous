---
id: plan:root/20261109020000-unlock-a-cap1-fail-closed
title: Unlock A G4 CEF/AT Cap=1 (missing CEF then realloc with host bundle + Desktop baseline)
status: active
created: 2026-11-08T16:58:00Z
updated: 2026-11-08T17:50:00Z
rel:
  depends_on:
    - plan:root/20261108233000-post-unlock-c-matrix-honesty
    - plan:root/20261108220600-cap-ready-g4-w2-harness
    - plan:root/20261108215600-g4-at-cef-blocked-claim
    - sot:root/program-roadmap
  governs: []
---

# Unlock A Cap=1 — CEF missing then host-bundle + Desktop-baseline realloc

Brief: Unlock **A** GRANT+transfer-apply bound on claimable
`scripts/ci/cef-pilot/**`. Cap CLI `--g4-cef-at` **bound**.

1. Cap=1 @ `72d0f8ad`/`0026c293` EXIT **1** `fail_closed_missing_cef_bundle`
   — env unset; no CEF binary invented (evidence `20261109020000`).
2. Host search found complete private CEF runtime (CI artifacts/caches empty).
   Cap **realloc** (structural: bundle available) + Cap=1 once with
   `HUMANYMOUS_CEF_PILOT_BUNDLE` set → `g4_cef_at` component **executed**;
   process EXIT **1** `component_executed_cleanup_failed` (Desktop baseline
   env unset). Cap **free=false**. Evidence `20261109020200`.
3. Cap **realloc** (structural: Desktop baseline + host-only Cap cleanup skip
   compose) + Cap=1 once with `HUMANYMOUS_CEF_PILOT_BUNDLE` +
   `HUMANYMOUS_DOCKER_DESKTOP_BASELINE`; `CARGO_BUILD_JOBS=1` → EXIT **0**
   component executed + cleanup_passed. Cap **free=false**. `product=0`.
   BrowserReady=**false**. Detection freeze. No N06 / W2 / authority Cap
   reburn. Evidence `20261109020800`.

Private evidence (gitignored `.spec.local/`):

- `…/20261109020000-cursor-unlock-a-cap-ready/` — first Cap EXIT1 missing env
- `…/20261109020200-cursor-unlock-a-cef-bundle-realloc/` — search, realloc,
  Cap1 receipts (component executed; cleanup_failed)
- `…/20261109020800-cursor-unlock-a-desktop-baseline-realloc/` — Desktop
  baseline realloc Cap1 EXIT0
- `…/cap-ready/g4-cef-at/` — GRANT.json, UNLOCK-A-APPLIED, TRANSFER-APPLY

## Decision rules

R-A1. **Claimable path release ≠ CEF binary.** Scope
`scripts/ci/cef-pilot/**` authorizes identity Cap-ready candidates + GRANT
apply. Cap day still refuses without `HUMANYMOUS_CEF_PILOT_BUNDLE`.

R-A2. **Cap=1 once after Unlock A bound; realloc only with structural delta.**
Host-discovered complete private CEF tree is structural for Cap realloc.
Desktop baseline + host-only Cap cleanup (no compose down) is structural for
cleanup EXIT0. Further Cap reburn needs new delta (Root interactive AT).

R-A3. **Component ≠ product.** `g4_cef_at` executed ≠ `BrowserReady` ≠
`pass_product_clean_room`. Cap EXIT 0 does not invent product.

R-A4. **No N06/W2/authority Cap reburn** without new structural delta.

## Verification

- Branch `codex/rust` @ `b60f809d`+
- Cap `--g4-cef-at` Desktop-baseline realloc Cap=1: EXIT **0** component
  executed + cleanup_passed
- `HUMANYMOUS_CEF_PILOT_BUNDLE` + `HUMANYMOUS_DOCKER_DESKTOP_BASELINE` present
- Cap free: **false**; product: **0**; BrowserReady: **false**

## Next

Cap=0 honesty landed: matrix A/B/C earned-only remap; N06 readiness binds
Unlock A Cap EXIT0 closer + hosted Cap EXIT0 (join still open). Cap=0
`--n06-owner-auth-handoff-honesty` EXIT0 binds hosted Cap + Unlock A + N06
readiness receipts (`n06_owner_join=false`; Root owner-auth absent
refuse-closed). Cap=0 G2 collector product-binding readiness + G5 witness
transport honesty gates landed (component bound; product still open). Cap=0
BrowserReady / interactive AT honesty gate landed
(`--browser-ready-interactive-at-honesty` EXIT0; Unlock A g4 component bound;
`browser_ready=false`; Root AT handoff absent). Matrix serialize after A/B/C =
**0** (hold components). Cap free=**false**. No Cap realloc (no Root owner-auth
/ interactive AT; no claimable Cap CLI+topology; no A/B/C Cap spam).

**Decision: `hard_stop_root_ask`.** Tip audit @ `e38ce6c5` found no residual
Cursor-claimable Cap=0 product-feeder. Cap=0 CONSOLIDATED-ROOT-ASK filed at
`.spec.local/root/evidence/20261109025000-cursor-cap0-hard-stop-root-ask/`.
Tip-reconfirm @ `e54c51e6`: AUTH/AT still absent; Cap free=false; short
`HARD-STOP-RECEIPT.json` only (no duplicate CONSOLIDATED-ROOT-ASK). Clean-room
matrix tip-bound to `e54c51e6` with Cap=0 honesty path binding. No Cap=1.
No more empty honesty CLIs. product=0.

1. Root places `AUTH-HANDOFF.json` schema
   `humanymous.root-n06-owner-join-auth-handoff/1`
   (`n06_owner_join_authorized=true`, `product=0`, `pass_product_clean_room=0`)
2. Else Root places `AT-HANDOFF.json` schema
   `humanymous.root-interactive-at-windows-tuple/1`
   (`browser_ready_authorized=true`, `product=0`, `pass_product_clean_room=0`)
