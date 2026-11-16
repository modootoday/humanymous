<!--
  Security fixes must NOT go through a public PR: follow SECURITY.md (private disclosure).
  Agent pull requests come from a `<provider>/<slug>` branch and fill every field below.
-->

Agent-Provider: <!-- claude | codex | cursor | grok | gemini | human -->
Work package: <!-- plan unit, lane or issue, e.g. plan:root/20261116070000 step 5 -->
Lane: <!-- one lane from .agents/sessions/lanes.json -->

## Summary

<!-- What changes, and why. Link related issues (e.g. "Closes #123"). -->

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Documentation
- [ ] Tooling / CI / build
- [ ] Refactor (no behaviour change)

## Checks

- [ ] Scoped `cargo test --locked`, `cargo fmt --check` and Clippy pass for the changed crates.
- [ ] `spec check --no-local` passes.
- [ ] **Detection freeze**: no scoring weights, thresholds, hard-rule predicates or verdict bands changed (or the owner asked for it).
- [ ] Commit subjects follow Conventional Commits; no `Co-authored-by` unless `@users.noreply.github.com`; author and committer are noreply addresses.
- [ ] Product e2e claims come only from the Docker runner (rule 60), with source, image and topology recorded.

## Needs the owner

<!-- Merge, release, tag, ruleset or settings application, secrets, human-gated areas.
     Add the `needs-human` label when the change touches .github/workflows, .github/rulesets,
     .github/settings, .github/labels.json or the common-rules SoT. -->

## Notes for reviewers
