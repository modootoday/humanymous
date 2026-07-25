# Session coordination (multi-provider overlap)

When **Claude Code, Grok Build, Codex, and Gemini CLI** sessions run on this repo at
the same time (or hand off without a clean boundary), they race on files, commits,
and “done” claims. This directory is the **shared, provider-neutral board**.

## Files

| Path | Tracked? | Purpose |
|------|----------|---------|
| `LANES.md` | yes | Stable work-lane catalog (what may run in parallel) |
| `PROTOCOL.md` | yes | Start / claim / work / release rules |
| `GIT-PROTOCOL.md` | yes | Exclusive git writes (commit/push/rebase) |
| `ACTIVE.example.md` | yes | Template for the live board |
| `ACTIVE.md` | **no** (gitignored) | Live claims — every agent reads/writes this |
| `claims/*.json` | **no** (gitignored) | Machine-readable claim records |

## Quick start (any provider, session start)

```powershell
# 1) See who holds what
pwsh -File scripts/agents/session-board.ps1 list

# 2) Claim a lane before editing
pwsh -File scripts/agents/session-board.ps1 claim `
  -Lane docs `
  -Provider grok `
  -Goal "Update e2e docs for Docker-only path" `
  -Paths "docs/how-to/*,AGENTS.md"

# 3) Work only inside claimed paths (plus pure reads elsewhere)

# 4) Release when done (or stalled)
pwsh -File scripts/agents/session-board.ps1 release -Lane docs -Note "handover in ACTIVE.md"
```

Bash:

```bash
bash scripts/agents/session-board.sh list
bash scripts/agents/session-board.sh claim docs grok "Update e2e docs" "docs/how-to/*,AGENTS.md"
bash scripts/agents/session-board.sh release docs "done"
```

If `ACTIVE.md` is missing, the script seeds it from `ACTIVE.example.md`.

## Parallelism rule of thumb

- **Different work lanes** → OK to run in parallel (e.g. `docs` + `agents-meta`).
- **Same work lane** → one writer only; second session must wait, take another lane, or
  coordinate with the user.
- **`detection-core` and `gate-edge`** may both touch scoring seams — treat as
  **mutex with each other** unless paths are explicitly disjoint and listed.
- **All git writes** → exclusive **`git-ops`** lane (short hold). Use
  `scripts/agents/git-coord.ps1` preflight/claim/release.

```powershell
pwsh -File scripts/agents/git-coord.ps1 preflight
pwsh -File scripts/agents/git-coord.ps1 claim -Provider grok -Session "…"
# git add / commit / push
pwsh -File scripts/agents/git-coord.ps1 release -Note "ok"
```
