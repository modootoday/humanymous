# Git action contention (always-on)

File lanes are not enough. **Git index / HEAD / remote updates are a global mutex.**

## Mandatory shared branch

All repository work in this shared session stays in `C:\workspaces\humanymous`
on `codex/rust`. Read the actual branch before every write; on mismatch stop and
coordinate with Root, never switch automatically. Do not create/switch branches,
create worktrees, or recreate deleted validation refs. Owners do not push/delete
refs themselves. Root alone performs explicitly user-authorized integration/ref
work. A `git-ops` claim does not waive these restrictions. Isolated Root source is
read-only; candidate edits remain under the author's exact claimed evidence paths.

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

## Public metadata bytes

New authored public metadata must be UTF-8 with LF in both the worktree and Git
blob. Attachment-index hashes and sizes must reproduce from a fresh Git checkout;
a Windows worktree-only `spec check` pass is insufficient. Before claiming a
canonical receipt, verify raw worktree bytes equal the exact committed blob.

Do not normalize raw logs, immutable fixtures or operational-profile bytes.
For an authorized metadata encoding correction, preserve the original bytes in
private evidence and retain old commits. Record an explicit manifest separating
retained worktree hashes/sizes from canonical Git hashes/sizes; never silently
reinterpret a prior SHA. No global attributes or vendor/raw rewrite follows from
this rule. Writer details and verification steps: `coordinate-sessions` skill.

Protocol: `.agents/sessions/GIT-PROTOCOL.md`  
Tooling: `scripts/agents/git-coord.ps1` / `git-coord.sh` / `git-commit.sh`  
Skill: `coordinate-sessions` (git section).
