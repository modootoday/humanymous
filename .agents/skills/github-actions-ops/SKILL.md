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

## Workflow identities and names

Read `scripts/ci/workflows.json` and `.spec/sot/delivery.sot.md` before editing.
The catalog is the machine-readable naming authority. Display names use
`CI / ...`, `Docs / ...`, `Release / ...`, or `Legacy / ...`; filenames, job keys,
artifact identities and concurrency groups are stable integration contracts.
Use filenames or numeric workflow/run IDs in CLI commands, not display names.
Rename a technical identifier only with all callers, policy checks, registry
trust bindings and documentation updated together. Never rename `release.yml`
as a cosmetic cleanup: trusted-publisher identity includes that filename.

## Workflow map

| Workflow | Trigger | Agent may… | Must not… |
|----------|---------|------------|-----------|
| `.github/workflows/ci.yml` | push `main`/`codex/rust`, PR, manual | Maintain the required Rust validation entrypoint | Replace Docker e2e with host loopback as authority |
| `.github/workflows/rust-validation.yml` | reusable workflow | Maintain the two bounded hosted lanes and strict final result accounting | Drop required failures/skips or infer full product readiness from components |
| `.github/workflows/collector-component.yml` | component/manual | Maintain actual isolated Firefox WASM coverage | Treat component tests as human/AT/full-product qualification |
| `.github/workflows/cef-native-component.yml` | component/manual | Maintain actual CEF capability/rejection evidence | Disable sandbox or infer interactive readiness from headless success |
| `.github/workflows/cross-os-probe.yml` | component/manual | Preserve OS/arch/feature-specific native accounting | Relabel required ignored tests as helpers |
| `.github/workflows/pages-build.yml` | reusable workflow | Validate both baseurls, links and source freshness | Refresh review digests automatically to hide stale reviews |
| `.github/workflows/pages.yml` | main/manual | Deploy only its validated Pages artifact | Expand deployment privileges into ordinary validation |
| `.github/workflows/release.yml` | tags `v*.*.*` | Fix build-args, matrix build issues, notes step | Add bots matrix; PR/workflow_dispatch package push; strip cosign/SBOM/provenance; set `cancel-in-progress: true` on release concurrency; remove `:latest` on tag cuts |
| `.github/workflows/codeql.yml` | legacy oracle | Preserve frozen historical analysis during migration | Treat Go analysis as Rust qualification or start Go maintenance |
| `.github/workflows/frozen-go-oracle.yml` | manual legacy oracle | Compare frozen behavior where explicitly required | Treat oracle success as migrated-product completion |

## Lanes

- **`e2e-infra`**: required Rust workflows, Compose and image/advisory validation
- **`release`**: `release.yml`, `cliff.toml`  
- **`agents-meta`**: agent-layout verify job only  
- Commits: `git-coord` + Conventional Commits + Co-Authored-By (`92`)

## Authoring constraints

1. Pin actions to verified full commit SHAs. Preserve commit identity and actual entrypoint checks; public third-party metadata reads must not require privileged repository credentials.
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
9. Local work is coding, bounded unit tests, fmt and scoped Clippy; actual
   process/IPC/SCM/browser/Docker/Cross-OS validation belongs on standard hosted
   runners. Classify by actual resource use, not `#[test]` or `--lib` alone.
10. The free branch profile disables remote cache writes and automatic artifact
    uploads, including nested action defaults. Keep public logs allowlisted;
    never upload private Specfold bundles, credentials or user traces.
11. Keep the two-lane result aggregator fail-closed. A predecessor's failure may
    not hide independent results, and skipped/unknown required work is not green.

## Failure playbook

| Symptom | First checks | Fix shape |
|---------|--------------|-----------|
| Rust CI red | exact job/source, locked Cargo tests, native/wasm matrix | fix the owning Rust crate; no branch switch or full-product claim from component green |
| CI product Docker red | exact source/image, owning Rust lab report, isolation and freeze regression | Fix the owning Rust implementation; rerun the hosted Docker gate without skipping it |
| Release validation/readiness red | reusable required validation or registry/source readiness failed on tag SHA | Fix before publishing, or fix-forward with a new tag after a partial publication |
| Release `images` one leg red | Dockerfile, TARGETARCH, VERSION build-arg | fix main → **new** tag if prior tag incomplete |
| Cosign / id-token | `permissions.id-token: write`, OIDC | restore perms; no ad-hoc keys |
| `gh-release` empty body | `fetch-depth: 0`, cliff, unconventional commits | fix cliff/hygiene; `gh release edit` only with user OK |
| packages: write denied | GITHUB_TOKEN packages permission, first package create | restore workflow perms / org package settings |
| Agent layout job red | Rust canon check and retirement manifest, adapter drift | preserve live mirrors; root coordinates qualified final sync/retirement |
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
