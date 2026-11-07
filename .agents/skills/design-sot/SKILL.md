---
name: design-sot
description: >
  Use when writing or extending a SoT or plan before code: new specification,
  design panel, multi-perspective or spec-first design. Prefer over implementing
  when the user asks for design/SoT (even without saying "SoT").
---

# Design SoT

## Goal

Normative SoT/plan slice: vocabulary, decision rules, non-coverage — **before** code.

## Steps

1. Brief (problem, constraints, non-goals ≤15 lines).
2. Run `spec context <path>` and read current `.spec/sot`/`.spec/plan` plus historical oracle only where needed; preserve L1–L7 vocabulary.
3. Panel: load `.agents/agents/*` (blue, red, sre-ops, compliance-dpo, evaluator). Do not spawn agents unless the task or workflow authorizes delegation.
4. Synthesize one draft (version, scope, schemas/ids, wiring, verification).
5. Hand off to skill `adversarial-critique` before implement.
6. Do not implement unless the user also asked for code.

## Progressive disclosure

- Output template: read `assets/sot-outline.md` when drafting structure.
- Workflow: `.agents/workflows/design-panel.yaml`
- Method: `docs/explanation/how-this-was-built.md` if SoT tree missing.

## Gotchas

- User-facing docs are **not** SoT dumps (use `docs-from-sot` later).
- Critique findings must become requirements ("critique reflected"), not meeting notes.
- New implementation/tests are Rust 1.99.1/edition 2024. Define source-bound unit, actual pilot and Docker product gates separately; no foreign templates or script-string workaround.
