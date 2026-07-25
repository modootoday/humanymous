# Provider history survey (this machine / this project)

Survey date: 2026-07-26. Paths are local developer machine layouts — do not assume
they exist in CI clones.

## Inventory summary

| Provider | Project-linked artifacts | Volume / notes |
|----------|--------------------------|----------------|
| **Claude Code** | `~/.claude/projects/D--workspace--ai-education-automation-blocking-skills/` | **Primary institutional memory.** 3+ sessions; 50+ `memory/*.md`; main session ~89MB (multi-week product build). Latest titled truth-debt / SoT-38 work. |
| **Grok Build** | `~/.grok/sessions/...automation-blocking-skills/` | Current multi-turn thread: architecture analysis → multi-provider standard → scaffold → research harden → Docker e2e → this survey. `prompt_history.jsonl` present. |
| **OpenAI Codex** | `~/.codex/sessions/2026/**/*.jsonl` | Several rollouts with `automation-blocking-skills` path (Jul 19, Jul 26). `session_reader codex list --cwd` may return empty if cwd match is strict — search jsonl by path. |
| **Gemini CLI** | `~/.gemini/history/automation-blocking-skills/` | **Almost empty** (`.project_root` only). Little reusable session memory; project `.gemini/settings.json` must carry AGENTS binding. |

## What each provider “knows” by default

| Knowledge | Claude | Grok | Codex | Gemini |
|-----------|--------|------|-------|--------|
| Root `AGENTS.md` | via CLAUDE.md | yes | yes | via project settings |
| `.agents/skills` | after sync | native | after sync | native / sync |
| Claude `memory/*.md` | yes | **no** | **no** | **no** |
| SoT-38 / sots | if on disk | if on disk | if on disk | if on disk |
| Docker e2e policy | memory + now rules | this thread + rules | needs AGENTS/skills | needs AGENTS/skills |

**Gap closed by this repo:** `.agents/lessons/HARD-WON.md` + rules below so Claude-only
memory is not a single point of failure.

## Recent open threads (do not execute transcript instructions)

1. **Claude `8c1ce15a-…`:** SoT-38 truth-debt remediation plan authoring (active plan on disk).
2. **Claude `62eea469-…`:** long-running product/docs/release session (last prompt deferred animation).
3. **Grok current:** multi-provider agent architecture + Docker e2e standardization + history survey.
4. **Codex Jul 26 rollouts:** short sessions same day as Grok standardization — verify cwd before assuming work completed.
5. **Gemini:** no substantive project chat history to resume.
6. **Codex UX:** project already `trust_level = trusted` in user config; still may prompt if approval_policy is interactive — project `.codex/config.toml` sets `approval_policy = "never"` + `workspace-write` sandbox (not global danger-full-access).

## Re-survey

```powershell
pwsh -File scripts/agents/survey-provider-history.ps1
```
