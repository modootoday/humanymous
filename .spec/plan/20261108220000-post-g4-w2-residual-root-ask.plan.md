---
id: plan:root/20261108220000-post-g4-w2-residual-root-ask
title: Post-G4/W2 residual inventory — consolidated ROOT-ASK (Cap free)
status: active
created: 2026-11-08T13:01:00Z
updated: 2026-11-08T14:13:17Z
rel:
  depends_on:
    - plan:root/20261108215600-g4-at-cef-blocked-claim
    - plan:root/20261108213000-g2-collector-provenance
    - plan:root/20261108210000-r14-h2-serving-consumer
    - plan:root/20261108221000-clean-room-matrix-cap-driver
    - sot:root/program-roadmap
    - sot:root/pass-protocol
  governs: []
---

# Post-G4/W2 residual inventory — Cap free / wait Root

Brief: Tip `bb2516af`+ — Unlock C TRANSFER applied (user GRANT + phrases cited);
Cap=1 `--authority-node-ipc` EXIT 0 component green; product=0;
`pass_product_clean_room` not invented. Detection freeze. G1/G5/G6 still open.
A/B unlocks remain available for other Cap burns. Cap conserved until next Root
allocation.

Private packet (update in place):  
`.spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/CONSOLIDATED-ROOT-ASK.md`

Unlock C candidate TRANSFER:  
`.spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/authority-node-ipc/candidate-pack/TRANSFER.md`

## 1. Problem

No Cursor-executable product Cap remains without a Root path release. Cap=1 may
burn only on a structural delta with honest fail-closed receipts. Cap-spam of
`--require-full` is forbidden when Cap is merely free.

## 2. Decision rules

R-D1. **All closers Root-blocked** until unlock A, B, or C.

R-D2. **No Cap spam.** Cap is free and conserved.

R-D3. **Matrix honesty hold.** Cells stay `required_unavailable` /
`pass_component_*` / `blocked_claim`. Never invent `pass_product_clean_room`.

R-D4. **Specfold out of scope.** require-go cover already
`pass_component_hosted` (run 42476157423).

R-D5. **Wake on any one unlock** with Cap free and no Cap-spam reburn.

R-D6. **Candidates stay private** until Root allocates and transfers matching
preimages. Cursor does not write live Root-owned released paths without allocation.

## 3. Three mutually sufficient unlocks

| Unlock | Need | Exact paths / note | Cap-ready |
| --- | --- | --- | --- |
| **A** G4 CEF/AT | Interactive CEF/GUI/IME/AT; a11y non-motor | Release `crates/browser/**`, `scripts/ci/cef-pilot/**` (or admit `browser-ui` execution) | `cap-ready/g4-cef-at/` |
| **B** W2 H2 binding | Hosted gateway H2 product-binding + N06 | Root-named gateway/readiness paths; not capture `observe_h2` alone | `cap-ready/w2-h2-product-binding/` |
| **C** Authority Cap | Cap CLI execute path (today `blocked_claim`) | `deployments/lab/INTEGRATION.md`, `deployments/lab/authority/**`, `build/rust/authority-lab.Dockerfile` (+ dockerignore), `test/lab/runner/src/snapshot.rs`, `test/lab/runner/src/orchestration.rs` (+ `orchestration/authority.rs`), optional `adapters/node/src/**`, optional `crates/authority/**` | `cap-ready/authority-node-ipc/` + **`candidate-pack/TRANSFER.md`** |

Success (shared): measured Cap receipts stay **fail-closed**; component ≠ product
cell; authority Cap green alone is not `pass_product_clean_room`.

## 4. Verification

- Private: `CONSOLIDATED-ROOT-ASK.md`, `goal-wake.json`, `cap-ready/` (incl.
  authority `candidate-pack/`), matrix-driver `COORDINATOR-PING.md`.
- Cap burn this turn: **false**. Cap free: **true**. unlock: **false**.
- Matrix: untouched. `product=0` / `pass_product_clean_room=0`.
- Unlock C candidate-pack authored with exact preimage hashes; not live-applied.

## 5. Residual / next gap

Root/Codex reviews Unlock C `candidate-pack/TRANSFER.md`, allocates **any one**
of A/B/C + exact path transfer / INTEGRATION amendment. Cursor Cap=1 then burns
the matching Cap-ready pack. Prefer Cap burn over more docs if A/B/C unlocks.
No architecture invention; detection freeze.

Follow-on Cap-ready: `plan:root/20261108220600-cap-ready-g4-w2-harness` +
authority pack under the same private `cap-ready/` tree. Matrix Cap planner:
`plan:root/20261108221000-clean-room-matrix-cap-driver` (serialize=0).
