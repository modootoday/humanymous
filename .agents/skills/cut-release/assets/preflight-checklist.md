# Cut-release preflight (checkbox)

- [ ] User intent: **prepare** only, or **publish** (explicit version)?
- [ ] Lane `release` claimed; `git-ops` free or held only for tag/push
- [ ] Worktree clean (or only unrelated WIP ignored)
- [ ] `HEAD` == `origin/main` (or human-named SHA)
- [ ] `ci.yml` required Rust validation **success** on that exact SHA; no missing/ignored required work
- [ ] Actual source-bound hosted Docker product e2e qualified; component/oracle success alone is insufficient
- [ ] Workflow/catalog names match and OIDC `release.yml`/environment identities are unchanged or explicitly migrated
- [ ] Registry trust matches package environments: crates.io=`crates`, npm=`npm`; GitHub environment setup alone is not external publisher verification
- [ ] Bump class from SoT-37 (MAJOR / MINOR / PATCH / pre-1.0)
- [ ] `make changelog-unreleased` reviewed
- [ ] Proposed `vX.Y.Z` free on `origin`
- [ ] No freeze-surface change without `!` / `BREAKING CHANGE:`
- [ ] `release.yml` unchanged in range (or human waived)
- [ ] Publish scope: core + gate only — **not** bots
- [ ] (publish) User confirmed version + push
- [ ] (publish) Annotated tag pushed; `git-ops` released
- [ ] (publish) `release.yml` green; both images + GH Release checked
- [ ] (publish) Point to `docs/how-to/verify-the-image.md`
