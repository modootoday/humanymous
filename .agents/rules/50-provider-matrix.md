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
| Commit Co-Authored-By | `Claude <noreply@anthropic.com>` (product default + avatar) | `Grok <grok@x.ai>` (community de-facto; **no avatar** until xAI links email) | `codex <codex@openai.com>` (avatar) / `Codex <noreply@openai.com>` | no vendor default; placeholder until Google ships |

**Attribution emails:** Claude/Codex are GitHub-linked for profile photos. Grok follows community convention `grok@x.ai` (unlinked as of 2026-07). Official org [`xai-org`](https://github.com/xai-org) — **not** third-party `github.com/grok` / `github.com/xai`. Full registry: `.agents/sessions/COMMIT-CONVENTION.md`.

Edit canon under `.agents/` + nested AGENTS; run `scripts/agents/sync-adapters`.

**Claude-only memory is not portable.** After a Claude-heavy session, promote durable
rules into `.agents/lessons/HARD-WON.md` (skill `survey-provider-history`).
