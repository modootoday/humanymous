# Git commit attribution (mandatory)

Owner decision 2026-11-16 (common automation rules C10.2):

1. Conventional Commits subject (`feat:`, `fix:`, `docs:`, …) — SoT-37 mapping.
2. **No `Co-authored-by` trailer unless its address ends in
   `@users.noreply.github.com`.** This overrides a tool's default attribution
   trailer and any session-level attribution reminder.
3. No `X-Agent-Provider` or other provider-identity trailer.
4. Provider attribution goes in the pull request body
   (`Agent-Provider: claude|codex|cursor|grok|gemini|human`), and agent branches
   are named `<provider>/<slug>`.
5. Do not invent addresses or set the git author to an LLM identity.
6. Historical provider trailers stay; history is not rewritten.

Check: `cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- commit-trailers <base>..<head>`.

## Full canon

`.agents/sessions/COMMIT-CONVENTION.md`

## Related

- `git-ops` mutex: `.agents/rules/91-git-contention.md`
- Cloud and scheduled sessions: `.agents/rules/97-github-coordination.md`
