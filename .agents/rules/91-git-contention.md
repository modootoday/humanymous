# Git action contention (always-on)

File lanes are not enough. **Git index / HEAD / remote updates are a global mutex.**

## Rules

1. Before `git add`, `commit`, `pull`/`rebase`/`merge` that updates HEAD, `push`, `stash`,
   or branch switch: claim lane **`git-ops`** via `session-board` / `git-coord claim`.
2. Hold `git-ops` only for the git transaction, then **release immediately**.
3. Read-only git (`status`, `log`, `diff`) does not need the claim.
4. Stage only your lane’s files unless the user asked for a combined commit.
5. **Never** force-push shared branches or amend already-pushed commits without user OK.
6. If `.git/index.lock` exists, do not start another git write — wait or diagnose.
7. On push rejection: stop, re-fetch under `git-ops`, no force.
8. **Attribution:** every agent commit MUST carry `Co-Authored-By` for the provider
   (rule `92-git-commit-attribution.md`, canon `COMMIT-CONVENTION.md`). Prefer
   `git-coord commit -Provider …`.

Protocol: `.agents/sessions/GIT-PROTOCOL.md`  
Tooling: `scripts/agents/git-coord.ps1` / `git-coord.sh` / `git-commit.sh`  
Skill: `coordinate-sessions` (git section).
