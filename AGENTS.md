# AGENTS.md — humanymous

Cross-tool always-on rules (Claude Code · Grok Build · OpenAI Codex · Gemini CLI).
Vendor stubs (`CLAUDE.md`, `GEMINI.md`) point here. Keep this file **short** — details load on demand.

## Mission

Raise the cost of automation. Apache-2.0 **reference** build: honesty over over-claim. T4 (coherent real browser + human pace) is an intentional ceiling.

## Hard constraints

1. **Detection freeze** unless the user explicitly changes scoring (weights, thresholds, hard-rule predicates, verdict bands).
2. **Fail-closed** enforcement when required signals are missing; dual-control for erasure / kill switch / permanent bans.
3. **A11y**: never gate Pass/CHALLENGE on motor richness or speed.
4. **Defense-only**: red catalog targets this deployment only — no third-party probing.
5. **No folklore**: every shipped tell/rule traces to a SoT (or a plan delta that becomes SoT).

## Progressive disclosure (read more only when needed)

| If you are working on… | Read next |
|------------------------|-----------|
| Package layout / scoring / signals | `internal/AGENTS.md` |
| Edge proxy, audit, admin | `cmd/gate/AGENTS.md` + `internal/gate` patterns |
| Public docs (Diátaxis) | `docs/AGENTS.md` |
| Attack catalog / e2e | `test/AGENTS.md` |
| Client WASM/JS | `web/AGENTS.md` |
| Spec / design before code | skill `design-sot` → `sots/`, `plan/` |
| Deferred / misdocumented claims | `sots/38-truth-debt-remediation.md` if present + `.agents/rules/80-truth-debt.md` |
| Hard-won multi-session lessons | `.agents/lessons/HARD-WON.md` (Claude memory is **not** visible to other providers) |
| Full verification | skill `red-blue-validate` (**Docker only**) |
| Multi-perspective panel | `.agents/workflows/design-panel.yaml` + personas |
| Prior LLM sessions on this machine | skill `survey-provider-history` |
| SemVer tag / ghcr release / release.yml failure | skill `cut-release` (user-invoked publish) or `github-actions-ops` + rule `93` + `docs/how-to/cut-a-release.md` |

Domain canon: `sots/`, `plan/` (may be gitignored in publish snapshots). Public method: `docs/explanation/how-this-was-built.md`.

## Done means

| Scope | Gate |
|-------|------|
| Any code | `go test` on touched packages; prefer `go test ./...` |
| WASM/detect | `GOOS=js GOARCH=wasm go build ./cmd/wasm/` |
| **Any e2e** | **Docker only** — `make e2e` / `bash scripts/e2e-docker.sh` (skill `red-blue-validate`) |

Host/loopback `node test/e2e/runner.mjs` or host `node test/gate/e2e.mjs` is **not** completion authority (misses L5 topology / lab isolation). See `.agents/rules/60-e2e-docker-only.md`.

## Skills & agents

- Canon: `.agents/skills/`, `.agents/personas/`, `.agents/rules/`, `.agents/workflows/`
- After clone (Claude/Codex): `pwsh -File scripts/agents/sync-adapters.ps1` or `bash scripts/agents/sync-adapters.sh`
- Verify: `scripts/agents/verify-agents-layout.*`
- Edit **only** `.agents/**`, nested `AGENTS.md`, and this file — re-run sync; never hand-edit generated `.claude/` / `.grok/` / `.codex/` mirrors.

## Multi-provider / nested sessions (overlap)

Several LLM chats may run on this repo **at once**. Before non-trivial edits:

1. `pwsh -File scripts/agents/session-board.ps1 list` (or `bash scripts/agents/session-board.sh list`)
2. **Claim a work lane** — skill `coordinate-sessions` / `.agents/sessions/LANES.md`
3. Work only under claimed paths
4. **Git writes are serial + attributed** — before `add`/`commit`/`pull`/`push`/`stash`/branch switch:

   ```powershell
   pwsh -File scripts/agents/git-coord.ps1 preflight
   pwsh -File scripts/agents/git-coord.ps1 claim -Provider <claude|grok|codex|gemini>
   git add <only-your-paths>
   pwsh -File scripts/agents/git-coord.ps1 commit -Provider <claude|grok|codex|gemini> `
     -Subject "feat(scope): summary" -Body "optional why…"
   # injects vendor GitHub-linked Co-Authored-By (avatars):
   #   claude → Claude <noreply@anthropic.com>
   #   grok   → Grok <grok@x.ai>  (community de-facto; avatar may be absent)
   #   codex  → codex <codex@openai.com>
   pwsh -File scripts/agents/git-coord.ps1 release -Note "…"
   ```

   Commit message canon: `.agents/sessions/COMMIT-CONVENTION.md` (rule `92`).
   Emails must be GitHub-linked so co-author **profile photos** resolve.

5. Release the **work lane** when done or switching providers; incomplete → board handover + `handover-pack`

Same work lane = one writer. Different work lanes may dirty the tree in parallel;  
**`git-ops` is a global mutex** for the index/HEAD/remote.  
`detection-core` ↔ `gate-edge` mutex on scoring seams.  
Details: `.agents/rules/90-session-overlap.md`, `91-git-contention.md`,  
`.agents/sessions/PROTOCOL.md`, `GIT-PROTOCOL.md`.

## Ambiguity

If requirements conflict or scope is unclear: **stop and ask** before large edits (agents default to silent assumption otherwise).

## Cross-provider note

Claude Code may have rich `~/.claude/.../memory/*.md` for this repo. **Grok, Codex, and Gemini do not load that tree.** Prefer `.agents/lessons/` and the **session board** (`.agents/sessions/ACTIVE.md`) for anything that must survive provider switches or stacked chats.
