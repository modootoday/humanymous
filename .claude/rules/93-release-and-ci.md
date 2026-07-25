# Release & CI (always-on)

SemVer tags and GitHub Actions publish paths have high blast radius. Agents must
not freestyle production publishes.

## Rules

1. **No autonomous publish.** Do not create or push `vMAJOR.MINOR.PATCH` tags
   unless the user **explicitly** asks to cut/publish that version (skill
   `cut-release`, user-invoked only).
2. **Tag push = ship.** A pushed `v*.*.*` tag runs `.github/workflows/release.yml`
   (core + gate → ghcr, cosign, SBOM/provenance, GitHub Release). Treat as production.
3. **Never publish bots/red.** Do not add attacker images to any registry workflow
   or manually push the red catalog image to ghcr.
4. **Immutability.** No force-move of a tag that has left the machine; no
   force-push to `main`/`master`. Fix-forward with a **new** SemVer tag.
5. **Detection freeze → bump class.** Verdict-altering changes in the tag range
   are MAJOR / `!` (or pre-1.0 minor **with** break marker per SoT-37) — never a
   silent PATCH. Rule `20` still blocks casual detection edits.
6. **Lanes.** Cutting a release claims exclusive **`release`**; tag/push uses
   short **`git-ops`**. CI/detector workflow edits → `e2e-infra` or `release` as
   appropriate.
7. **Secrets.** Do not invent long-lived registry or cosign keys; prefer
   `GITHUB_TOKEN` + OIDC as shipped in `release.yml`.
8. **Skills.** Publish path → `cut-release`; CI/workflow failure or authoring →
   `github-actions-ops`; e2e authority → `red-blue-validate` (Docker only).
9. **Attribution.** Commits that change workflows/skills still need Conventional
   Commits + `Co-Authored-By` (rule `92`).
10. **Do not strip supply-chain.** Do not remove cosign, `id-token: write`,
    `provenance: mode=max`, or `sbom: true` from `release.yml` without explicit
    maintainer approval.

## Related

- Skill `cut-release`, skill `github-actions-ops`
- `docs/how-to/cut-a-release.md`, `docs/how-to/verify-the-image.md`
- SoT-37, `cliff.toml`, rules `20`, `60`, `91`, `92`
