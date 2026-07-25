# `.agents/` — multi-provider agent canon (v4)

Vendor-neutral source of truth for Claude Code, Grok Build, OpenAI Codex, and Gemini CLI.

## Architecture (research-backed)

| Layer | What | Loading |
|-------|------|---------|
| **L0 Hooks** | `hooks/` + `scripts/agents/hooks/` | Deterministic PreToolUse guard |
| **L1 Rules** | root + nested `AGENTS.md`, `rules/` | Always-on; root stays **thin** (router) |
| **L2 Skills** | `skills/*/SKILL.md` + `assets/` | Progressive disclosure ([Agent Skills](https://agentskills.io)) |
| **L3 Personas** | `personas/` | Multi-perspective panels |
| **L4 Workflows** | `workflows/` + `scripts/agents/workflow-runner.mjs` | Local state machine, approvals, retries, JSONL journal |
| **Evals** | `evals/trigger-{queries,rules}.json` | Executable no-LLM intent-routing proxy |
| **Lessons** | `lessons/HARD-WON.md` | Cross-provider durable lessons (Claude memory is not enough) |
| **Sessions** | `sessions/` | Multi-provider overlap: lanes, protocol, live ACTIVE board |

### Why nested AGENTS.md

Monorepo / large-repo guidance: root = org-wide constraints + **router**; subdirectory AGENTS.md = local conventions without restating inheritance. Agents load nearer files with higher precedence.

### Why skill descriptions matter

Only `name` + `description` load at discovery (~100 tokens). Description must use **imperative “Use when…”**, user intent, and stay **≤1024 characters**. See skill `optimize-skill-description` and [optimizing descriptions](https://agentskills.io/skill-creation/optimizing-descriptions).

### Why hooks

Prompts are soft. L0 hooks block high-risk shell patterns (rm -rf /, force-push main, non-local offensive scanners). Grok requires `/hooks-trust`; Codex requires trusting the project-local hook with `/hooks`.

### Why the workflow runner is local

The runner records phases, time budgets, retries, approvals, artifacts, and an append-only JSONL journal under the gitignored `.agent-runs/` directory. It never imports an LLM SDK or calls a provider API; agents remain interactive clients that execute the recorded phase.

## Commands

```powershell
pwsh -File scripts/agents/sync-adapters.ps1
pwsh -File scripts/agents/verify-agents-layout.ps1
```

```bash
bash scripts/agents/sync-adapters.sh
bash scripts/agents/verify-agents-layout.sh
node scripts/agents/workflow-runner.mjs validate
node scripts/agents/eval-skill-triggers.mjs
node scripts/agents/workflow-runner.mjs start --workflow feature-loop --objective "..." --provider codex
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
| `red-blue-wargame-round` | Sequential red→Docker→blue wargame series (rule `61`) |
| `pass-wargame-round` | Pass challenge round (SoT-36 only) |
| `docs-from-sot` | Diátaxis user docs |
| `cut-release` | SemVer release prepare/publish (user-invoked only) |
| `github-actions-ops` | CI/release.yml failures + workflow authoring |
| `review-changes` | Freeze / security review |
| `handover-pack` | Next LLM/human brief |
| `optimize-skill-description` | Trigger description tuning |
| `survey-provider-history` | Inventory Claude/Grok/Codex/Gemini sessions |
| `coordinate-sessions` | Claim/release work lanes when sessions overlap |
