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
2. Read `sots/00-overview.md` + related SoTs; do not break L1–L7 vocabulary.
3. Panel: load `.agents/personas/*` (blue, red, sre-ops, compliance-dpo, evaluator).
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
