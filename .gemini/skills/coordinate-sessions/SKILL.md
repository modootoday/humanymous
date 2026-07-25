---
name: coordinate-sessions
description: >
  Use when multiple LLM sessions may overlap (Claude, Grok, Codex, Gemini), when
  starting work on a shared repo, claiming a work lane, releasing a claim, or
  resolving session conflicts. Prefer at session start before large edits.
---

# Coordinate sessions (multi-provider overlap)

## Goal

Prevent file races and lost handoffs when several agents (or stacked chats) touch
this repository.

## Steps

### Session start

1. `pwsh -File scripts/agents/session-board.ps1 list`  
   (or `bash scripts/agents/session-board.sh list`)
2. `git status -sb` — note dirty paths vs claimed lanes.
3. Choose a lane from `.agents/sessions/LANES.md`.
4. Claim:

   ```powershell
   pwsh -File scripts/agents/session-board.ps1 claim `
     -Lane <lane> -Provider <claude|grok|codex|gemini|human> `
     -Goal "<one line>" -Paths "<glob1,glob2>"
   ```

5. If claim fails (lane taken): stop writing those paths; ask user or pick another lane.

### During work

- Stay inside claimed paths; read-only elsewhere.
- If you need another lane: claim it only if free, or extend Paths after checking mutex rules
  (`detection-core` ↔ `gate-edge`).

### Git writes (always serial + attributed)

Even if work lanes differ, only one agent may own git mutations:

```powershell
pwsh -File scripts/agents/git-coord.ps1 preflight
pwsh -File scripts/agents/git-coord.ps1 claim -Provider <provider> -Session "<id>"
git add <only-your-paths>
pwsh -File scripts/agents/git-coord.ps1 commit -Provider <provider> `
  -Subject "feat(scope): summary" -Body "why…"
# or: bash scripts/agents/git-commit.sh <provider> "feat(scope): summary" "body"
pwsh -File scripts/agents/git-coord.ps1 release -Note "commit <sha>"
```

- **Required trailer:** vendor GitHub-linked `Co-Authored-By` (avatar-capable), e.g.
  `Claude <noreply@anthropic.com>`, `Grok Build <304785771+grokkybara[bot]@users.noreply.github.com>`,
  `codex <codex@openai.com>` — full registry in `COMMIT-CONVENTION.md`.
- Do not use bare `git commit -m` without that trailer; do not invent unowned emails.
- Read-only `git status`/`diff`/`log` — no claim.
- If preflight reports `index.lock` or active `git-ops` → wait.
- No force-push to shared branches; no amend of pushed commits without user OK.
- Full rules: `GIT-PROTOCOL.md`, `COMMIT-CONVENTION.md`, rules `91` + `92`.

### Session end / provider switch

1. Summarize open work (skill `handover-pack` if multi-step remains).
2. Ensure `git-ops` is released (abort half-finished commits carefully).
3. Release work lane:

   ```powershell
   pwsh -File scripts/agents/session-board.ps1 release -Lane <lane> -Note "<handover one-liner>"
   ```

4. Promote durable lessons to `.agents/lessons/HARD-WON.md` if any.

### Conflict / stale

- `list` marks claims older than 4h as stale.
- Force-release only with user OK or clear abandonment:

  ```powershell
  pwsh -File scripts/agents/session-board.ps1 release -Lane <lane> -Force -Note "stale reclaim"
  ```

## References

- `.agents/sessions/PROTOCOL.md`
- `.agents/sessions/GIT-PROTOCOL.md`
- `.agents/sessions/COMMIT-CONVENTION.md`
- `.agents/sessions/LANES.md`
- `.agents/rules/90-session-overlap.md`
- `.agents/rules/91-git-contention.md`
- `.agents/rules/92-git-commit-attribution.md`
