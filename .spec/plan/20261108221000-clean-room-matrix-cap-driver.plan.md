---
id: plan:root/20261108221000-clean-room-matrix-cap-driver
title: Lab-runner clean-room matrix Cap planner (fail-closed; no Cap burn)
status: active
created: 2026-11-08T13:15:00Z
updated: 2026-11-08T13:15:00Z
rel:
  depends_on:
    - plan:root/20261108220600-cap-ready-g4-w2-harness
    - plan:root/20261108220000-post-g4-w2-residual-root-ask
    - sot:root/program-roadmap
    - sot:root/container-test-environments
  governs: []
---

# Lab-runner clean-room matrix Cap planner

Brief: While G4/W2 unlock remains Root-blocked and Cap is free, Cursor lands a
**dry-run** lab-runner `matrix-plan` driver that Cap=1 can later serialize. Fail-closed
SoT honesty for matrix cells. Detection freeze. No Cap burn. product=0. Never invent
`pass_product_clean_room`.

## 1. Problem

Cap-ready G4/W2 packs at `534d88c3` shorten unlock→Cap for two closers, but the full
clean-room product matrix still needs a Cap-serializable planner that:

- refuses Cap for `required_unavailable` / blocked cells,
- holds component/fixture statuses without `--require-full` Cap spam,
- emits exact lab-runner command preimages only for required `unrun` lab-runner gates,
- never invents product green.

## 2. Decision rules

R-M1. **No Cap burn this turn.** Planner is dry-run (`cap_execute=false`).

R-M2. **Fail-closed honesty.** `required_unavailable` → `refuse_cap`. Component /
fixture → `hold_component`. Never emit invented `pass_product_clean_room`.

R-M3. **Serialize only.** `unrun` + required `lab_runner_*` →
`serialize_cap_when_unlocked` with command preimage; Cap=1 later executes after Root
unlock — not now.

R-M4. **No idle ROOT-ASK re-push.** Wake ask unchanged: G4 CEF/AT path release **or**
W2 hosted H2 product-binding. Board protocol ping records Cap free + planner ready.

## 3. Surface

- `test/lab/runner/src/clean_room_matrix.rs`
- CLI: `humanymous-lab-runner matrix-plan --matrix PATH [--emit PATH]`
- Private receipts:
  `.spec.local/root/evidence/20261108301100-cursor-clean-room-matrix-driver/`

## 4. Verification

- `cargo test --locked -j1 -p humanymous-lab-runner clean_room_matrix`
- Optional dry-run against claimed matrix evidence (read-only); emit plan receipt
- Cap burn=0; Docker Cap unused; product=0

## 5. Residual / next gap

Root allocates G4 or W2 unlock → Cursor Cap=1 burns matching `cap-ready/` pack;
matrix-plan then guides subsequent Cap serialization without Cap spam.
