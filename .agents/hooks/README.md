# Hooks (L0 runtime policy)

Hooks enforce what prompts cannot reliably enforce. Project hooks require **folder trust**
in Grok (`/hooks-trust`) and workspace trust in Claude Code.

| Event | Purpose |
|-------|---------|
| PreToolUse (shell) | Block dangerous / third-party offensive patterns |
| SessionStart (optional) | Remind agent of layout verify |

## Canon scripts

- `scripts/agents/hooks/pre-tool-guard.mjs` — dependency-free Node guard shared by Claude, Grok, and Codex

## Vendor config (generated/synced)

- Claude: `.claude/settings.json` → hooks
- Grok: `.grok/hooks/project-safety.json`
- Codex: `.codex/hooks.json`

Do not put secrets in hooks. Prefer deny with a clear reason so the model can recover.
