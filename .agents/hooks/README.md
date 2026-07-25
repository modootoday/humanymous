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

Successful PreToolUse checks must exit `0` without writing to stdout. Codex treats
JSON-looking stdout as a structured response and fails the hook when that output
is malformed or does not match its schema.

Project `.codex/config.toml` excludes `GH_TOKEN` and `GITHUB_TOKEN` from spawned
commands. Authenticate local GitHub CLI sessions through the OS keyring instead
of persistent token environment variables.

The guard appends secret-safe execution metadata to
`.agent-runs/hooks/pre-tool-guard.jsonl` for live diagnosis. It records byte
counts and SHA-256 hashes, never raw commands, payloads, or environment values.

## Live Codex hook watch

`watch-codex-sqlite.mjs` opens Codex's `logs_2.sqlite` read-only and polls new
hook rows every 250 ms. It records event metadata and body hashes only; raw log
bodies, commands, payloads, and environment values are not copied.

```powershell
$db = Join-Path $env:USERPROFILE '.codex\logs_2.sqlite'
$out = '.agent-runs\hooks\codex-sqlite-live.jsonl'
$pidFile = '.agent-runs\hooks\codex-sqlite-live.pid'
Start-Process node -WindowStyle Hidden -ArgumentList @(
  'scripts/agents/hooks/watch-codex-sqlite.mjs',
  '--db', $db,
  '--out', $out,
  '--pid-file', $pidFile
)

# Live view; no timeout wait is required.
Get-Content $out -Tail 20 -Wait

# Stop the background watcher.
$watchPid = [int](Get-Content -Raw $pidFile)
Stop-Process -Id $watchPid
```
