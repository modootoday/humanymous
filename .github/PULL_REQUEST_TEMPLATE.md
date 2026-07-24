<!--
  Thanks for contributing to humanymous!
  Security fixes must NOT go through a public PR — follow SECURITY.md (private disclosure).
-->

## Summary

<!-- What does this PR change, and why? Link any related issue (e.g. "Closes #123"). -->

## Type of change

- [ ] Bug fix
- [ ] New feature / new detection signal
- [ ] Documentation
- [ ] Tooling / CI / build
- [ ] Refactor (no behaviour change)

## Checklist

- [ ] Unit tests pass locally: `go test ./...`
- [ ] The WASM build succeeds: `GOOS=js GOARCH=wasm go build -o web/detector.wasm ./cmd/wasm/`
- [ ] **Detection catalog unaffected**: I did not change frozen upstream detection weights or thresholds; the bots-vs-engine catalog still passes (all bots blocked, no false positives) and the Gate conformance suite is green. (New signals: I opened an issue first and kept the existing catalog passing.)
- [ ] The commit subject follows [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, …).
- [ ] Docs are updated for any user-facing or behavioural change.

## Notes for reviewers

<!-- Anything reviewers should know: trade-offs, follow-ups, areas needing extra scrutiny. -->
