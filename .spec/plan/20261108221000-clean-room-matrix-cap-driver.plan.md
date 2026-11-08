---
id: plan:root/20261108221000-clean-room-matrix-cap-driver
title: Lab-runner clean-room matrix Cap planner (fail-closed; no Cap burn)
status: active
created: 2026-11-08T13:15:00Z
updated: 2026-11-08T14:13:17Z
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
`pass_product_clean_room`. Authority Cap CLI recorded as `blocked_claim` with exact
Root path ask (INTEGRATION forbids runner `pilot.mjs`).

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

Cap=1 (2026-11-08T13:20:03Z): sole prior serialize cell
(`authority-node-ipc|lab_runner_require_full`) was a **false positive** — no Cap
CLI binding; bare `--require-full` would Cap-spam wrong composition. Cap burn=0.
Planner now refuses unbound scenarios (serialize=0). Matrix cell stays `unrun`.

Follow-on Spec honesty (2026-11-08T13:30:00Z): `authority-node-ipc` Cap CLI status
is **`blocked_claim`** (selector recognized; never Cap-serialize). Fail-closed
`--authority-node-ipc` refuses before Docker. Exact Root path ask:

- `deployments/lab/INTEGRATION.md` (amend runner-never-executes, or authorize Cap exception)
- `deployments/lab/authority/**` (Rust Cap pilot replacement for `pilot.mjs`)
- `build/rust/authority-lab.Dockerfile` (+ `.dockerignore`)
- `test/lab/runner/src/snapshot.rs` freeze allowlist expansion
- `adapters/node/src/**` (build inputs if Node adapter retained in image)
- `crates/authority/**` if Rust IPC pilot lives there

Cap burn remains 0 until unlock **A** G4, **B** W2, or **C** authority Cap paths.
Never invent `pass_product_clean_room`. Canonical Root packet:
`.spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/CONSOLIDATED-ROOT-ASK.md`
(+ `cap-ready/authority-node-ipc/`).

Unlock C **applied** 2026-11-08T14:16Z+ (user GRANT + autonomy phrases); Cap=1
`--authority-node-ipc` EXIT 0 component receipt under private evidence
`20261108232200-cursor-authority-node-ipc-cap1/`. product=0; never invent
`pass_product_clean_room`. Matrix may now serialize authority Cap when bound;
Cap conserved until next Root allocation.

Root allocates A/B/C → Cursor Cap=1 burns matching `cap-ready/` pack; matrix-plan
then guides subsequent Cap serialization without Cap spam.
