---
name: optimize-skill-description
description: >
  Use when improving a skill's description so it triggers on the right prompts:
  description tuning, trigger evals, false-positive skill activation, or agentskills
  description optimization. Not for writing product code.
when_to_use: >
  "skill doesn't trigger", "optimize description", "trigger eval", "description too broad".
---

# Optimize skill description

Follow [agentskills.io optimizing descriptions](https://agentskills.io/skill-creation/optimizing-descriptions):

1. Load eval queries: `.agents/evals/trigger-queries.json` (or skill-local `evals/`).
2. Principles:
   - Imperative: "Use when…"
   - User intent, not implementation
   - Pushy on relevant contexts; ≤1024 characters
3. Train (~60%) vs validation (~40%); do not overfit keywords from failed queries.
4. Revise `description` (and optional `when_to_use`) only.
5. Re-run static check: `scripts/agents/verify-agents-layout` (description length).

## Output

Before/after description + which queries expected to flip.
