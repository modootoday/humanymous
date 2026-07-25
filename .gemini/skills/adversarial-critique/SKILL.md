---
name: adversarial-critique
description: >
  Use when the user wants an adversarial or completeness critique of a design,
  SoT, plan, or PR: find weakest assumptions, contradictions, and ship-blockers.
  Prefer after design-panel synthesis and before implement-sot-slice.
when_to_use: >
  "adversarial critique", "completeness critique", "find holes in this design",
  "pressure-test the SoT", "what did we miss".
---

# Adversarial critique

## Goal

**Break the design.** Do not rewrite product code unless asked.

## Axes

Trust boundaries · fail-open/closed · admin authN/dual-control · erasure/re-id ·
topology (CDN re-TLS) · a11y · over-claim / unmeasured guarantees.

## Steps

1. Read draft + claimed code paths.
2. Rank **P0 ship-blocker** · **P1** · **P2 residual**.
3. Evidence + local-only attack sketch + required SoT delta per finding.
4. Verdict: `ready` | `needs-work` | `block`.

## Output

Use structure in `assets/critique-report.md`. Fold P0/P1 into SoT before implement unless residual is explicit.
