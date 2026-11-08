---
id: plan:root/20261109030000-g2-collector-product-binding-cap-ready
title: G2 collector product-binding Cap CLI Cap=0 scaffold + Cap-ready topology
status: active
created: 2026-11-08T18:00:00Z
updated: 2026-11-08T18:05:00Z
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
scaffold + Cap-ready compose/Dockerfile/contract TRANSFER pack. Cap realloc is
**eligible** but **not executed** (live topology absent). Cap burn=**0**.
`product=0`. Never invent `pass_product_clean_room`. Detection freeze.

Private evidence:
`.spec.local/root/evidence/20261109030000-cursor-g2-cap-ready-feeder/`

## 1. Problem

G2 product-binding Cap CLI+topology was absent while Cap free=false. Cap realloc
requires a real structural feeder. Cap=1 without live topology would refuse and
waste Cap. Root AUTH/AT handoffs remain separately required for N06/BrowserReady
— not forged here; CONSOLIDATED-ROOT-ASK not duplicated.

## 2. Decision rules

R-G2-1. **Cap=0 Cap CLI scaffold first.** Cap CLI refuse-closed Cap=0 when Cap
free=false without Cap realloc **or** Cap topology missing.

R-G2-2. **Candidates out of live paths** until GRANT TRANSFER matching
candidateSha256 (`deployments/lab/**` Cap topology + `build/rust/g2-*`).

R-G2-3. **Cap realloc only after live topology.** Schema
`humanymous.cursor-g2-collector-product-binding-cap-realloc/1`. Cap free restore
is one Cap=1 authorization only — never invent Cap free without realloc receipt.

R-G2-4. **Component ≠ product.** Desktop collector Cap1 /
`pass_component_desktop` ≠ G2 closed ≠ `pass_product_clean_room`.

R-G2-5. **No A/B/C Cap spam.** No AUTH/AT invent. Detection freeze.

## 3. Surfaces

Tracked:

- Cap CLI: `test/lab/runner/src/orchestration/g2_collector_product_binding.rs`
- Freeze allowlist Cap=0: `test/lab/runner/src/snapshot.rs`
- This plan + honesty mirrors on post-unlock / Unlock A plans

Private Cap-ready:

- `…/cap-ready/g2-collector-product-binding/candidate-pack/TRANSFER.md`
- `…/transfer-manifest.json` · `GRANT.TEMPLATE.json`
- `…/CAP-REALLOC-ELIGIBLE.json`

## 4. Verification (this turn)

- Branch `codex/rust` @ `8a129ab7`+
- Cap CLI Cap=0 scaffold bound; Cap topology live=**false**
- Cap free before/after: **false** / **false**
- Cap realloc attempted: **false** (eligible only)
- Cap EXIT: **none** / Cap burn=**0**
- product: **0** · `pass_product_clean_room`: **0**

## 5. Exact next closer

1. Root TRANSFER-apply GRANT schema
   `humanymous.cursor-g2-collector-product-binding-transfer-grant/1` with matching
   candidateSha256 for Cap topology destinations.
2. Cap realloc receipt schema
   `humanymous.cursor-g2-collector-product-binding-cap-realloc/1`.
3. Cap=1 once: `cargo run --locked -p humanymous-lab-runner -- run --evidence
   BUNDLE --g2-collector-product-binding` (jobs=1).
4. Separately: Root AUTH-HANDOFF and/or AT-HANDOFF for N06/BrowserReady (unchanged
   blocked_on_root; do not forge).

Never invent `pass_product_clean_room`.
