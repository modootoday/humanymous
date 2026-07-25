---
name: github-actions-ops
description: >
  Use when editing or debugging GitHub Actions (ci.yml, release.yml, codeql),
  release job failures (buildx, cosign, git-cliff, ghcr), or CI red on main/PR.
  Defense-only; never add a workflow that publishes the bots/red image.
when_to_use: >
  "release workflow failed", "Actions red", "cosign sign failed",
  "edit ci.yml", "ghcr push", "git-cliff in CI", "workflow permissions",
  "detector-vs-bots job failed".
---

# GitHub Actions ops (this repo)

Repo-specific playbook — not a general GA tutorial. Publish **timing** is still
human-gated via skill `cut-release` + rule `93`.

## Workflow map

| Workflow | Trigger | Agent may… | Must not… |
|----------|---------|------------|-----------|
| `.github/workflows/ci.yml` | push `main`, PR | Fix jobs, caches, Trivy, agent-layout verify, cliff config check | Replace Docker e2e with host loopback as authority |
| `.github/workflows/release.yml` | tags `v*.*.*` | Fix build-args, matrix build issues, notes step | Add bots matrix; PR/workflow_dispatch package push; strip cosign/SBOM/provenance; set `cancel-in-progress: true` on release concurrency; remove `:latest` on tag cuts |
| `.github/workflows/codeql.yml` | as configured | Keep green; align Go version | Weaken analysis without user OK |

## Lanes

- **`e2e-infra`**: `ci.yml` detector-vs-bots / compose / Trivy image builds  
- **`release`**: `release.yml`, `cliff.toml`  
- **`agents-meta`**: agent-layout verify job only  
- Commits: `git-coord` + Conventional Commits + Co-Authored-By (`92`)

## Authoring constraints

1. Prefer action majors already used (`checkout@v4`, `setup-go@v7`, `build-push-action@v7`, `git-cliff-action@v4`).
2. Default `permissions: contents: read`; elevate only where needed (`packages: write`, `id-token: write`, `contents: write` on `gh-release` job only).
3. CI may use concurrency cancel-in-progress; **release** must keep
   `concurrency.cancel-in-progress: false` (queue tags; never abort mid-push).
4. Tag-only releases use metadata-action `flavor: latest=true` so `:latest`
   moves on every successful `v*.*.*` cut — **not** `enable={{is_default_branch}}`
   (false on tag refs).
5. Never add `bots.Dockerfile` / `humanymous-bots` / red paths to any push-to-registry job.
6. Privilege expansion (`id-token`, `packages: write`, new secrets) requires **explicit human approval of the diff** before commit.
7. Do not invent long-lived cosign keys or `CR_PAT` unless the user names them.
8. Matrix uses `fail-fast: false` + `images-ok` job before `gh-release`. A failed
   leg can still leave the other image on ghcr — fix-forward with a new SemVer.

## Failure playbook

| Symptom | First checks | Fix shape |
|---------|--------------|-----------|
| CI `go` red | logs, `go test`, wasm job | fix code on main |
| CI `detector-vs-bots` red | compose, assert profiles, freeze regression | Docker e2e locally; do not “skip” job |
| Release `images` one leg red | Dockerfile, TARGETARCH, VERSION build-arg | fix main → **new** tag if prior tag incomplete |
| Cosign / id-token | `permissions.id-token: write`, OIDC | restore perms; no ad-hoc keys |
| `gh-release` empty body | `fetch-depth: 0`, cliff, unconventional commits | fix cliff/hygiene; `gh release edit` only with user OK |
| packages: write denied | GITHUB_TOKEN packages permission, first package create | restore workflow perms / org package settings |
| Agent layout job red | adapter drift | `sync-adapters` + verify; no hand-edit mirrors |
| Trivy HIGH/CRITICAL | image deps | patch; no severity ignore without user OK |

Observe:

```bash
gh run list --workflow=ci.yml --limit 5
gh run list --workflow=release.yml --limit 5
gh run view <id> --log-failed
```

## Relation to cut-release

- **CI red before tag** → fix here; do not tag (`cut-release` preflight).  
- **Tag already pushed, release red** → diagnose here; recovery is fix-forward (new SemVer), never force-tag.  
- **Bump/notes/publish decision** → `cut-release` only.

## Canon

- Rule `93-release-and-ci.md`, `60-e2e-docker-only.md`, `91`, `92`
- `docs/how-to/cut-a-release.md`, `docs/how-to/verify-the-image.md`
