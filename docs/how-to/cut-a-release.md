---
title: Cut a Release (semver tags + automated changelog)
description: "Push a SemVer git tag (vX.Y.Z) and GitHub Actions builds signed core and gate images while git-cliff writes the release notes from Conventional Commits."
keywords: ["cut a release","SemVer git tag","git-cliff changelog","Conventional Commits","GitHub Actions release workflow","cosign keyless signing","SLSA provenance SBOM","ghcr.io core gate images","Keep a Changelog","humanymous maintainer"]
---

# Cut a release

**Diátaxis quadrant:** How-to. **Audience:** maintainers of humanymous who tag releases.

humanymous releases are driven by **SemVer git tags** (`vMAJOR.MINOR.PATCH`) and **automated release notes** generated from [Conventional Commits](https://www.conventionalcommits.org/) by [git-cliff](https://git-cliff.org) (config in `cliff.toml`). Tagging is deliberate and manual — the tooling never decides *when* to release; it writes the release notes for whatever tag you cut. This guide is the end-to-end recipe.

## What a tag triggers

Pushing a tag matching `v*.*.*` runs `.github/workflows/release.yml`:

1. **Images** — builds the `core` and `gate` images for `linux/amd64` and `linux/arm64`, pushes them to `ghcr.io/<owner>/humanymous-core` and `-gate` with SemVer tags (`X.Y.Z`, `X.Y`, `X`), **`latest`**, and a long `sha` tag, attaches SLSA provenance + an SPDX SBOM, and signs each with cosign (keyless OIDC). The `bots` attacker image is never published. Concurrent tag pushes are **queued** (not cancelled mid-push). Both matrix legs must succeed and pass an `images-ok` inspect before the GitHub Release job runs.
2. **GitHub release** — git-cliff generates the release notes from the Conventional Commits **since the previous tag**, grouped into Keep a Changelog sections, and `softprops/action-gh-release` publishes them.

## Choose the version

The commit types since the last tag decide the bump:

| Commits since the last tag | Bump | Example |
|----------------------------|------|---------|
| Any `feat:` | **minor** — `0.1.0 → 0.2.0` | a new capability |
| Only `fix:` / `perf:` / `harden:` / `security:` / `docs:` | **patch** — `0.1.0 → 0.1.1` | fixes, hardening, docs |
| Any commit with a `!` or a `BREAKING CHANGE:` footer | **major** — `0.1.0 → 1.0.0` | an incompatible change |

Before `1.0.0` the project is pre-stable: a breaking change may bump the minor rather than the major, at the maintainer's discretion. Preview what a version would contain with `make changelog-unreleased`.

## Steps

1. **Land your work on `main`** with Conventional Commit subjects (`feat(gate): …`, `fix(core): …`, `docs: …`). The release-notes quality is only as good as the commit subjects — the first line becomes the release-notes entry.

2. **Confirm CI is green on the exact SHA you will tag** (the `ci` workflow jobs for build/unit tests — including `go test -race` — and the Docker detector-vs-bots gate). The release workflow **refuses to publish** until a successful `ci.yml` run exists for that commit (`require-ci` job); it does **not** re-run unit/e2e itself.

3. **Preview notes** with `make changelog-unreleased` and choose the SemVer bump using the table above.

4. **Tag and push** only when you intend to publish.
   ```bash
   git tag -a v0.2.0 -m "v0.2.0"
   git push origin main
   git push origin v0.2.0
   ```

5. **Watch the release workflow** in the Actions tab. When it is green, the images are on ghcr.io and the GitHub release carries the git-cliff notes. Verify supply-chain trust with [Verify the image](verify-the-image.md) (digest pin + cosign), then confirm a pulled image runs (see [Deployment & policy operations](deployment-policy-operations.md)).

**If a cut goes wrong after the tag has left this machine:** do **not** force-move or delete-and-recreate the published tag. Fix on `main` and cut the **next** SemVer (fix-forward). Only re-tag a version that never left this machine.

## How the changelog is generated

`cliff.toml` maps Conventional Commit types to Keep a Changelog sections:

| Commit type | Section |
|-------------|---------|
| `feat` | Added |
| `fix` | Fixed |
| `harden`, `security`, or a body mentioning security | Security |
| `perf` | Performance |
| `refactor` (and anything unmatched) | Changed |
| `revert` | Reverted |
| `docs` | Documentation |
| `test`, `ci`, `build`, `chore`, `style` | *(omitted — not user-facing)* |

Non-Conventional commits and merge commits are filtered out. To preview exactly what a tag's notes will be: `make release-notes`.

## Cutting the next release

Releases are cut by tagging the next SemVer version (`vX.Y.Z`) per the bump rules above and pushing it, following the steps above. The release workflow then builds the ghcr images. Prefer adopters pin by **digest**; floating tags (`X.Y`, `X`, and `latest` when applied) move on later releases. Once a tag has been pushed and may have been pulled, force-moving it is forbidden — only ever re-tag a tag that has never left this machine; otherwise fix-forward with a new SemVer.

## Related

- [Upgrade, migration & zero-downtime](upgrade-migration.md) — the upgrade posture releases are consumed under.
- [CLI, configuration & policy reference](../reference/cli-config-policy.md) — the `VERSION` build-arg the images self-report.
