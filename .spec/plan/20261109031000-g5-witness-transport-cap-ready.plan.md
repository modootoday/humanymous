---
id: plan:root/20261109031000-g5-witness-transport-cap-ready
title: G5 witness-transport Cap CLI Cap=0 scaffold + Cap-ready topology
status: active
created: 2026-11-08T18:20:00Z
updated: 2026-11-08T19:05:00Z
rel:
  depends_on:
    - plan:root/20261109030000-g2-collector-product-binding-cap-ready
    - plan:root/20261108233000-post-unlock-c-matrix-honesty
    - plan:root/20261109020000-unlock-a-cap1-fail-closed
    - sot:root/program-roadmap
    - sot:root/structural-development
  governs: []
---

# G5 Cap CLI Cap=0 scaffold + Cap-ready topology feeder

Brief: Cap free=**false** after Unlock A EXIT0. Cap=0 Cap CLI
`--g5-witness-transport` + Cap-ready TRANSFER pack landed first. Standing
autonomy Cap-ready GRANT apply + transfer-apply placed live Cap topology; Cap
realloc + Cap=1 EXIT0 landed (component Cap topology only;
`authenticated_witness_transport=false`; `pass_product_clean_room=0`). Cap free
after Cap=1=**false**. Detection freeze.

Private evidence:
`.spec.local/root/evidence/20261109031000-cursor-g5-cap-ready-feeder/`

## 1. Problem

G5 Cap CLI+topology was absent while Cap free=false and G2 GRANT still
Root-blocked. Cap realloc requires a real structural feeder. Cap=1 without live
topology would refuse and waste Cap. Live topology destinations are not
Cursor-claimable / not in released integration scope → Cursor must not place
them without Root GRANT. Root AUTH/AT handoffs remain separately required for
N06/BrowserReady — not forged here; CONSOLIDATED-ROOT-ASK not duplicated.

## 2. Decision rules

R-G5-1. **Cap=0 Cap CLI scaffold first.** Cap CLI refuse-closed Cap=0 when Cap
free=false without Cap realloc **or** Cap topology missing.

R-G5-2. **Candidates out of live paths** until GRANT TRANSFER matching
candidateSha256 (`deployments/lab/**` Cap topology + `build/rust/g5-*`).
Scope verdict: `root_transfer_required` (not `cursor_can_place_topology`).

R-G5-3. **Cap realloc only after live topology.** Schema
`humanymous.cursor-g5-witness-transport-cap-realloc/1`. Cap free restore is one
Cap=1 authorization only — never invent Cap free without realloc receipt.
Topology TRANSFER apply alone does not invent Cap free or authenticated
transport.

R-G5-4. **Component ≠ product.** Custody/witness Cap1 /
`pass_component_*` ≠ G5 closed ≠ `pass_product_clean_room` ≠ authenticated
witness transport.

R-G5-5. **No A/B/C/G2 Cap spam.** No AUTH/AT invent. Detection freeze. Do not
duplicate G2 GRANT/ROOT-ONE-STEP.

R-G5-6. **Cap=0 TRANSFER helper when Root TRANSFER required.** Lab-runner
`transfer-probe` / `transfer-apply` UnlockLetter::G5 + `ROOT-ONE-STEP.md` so
Root can complete apply in one documented step (mirror G2 pattern).

R-G5-7. **Bind Cap=0 honesty.** Keep `--g5-witness-transport-honesty` receipts
and `authenticated_witness_transport=false` until Cap measured.

## 3. Surfaces

Tracked:

- Cap CLI: `test/lab/runner/src/orchestration/g5_witness_transport.rs`
- Freeze allowlist Cap=0: `test/lab/runner/src/snapshot.rs`
- Transfer helper Cap=0: `test/lab/runner/src/transfer_applicator.rs` (G5)
- This plan + honesty mirrors on post-unlock / Unlock A / G2 plans

Private Cap-ready:

- `…/cap-ready/g5-witness-transport/candidate-pack/TRANSFER.md`
- `…/ROOT-ONE-STEP.md` · `transfer-manifest.json` · `GRANT.TEMPLATE.json`
- `…/TRANSFER-PROBE.json` (`g5_witness_transport_preimage_ready=true`)
- `…/CAP-REALLOC-ELIGIBLE.json` · `SCOPE-VERDICT.json`

## 4. Verification (landed)

- Branch `codex/rust` @ `6ba329ad`+
- Cap-ready GRANT apply (standing autonomy): `GRANT.TEMPLATE` → `GRANT.json`
  schema `humanymous.cursor-g5-witness-transport-transfer-grant/1`
- transfer-apply UnlockLetter::G5 **applied=true** · live Cap topology present
- Cap realloc schema `humanymous.cursor-g5-witness-transport-cap-realloc/1`
  executed (live topology then Desktop baseline structural delta after EXIT1)
- Cap free before / after realloc / after Cap=1: **false** / **true** / **false**
- Cap=1 once jobs=1: EXIT **0** passed=7 cleanup_passed
  (prior EXIT1 Desktop baseline missing → Cap realloc; no Cap spam)
- product: **0** · `pass_product_clean_room`: **0** · `g5_closed`: **false**
- `authenticated_witness_transport`: **false**
- counts_as: `pass_component_desktop_cap_topology` only
- AUTH/AT handoffs: **absent** (not forged)
- Evidence: `.spec.local/root/evidence/20261109035200-cursor-g5-desktop-baseline-realloc/`

## 5. Exact next closer

1. **Done this wave:** Cap=1 component bound into clean-room matrix as
   `pass_component_desktop_cap_topology` (still product=0;
   `authenticated_witness_transport=false`).
2. Separately: Root AUTH-HANDOFF and/or AT-HANDOFF for N06/BrowserReady
   (blocked_on_root shrunk to AUTH/AT only; do not forge).

Never invent `pass_product_clean_room` or `authenticated_witness_transport`.
