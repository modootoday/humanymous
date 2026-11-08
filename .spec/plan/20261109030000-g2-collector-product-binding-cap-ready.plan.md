---
id: plan:root/20261109030000-g2-collector-product-binding-cap-ready
title: G2 collector product-binding Cap CLI Cap=0 scaffold + Cap-ready topology
status: active
created: 2026-11-08T18:00:00Z
updated: 2026-11-08T19:05:00Z
rel:
  depends_on:
    - plan:root/20261108233000-post-unlock-c-matrix-honesty
    - plan:root/20261109020000-unlock-a-cap1-fail-closed
    - plan:root/20261108213000-g2-collector-provenance
    - sot:root/program-roadmap
    - sot:root/structural-development
  governs: []
---

# G2 Cap CLI Cap=0 scaffold + Cap-ready topology feeder

Brief: Cap free=**false** after Unlock A EXIT0. Cap=0 Cap CLI
`--g2-collector-product-binding` + Cap-ready TRANSFER pack landed first. Standing
autonomy Cap-ready GRANT apply + transfer-apply placed live Cap topology; Cap
realloc + Cap=1 EXIT0 landed (component Cap topology only;
`pass_product_clean_room=0`). Cap free after Cap=1=**false**. Detection freeze.

Private evidence:
`.spec.local/root/evidence/20261109030000-cursor-g2-cap-ready-feeder/`

## 1. Problem

G2 product-binding Cap CLI+topology was absent while Cap free=false. Cap realloc
requires a real structural feeder. Cap=1 without live topology would refuse and
waste Cap. Live topology destinations are not Cursor-claimable / not in released
integration scope → Cursor must not place them without Root GRANT. Root AUTH/AT
handoffs remain separately required for N06/BrowserReady — not forged here;
CONSOLIDATED-ROOT-ASK not duplicated.

## 2. Decision rules

R-G2-1. **Cap=0 Cap CLI scaffold first.** Cap CLI refuse-closed Cap=0 when Cap
free=false without Cap realloc **or** Cap topology missing.

R-G2-2. **Candidates out of live paths** until GRANT TRANSFER matching
candidateSha256 (`deployments/lab/**` Cap topology + `build/rust/g2-*`).
Scope verdict: `root_transfer_required` (not `cursor_can_place_topology`).

R-G2-3. **Cap realloc only after live topology.** Schema
`humanymous.cursor-g2-collector-product-binding-cap-realloc/1`. Cap free restore
is one Cap=1 authorization only — never invent Cap free without realloc receipt.
Topology TRANSFER apply alone does not invent Cap free.

R-G2-4. **Component ≠ product.** Desktop collector Cap1 /
`pass_component_desktop` ≠ G2 closed ≠ `pass_product_clean_room`.

R-G2-5. **No A/B/C Cap spam.** No AUTH/AT invent. Detection freeze.

R-G2-6. **Cap=0 TRANSFER helper when Root TRANSFER required.** Lab-runner
`transfer-probe` / `transfer-apply` UnlockLetter::G2 + `ROOT-ONE-STEP.md` so Root
can complete apply in one documented step (like Unlock C). While G2 GRANT still
Root-absent, Cap=0 **G5 Cap CLI Cap-ready twin** may land in parallel (separate
pack/GRANT; see plan `20261109031000-g5-witness-transport-cap-ready`) — do not
duplicate G2 GRANT/TRANSFER.

## 3. Surfaces

Tracked:

- Cap CLI: `test/lab/runner/src/orchestration/g2_collector_product_binding.rs`
- Freeze allowlist Cap=0: `test/lab/runner/src/snapshot.rs`
- Transfer helper Cap=0: `test/lab/runner/src/transfer_applicator.rs` (G2)
- This plan + honesty mirrors on post-unlock / Unlock A plans

Private Cap-ready:

- `…/cap-ready/g2-collector-product-binding/candidate-pack/TRANSFER.md`
- `…/ROOT-ONE-STEP.md` · `transfer-manifest.json` · `GRANT.TEMPLATE.json`
- `…/TRANSFER-PROBE.json` (`g2_collector_product_binding_preimage_ready=true`)
- `…/CAP-REALLOC-ELIGIBLE.json` · `SCOPE-VERDICT.json`

## 4. Verification (landed)

- Branch `codex/rust` @ `ac2a4da8`+
- Cap-ready GRANT apply (standing autonomy): `GRANT.TEMPLATE` → `GRANT.json`
  schema `humanymous.cursor-g2-collector-product-binding-transfer-grant/1`
- transfer-apply UnlockLetter::G2 **applied=true** · live Cap topology present
- Cap realloc schema `humanymous.cursor-g2-collector-product-binding-cap-realloc/1`
  executed (live topology + compose-down cleanup structural delta)
- Cap free before / after realloc / after Cap=1: **false** / **true** / **false**
- Cap=1 once jobs=1: EXIT **0** passed=7 cleanup_passed
  (prior EXIT1 cleanup_failed → Cap realloc; no Cap spam)
- product: **0** · `pass_product_clean_room`: **0** · `g2_closed`: **false**
- counts_as: `pass_component_desktop_cap_topology` only
- AUTH/AT handoffs: **absent** (not forged)
- Evidence: `.spec.local/root/evidence/20261109034100-cursor-g2-cap-cleanup-realloc/`

## 5. Exact next closer

1. **Done this wave:** Cap=0 bind G2 Cap=1 component into clean-room matrix
   tip-bound to `6ba329ad` as `pass_component_desktop_cap_topology` (still
   product=0; evidence `20261109035000-cursor-g2-matrix-bind`).
2. **Done this wave (G5 twin):** Cap-ready GRANT apply + transfer + Cap realloc
   + Cap=1 EXIT0 (see plan `20261109031000-g5-witness-transport-cap-ready`).
3. Separately: Root AUTH-HANDOFF and/or AT-HANDOFF for N06/BrowserReady
   (blocked_on_root shrunk to AUTH/AT only; do not forge).

Never invent `pass_product_clean_room`.
