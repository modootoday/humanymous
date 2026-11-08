---
id: plan:root/20261109030000-g2-collector-product-binding-cap-ready
title: G2 collector product-binding Cap CLI Cap=0 scaffold + Cap-ready topology
status: active
created: 2026-11-08T18:00:00Z
updated: 2026-11-08T18:25:00Z
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

Brief: Cap free=**false** after Unlock A EXIT0. Prior Cap=0 G2 readiness bound
Desktop collector component only (`no_g2_cap_cli_with_topology`). This plan lands
the **structural next feeder**: Cap CLI `--g2-collector-product-binding` Cap=0
scaffold + Cap-ready compose/Dockerfile/contract TRANSFER pack + Cap=0 G2
transfer-probe/apply helper (Root one-step). Cap realloc is **eligible** but
**not executed** (live topology absent; scope=`root_transfer_required`). Cap
burn=**0**. `product=0`. Never invent `pass_product_clean_room`. Detection freeze.

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

## 4. Verification (this turn)

- Branch `codex/rust` @ `76819bb4`+
- Cap CLI Cap=0 scaffold bound; Cap topology live=**false**
- Scope verdict: **root_transfer_required**
- Cap=0 G2 transfer-probe EXIT0 · preimage_ready=**true**
- Cap free before/after: **false** / **false**
- Cap realloc attempted: **false** (eligible only)
- Cap EXIT: **none** / Cap burn=**0**
- product: **0** · `pass_product_clean_room`: **0**
- AUTH/AT handoffs: **absent** (not forged)

## 5. Exact next closer

1. Root one-step `ROOT-ONE-STEP.md`: copy `GRANT.TEMPLATE.json` → `GRANT.json`,
   then lab-runner `transfer-apply` (schema
   `humanymous.cursor-g2-collector-product-binding-transfer-grant/1`) with matching
   candidateSha256 for Cap topology destinations.
2. Cap realloc receipt schema
   `humanymous.cursor-g2-collector-product-binding-cap-realloc/1`.
3. Cap=1 once: `cargo run --locked -p humanymous-lab-runner -- run --evidence
   BUNDLE --g2-collector-product-binding` (jobs=1).
4. Separately: Root AUTH-HANDOFF and/or AT-HANDOFF for N06/BrowserReady (unchanged
   blocked_on_root; do not forge).
5. Cap=0 G5 Cap CLI Cap-ready twin landed separately (plan
   `20261109031000-g5-witness-transport-cap-ready`); Cap free still false;
   product=0; do not Cap=1 either feeder without Cap realloc + live topology.

Never invent `pass_product_clean_room`.
