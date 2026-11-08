---
id: plan:root/20261109031000-g5-witness-transport-cap-ready
title: G5 witness-transport Cap CLI Cap=0 scaffold + Cap-ready topology
status: active
created: 2026-11-08T18:20:00Z
updated: 2026-11-08T18:25:00Z
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

Brief: Cap free=**false** after Unlock A EXIT0. G2 Cap-ready + TRANSFER helper
already landed; G2 GRANT still Root-absent (wake). Prior Cap=0 G5 honesty bound
custody/witness component only (`authenticated_witness_transport=false`). This
plan lands the **structural next feeder twin** (not a G2 duplicate): Cap CLI
`--g5-witness-transport` Cap=0 scaffold + Cap-ready compose/Dockerfile/contract
TRANSFER pack + Cap=0 G5 transfer-probe/apply helper (Root one-step). Cap
realloc is **eligible** but **not executed** (live topology absent; scope=
`root_transfer_required`). Cap burn=**0**. `product=0`.
`authenticated_witness_transport` stays **false** until Cap measured. Never
invent `pass_product_clean_room`. Detection freeze.

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

## 4. Verification (this turn)

- Branch `codex/rust` @ `1f296c90`+
- Cap CLI Cap=0 scaffold EXIT**0**; Cap topology live=**false**
- Scope verdict: **root_transfer_required**
- Cap=0 G5 transfer-probe EXIT0 · preimage_ready=**true**
- Cap free before/after: **false** / **false**
- Cap realloc attempted: **false** (eligible only)
- Cap EXIT: **none** / Cap burn=**0**
- product: **0** · `pass_product_clean_room`: **0**
- `authenticated_witness_transport`: **false**
- AUTH/AT / G2 GRANT handoffs: **absent** (not forged)

## 5. Exact next closer

1. Prefer Root G2 `ROOT-ONE-STEP.md` GRANT→TRANSFER if Cap-conserving product
   binding is the chosen next Cap=1 (G2 pack already Cap-ready).
2. Else Root G5 `ROOT-ONE-STEP.md`: copy `GRANT.TEMPLATE.json` → `GRANT.json`,
   then lab-runner `transfer-apply` (schema
   `humanymous.cursor-g5-witness-transport-transfer-grant/1`) with matching
   candidateSha256 for Cap topology destinations.
3. Cap realloc receipt schema
   `humanymous.cursor-g5-witness-transport-cap-realloc/1`.
4. Cap=1 once: `cargo run --locked -p humanymous-lab-runner -- run --evidence
   BUNDLE --g5-witness-transport` (jobs=1).
5. Separately: Root AUTH-HANDOFF and/or AT-HANDOFF for N06/BrowserReady
   (unchanged blocked_on_root; do not forge).

Never invent `pass_product_clean_room` or `authenticated_witness_transport`.
