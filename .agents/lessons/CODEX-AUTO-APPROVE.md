# Codex auto-approve (this repo)

## Applied settings

| Scope | File | Keys |
|-------|------|------|
| Project (committed) | `.codex/config.toml` | `approval_policy = "never"`, `sandbox_mode = "danger-full-access"` |
| User (machine-local) | `~/.codex/config.toml` | `approval_policy = "never"` (global default) |
| Trust | `~/.codex/config.toml` `[projects.'\\?\D:\workspace\@ai-education\automation-blocking-skills']` | `trust_level = "trusted"` |

Project config is loaded **only when the project is trusted**.

## CLI one-shot (if UI still prompts)

```bash
codex -a never --sandbox danger-full-access
# or
codex -c 'approval_policy="never"' -c 'sandbox_mode="danger-full-access"'
```

## If approvals still appear

1. Restart Codex CLI / IDE extension after config change.
2. Confirm the workspace path matches the trusted project key (Windows `\\?\D:\...` vs `D:\...`).
3. Known issue: some IDE builds intermittently ignore `approval_policy=never` for shell — use CLI flags above or update Codex.
4. Do not commit secrets; auto-approve increases blast radius — keep defense-only e2e (Docker lab).
