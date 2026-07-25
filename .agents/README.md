# `.agents/` — multi-provider agent canon (v2)

Vendor-neutral source of truth for Claude Code, Grok Build, OpenAI Codex, and Gemini CLI.

## Architecture (research-backed)

| Layer | What | Loading |
|-------|------|---------|
| **L0 Hooks** | `hooks/` + `scripts/agents/hooks/` | Deterministic PreToolUse guard |
| **L1 Rules** | root + nested `AGENTS.md`, `rules/` | Always-on; root stays **thin** (router) |
| **L2 Skills** | `skills/*/SKILL.md` + `assets/` | Progressive disclosure ([Agent Skills](https://agentskills.io)) |
| **L3 Personas** | `personas/` | Multi-perspective panels |
| **L4 Workflows** | `workflows/` | Tool-agnostic loops |
| **Evals** | `evals/trigger-queries.json` | Description trigger quality |
| **Lessons** | `lessons/HARD-WON.md` | Cross-provider durable lessons (Claude memory is not enough) |

### Why nested AGENTS.md

Monorepo / large-repo guidance: root = org-wide constraints + **router**; subdirectory AGENTS.md = local conventions without restating inheritance. Agents load nearer files with higher precedence.

### Why skill descriptions matter

Only `name` + `description` load at discovery (~100 tokens). Description must use **imperative “Use when…”**, user intent, and stay **≤1024 characters**. See skill `optimize-skill-description` and [optimizing descriptions](https://agentskills.io/skill-creation/optimizing-descriptions).

### Why hooks

Prompts are soft. L0 hooks block high-risk shell patterns (rm -rf /, force-push main, non-local offensive scanners). Grok requires `/hooks-trust` once per project.

## Commands

```powershell
pwsh -File scripts/agents/sync-adapters.ps1
pwsh -File scripts/agents/verify-agents-layout.ps1
```

```bash
bash scripts/agents/sync-adapters.sh
bash scripts/agents/verify-agents-layout.sh
```

## Edit policy

1. Edit **only** `.agents/**`, root/nested `AGENTS.md`, `CLAUDE.md`/`GEMINI.md` stubs if needed.
2. Re-run **sync** after skill/rule/persona/hook changes.
3. Never hand-edit generated `.claude/skills`, `.grok/skills`, etc.

## Skills catalog

| Skill | Purpose |
|-------|---------|
| `design-sot` | Spec-first SoT / design panel |
| `adversarial-critique` | Completeness / weak assumptions |
| `implement-sot-slice` | Vertical implement + tests |
| `red-blue-validate` | Full defensive gate (**Docker e2e only**) |
| `pass-wargame-round` | Pass challenge round |
| `docs-from-sot` | Diátaxis user docs |
| `cut-release` | SemVer release (manual) |
| `review-changes` | Freeze / security review |
| `handover-pack` | Next LLM/human brief |
| `optimize-skill-description` | Trigger description tuning |
| `survey-provider-history` | Inventory Claude/Grok/Codex/Gemini sessions |
