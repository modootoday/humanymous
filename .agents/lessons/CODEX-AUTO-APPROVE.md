# Codex auto-approve (this repo)

## Applied settings

| Scope | File | Keys |
|-------|------|------|
| Project (committed) | `.codex/config.toml` | `approval_policy = "on-request"`, `sandbox_mode = "workspace-write"` |
| Safety hook | `.codex/hooks.json` | Canonical shell guard from `.agents/hooks/codex-hooks.json` |
| Trust | `~/.codex/config.toml` `[projects.'\\?\D:\workspace\@ai-education\automation-blocking-skills']` | `trust_level = "trusted"` |

Project config is loaded **only when the project is trusted**.

## CLI one-shot (if UI still prompts)

```bash
codex -a on-request --sandbox workspace-write
```

## If approvals still appear

1. Restart Codex CLI / IDE extension after config change.
2. Confirm the workspace path matches the trusted project key (Windows `\\?\D:\...` vs `D:\...`).
3. Review and trust the project hook with `/hooks`; changed hooks are skipped until trusted.
4. Workspace work is automatic. Network, writes outside the workspace, and other sandbox escapes still ask.
