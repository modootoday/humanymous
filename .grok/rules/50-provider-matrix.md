# Provider matrix (what loads where)

| Asset | Claude Code | Grok Build | OpenAI Codex | Gemini CLI |
|-------|-------------|------------|--------------|------------|
| `AGENTS.md` | via `CLAUDE.md` stub / compat | native | native | via `.gemini/settings.json` |
| Nested `**/AGENTS.md` | yes (when path touched) | yes (root→cwd) | yes | context-dependent |
| `.agents/skills` | after sync → `.claude/skills` | native + sync | after sync → `.codex/skills` | native `.agents/skills` |
| Hooks | `.claude/settings.json` | `.grok/hooks/` + Claude compat | `.codex/hooks.json` (trusted project) | limited |
| Personas | `.claude/agents` (synced) | load as files / subagents | load via skill | load via skill |
| Lessons | `.claude/lessons` (synced) + `.agents/lessons` | `.agents/lessons` | `.agents/lessons` | `.agents/lessons` |
| Approval UX | workspace trust | `/hooks-trust` for hooks | lower-risk Auto: `approval_policy=on-request` + `sandbox_mode=workspace-write` | project settings |
| Commit Co-Authored-By | `Claude <noreply@anthropic.com>` (product default) | `Grok Build <304785771+grokkybara[bot]@…>` (via `xai-org` / `grokkybara[bot]`; no product default) | `codex <codex@openai.com>` (avatar) / `Codex <noreply@openai.com>` | no vendor default; placeholder until Google ships |

**GitHub channels (avatar source of truth):** Claude product email; Codex `codex@openai.com`; Grok official org [`xai-org`](https://github.com/xai-org) + monorepo bot `grokkybara[bot]` — **not** third-party users `github.com/grok` or `github.com/xai`. Full registry: `.agents/sessions/COMMIT-CONVENTION.md`.

Edit canon under `.agents/` + nested AGENTS; run `scripts/agents/sync-adapters`.

**Claude-only memory is not portable.** After a Claude-heavy session, promote durable
rules into `.agents/lessons/HARD-WON.md` (skill `survey-provider-history`).
