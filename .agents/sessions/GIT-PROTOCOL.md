# Git contention protocol (multi-provider)

File lanes stop edit races; **git actions still race** on the index, `HEAD`, the
reflog, and the remote. Two agents running `git commit` / `git pull` / `git push`
at once produce: `index.lock`, lost commits, thrashy rebases, or rejected pushes.

## Exclusive lane: `git-ops`

| Operation | Requires `git-ops` claim? |
|-----------|---------------------------|
| `git status` / `git log` / `git diff` (read-only) | No |
| `git add` / `git commit` / `git restore --staged` | **Yes** |
| `git pull` / `git fetch` + merge/rebase onto current branch | **Yes** |
| `git push` / `git push --tags` | **Yes** |
| `git merge` / `git rebase` / `git cherry-pick` | **Yes** |
| `git branch` create (no checkout) | Prefer yes if others may push |
| `git checkout` / `git switch` of a shared branch | **Yes** (changes others’ context) |
| `git stash` / `git stash pop` | **Yes** (mutates worktree + reflog) |
| `git tag` (version tags) | **Yes** (+ usually `release` lane) |
| `git reset` / `git commit --amend` / force-push | **Yes** + user approval (see forbidden) |

Hold `git-ops` only for the **duration of the git transaction** (seconds–minutes),
not for the whole coding session. Typical pattern:

```text
[edit under work lane] → claim git-ops → status/diff → add → commit [→ push] → release git-ops
```

## Forbidden without explicit user approval

- `git push --force` / `--force-with-lease` to `main` / `master` / shared release branches
- `git commit --amend` of a commit already pushed
- `git reset --hard` discarding others’ uncommitted work
- Deleting remote branches others may use
- Rewriting published history

## Pre-commit checklist (holder of `git-ops`)

1. `session-board list` — no unexpected active claims on paths you are committing.
2. `git status -sb` — understand *all* dirty files (other sessions may have written).
3. Stage **only** paths belonging to your work (or explicitly agreed multi-lane batch).
4. Do not stage secrets (`.env`, keys, provider OAuth, MCP tokens).
5. Message: Conventional Commits style used by this repo (`feat:`, `fix:`, `docs:`, …).
6. Prefer **one logical commit per lane batch**; avoid mega-commits that mix `docs` +
   detection freeze breaks.

## Pre-push checklist

1. Still hold `git-ops`.
2. `git fetch` then if behind: rebase/merge **under git-ops** (never parallel).
3. Push once; on rejection, stop — re-fetch, do not force.
4. Release `git-ops` immediately after success or failure (leave note on failure).

## Parallel coding vs serial git

```text
  Claude [docs] ──edits──┐
  Grok  [agents-meta] ───┼──►  SERIAL git-ops queue  ──►  push
  Codex [red-catalog] ───┘         (one claim)
```

Multiple work lanes may dirty the tree; **only one agent owns the index** at a time.

## Tooling

```powershell
pwsh -File scripts/agents/git-coord.ps1 preflight
pwsh -File scripts/agents/git-coord.ps1 claim -Provider grok -Session "…"
# … add/commit/push under this claim …
pwsh -File scripts/agents/git-coord.ps1 release -Note "committed 4ece28c"
```

`preflight` fails if `git-ops` is held by someone else, or if `.git/index.lock` exists.

## Recovery

| Symptom | Action |
|---------|--------|
| `fatal: Unable to create … index.lock` | Wait; if no other git process, remove stale lock only after checking no live `git` PID |
| Push rejected (non-fast-forward) | Hold `git-ops`, fetch, rebase/merge, push; never force main |
| Two commits for the same fix | Prefer revert+redo or document; don’t amend remote history |
| Stale `git-ops` claim (>30 min) | `session-board release -Lane git-ops -Force` after user OK |

Default **TTL for `git-ops` is 30 minutes** (shorter than work lanes). Scripts warn at list time.
