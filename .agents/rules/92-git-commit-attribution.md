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
5. **Author and committer use a GitHub noreply address** (C10.5). Agent
   sessions commit as the owner's `<id>+<login>@users.noreply.github.com`
   identity, set repo-locally (`git config user.name` / `user.email`), never
   globally. The committer may also be GitHub's web-flow `noreply@github.com`.
   Never set the author to an LLM identity or invent an address.
6. Historical provider trailers and identities stay; history is not rewritten.
   The identity check starts after commit `7e986a5`.

Check: `cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- commit-trailers <base>..<head>`
(trailers on every commit; author/committer on commits after the cutoff).

## Full canon

`.agents/sessions/COMMIT-CONVENTION.md`

## Related

- `git-ops` mutex: `.agents/rules/91-git-contention.md`
- Cloud and scheduled sessions: `.agents/rules/97-github-coordination.md`
