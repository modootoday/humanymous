# Provider matrix (what loads where)

| Asset | Claude Code | Grok Build | OpenAI Codex | Gemini CLI |
|-------|-------------|------------|--------------|------------|
| `AGENTS.md` | via `CLAUDE.md` stub / compat | native | native | via `.gemini/settings.json` |
| Nested `**/AGENTS.md` | yes (when path touched) | yes (root→cwd) | yes | context-dependent |
| `.agents/skills` | after sync → `.claude/skills` | native + sync | after sync → `.codex/skills` | native `.agents/skills` |
| Hooks | `.claude/settings.json` | `.grok/hooks/` + Claude compat | limited | limited |
| Personas | `.claude/agents` (synced) | load as files / subagents | load via skill | load via skill |
| Lessons | `.claude/lessons` (synced) + `.agents/lessons` | `.agents/lessons` | `.agents/lessons` | `.agents/lessons` |
| Approval UX | workspace trust | `/hooks-trust` for hooks | project `.codex/config.toml`: `approval_policy=never` + `sandbox_mode=danger-full-access` (trusted project) | project settings |

Edit canon under `.agents/` + nested AGENTS; run `scripts/agents/sync-adapters`.

**Claude-only memory is not portable.** After a Claude-heavy session, promote durable
rules into `.agents/lessons/HARD-WON.md` (skill `survey-provider-history`).
