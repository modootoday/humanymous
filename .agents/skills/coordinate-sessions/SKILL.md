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

0. Require `C:\workspaces\humanymous` and actual branch `codex/rust` before
   every write. Stop on mismatch; do not create/switch branches, create worktrees,
   push/delete refs, or recreate removed validation branches. Root owns separately
   authorized ref work; Cursor owns common Cargo/source application only after
   recorded path release/claim under rule95. Root's isolated source
   is read-only; author candidates only in the claimed evidence paths. Keep this
   migration goal active and preserve incomplete gates; no Cursor handover document.

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

5. If claim fails (lane taken): stop writing those paths. Continue independent read-only analysis/interface coordination; recheck the same lane after its owner releases it. Never force-release or invent an alternate claim for overlapping paths. Do not repeat a permission request when existing session authorization already covers the task.

### During work

- Stay inside claimed paths; read-only elsewhere.
- If you need another lane: claim it only if free, or extend Paths after checking mutex rules
  (`detection-core` ↔ `gate-edge`).

### Git writes (always serial + attributed)

Every external native command must have its exit code checked immediately.
PowerShell's `$ErrorActionPreference` alone does not guarantee that a nonzero
native exit stops a block. After a successful claim, read the actual
`.agents/sessions/claims/git-ops.json` and require both provider and session to
match this transaction before staging or committing. A prior free observation
or a successful preflight is not ownership. Claim failure stops the transaction;
never continue to add/commit/release, and never release a foreign claim.

`& script.ps1` is a PowerShell script invocation, not a fresh native process:
`$LASTEXITCODE` can retain an earlier command's result. Let terminating script
throws stop the transaction and capture `$?` immediately after a direct invocation.
When using `pwsh -File`, inspect that native wrapper's exit code instead. Always
verify the actual claim provider/session independently before writes.

Even if work lanes differ, only one agent may own git mutations:

```powershell
pwsh -File scripts/agents/git-coord.ps1 preflight
if ($LASTEXITCODE -ne 0) { throw 'Git preflight failed' }
pwsh -File scripts/agents/git-coord.ps1 claim -Provider <provider> -Session "<id>"
if ($LASTEXITCODE -ne 0) { throw 'Git claim failed' }
$transactionClaim = Get-Content -Raw .agents/sessions/claims/git-ops.json | ConvertFrom-Json
if ($transactionClaim.provider -ne '<provider>' -or $transactionClaim.session -ne '<id>') {
  throw 'Git claim owner mismatch'
}
git add <only-your-paths>
if ($LASTEXITCODE -ne 0) { throw 'Git staging failed' }
pwsh -File scripts/agents/git-coord.ps1 commit -Provider <provider> `
  -Subject "feat(scope): summary" -Body "why…"
if ($LASTEXITCODE -ne 0) { throw 'Git commit failed; inspect the owned transaction' }
# or: bash scripts/agents/git-commit.sh <provider> "feat(scope): summary" "body"
pwsh -File scripts/agents/git-coord.ps1 release -Note "commit <sha>"
if ($LASTEXITCODE -ne 0) { throw 'Git release failed' }
```

- **Required trailer:** vendor GitHub-linked `Co-Authored-By` (avatar-capable), e.g.
  `Claude <noreply@anthropic.com>`, `Grok <grok@x.ai>`,
  `codex <codex@openai.com>` — full registry in `COMMIT-CONVENTION.md`.
- Do not use bare `git commit -m` without that trailer; do not invent unowned emails.
- Read-only `git status`/`diff`/`log` — no claim.
- Shared index contains another writer's staged work: use `git-coord commit -OnlyPaths @('own/literal/path')`; never restore/unstage their files. Cursor claims released common Cargo/source integration paths; Root retains roadmap/design/allocation/review. Follow rule95 and actual claims, never inferred ownership.
- If preflight reports `index.lock` or active `git-ops` → wait.
- No force-push to shared branches; no amend of pushed commits without user OK.
- Full rules: `GIT-PROTOCOL.md`, `COMMIT-CONVENTION.md`, rules `91` + `92`.

### Reporting proof

- Say **prepared**, **staged** or **blocked by preflight/mutex** until the actual
  commit command succeeds. Never preannounce a commit or use a placeholder SHA.
- Before releasing the owned Git transaction, read literal HEAD and verify the
  intended changed paths in that exact commit. Report that SHA with the result;
  an intended command, prior HEAD or another owner's commit is not proof.
- Claim an actual test pass only after the run's handle is terminal with its
  exit status and exact footer, and the relevant before/after source hashes are
  available. Keep venue, source checkpoint and missing/ignored/filtered cases explicit.
- Closure file count and difference-record count are separate quantities.
  Compare-Object `<=`/`=>` rows indicate differences, not the number of files
  compared. A missing/partial snapshot cannot prove source closure equality.
- Keep source-only checkpoints, actual component tests, full qualification and
  publication distinct. Retain first failures and corrections in the evidence.

### Public metadata encoding and receipts

- Write new public metadata as UTF-8 with LF in the worktree and Git blob.
  Windows `ConvertTo-Json` can embed CRLF; remove CR from newly authored JSON
  before writing. Text writers can also convert newlines. Check actual bytes.
- Attachment-index hashes and sizes must reproduce from a fresh Git checkout.
  Before reporting a canonical receipt, compare raw worktree bytes with the exact
  committed blob and recompute its attachment hashes/sizes from that Git view.
  A green worktree-only `spec check` does not establish this equality.
- Encoding repair requires an authorized owned-metadata scope. First retain the
  original worktree bytes in private evidence and preserve old Git commits.
  Record both retained-worktree and canonical-Git hashes/sizes in an explicit
  view manifest; reattach corrected metadata against canonical bytes. Never
  silently reinterpret an existing SHA as a different byte representation.
- Do not normalize raw logs, immutable fixtures or operational profiles. Do not
  change global attributes or vendor/raw trees to repair owned metadata. These
  instructions authorize no new foreign implementation; authored implementation
  remains Rust under the repository's existing rules.

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
