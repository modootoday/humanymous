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

7. **Branch history (playpilot H1–H7, adopted 2026-11-19):** merge commit stays
   the merge method, so a topic branch is clean at merge time: no merge
   commits (rebase onto `codex/rust`), no `WIP`/`fixup!`/`squash!`/`amend!` or
   empty commits, generated churn in its own `chore(deps)`/`build` commit, no
   session or tool trailers. An agent may tidy **its own unmerged**
   `<provider>/<slug>` branch and push it with
   `--force-with-lease=<branch>:<expected-sha>` (narrows common rule C1.3);
   never plain `--force`, `+refspec`, or another branch. Turn auto-merge on
   right after opening a pull request; turn it off before adding commits, then
   on again. Details: `COMMIT-CONVENTION.md` §5.

Check: `cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- commit-trailers <base>..<head>`
(trailers on every commit; author/committer on commits after the cutoff).
History: the non-required `History` CI job runs `playpilot history-check --method merge`.

## Full canon

`.agents/sessions/COMMIT-CONVENTION.md`

## Related

- `git-ops` mutex: `.agents/rules/91-git-contention.md`
- Cloud and scheduled sessions: `.agents/rules/97-github-coordination.md`
