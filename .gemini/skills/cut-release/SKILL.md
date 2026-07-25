---
name: cut-release
description: >
  Use when preparing or cutting a SemVer release (vMAJOR.MINOR.PATCH): bump
  decision per SoT-37, changelog preview, annotated tag, and watching release.yml.
  Manual publish only — never push tags or force-move published tags unless the
  user explicitly asks to publish.
disable-model-invocation: true
when_to_use: >
  "cut a release", "tag v0.", "SoT-37", "prepare release notes",
  "publish to ghcr", "bump version", "git-cliff release", "annotated tag".
---

# Cut release

**User-invoked only** (`disable-model-invocation: true`). Do not auto-route into
publish. Default mode is **prepare**; **publish** only when the user explicitly
asks to push a tag / cut the release.

Human recipe (operator voice): `docs/how-to/cut-a-release.md`  
Policy: `sots/37-versioning-convention.md` · Automation: `.github/workflows/release.yml`  
Hard gates: rule `93-release-and-ci.md`

## Modes

| Mode | User said… | Agent may… | Agent must not… |
|------|------------|------------|-----------------|
| **prepare** | “prepare notes”, “what version?”, “changelog” | Bump math, `make changelog-unreleased`, local dry-run notes, checklist | `git push origin v*`, unsolicited annotated tag on shared branches |
| **publish** | “publish”, “push the tag”, “cut vX.Y.Z” | Tag + push **after** preflight + confirmation of version | Force-move tags, publish bots, tag red CI |

## Hard stops

- Another session holds `release` or `git-ops` without agreement.
- Dirty tree or HEAD is not the intended SHA (`origin/main` tip or human-named SHA).
- CI (`ci.yml` jobs **go** including `go test -race` + **detector-vs-bots**) not **success** on that SHA.
  (`release.yml` also hard-gates on a green `ci.yml` via `require-ci`.)
- Freeze surfaces changed without `!` / `BREAKING CHANGE:` — refuse PATCH; demand markers + correct bump.
- User only asked to prepare — stop before any remote tag push.

## Steps

### 1. Lanes

```powershell
pwsh -File scripts/agents/session-board.ps1 claim `
  -Lane release -Provider <claude|grok|codex|gemini> `
  -Goal "prepare/cut vX.Y.Z" -Paths "cliff.toml,.github/workflows/release.yml"
```

For the actual tag/push transaction only:

```powershell
pwsh -File scripts/agents/git-coord.ps1 claim -Provider <provider> -Session "cut-vX.Y.Z"
```

### 2. Preflight (both modes)

1. `git fetch origin --tags` · `git status -sb` clean for release-critical paths.
2. `test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)"` (or human-named SHA).
3. CI green on that SHA:

   ```bash
   gh run list --branch main --commit "$(git rev-parse HEAD)" --workflow ci.yml --limit 5
   ```

4. Bump from Conventional Commits since previous tag (SoT-37):

   | Since last tag | Bump |
   |----------------|------|
   | `!` or `BREAKING CHANGE:` | MAJOR (pre-1.0 may be minor **with** marker) |
   | any `feat:` | MINOR |
   | only fix/perf/harden/security/docs | PATCH |

5. Preview notes: `make changelog-unreleased` (optional after local tag: `make release-notes`).
6. Tag free: `git ls-remote --tags origin "vX.Y.Z"` empty.
7. Freeze surfaces: if `git diff --name-only $(git describe --tags --abbrev=0)..HEAD` hits
   `internal/signals/`, `internal/scoring/policy.go`, hard-rule paths → require break markers.
8. `git diff $(git describe --tags --abbrev=0)..HEAD -- .github/workflows/release.yml` empty
   **or** human explicitly accepted the workflow delta.
9. Confirm publish scope: **core + gate only** — never bots/red.

Checkbox dump: `assets/preflight-checklist.md`.

### 3. Publish (only if user ordered)

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z   # and main only if needed and already green
```

Immediately release `git-ops`. Watch:

```bash
gh run list --workflow=release.yml --limit 5
gh run watch   # when run id known
```

### 4. What the tag triggers (fidelity)

`release.yml` on `v*.*.*`:

1. **images** matrix (`fail-fast: false`): `build/core.Dockerfile` →
   `ghcr.io/<owner>/humanymous-core`, `build/gate.Dockerfile` → `…/humanymous-gate`
   (amd64+arm64), tags include SemVer + **`latest`** + sha, SBOM, provenance, cosign OIDC.
2. **images-ok**: both digests present; imagetools inspect version + `latest`.
3. **gh-release** (`needs: [images, images-ok]`): git-cliff → GitHub Release body.
   Concurrent releases use `concurrency.group: release` with **cancel-in-progress: false**.

Bots image is **never** in the matrix.

### 5. Post-publish verify

- Both image tags resolve; prefer digests.
- `gh release view vX.Y.Z` has notes.
- Operator trust path: `docs/how-to/verify-the-image.md` (cosign identity + issuer).
- Do not claim “released” until images **and** Release job succeeded.

### 6. Failure / recovery

| Situation | Do | Do not |
|-----------|-----|--------|
| CI red | Fix main; re-check SHA | Tag anyway |
| Images red / partial matrix | Fix-forward new tag; document partial ghcr | Force-move tag |
| Notes empty / wrong | `gh release edit` only with user OK; fix commit hygiene next cut | Rewrite history |
| Wrong version after push | Next SemVer | Delete/recreate published tag |

## Canon links

- `docs/how-to/cut-a-release.md`, `docs/how-to/verify-the-image.md`
- `sots/37-versioning-convention.md`, `cliff.toml`, `.github/workflows/release.yml`
- Rules `20`, `60`, `91`, `92`, `93` · skills `github-actions-ops`, `red-blue-validate`, `coordinate-sessions`
