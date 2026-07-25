# CLAUDE.md (adapter)

Follow **[AGENTS.md](./AGENTS.md)** as the project rules file. Nested `**/AGENTS.md` apply by directory.

Do not fork instructions here — edit root/nested `AGENTS.md` and `.agents/**` only.

After clone:

```bash
bash scripts/agents/sync-adapters.sh
# or: pwsh -File scripts/agents/sync-adapters.ps1
```

This creates `.claude/skills`, `.claude/rules`, `.claude/agents`, and `.claude/settings.json` (PreToolUse guard). Project hooks require workspace trust.
