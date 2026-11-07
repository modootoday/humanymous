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
5. Run the Rust maintenance `check` and `routing` commands. Regex routing is a static proxy, not model activation.
6. Discover modoo.today's `codex-plugin-eval` without filtering away unverified-runtime results; inspect the complete plugin/shared dependencies and use its isolated harness only. Do not invent missing shared files or use --isolation none.
7. Define at least two positive cases and one near miss before measuring. Run with/without arms two times each, score behavior in both arms, and record skill discovery/read separately from claimed skill name. Use a bounded token/call/time budget with the existing authorized login; do not expose credentials.
8. A blocked skill read, missing harness dependency, tool error or partial budget stop is infrastructure/partial, not a successful model quality comparison. Preserve traces privately and sanitized results in public Specfold evidence. An independent CLI pilot is never a substitute for the named plugin harness.
9. Fix a missed trigger in the description before the body. Do not tune wording to compensate for a sandbox defect, weaken graders or report improvement without a complete paired delta. Stop after two fix rounds.

## Output

Before/after description + which queries expected to flip.
